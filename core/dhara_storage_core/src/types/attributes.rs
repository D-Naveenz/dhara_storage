//! Settable filesystem attribute flags (data only; no path I/O).

/// Settable storage attributes shared by files and directories.
///
/// Temporary and symbolic-link state are **not** attributes here: they are
/// creation-time / detection concerns and cannot be toggled like Hidden.
///
/// The runtime applies and reads these flags from the filesystem. This type is
/// the portable value shape only.
///
/// # Platform notes
///
/// - **Windows:** maps to Win32 file attributes (`FILE_ATTRIBUTE_*`).
/// - **Unix:** `read_only` uses `Permissions::set_readonly`; `hidden` follows
///   the leading-dot name convention (renaming is not performed by setters);
///   `system` has no portable equivalent and is ignored on set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StorageAttributes {
    /// Read-only bit / owner write disabled.
    pub read_only: bool,
    /// Hidden (Windows attribute, or Unix leading-dot name convention on read).
    pub hidden: bool,
    /// Windows system attribute.
    pub system: bool,
}
