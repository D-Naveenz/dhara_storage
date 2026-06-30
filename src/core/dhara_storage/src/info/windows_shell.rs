/// Windows shell display metadata loaded lazily when requested.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellDetails {
    /// Human-friendly display name reported by the Windows shell.
    pub display_name: Option<String>,
    /// Type label reported by the Windows shell.
    pub type_name: Option<String>,
}

#[cfg(windows)]
mod imp {
    use std::mem::size_of;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    use windows::Win32::UI::Shell::{
        SHFILEINFOW, SHGFI_DISPLAYNAME, SHGFI_TYPENAME, SHGetFileInfoW,
    };
    use windows::core::PCWSTR;

    use super::ShellDetails;

    pub(crate) fn load_shell_details(path: &Path) -> Option<ShellDetails> {
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

        Some(ShellDetails {
            display_name: wide_buf_to_string(&file_info.szDisplayName),
            type_name: wide_buf_to_string(&file_info.szTypeName),
        })
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

    use super::ShellDetails;

    pub(crate) fn load_shell_details(_path: &Path) -> Option<ShellDetails> {
        None
    }
}

pub(crate) use imp::load_shell_details;
