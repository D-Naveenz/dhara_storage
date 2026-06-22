use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use tracing::{debug, info};

use crate::definitions::{DefinitionRecord, database};
use crate::error::StorageError;

const SCAN_WINDOW_SIZE: usize = 8 * 1024;

/// High-level content classification for the scanned bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentKind {
    /// The scanned content looks like text.
    Text,
    /// The scanned content looks like binary data.
    Binary,
    /// The scanner could not determine the content kind.
    Unknown,
}

/// A ranked match produced by the legacy definitions package.
#[derive(Debug, Clone, PartialEq)]
pub struct DetectedDefinition {
    /// Human-friendly file type label.
    pub file_type_label: String,
    /// Detected MIME type.
    pub mime_type: String,
    /// Candidate filename extensions, normalized without leading dots.
    pub extensions: Vec<String>,
    /// Raw score before confidence normalization.
    pub score: u64,
    /// Confidence percentage normalized across the ranked matches.
    pub confidence: f64,
}

/// The result of analyzing a file path or stream.
#[derive(Debug, Clone, PartialEq)]
pub struct AnalysisReport {
    /// Ranked content matches ordered from most likely to least likely.
    pub matches: Vec<DetectedDefinition>,
    /// Highest-ranked MIME type after aggregating scores by MIME.
    pub top_mime_type: Option<String>,
    /// Highest-ranked detected extension after aggregating scores by extension.
    pub top_detected_extension: Option<String>,
    /// Heuristic classification of the scanned content.
    pub content_kind: ContentKind,
    /// Number of bytes examined from the file header.
    pub bytes_scanned: usize,
    /// Total file size in bytes.
    pub file_size: u64,
    /// Filename extension derived from the path or optional source name.
    pub source_extension: Option<String>,
}

impl AnalysisReport {
    /// Returns true when the report contains no ranked matches.
    pub fn is_empty(&self) -> bool {
        self.matches.is_empty()
    }
}

/// Analyze a file on disk using shared-read semantics on Windows.
pub fn analyze_path(path: impl AsRef<Path>) -> Result<AnalysisReport, StorageError> {
    let path = path.as_ref();
    info!(target: "dhara_storage::analysis", path = %path.display(), "starting path analysis");
    if !path.exists() {
        return Err(StorageError::NotFound {
            path: path.to_path_buf(),
        });
    }
    if !path.is_file() {
        return Err(StorageError::NotAFile {
            path: path.to_path_buf(),
        });
    }

    let mut file = open_analysis_file(path)?;
    let report = analyze_reader_internal(&mut file, Some(path))?;
    info!(
        target: "dhara_storage::analysis",
        path = %path.display(),
        content_kind = ?report.content_kind,
        file_size = report.file_size,
        match_count = report.matches.len(),
        top_mime_type = report.top_mime_type.as_deref().unwrap_or(""),
        "completed path analysis"
    );
    Ok(report)
}

/// Analyze an arbitrary reader/seekable source.
pub fn analyze_reader(
    mut reader: impl Read + Seek,
    source_name: Option<&Path>,
) -> Result<AnalysisReport, StorageError> {
    debug!(
        target: "dhara_storage::analysis",
        source_name = source_name.map(|path| path.display().to_string()).unwrap_or_default(),
        "starting reader analysis"
    );
    analyze_reader_internal(&mut reader, source_name)
}

fn analyze_reader_internal(
    reader: &mut (impl Read + Seek),
    source_name: Option<&Path>,
) -> Result<AnalysisReport, StorageError> {
    let file_size = reader
        .seek(SeekFrom::End(0))
        .map_err(|err| StorageError::reader_io("seek to end of", err))?;
    reader
        .seek(SeekFrom::Start(0))
        .map_err(|err| StorageError::reader_io("rewind", err))?;

    let source_extension = source_name.and_then(normalized_extension);
    if file_size == 0 {
        return Ok(AnalysisReport {
            matches: Vec::new(),
            top_mime_type: None,
            top_detected_extension: None,
            content_kind: ContentKind::Unknown,
            bytes_scanned: 0,
            file_size,
            source_extension,
        });
    }

    let bytes_to_scan = file_size.min(SCAN_WINDOW_SIZE as u64) as usize;
    let mut header = vec![0_u8; bytes_to_scan];
    reader
        .read_exact(&mut header)
        .map_err(|err| StorageError::reader_io("read header from", err))?;
    let header = trim_trailing_null_bytes(header);
    let heuristic_content_kind = detect_content_kind(&header);

    let db = database()?;
    let candidate_indices = db.candidate_indices(&header);
    debug!(
        target: "dhara_storage::analysis",
        file_size,
        bytes_scanned = header.len(),
        candidate_count = candidate_indices.len(),
        "resolved definition candidates"
    );

    let mut full_buffer: Option<Vec<u8>> = None;
    let mut ranked_matches = Vec::new();

    for definition_idx in candidate_indices {
        let definition = db.definition(definition_idx);
        let score = score_definition(definition, &header, reader, &mut full_buffer)?;
        if score == 0 {
            continue;
        }

        ranked_matches.push(MatchedDefinition::from_legacy(definition, score));
    }

    if ranked_matches.is_empty() {
        ranked_matches.push(MatchedDefinition::fallback(
            source_extension.as_deref(),
            heuristic_content_kind,
        ));
    }

    ranked_matches.sort_by(compare_matches);

    let total_score = ranked_matches
        .iter()
        .map(|item| item.score)
        .sum::<u64>()
        .max(1);
    let matches = ranked_matches
        .iter()
        .map(|item| DetectedDefinition {
            file_type_label: item.file_type_label.clone(),
            mime_type: item.mime_type.clone(),
            extensions: item.extensions.clone(),
            score: item.score,
            confidence: item.score as f64 / total_score as f64 * 100.0,
        })
        .collect::<Vec<_>>();

    let top_mime_type = aggregate_top_value(
        ranked_matches
            .iter()
            .map(|item| (item.mime_type.clone(), item.score)),
    );
    let top_detected_extension = aggregate_top_value(ranked_matches.iter().flat_map(|item| {
        item.extensions
            .iter()
            .cloned()
            .map(move |ext| (ext, item.score))
    }));
    let content_kind = resolved_content_kind(heuristic_content_kind, top_mime_type.as_deref());
    debug!(
        target: "dhara_storage::analysis",
        content_kind = ?content_kind,
        match_count = matches.len(),
        "analysis report ready"
    );

    Ok(AnalysisReport {
        matches,
        top_mime_type,
        top_detected_extension,
        content_kind,
        bytes_scanned: header.len(),
        file_size,
        source_extension,
    })
}

#[derive(Debug, Clone)]
struct MatchedDefinition {
    file_type_label: String,
    mime_type: String,
    extensions: Vec<String>,
    priority_level: i32,
    score: u64,
}

impl MatchedDefinition {
    fn from_legacy(definition: &DefinitionRecord, score: u64) -> Self {
        Self {
            file_type_label: definition.file_type.clone(),
            mime_type: definition.mime_type.clone(),
            extensions: normalize_extensions(&definition.extensions),
            priority_level: definition.priority_level,
            score,
        }
    }

    fn fallback(source_extension: Option<&str>, content_kind: ContentKind) -> Self {
        let (file_type_label, mime_type, default_extension) = match content_kind {
            ContentKind::Text => ("Plain Text", "text/plain", "txt"),
            ContentKind::Binary => ("Binary Data", "application/octet-stream", "bin"),
            ContentKind::Unknown => ("Unknown", "application/octet-stream", "bin"),
        };

        let extension = source_extension
            .filter(|value| !value.is_empty())
            .unwrap_or(default_extension)
            .to_owned();

        Self {
            file_type_label: file_type_label.to_owned(),
            mime_type: mime_type.to_owned(),
            extensions: vec![extension],
            priority_level: -1000,
            score: 100,
        }
    }
}

fn compare_matches(left: &MatchedDefinition, right: &MatchedDefinition) -> Ordering {
    right
        .score
        .cmp(&left.score)
        .then_with(|| right.priority_level.cmp(&left.priority_level))
        .then_with(|| left.file_type_label.cmp(&right.file_type_label))
        .then_with(|| left.mime_type.cmp(&right.mime_type))
}

fn score_definition(
    definition: &DefinitionRecord,
    header: &[u8],
    reader: &mut (impl Read + Seek),
    full_buffer: &mut Option<Vec<u8>>,
) -> Result<u64, StorageError> {
    let mut score = 0_u64;

    for pattern in &definition.signature.patterns {
        let position = pattern.position as usize;
        if !matches_pattern(header, position, &pattern.data) {
            return Ok(0);
        }

        let weight = if position == 0 { 1000 } else { 100 };
        score += pattern.data.len() as u64 * weight;
    }

    if score == 0 {
        return Ok(0);
    }

    if definition.signature.strings.is_empty() {
        return Ok(score);
    }

    let buffer = load_full_buffer(reader, full_buffer)?;
    for string_pattern in &definition.signature.strings {
        if contains_sequence(buffer, string_pattern) {
            score += string_pattern.len() as u64 * 10;
        } else {
            return Ok(0);
        }
    }

    Ok(score)
}

fn load_full_buffer<'a>(
    reader: &mut (impl Read + Seek),
    full_buffer: &'a mut Option<Vec<u8>>,
) -> Result<&'a [u8], StorageError> {
    if full_buffer.is_none() {
        reader
            .seek(SeekFrom::Start(0))
            .map_err(|err| StorageError::reader_io("rewind", err))?;
        let mut buffer = Vec::new();
        reader
            .read_to_end(&mut buffer)
            .map_err(|err| StorageError::reader_io("read full contents from", err))?;
        *full_buffer = Some(buffer);
    }

    Ok(full_buffer.as_deref().unwrap_or(&[]))
}

fn matches_pattern(buffer: &[u8], offset: usize, pattern: &[u8]) -> bool {
    !pattern.is_empty()
        && offset
            .checked_add(pattern.len())
            .is_some_and(|end| end <= buffer.len())
        && buffer[offset..offset + pattern.len()] == *pattern
}

fn contains_sequence(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
}

fn trim_trailing_null_bytes(mut buffer: Vec<u8>) -> Vec<u8> {
    while buffer.last().is_some_and(|value| *value == 0) {
        buffer.pop();
    }
    buffer
}

fn normalized_extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.trim().trim_start_matches('.').to_ascii_lowercase())
        .filter(|ext| !ext.is_empty())
}

fn normalize_extensions(extensions: &[String]) -> Vec<String> {
    extensions
        .iter()
        .map(|ext| ext.trim().trim_start_matches('.').to_ascii_lowercase())
        .filter(|ext| !ext.is_empty())
        .collect()
}

fn aggregate_top_value(values: impl Iterator<Item = (String, u64)>) -> Option<String> {
    values
        .fold(HashMap::<String, u64>::new(), |mut acc, (value, score)| {
            *acc.entry(value).or_default() += score;
            acc
        })
        .into_iter()
        .max_by(|left, right| left.1.cmp(&right.1).then_with(|| right.0.cmp(&left.0)))
        .map(|(value, _)| value)
}

fn detect_content_kind(buffer: &[u8]) -> ContentKind {
    if buffer.is_empty() {
        return ContentKind::Unknown;
    }
    if is_text_content(buffer) {
        ContentKind::Text
    } else {
        ContentKind::Binary
    }
}

fn resolved_content_kind(
    heuristic_content_kind: ContentKind,
    top_mime_type: Option<&str>,
) -> ContentKind {
    match top_mime_type {
        Some(mime) if mime_looks_textual(mime) => ContentKind::Text,
        Some(_) => ContentKind::Binary,
        None => heuristic_content_kind,
    }
}

fn mime_looks_textual(mime: &str) -> bool {
    mime.starts_with("text/")
        || mime == "application/json"
        || mime == "application/xml"
        || mime.ends_with("+json")
        || mime.ends_with("+xml")
}

fn is_text_content(buffer: &[u8]) -> bool {
    if buffer.is_empty() {
        return false;
    }
    if try_detect_bom(buffer).is_some() {
        return true;
    }

    let mut null_bytes = 0_usize;
    let mut control_chars = 0_usize;
    let mut printable_chars = 0_usize;

    for &byte in buffer {
        match byte {
            0x00 => null_bytes += 1,
            0x09 | 0x0A | 0x0D => printable_chars += 1,
            0x20..=0x7E => printable_chars += 1,
            0x01..=0x08 | 0x0B..=0x0C | 0x0E..=0x1F | 0x7F => control_chars += 1,
            _ => {}
        }
    }

    let null_percentage = null_bytes as f64 / buffer.len() as f64 * 100.0;
    if null_percentage > 1.0 {
        return false;
    }
    if control_chars > printable_chars / 2 {
        return false;
    }

    if std::str::from_utf8(buffer).is_ok() {
        return true;
    }

    let printable_percentage = printable_chars as f64 / buffer.len() as f64 * 100.0;
    printable_percentage > 75.0
}

fn try_detect_bom(buffer: &[u8]) -> Option<&'static str> {
    match buffer {
        [0x00, 0x00, 0xFE, 0xFF, ..] => Some("utf-32be"),
        [0xFF, 0xFE, 0x00, 0x00, ..] => Some("utf-32le"),
        [0xEF, 0xBB, 0xBF, ..] => Some("utf-8"),
        [0xFE, 0xFF, ..] => Some("utf-16be"),
        [0xFF, 0xFE, ..] => Some("utf-16le"),
        _ => None,
    }
}

#[cfg(windows)]
fn open_analysis_file(path: &Path) -> Result<File, StorageError> {
    use std::fs::OpenOptions;
    use std::os::windows::fs::OpenOptionsExt;

    const FILE_SHARE_READ: u32 = 0x0000_0001;
    const FILE_SHARE_WRITE: u32 = 0x0000_0002;
    const FILE_SHARE_DELETE: u32 = 0x0000_0004;

    OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .open(path)
        .map_err(|err| StorageError::io("open", path.to_path_buf(), err))
}

#[cfg(not(windows))]
fn open_analysis_file(path: &Path) -> Result<File, StorageError> {
    File::open(path).map_err(|err| StorageError::io("open", path.to_path_buf(), err))
}
