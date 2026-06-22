use std::fs::{self, Metadata};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::error::StorageError;

/// Units used for formatting byte sizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeUnit {
    /// Format the value as raw bytes.
    Bytes,
    /// Format the value in kibibytes.
    KiB,
    /// Format the value in mebibytes.
    MiB,
    /// Format the value in gibibytes.
    GiB,
    /// Format the value in tebibytes.
    TiB,
}

/// Shared file-system metadata for files and directories.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageMetadata {
    pub(crate) path: PathBuf,
    pub(crate) name: String,
    pub(crate) is_read_only: bool,
    pub(crate) is_hidden: bool,
    pub(crate) is_system: bool,
    pub(crate) is_temporary: bool,
    pub(crate) is_symbolic_link: bool,
    pub(crate) link_target: Option<PathBuf>,
    pub(crate) created_at: Option<SystemTime>,
    pub(crate) modified_at: Option<SystemTime>,
    pub(crate) accessed_at: Option<SystemTime>,
}

impl StorageMetadata {
    pub(crate) fn from_path(path: impl AsRef<Path>) -> Result<(Self, Metadata), StorageError> {
        let path = normalize_existing_path(path.as_ref())?;
        let metadata = fs::symlink_metadata(&path)
            .map_err(|err| StorageError::io("read metadata for", &path, err))?;

        let name = path
            .file_name()
            .map(|value| value.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.as_os_str().to_string_lossy().into_owned());

        let is_symbolic_link = metadata.file_type().is_symlink();
        let link_target = if is_symbolic_link {
            fs::read_link(&path).ok()
        } else {
            None
        };
        let is_hidden = is_hidden(&metadata, path.as_path());
        let is_system = is_system(&metadata);
        let is_temporary = is_temporary(&metadata);

        let storage = Self {
            path,
            name,
            is_read_only: metadata.permissions().readonly(),
            is_hidden,
            is_system,
            is_temporary,
            is_symbolic_link,
            link_target,
            created_at: metadata.created().ok(),
            modified_at: metadata.modified().ok(),
            accessed_at: metadata.accessed().ok(),
        };

        Ok((storage, metadata))
    }

    /// Returns the absolute path represented by this metadata snapshot.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the last path segment for the represented entry.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns whether the entry was marked read-only when the snapshot was created.
    pub fn is_read_only(&self) -> bool {
        self.is_read_only
    }

    /// Returns whether the entry is hidden according to the current platform rules.
    pub fn is_hidden(&self) -> bool {
        self.is_hidden
    }

    /// Returns whether the entry carries the Windows system attribute.
    pub fn is_system(&self) -> bool {
        self.is_system
    }

    /// Returns whether the entry carries the Windows temporary attribute.
    pub fn is_temporary(&self) -> bool {
        self.is_temporary
    }

    /// Returns whether the snapshot describes a symbolic link.
    pub fn is_symbolic_link(&self) -> bool {
        self.is_symbolic_link
    }

    /// Returns the symbolic-link target when the entry is a link and the target could be resolved.
    pub fn link_target(&self) -> Option<&Path> {
        self.link_target.as_deref()
    }

    /// Returns the filesystem creation timestamp when the platform exposes it.
    pub fn created_at(&self) -> Option<SystemTime> {
        self.created_at
    }

    /// Returns the filesystem last-modified timestamp when the platform exposes it.
    pub fn modified_at(&self) -> Option<SystemTime> {
        self.modified_at
    }

    /// Returns the filesystem last-accessed timestamp when the platform exposes it.
    pub fn accessed_at(&self) -> Option<SystemTime> {
        self.accessed_at
    }
}

/// Formats a byte count using either the requested unit or an automatically selected one.
///
/// # Arguments
///
/// - `size` (`u64`) - The size in bytes to format.
/// - `unit` (`Option<SizeUnit>`) - The explicit display unit to use, or `None` to pick one automatically.
///
/// # Returns
///
/// - `String` - A human-friendly byte-size label using binary units.
pub fn format_size(size: u64, unit: Option<SizeUnit>) -> String {
    match unit.unwrap_or_else(|| auto_size_unit(size)) {
        SizeUnit::Bytes => format!("{size} B"),
        SizeUnit::KiB => format!("{:.2} KiB", size as f64 / 1024.0),
        SizeUnit::MiB => format!("{:.2} MiB", size as f64 / (1024.0 * 1024.0)),
        SizeUnit::GiB => format!("{:.2} GiB", size as f64 / (1024.0 * 1024.0 * 1024.0)),
        SizeUnit::TiB => format!(
            "{:.2} TiB",
            size as f64 / (1024.0 * 1024.0 * 1024.0 * 1024.0)
        ),
    }
}

fn auto_size_unit(size: u64) -> SizeUnit {
    match size {
        0..1024 => SizeUnit::Bytes,
        1024..1048576 => SizeUnit::KiB,
        1048576..1073741824 => SizeUnit::MiB,
        1073741824..1099511627776 => SizeUnit::GiB,
        _ => SizeUnit::TiB,
    }
}

fn normalize_existing_path(path: &Path) -> Result<PathBuf, StorageError> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .map_err(|err| StorageError::io("resolve current working directory for", path, err))
    }
}

#[cfg(windows)]
fn is_hidden(metadata: &Metadata, _path: &Path) -> bool {
    use std::os::windows::fs::MetadataExt;

    metadata.file_attributes() & 0x2 != 0
}

#[cfg(not(windows))]
fn is_hidden(_metadata: &Metadata, path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with('.'))
}

#[cfg(windows)]
fn is_system(metadata: &Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    metadata.file_attributes() & 0x4 != 0
}

#[cfg(not(windows))]
fn is_system(_metadata: &Metadata) -> bool {
    false
}

#[cfg(windows)]
fn is_temporary(metadata: &Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    metadata.file_attributes() & 0x100 != 0
}

#[cfg(not(windows))]
fn is_temporary(_metadata: &Metadata) -> bool {
    false
}
