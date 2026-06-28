/// Windows shell display metadata loaded lazily when requested.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowsShellDetails {
    /// Human-friendly display name reported by the Windows shell.
    pub display_name: Option<String>,
    /// Type label reported by the Windows shell.
    pub type_name: Option<String>,
}

/// Windows shell icon pixels loaded lazily when requested.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowsShellIcon {
    /// Icon width in pixels.
    pub width: i32,
    /// Icon height in pixels.
    pub height: i32,
    /// RGBA pixel bytes laid out row-major from top-left to bottom-right.
    pub rgba: Vec<u8>,
}

#[cfg(windows)]
mod imp {
    use std::mem::size_of;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Gdi::{
        BI_RGB, BITMAP, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, DeleteObject, GetDC,
        GetDIBits, GetObjectW, ReleaseDC,
    };
    use windows::Win32::UI::Shell::{
        SHFILEINFOW, SHGFI_DISPLAYNAME, SHGFI_ICON, SHGFI_LARGEICON, SHGFI_TYPENAME, SHGetFileInfoW,
    };
    use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, ICONINFO};
    use windows::core::PCWSTR;

    use super::{WindowsShellDetails, WindowsShellIcon};

    pub(crate) fn load_shell_details(path: &Path) -> Option<WindowsShellDetails> {
        let wide_path = to_wide_path(path);
        let mut file_info = SHFILEINFOW::default();

        let result = unsafe {
            SHGetFileInfoW(
                PCWSTR(wide_path.as_ptr()),
                Default::default(),
                Some(&mut file_info),
                size_of::<SHFILEINFOW>() as u32,
                SHGFI_DISPLAYNAME | SHGFI_TYPENAME,
            )
        };

        if result == 0 {
            return None;
        }

        Some(WindowsShellDetails {
            display_name: wide_buf_to_string(&file_info.szDisplayName),
            type_name: wide_buf_to_string(&file_info.szTypeName),
        })
    }

    pub(crate) fn load_shell_icon(path: &Path) -> Option<WindowsShellIcon> {
        let wide_path = to_wide_path(path);
        let mut file_info = SHFILEINFOW::default();

        let result = unsafe {
            SHGetFileInfoW(
                PCWSTR(wide_path.as_ptr()),
                Default::default(),
                Some(&mut file_info),
                size_of::<SHFILEINFOW>() as u32,
                SHGFI_ICON | SHGFI_LARGEICON,
            )
        };

        if result == 0 || file_info.hIcon.is_invalid() {
            return None;
        }

        let icon = unsafe { icon_to_rgba(file_info.hIcon) };
        unsafe {
            let _ = DestroyIcon(file_info.hIcon);
        }
        icon
    }

    unsafe fn icon_to_rgba(
        hicon: windows::Win32::UI::WindowsAndMessaging::HICON,
    ) -> Option<WindowsShellIcon> {
        let mut icon_info = ICONINFO::default();
        if unsafe { GetIconInfo(hicon, &mut icon_info) }.is_err() {
            return None;
        }

        let bitmap_handle = if !icon_info.hbmColor.is_invalid() {
            icon_info.hbmColor
        } else {
            icon_info.hbmMask
        };

        let mut bitmap = BITMAP::default();
        if unsafe {
            GetObjectW(
                bitmap_handle.into(),
                size_of::<BITMAP>() as i32,
                Some((&mut bitmap as *mut BITMAP).cast()),
            )
        } == 0
        {
            unsafe { cleanup_icon_bitmaps(&icon_info) };
            return None;
        }

        let width = bitmap.bmWidth;
        let height = if !icon_info.hbmColor.is_invalid() {
            bitmap.bmHeight
        } else {
            bitmap.bmHeight / 2
        };

        if width <= 0 || height <= 0 {
            unsafe { cleanup_icon_bitmaps(&icon_info) };
            return None;
        }

        let mut bitmap_info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut bgra = vec![0_u8; width as usize * height as usize * 4];
        let screen_dc = unsafe { GetDC(Some(HWND::default())) };
        if screen_dc.is_invalid() {
            unsafe { cleanup_icon_bitmaps(&icon_info) };
            return None;
        }

        let copied = unsafe {
            GetDIBits(
                screen_dc,
                bitmap_handle,
                0,
                height as u32,
                Some(bgra.as_mut_ptr().cast()),
                &mut bitmap_info,
                DIB_RGB_COLORS,
            )
        };
        let _ = unsafe { ReleaseDC(Some(HWND::default()), screen_dc) };
        unsafe { cleanup_icon_bitmaps(&icon_info) };

        if copied == 0 {
            return None;
        }

        let mut rgba = vec![0_u8; bgra.len()];
        let mut saw_non_zero_alpha = false;
        for (src, dst) in bgra.chunks_exact(4).zip(rgba.chunks_exact_mut(4)) {
            dst[0] = src[2];
            dst[1] = src[1];
            dst[2] = src[0];
            dst[3] = src[3];
            if src[3] != 0 {
                saw_non_zero_alpha = true;
            }
        }

        if !saw_non_zero_alpha {
            for chunk in rgba.chunks_exact_mut(4) {
                chunk[3] = 255;
            }
        }

        Some(WindowsShellIcon {
            width,
            height,
            rgba,
        })
    }

    unsafe fn cleanup_icon_bitmaps(icon_info: &ICONINFO) {
        if !icon_info.hbmColor.is_invalid() {
            let _ = unsafe { DeleteObject(icon_info.hbmColor.into()) };
        }
        if !icon_info.hbmMask.is_invalid() {
            let _ = unsafe { DeleteObject(icon_info.hbmMask.into()) };
        }
    }

    fn to_wide_path(path: &Path) -> Vec<u16> {
        path.as_os_str().encode_wide().chain(Some(0)).collect()
    }

    fn wide_buf_to_string(buffer: &[u16]) -> Option<String> {
        let len = buffer
            .iter()
            .position(|ch| *ch == 0)
            .unwrap_or(buffer.len());
        if len == 0 {
            return None;
        }
        Some(String::from_utf16_lossy(&buffer[..len]))
    }
}

#[cfg(not(windows))]
mod imp {
    use std::path::Path;

    use super::{WindowsShellDetails, WindowsShellIcon};

    pub(crate) fn load_shell_details(_path: &Path) -> Option<WindowsShellDetails> {
        None
    }

    pub(crate) fn load_shell_icon(_path: &Path) -> Option<WindowsShellIcon> {
        None
    }
}

pub(crate) use imp::{load_shell_details, load_shell_icon};
