//! Shared value types and operation option bundles for the storage framework.
//!
//! Path I/O, shell integration, and handle types live in `dhara_storage`. This
//! module owns portable shapes that the runtime and extension crates share.

mod attributes;
mod options;
mod permissions;
mod size;

pub use attributes::StorageAttributes;
pub use options::{
    DirectoryDeleteOptions, FileShareMode, OpenReadOptions, OpenWriteOptions, ReadOptions,
    TransferOptions, WriteOptions,
};
pub use permissions::StoragePermissions;
pub use size::{SizeUnit, StorageSize, format_size};
