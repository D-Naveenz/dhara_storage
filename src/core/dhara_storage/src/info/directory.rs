use std::fs;
use std::path::Path;
use std::thread;

use once_cell::sync::OnceCell;
use tracing::{debug, info};

use crate::error::StorageError;

use super::common::{StorageMetadata, format_size};
use super::windows::{WindowsShellDetails, WindowsShellIcon, load_shell_details, load_shell_icon};

/// Recursive directory statistics computed on demand.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DirectorySummary {
    /// Total size of recursively discovered files.
    pub total_size: u64,
    /// Total number of recursively discovered files.
    pub file_count: u64,
    /// Total number of recursively discovered subdirectories.
    pub directory_count: u64,
}

impl DirectorySummary {
    /// Formatted recursive size.
    pub fn formatted_size(&self) -> String {
        format_size(self.total_size, None)
    }
}

/// Immutable directory metadata with lazy, cached recursive summary.
#[derive(Debug)]
pub struct DirectoryInfo {
    metadata: StorageMetadata,
    summary: OnceCell<DirectorySummary>,
    shell_details: OnceCell<Option<WindowsShellDetails>>,
    shell_icon: OnceCell<Option<WindowsShellIcon>>,
}

impl DirectoryInfo {
    /// Load basic directory metadata without walking the tree.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        debug!(
            target: "dhara_storage::info::directory",
            path = %path.as_ref().display(),
            "loading directory metadata"
        );
        let (metadata, fs_metadata) = StorageMetadata::from_path(path)?;
        if !fs_metadata.is_dir() {
            return Err(StorageError::NotADirectory {
                path: metadata.path().to_path_buf(),
            });
        }

        Ok(Self {
            metadata,
            summary: OnceCell::new(),
            shell_details: OnceCell::new(),
            shell_icon: OnceCell::new(),
        })
    }

    /// Load basic directory metadata while precomputing recursive summary in parallel.
    pub fn from_path_with_summary(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let owned_path = path.as_ref().to_path_buf();
        info!(
            target: "dhara_storage::info::directory",
            path = %owned_path.display(),
            "loading directory metadata with eager summary"
        );

        thread::scope(|scope| {
            let summary_handle = scope.spawn(|| scan_directory_summary(&owned_path));
            let info = Self::from_path(&owned_path)?;
            let summary = summary_handle
                .join()
                .expect("directory summary preload thread panicked")?;
            let _ = info.summary.set(summary);
            Ok(info)
        })
    }

    /// Returns the shared storage metadata.
    pub fn metadata(&self) -> &StorageMetadata {
        &self.metadata
    }

    /// Absolute path to the directory.
    pub fn path(&self) -> &Path {
        self.metadata.path()
    }

    /// Directory name.
    pub fn name(&self) -> &str {
        self.metadata.name()
    }

    /// Human-friendly display name.
    pub fn display_name(&self) -> &str {
        self.name()
    }

    /// Directory type label using shell data as a lazy fallback.
    pub fn type_name(&self) -> String {
        self.shell_details()
            .and_then(|shell| shell.type_name.as_deref())
            .filter(|value| !value.is_empty())
            .unwrap_or("Directory")
            .to_owned()
    }

    /// Lazily compute and cache recursive directory statistics.
    pub fn summary(&self) -> Result<&DirectorySummary, StorageError> {
        debug!(
            target: "dhara_storage::info::directory",
            path = %self.path().display(),
            "loading directory summary on demand"
        );
        self.summary
            .get_or_try_init(|| scan_directory_summary(self.path()))
    }

    /// Total recursive size in bytes.
    pub fn size(&self) -> Result<u64, StorageError> {
        Ok(self.summary()?.total_size)
    }

    /// Total recursive file count.
    pub fn file_count(&self) -> Result<u64, StorageError> {
        Ok(self.summary()?.file_count)
    }

    /// Total recursive subdirectory count.
    pub fn directory_count(&self) -> Result<u64, StorageError> {
        Ok(self.summary()?.directory_count)
    }

    /// Returns a cached summary if it has already been computed.
    pub fn summary_if_loaded(&self) -> Option<&DirectorySummary> {
        self.summary.get()
    }

    /// Lazily load Windows shell display/type information when requested.
    pub fn shell_details(&self) -> Option<&WindowsShellDetails> {
        debug!(
            target: "dhara_storage::info::directory",
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
            target: "dhara_storage::info::directory",
            path = %self.path().display(),
            "loading Windows shell icon"
        );
        self.shell_icon
            .get_or_init(|| load_shell_icon(self.path()))
            .as_ref()
    }
}

fn scan_directory_summary(path: &Path) -> Result<DirectorySummary, StorageError> {
    debug!(
        target: "dhara_storage::info::directory",
        path = %path.display(),
        "scanning directory summary"
    );
    let entries = fs::read_dir(path)
        .map_err(|err| StorageError::io("read directory for", path.to_path_buf(), err))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| {
            StorageError::io("enumerate directory entries for", path.to_path_buf(), err)
        })?;

    thread::scope(|scope| {
        let handles = entries
            .into_iter()
            .map(|entry| {
                let child_path = entry.path();
                scope.spawn(move || scan_entry_recursive(&child_path))
            })
            .collect::<Vec<_>>();

        let mut summary = DirectorySummary::default();
        for handle in handles {
            summary += handle.join().expect("directory summary worker panicked")?;
        }

        debug!(
            target: "dhara_storage::info::directory",
            path = %path.display(),
            total_size = summary.total_size,
            file_count = summary.file_count,
            directory_count = summary.directory_count,
            "directory summary completed"
        );
        Ok(summary)
    })
}

fn scan_entry_recursive(path: &Path) -> Result<DirectorySummary, StorageError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|err| StorageError::io("read metadata for", path.to_path_buf(), err))?;
    let file_type = metadata.file_type();

    if file_type.is_symlink() || file_type.is_file() {
        return Ok(DirectorySummary {
            total_size: metadata.len(),
            file_count: 1,
            directory_count: 0,
        });
    }

    if file_type.is_dir() {
        let mut summary = DirectorySummary {
            total_size: 0,
            file_count: 0,
            directory_count: 1,
        };

        for entry in fs::read_dir(path)
            .map_err(|err| StorageError::io("read directory for", path.to_path_buf(), err))?
        {
            let child = entry.map_err(|err| {
                StorageError::io("enumerate directory entries for", path.to_path_buf(), err)
            })?;
            summary += scan_entry_recursive(&child.path())?;
        }

        return Ok(summary);
    }

    Ok(DirectorySummary::default())
}

impl std::ops::AddAssign for DirectorySummary {
    fn add_assign(&mut self, rhs: Self) {
        self.total_size += rhs.total_size;
        self.file_count += rhs.file_count;
        self.directory_count += rhs.directory_count;
    }
}
