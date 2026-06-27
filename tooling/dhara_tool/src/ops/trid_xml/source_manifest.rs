use std::path::{Path, PathBuf};

use chrono::NaiveDate;
use serde::Deserialize;
use tracing::debug;

use crate::ops::builder::BuilderError;

#[derive(Debug, Deserialize)]
struct SourceManifest {
    definitions_release: String,
}

/// Resolve the sidecar manifest path for a TrID source archive or directory.
///
/// Uses `{stem}.source.toml` beside the source path.
pub(crate) fn sidecar_path_for_source(source: &Path) -> PathBuf {
    let parent = source.parent().unwrap_or_else(|| Path::new("."));
    let stem = source
        .file_stem()
        .or_else(|| source.file_name())
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| "filedefs".to_owned());
    parent.join(format!("{stem}.source.toml"))
}

/// Load and normalize the upstream definitions release date from a sidecar manifest.
pub(crate) fn load_definitions_release(source: &Path) -> Result<String, BuilderError> {
    let manifest_path = sidecar_path_for_source(source);
    debug!(
        source = %source.display(),
        manifest = %manifest_path.display(),
        "loading TrID source manifest"
    );

    let contents = std::fs::read_to_string(&manifest_path).map_err(|source_error| {
        if source_error.kind() == std::io::ErrorKind::NotFound {
            BuilderError::MissingDefinitionsRelease {
                path: manifest_path.clone(),
            }
        } else {
            BuilderError::Io {
                operation: "read source manifest",
                path: manifest_path.clone(),
                source: source_error,
            }
        }
    })?;

    let manifest: SourceManifest =
        toml::from_str(&contents).map_err(|error| BuilderError::InvalidDefinitionsRelease {
            path: manifest_path.clone(),
            message: error.to_string(),
        })?;

    normalize_definitions_release(&manifest.definitions_release).map_err(|message| {
        BuilderError::InvalidDefinitionsRelease {
            path: manifest_path,
            message,
        }
    })
}

fn normalize_definitions_release(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("definitions_release must not be empty".to_owned());
    }

    if let Ok(date) = NaiveDate::parse_from_str(trimmed, "%Y-%m-%d") {
        return Ok(date.format("%Y-%m-%d").to_string());
    }

    if let Ok(date) = NaiveDate::parse_from_str(trimmed, "%d/%m/%Y") {
        return Ok(date.format("%Y-%m-%d").to_string());
    }

    Err(format!(
        "definitions_release '{trimmed}' must use YYYY-MM-DD or DD/MM/YYYY"
    ))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{normalize_definitions_release, sidecar_path_for_source};

    #[test]
    fn sidecar_path_uses_source_stem() {
        let path = sidecar_path_for_source(Path::new("package/triddefs_xml.7z"));
        assert_eq!(path, Path::new("package/triddefs_xml.source.toml"));
    }

    #[test]
    fn normalize_accepts_iso_and_european_dates() {
        assert_eq!(
            normalize_definitions_release("2026-06-24").expect("iso date"),
            "2026-06-24"
        );
        assert_eq!(
            normalize_definitions_release("24/06/2026").expect("european date"),
            "2026-06-24"
        );
    }
}
