use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::{TempDir, tempdir};
use tracing::{debug, info};

use crate::builder::BuilderError;

use super::{
    ParsedTridDefinition, TridBuildProgress, TridBuildStage, TridBuildStats,
    model::parse_trid_xml_definition,
};

pub(crate) fn load_trid_definitions(
    source: &Path,
    progress: &mut dyn FnMut(TridBuildProgress),
) -> Result<Vec<ParsedTridDefinition>, BuilderError> {
    info!(source = %source.display(), "loading TrID XML source");
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
    progress(TridBuildProgress {
        stage: TridBuildStage::ParseDefinitions,
        message: "Parsing XML definitions".to_string(),
        current: 0,
        total: Some(xml_files.len()),
        current_item: None,
        stats: TridBuildStats::default(),
    });

    let total_files = xml_files.len();
    let mut definitions = Vec::with_capacity(total_files);
    for (index, xml_file) in xml_files.into_iter().enumerate() {
        let xml = fs::read_to_string(&xml_file).map_err(|error| BuilderError::Io {
            operation: "read TrID XML source",
            path: xml_file.clone(),
            source: error,
        })?;
        definitions.push(parse_trid_xml_definition(&xml, &xml_file)?);
        progress(TridBuildProgress {
            stage: TridBuildStage::ParseDefinitions,
            message: "Parsing XML definition".to_string(),
            current: index + 1,
            total: Some(total_files),
            current_item: xml_file
                .file_name()
                .map(|name| name.to_string_lossy().to_string()),
            stats: TridBuildStats {
                parsed_count: index + 1,
                ..TridBuildStats::default()
            },
        });
    }
    info!(
        count = definitions.len(),
        "loaded TrID XML definitions from directory"
    );
    Ok(definitions)
}

fn load_from_archive(
    source: &Path,
    progress: &mut dyn FnMut(TridBuildProgress),
) -> Result<Vec<ParsedTridDefinition>, BuilderError> {
    info!(source = %source.display(), "extracting TrID XML archive");
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
