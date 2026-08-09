//! On-demand storage metadata, attributes, permissions, and size helpers.
//!
//! Paths and size live on [`crate::storage`] handles. Metadata is loaded when
//! requested and is not cached on those handles. [`crate::storage::FileStorage::analyze`]
//! caches an [`crate::analysis::AnalysisReport`] on the file handle; [`FileMetadata`]
//! is enriched from that cache when present.

mod attributes;
mod directory;
mod file;
mod permissions;
mod shell_icon;
mod temporary;
mod traits;
mod windows_shell;

pub use attributes::{StorageAttributes, apply_storage_attributes};
pub(crate) use attributes::attributes_from_path;
pub use directory::{DirectoryMetadata, DirectorySummary, scan_directory_summary};
pub use dhara_storage_core::{SizeUnit, StorageSize, format_size};
pub use file::{FileExtension, FileMetadata, StorageType};
pub use permissions::StoragePermissions;
pub(crate) use permissions::permissions_from_path;
pub use shell_icon::{DEFAULT_SHELL_ICON_SIZE, ShellIcon};
pub use temporary::is_temporary_path;
pub use traits::StorageMetadata;
