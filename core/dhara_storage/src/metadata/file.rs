//! [`FileMetadata`] — on-demand file metadata snapshot.
//!
//! Maps roughly to Explorer's Details tab (type, dates, attributes) with Dhara
//! extensions: content-based [`StorageType`] / [`FileExtension`] when enriched from
//! a prior [`crate::storage::FileStorage::analyze`]. Size and paths live on
//! [`crate::storage::FileStorage`].

use std::fmt;
use std::path::{Path, PathBuf};

use once_cell::sync::OnceCell;

use crate::analysis::AnalysisReport;
use crate::error::StorageError;

use super::attributes::StorageAttributes;
use super::permissions::StoragePermissions;
use super::shell_icon::{DEFAULT_SHELL_ICON_SIZE, ShellIcon, load_shell_icon};
use super::traits::{CommonFields, StorageMetadata};

/// Human type label plus optional MIME type.
///
/// After enrichment from [`crate::storage::FileStorage::analyze`], both fields typically
/// come from the analysis report. Before analysis, `name` falls back to an
/// extension-based label; `mime_type` stays empty until analysis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageType {
    /// Human-friendly type label (for example `"PNG Image"` or `"JSON Source File"`).
    pub name: String,
    /// MIME type when known (for example `Some("image/png")`).
    pub mime_type: Option<String>,
}

/// Path extension versus content-detected extension.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FileExtension {
    source: Option<String>,
    detected: Option<String>,
}

impl FileExtension {
    /// Extension from the file name (no leading dot), resolved from the storage path.
    pub fn source(&self) -> Option<&str> {
        self.source.as_deref()
    }

    /// Extension detected by content analysis, when the metadata was enriched from a report.
    pub fn detected(&self) -> Option<&str> {
        self.detected.as_deref()
    }

    fn from_path(path: &Path) -> Self {
        Self {
            source: normalized_extension(path),
            detected: None,
        }
    }

    fn with_detected(mut self, detected: Option<String>) -> Self {
        self.detected = detected;
        self
    }
}

impl fmt::Display for FileExtension {
    /// Formats as `JPEG`, or `JPEG (PNG)` when source and detected differ.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let source = self.source.as_deref().map(uppercase_ext);
        let detected = self.detected.as_deref().map(uppercase_ext);
        match (source, detected) {
            (None, None) => Ok(()),
            (Some(only), None) | (None, Some(only)) => write!(f, "{only}"),
            (Some(src), Some(det)) if src == det => write!(f, "{src}"),
            (Some(src), Some(det)) => write!(f, "{src} ({det})"),
        }
    }
}

fn uppercase_ext(value: &str) -> String {
    value.to_ascii_uppercase()
}

fn normalized_extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.trim().trim_start_matches('.').to_ascii_lowercase())
        .filter(|ext| !ext.is_empty())
}

/// File metadata snapshot, optionally enriched from a prior content analysis.
#[derive(Debug)]
pub struct FileMetadata {
    /// Private absolute path used for on-demand loads (not part of the public metadata API).
    absolute_path: PathBuf,
    common: CommonFields,
    extension: FileExtension,
    analysis: Option<AnalysisReport>,
    shell_icon: OnceCell<Option<ShellIcon>>,
}

impl FileMetadata {
    /// Load file metadata for an absolute path without content analysis.
    pub fn load(absolute_path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let absolute_path = absolute_path.as_ref().to_path_buf();
        let (common, fs_metadata) = CommonFields::load(&absolute_path)?;
        let is_file = fs_metadata.is_file()
            || (fs_metadata.file_type().is_symlink() && absolute_path.is_file());
        if !is_file {
            return Err(StorageError::NotAFile {
                path: absolute_path,
            });
        }

        Ok(Self {
            extension: FileExtension::from_path(&absolute_path),
            absolute_path,
            common,
            analysis: None,
            shell_icon: OnceCell::new(),
        })
    }

    /// Apply a content-analysis report, updating type/extension derived views.
    pub(crate) fn apply_analysis(&mut self, report: AnalysisReport) {
        self.extension = FileExtension::from_path(&self.absolute_path)
            .with_detected(report.top_detected_extension.clone());
        self.analysis = Some(report);
    }

    /// File content/identity type (analysis-aware when a report is stored).
    pub fn file_type(&self) -> StorageType {
        if let Some(report) = self.analysis.as_ref() {
            let name = report
                .matches
                .first()
                .map(|item| item.file_type_label.clone())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| self.fallback_type_name());
            return StorageType {
                name,
                mime_type: report.top_mime_type.clone(),
            };
        }

        StorageType {
            name: self.fallback_type_name(),
            mime_type: None,
        }
    }

    /// Source and (after enrichment) detected extensions.
    pub fn extension(&self) -> FileExtension {
        self.extension.clone()
    }

    /// Returns a previously applied analysis report, if any.
    pub fn analysis(&self) -> Option<&AnalysisReport> {
        self.analysis.as_ref()
    }

    fn fallback_type_name(&self) -> String {
        self.extension
            .source()
            .map(|ext| format!("{} File", ext.to_ascii_uppercase()))
            .unwrap_or_else(|| "File".to_owned())
    }
}

impl StorageMetadata for FileMetadata {
    fn name(&self) -> &str {
        &self.common.name
    }

    fn created_at(&self) -> Option<std::time::SystemTime> {
        self.common.created_at
    }

    fn modified_at(&self) -> Option<std::time::SystemTime> {
        self.common.modified_at
    }

    fn accessed_at(&self) -> Option<std::time::SystemTime> {
        self.common.accessed_at
    }

    fn attributes(&self) -> StorageAttributes {
        self.common.attributes
    }

    fn permissions(&self) -> StoragePermissions {
        self.common.permissions
    }

    fn is_symbolic_link(&self) -> bool {
        self.common.is_symbolic_link
    }

    fn link_target(&self) -> Option<&Path> {
        self.common.link_target.as_deref()
    }

    fn is_temporary(&self) -> bool {
        self.common.is_temporary
    }

    fn icon(&self) -> Option<&ShellIcon> {
        self.shell_icon
            .get_or_init(|| load_shell_icon(&self.absolute_path, DEFAULT_SHELL_ICON_SIZE))
            .as_ref()
    }

    fn load_icon_at(&self, size: u32) -> Option<ShellIcon> {
        load_shell_icon(&self.absolute_path, size)
    }
}
