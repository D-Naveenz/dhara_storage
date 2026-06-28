use std::path::Path;
use std::thread;

use once_cell::sync::OnceCell;
use tracing::{debug, info};

use crate::analysis::{AnalysisReport, ContentKind, DetectedDefinition, analyze_path};
use crate::error::StorageError;

use super::common::{StorageMetadata, format_size};
use super::windows::{WindowsShellDetails, WindowsShellIcon, load_shell_details, load_shell_icon};

/// Immutable file metadata with lazy, cached content analysis.
#[derive(Debug)]
pub struct FileInfo {
    metadata: StorageMetadata,
    size: u64,
    filename_extension: Option<String>,
    analysis: OnceCell<AnalysisReport>,
    shell_details: OnceCell<Option<WindowsShellDetails>>,
    shell_icon: OnceCell<Option<WindowsShellIcon>>,
}

impl FileInfo {
    /// Load basic file metadata without running content analysis.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        debug!(
            target: "dhara_storage::info::file",
            path = %path.as_ref().display(),
            "loading file metadata"
        );
        let (metadata, fs_metadata) = StorageMetadata::from_path(path)?;
        if !fs_metadata.is_file() {
            return Err(StorageError::NotAFile {
                path: metadata.path().to_path_buf(),
            });
        }

        Ok(Self {
            filename_extension: metadata
                .path()
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.trim().trim_start_matches('.').to_ascii_lowercase())
                .filter(|ext| !ext.is_empty()),
            size: fs_metadata.len(),
            metadata,
            analysis: OnceCell::new(),
            shell_details: OnceCell::new(),
            shell_icon: OnceCell::new(),
        })
    }

    /// Load basic file metadata while precomputing content analysis in parallel.
    pub fn from_path_with_analysis(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let owned_path = path.as_ref().to_path_buf();
        info!(
            target: "dhara_storage::info::file",
            path = %owned_path.display(),
            "loading file metadata with eager analysis"
        );

        thread::scope(|scope| {
            let analysis_handle = scope.spawn(|| analyze_path(&owned_path));
            let info = Self::from_path(&owned_path)?;
            let analysis = analysis_handle
                .join()
                .expect("analysis preload thread panicked")?;
            let _ = info.analysis.set(analysis);
            Ok(info)
        })
    }

    /// Returns the shared storage metadata.
    pub fn metadata(&self) -> &StorageMetadata {
        &self.metadata
    }

    /// Absolute path to the file on disk.
    pub fn path(&self) -> &Path {
        self.metadata.path()
    }

    /// File name including extension.
    pub fn name(&self) -> &str {
        self.metadata.name()
    }

    /// Human-friendly display name, typically the file stem.
    pub fn display_name(&self) -> &str {
        self.path()
            .file_stem()
            .and_then(|value| value.to_str())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| self.name())
    }

    /// File type label with performance-aware priority:
    /// cached analysis first, then lazy Windows shell data, then a cheap fallback.
    pub fn type_name(&self) -> String {
        if let Some(analysis) = self.analysis_if_loaded()
            && let Some(found) = analysis
                .matches
                .first()
                .map(|item| item.file_type_label.as_str())
                .filter(|value| !value.is_empty())
        {
            return found.to_owned();
        }

        if let Some(shell) = self.shell_details()
            && let Some(found) = shell.type_name.as_deref().filter(|value| !value.is_empty())
        {
            return found.to_owned();
        }

        self.filename_extension()
            .map(|ext| format!("{} File", ext.to_ascii_uppercase()))
            .unwrap_or_else(|| "File".to_owned())
    }

    /// File size in bytes.
    pub fn size(&self) -> u64 {
        self.size
    }

    /// Formatted file size.
    pub fn formatted_size(&self) -> String {
        format_size(self.size, None)
    }

    /// Filename extension normalized without a leading dot.
    pub fn filename_extension(&self) -> Option<&str> {
        self.filename_extension.as_deref()
    }

    /// Lazily compute and cache content analysis.
    pub fn analysis(&self) -> Result<&AnalysisReport, StorageError> {
        debug!(
            target: "dhara_storage::info::file",
            path = %self.path().display(),
            "loading file analysis on demand"
        );
        self.analysis.get_or_try_init(|| analyze_path(self.path()))
    }

    /// Returns a cached analysis if it has already been computed.
    pub fn analysis_if_loaded(&self) -> Option<&AnalysisReport> {
        self.analysis.get()
    }

    /// Top detected extension from content analysis.
    pub fn detected_extension(&self) -> Result<Option<&str>, StorageError> {
        Ok(self.analysis()?.top_detected_extension.as_deref())
    }

    /// Top detected MIME type from content analysis.
    pub fn mime_type(&self) -> Result<Option<&str>, StorageError> {
        Ok(self.analysis()?.top_mime_type.as_deref())
    }

    /// Heuristic content kind derived from analysis.
    pub fn content_kind(&self) -> Result<ContentKind, StorageError> {
        Ok(self.analysis()?.content_kind)
    }

    /// Ranked content matches.
    pub fn matches(&self) -> Result<&[DetectedDefinition], StorageError> {
        Ok(&self.analysis()?.matches)
    }

    /// Lazily load Windows shell display/type information when requested.
    pub fn shell_details(&self) -> Option<&WindowsShellDetails> {
        debug!(
            target: "dhara_storage::info::file",
            path = %self.path().display(),
            "loading Windows shell details"
        );
        self.shell_details
            .get_or_init(|| load_shell_details(self.path()))
            .as_ref()
    }

    /// Lazily load the Windows shell icon when requested.
    pub fn icon(&self) -> Option<&WindowsShellIcon> {
        debug!(
            target: "dhara_storage::info::file",
            path = %self.path().display(),
            "loading Windows shell icon"
        );
        self.shell_icon
            .get_or_init(|| load_shell_icon(self.path()))
            .as_ref()
    }
}
