use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use rayon::prelude::*;
use sevenz_rust::{Archive, default_entry_extract_fn, decompress_file_with_extract_fn};
use tempfile::{TempDir, tempdir};
use tracing::debug;

use crate::filedefs::BuilderError;

use super::{
    ParsedTridDefinition, TridBuildProgress, TridBuildStage, TridBuildStats,
    model::parse_trid_xml_definition,
};

const PARALLEL_PARSE_THRESHOLD: usize = 8;
const EXTRACT_PROGRESS_INTERVAL: usize = 50;

pub(crate) fn load_trid_definitions(
    source: &Path,
    progress: &mut dyn FnMut(TridBuildProgress),
) -> Result<Vec<ParsedTridDefinition>, BuilderError> {
    debug!(source = %source.display(), "loading TrID XML source");
    if source.is_dir() {
        return load_from_directory(source, progress);
    }

    if is_xml_file(source) {
        return load_single_xml_file(source);
    }

    if is_7z_file(source) {
        return load_from_archive(source, progress);
    }

    Err(BuilderError::UnsupportedSource {
        path: source.to_path_buf(),
    })
}

fn load_single_xml_file(source: &Path) -> Result<Vec<ParsedTridDefinition>, BuilderError> {
    debug!(source = %source.display(), "reading single TrID XML file");
    let xml = fs::read_to_string(source).map_err(|error| BuilderError::Io {
        operation: "read TrID XML source",
        path: source.to_path_buf(),
        source: error,
    })?;
    let definition = parse_trid_xml_definition(&xml, source)?;
    Ok(vec![definition])
}

fn load_from_directory(
    source: &Path,
    progress: &mut dyn FnMut(TridBuildProgress),
) -> Result<Vec<ParsedTridDefinition>, BuilderError> {
    debug!(source = %source.display(), "enumerating TrID XML directory");
    let mut xml_files = Vec::new();
    collect_xml_files(source, &mut xml_files)?;
    xml_files.sort();
    let total_files = xml_files.len();
    let started = Instant::now();

    progress(TridBuildProgress {
        stage: TridBuildStage::ParseDefinitions,
        message: format!("Reading definition files ({total_files} found)"),
        current: 0,
        total: Some(total_files),
        current_item: None,
        stats: TridBuildStats::default(),
        trace_detail: None,
    });

    let definitions = if total_files <= PARALLEL_PARSE_THRESHOLD {
        parse_definitions_sequential(&xml_files, total_files, progress)?
    } else {
        parse_definitions_parallel(&xml_files, total_files, progress)?
    };

    let duration = format_elapsed(started.elapsed());
    progress(TridBuildProgress {
        stage: TridBuildStage::ParseDefinitions,
        message: format!("Parsed {total_files} definitions in {duration}"),
        current: total_files,
        total: Some(total_files),
        current_item: None,
        stats: TridBuildStats {
            parsed_count: total_files,
            ..TridBuildStats::default()
        },
        trace_detail: None,
    });

    debug!(
        count = definitions.len(),
        "loaded TrID XML definitions from directory"
    );
    Ok(definitions)
}

fn parse_definitions_sequential(
    xml_files: &[PathBuf],
    total_files: usize,
    progress: &mut dyn FnMut(TridBuildProgress),
) -> Result<Vec<ParsedTridDefinition>, BuilderError> {
    let mut definitions = Vec::with_capacity(total_files);
    for (index, xml_file) in xml_files.iter().enumerate() {
        definitions.push(parse_xml_file(xml_file)?);
        report_parse_progress(progress, index + 1, total_files);
    }
    Ok(definitions)
}

fn parse_definitions_parallel(
    xml_files: &[PathBuf],
    total_files: usize,
    _progress: &mut dyn FnMut(TridBuildProgress),
) -> Result<Vec<ParsedTridDefinition>, BuilderError> {
    let done = AtomicUsize::new(0);
    xml_files
        .par_iter()
        .map(|xml_file| {
            let definition = parse_xml_file(xml_file)?;
            let completed = done.fetch_add(1, Ordering::Relaxed) + 1;
            report_parse_progress_parallel(completed, total_files);
            Ok(definition)
        })
        .collect()
}

fn report_parse_progress_parallel(done: usize, total: usize) {
    if done != total && done != 1 && !done.is_multiple_of(250) {
        return;
    }
    crate::logging::emit_trid_progress(TridBuildProgress {
        stage: TridBuildStage::ParseDefinitions,
        message: "Parsing XML definitions".to_string(),
        current: done,
        total: Some(total),
        current_item: None,
        stats: TridBuildStats {
            parsed_count: done,
            ..TridBuildStats::default()
        },
        trace_detail: None,
    });
}

fn parse_xml_file(xml_file: &Path) -> Result<ParsedTridDefinition, BuilderError> {
    let xml = fs::read_to_string(xml_file).map_err(|error| BuilderError::Io {
        operation: "read TrID XML source",
        path: xml_file.to_path_buf(),
        source: error,
    })?;
    parse_trid_xml_definition(&xml, xml_file)
}

fn report_parse_progress(progress: &mut dyn FnMut(TridBuildProgress), done: usize, total: usize) {
    if done != total && done != 1 && !done.is_multiple_of(250) {
        return;
    }
    progress(TridBuildProgress {
        stage: TridBuildStage::ParseDefinitions,
        message: "Parsing XML definitions".to_string(),
        current: done,
        total: Some(total),
        current_item: None,
        stats: TridBuildStats {
            parsed_count: done,
            ..TridBuildStats::default()
        },
        trace_detail: None,
    });
}

fn format_elapsed(duration: std::time::Duration) -> String {
    let secs = duration.as_secs_f64();
    if secs >= 1.0 {
        format!("{:.1}s", secs)
    } else {
        format!("{}ms", duration.as_millis())
    }
}

fn load_from_archive(
    source: &Path,
    progress: &mut dyn FnMut(TridBuildProgress),
) -> Result<Vec<ParsedTridDefinition>, BuilderError> {
    debug!(source = %source.display(), "extracting TrID XML archive");
    progress(TridBuildProgress {
        stage: TridBuildStage::ExtractArchive,
        message: format!("Analyzing archive {}", source.display()),
        current: 0,
        total: None,
        current_item: Some(source.display().to_string()),
        stats: TridBuildStats::default(),
        trace_detail: None,
    });
    let extraction_dir = extract_archive(source, progress)?;
    load_from_directory(extraction_dir.path(), progress)
}

fn extract_archive(
    source: &Path,
    progress: &mut dyn FnMut(TridBuildProgress),
) -> Result<TempDir, BuilderError> {
    match extract_archive_sevenz(source, progress) {
        Ok(temp) => Ok(temp),
        Err(sevenz_error) => {
            debug!(
                source = %source.display(),
                error = %sevenz_error,
                "sevenz extraction failed; falling back to tar"
            );
            extract_archive_tar(source, progress)
        }
    }
}

fn count_archive_entries(source: &Path) -> Result<usize, BuilderError> {
    let archive = Archive::open(source).map_err(|error| BuilderError::ArchiveCommand {
        operation: "analyze",
        path: source.to_path_buf(),
        message: error.to_string(),
    })?;
    Ok(archive
        .files
        .iter()
        .filter(|entry| !entry.is_anti_item)
        .count())
}

fn extract_archive_sevenz(
    source: &Path,
    progress: &mut dyn FnMut(TridBuildProgress),
) -> Result<TempDir, BuilderError> {
    let temp = tempdir().map_err(|error| BuilderError::Io {
        operation: "create temporary extraction directory for",
        path: std::env::temp_dir(),
        source: error,
    })?;

    let total_entries = count_archive_entries(source)?;
    let total = total_entries.max(1);
    report_extract_progress(progress, 0, total, source);

    let mut completed = 0usize;
    decompress_file_with_extract_fn(source, temp.path(), |entry, reader, dest| {
        default_entry_extract_fn(entry, reader, dest)?;
        completed += 1;
        report_extract_progress(progress, completed, total, source);
        Ok(true)
    })
    .map_err(|error| BuilderError::ArchiveCommand {
        operation: "extract",
        path: source.to_path_buf(),
        message: error.to_string(),
    })?;

    progress(TridBuildProgress {
        stage: TridBuildStage::ExtractArchive,
        message: "extracted archive".to_owned(),
        current: total,
        total: Some(total),
        current_item: None,
        stats: TridBuildStats::default(),
        trace_detail: None,
    });
    debug!(
        path = %source.display(),
        destination = %temp.path().display(),
        entries = total_entries,
        "archive extracted with sevenz-rust"
    );
    Ok(temp)
}

fn extract_archive_tar(
    source: &Path,
    progress: &mut dyn FnMut(TridBuildProgress),
) -> Result<TempDir, BuilderError> {
    let temp = tempdir().map_err(|error| BuilderError::Io {
        operation: "create temporary extraction directory for",
        path: std::env::temp_dir(),
        source: error,
    })?;

    report_extract_progress(progress, 0, 1, source);

    let output = Command::new("tar")
        .arg("-xf")
        .arg(source)
        .arg("-C")
        .arg(temp.path())
        .output()
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                BuilderError::ArchiveToolUnavailable { tool: "tar" }
            } else {
                BuilderError::ArchiveCommand {
                    operation: "extract",
                    path: source.to_path_buf(),
                    message: error.to_string(),
                }
            }
        })?;

    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(BuilderError::ArchiveCommand {
            operation: "extract",
            path: source.to_path_buf(),
            message,
        });
    }

    report_extract_progress(progress, 1, 1, source);
    progress(TridBuildProgress {
        stage: TridBuildStage::ExtractArchive,
        message: "extracted archive".to_owned(),
        current: 1,
        total: Some(1),
        current_item: None,
        stats: TridBuildStats::default(),
        trace_detail: None,
    });
    debug!(path = %source.display(), destination = %temp.path().display(), "archive extracted with tar");
    Ok(temp)
}

fn report_extract_progress(
    progress: &mut dyn FnMut(TridBuildProgress),
    completed: usize,
    total: usize,
    source: &Path,
) {
    if completed != 0
        && completed != total
        && completed != 1
        && !completed.is_multiple_of(EXTRACT_PROGRESS_INTERVAL)
    {
        return;
    }
    progress(TridBuildProgress {
        stage: TridBuildStage::ExtractArchive,
        message: if total > 1 {
            format!("Extracting archive ({completed}/{total})")
        } else {
            format!("Extracting {}", source.display())
        },
        current: completed,
        total: Some(total),
        current_item: None,
        stats: TridBuildStats::default(),
        trace_detail: None,
    });
}

fn collect_xml_files(root: &Path, xml_files: &mut Vec<PathBuf>) -> Result<(), BuilderError> {
    for entry in fs::read_dir(root).map_err(|error| BuilderError::Io {
        operation: "enumerate TrID XML directory",
        path: root.to_path_buf(),
        source: error,
    })? {
        let entry = entry.map_err(|error| BuilderError::Io {
            operation: "read TrID XML directory entry",
            path: root.to_path_buf(),
            source: error,
        })?;
        let path = entry.path();
        if path.is_dir() {
            collect_xml_files(&path, xml_files)?;
        } else if is_xml_file(&path) {
            xml_files.push(path);
        }
    }

    Ok(())
}

fn is_xml_file(path: &Path) -> bool {
    path.extension()
        .and_then(OsStr::to_str)
        .is_some_and(|extension| extension.eq_ignore_ascii_case("xml"))
}

fn is_7z_file(path: &Path) -> bool {
    path.extension()
        .and_then(OsStr::to_str)
        .is_some_and(|extension| extension.eq_ignore_ascii_case("7z"))
}
