//! [`StorageSize`] and human-readable size formatting helpers.

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

/// On-demand storage size: raw OS bytes plus a readable label.
///
/// Runtime handles measure size when asked; the value is not part of a metadata
/// snapshot and is not cached on the handle by the framework.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageSize {
    /// Size in bytes as reported by the operating system (or directory walk total).
    pub bytes: u64,
    /// Human-readable form using binary units (for example `"2.36 KiB"`).
    pub formatted: String,
}

impl StorageSize {
    /// Build a [`StorageSize`] from a byte count.
    pub fn from_bytes(bytes: u64) -> Self {
        Self {
            bytes,
            formatted: format_size(bytes, None),
        }
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
