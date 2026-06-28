use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use rayon::prelude::*;
use tempfile::{TempDir, tempdir};
use tracing::debug;

use crate::filedefs::BuilderError;

use super::{
    ParsedTridDefinition, TridBuildProgress, TridBuildStage, TridBuildStats,
    model::parse_trid_xml_definition,
};

const PARALLEL_PARSE_THRESHOLD: usize = 8;

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
        message: "Parsing XML definitions".to_string(),
        current: 0,
        total: Some(total_files),
        current_item: None,
        stats: TridBuildStats::default(),
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
    _total_files: usize,
    _progress: &mut dyn FnMut(TridBuildProgress),
) -> Result<Vec<ParsedTridDefinition>, BuilderError> {
    xml_files
        .par_iter()
        .map(|xml_file| parse_xml_file(xml_file))
        .collect()
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
        message: format!("Extracting {}", source.display()),
        current: 0,
        total: None,
        current_item: Some(source.display().to_string()),
        stats: TridBuildStats::default(),
    });
    let extraction_dir = extract_archive(source)?;
    load_from_directory(extraction_dir.path(), progress)
}

fn extract_archive(source: &Path) -> Result<TempDir, BuilderError> {
    let temp = tempdir().map_err(|error| BuilderError::Io {
        operation: "create temporary extraction directory for",
        path: std::env::temp_dir(),
        source: error,
    })?;

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

    debug!(path = %source.display(), destination = %temp.path().display(), "archive extracted successfully");
    Ok(temp)
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
