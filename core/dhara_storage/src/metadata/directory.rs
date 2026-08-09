//! [`DirectoryMetadata`] — on-demand directory metadata and recursive summary.

use std::fs;
use std::path::{Path, PathBuf};
use std::thread;

use once_cell::sync::OnceCell;
use tracing::debug;

use crate::error::StorageError;

use super::attributes::StorageAttributes;
use super::permissions::StoragePermissions;
use super::shell_icon::{DEFAULT_SHELL_ICON_SIZE, ShellIcon, load_shell_icon};
use super::size::{StorageSize, format_size};
use super::traits::{CommonFields, StorageMetadata};
use super::windows_shell::{ShellDetails, load_shell_details};

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

    /// Total size as [`StorageSize`].
    pub fn to_storage_size(self) -> StorageSize {
        StorageSize::from_bytes(self.total_size)
    }
}

/// Directory metadata snapshot (no content analysis).
#[derive(Debug)]
pub struct DirectoryMetadata {
    absolute_path: PathBuf,
    common: CommonFields,
    summary: OnceCell<DirectorySummary>,
    shell_details: OnceCell<Option<ShellDetails>>,
    shell_icon: OnceCell<Option<ShellIcon>>,
}

impl DirectoryMetadata {
    /// Load directory metadata without walking the tree.
    pub fn load(absolute_path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let absolute_path = absolute_path.as_ref().to_path_buf();
        debug!(
            target: "dhara_storage::metadata::directory",
            path = %absolute_path.display(),
            "loading directory metadata"
        );
        let (common, fs_metadata) = CommonFields::load(&absolute_path)?;
        if !fs_metadata.is_dir() && !(fs_metadata.file_type().is_symlink() && absolute_path.is_dir())
        {
            return Err(StorageError::NotADirectory {
                path: absolute_path,
            });
        }

        Ok(Self {
            absolute_path,
            common,
            summary: OnceCell::new(),
            shell_details: OnceCell::new(),
            shell_icon: OnceCell::new(),
        })
    }

    /// Lazily compute and cache recursive directory statistics.
    pub fn summary(&self) -> Result<&DirectorySummary, StorageError> {
        debug!(
            target: "dhara_storage::metadata::directory",
            path = %self.absolute_path.display(),
            "loading directory summary on demand"
        );
        self.summary
            .get_or_try_init(|| scan_directory_summary(&self.absolute_path))
    }

    /// Content/identity type label (shell-backed on Windows when available).
    pub fn file_type_name(&self) -> String {
        self.ensure_shell()
            .and_then(|shell| shell.type_name.as_deref())
            .filter(|value| !value.is_empty())
            .unwrap_or("Directory")
            .to_owned()
    }

    fn ensure_shell(&self) -> Option<&ShellDetails> {
        self.shell_details
            .get_or_init(|| load_shell_details(&self.absolute_path))
            .as_ref()
    }

    fn shell_display_name(&self) -> Option<&str> {
        self.ensure_shell()
            .and_then(|shell| shell.display_name.as_deref())
            .filter(|value| !value.is_empty())
    }
}

impl StorageMetadata for DirectoryMetadata {
    fn name(&self) -> &str {
        &self.common.name
    }

    fn display_name(&self) -> &str {
        self.shell_display_name().unwrap_or_else(|| self.name())
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

/// Scan a directory tree and return recursive size/count statistics.
pub fn scan_directory_summary(path: &Path) -> Result<DirectorySummary, StorageError> {
    debug!(
        target: "dhara_storage::metadata::directory",
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
            target: "dhara_storage::metadata::directory",
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
