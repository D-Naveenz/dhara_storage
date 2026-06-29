use std::path::Path;

/// Shell icon pixels loaded lazily when requested.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellIcon {
    /// Icon width in pixels.
    pub width: u32,
    /// Icon height in pixels.
    pub height: u32,
    /// RGBA pixel bytes laid out row-major from top-left to bottom-right.
    pub rgba: Vec<u8>,
}

/// Default shell icon dimension used when no explicit size is requested.
pub const DEFAULT_SHELL_ICON_SIZE: u32 = 32;

impl ShellIcon {
    /// Load the OS shell icon for `path` at the requested pixel dimension.
    ///
    /// Returns `None` when the desktop environment cannot provide an icon (headless
    /// Linux, off-main-thread GTK calls, missing file, etc.).
    pub fn load(path: &Path, size: u32) -> Option<Self> {
        let size = u16::try_from(size).ok()?;
        let icon = file_icon_provider::get_file_icon(path, size).ok()?;
        Some(Self {
            width: icon.width,
            height: icon.height,
            rgba: icon.pixels,
        })
    }
}

pub(crate) fn load_shell_icon(path: &Path, size: u32) -> Option<ShellIcon> {
    ShellIcon::load(path, size)
}
