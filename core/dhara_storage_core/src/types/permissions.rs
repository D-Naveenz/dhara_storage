//! Effective process capability flags (data only; no path I/O).

/// Best-effort effective permissions for the **current process**.
///
/// This is not a security boundary (TOCTOU applies) and does not enumerate
/// Windows ACL principals the way Explorer's Security tab does. The runtime
/// probes these flags from the filesystem; this type is the portable value
/// shape only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StoragePermissions {
    /// Whether the current process can read the entry.
    pub can_read: bool,
    /// Whether the current process can write the entry.
    pub can_write: bool,
    /// Whether the current process can modify/replace content (write without read-only).
    pub can_modify: bool,
    /// Whether the current process can execute the entry (files) or search (directories).
    pub can_execute: bool,
}
