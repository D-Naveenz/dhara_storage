//! Win32 DuplicateHandle helpers for the pilot data plane.

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use windows::Win32::Foundation::{
    CloseHandle, DuplicateHandle, GENERIC_READ, DUPLICATE_SAME_ACCESS, HANDLE, INVALID_HANDLE_VALUE,
};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
    OPEN_EXISTING,
};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcess, PROCESS_DUP_HANDLE};

/// Open `path` for reading and duplicate the handle into `parent_pid`.
///
/// Returns the handle value valid in the parent process, plus the file size.
pub fn open_and_duplicate(path: &Path, parent_pid: u32) -> Result<(u64, u64), String> {
    let wide: Vec<u16> = OsStr::new(path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let source = unsafe {
        CreateFileW(
            windows::core::PCWSTR(wide.as_ptr()),
            GENERIC_READ.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )
    }
    .map_err(|err| format!("CreateFileW failed: {err}"))?;

    if source.is_invalid() {
        return Err("CreateFileW returned INVALID_HANDLE_VALUE".into());
    }

    let size = std::fs::metadata(path)
        .map(|meta| meta.len())
        .map_err(|err| format!("metadata failed: {err}"))?;

    let parent = unsafe { OpenProcess(PROCESS_DUP_HANDLE, false, parent_pid) }
        .map_err(|err| format!("OpenProcess({parent_pid}) failed: {err}"))?;

    let mut target = HANDLE::default();
    let duplicated = unsafe {
        DuplicateHandle(
            GetCurrentProcess(),
            source,
            parent,
            &mut target,
            0,
            false,
            DUPLICATE_SAME_ACCESS,
        )
    };

    let _ = unsafe { CloseHandle(source) };
    let _ = unsafe { CloseHandle(parent) };

    duplicated.map_err(|err| format!("DuplicateHandle failed: {err}"))?;

    if target == INVALID_HANDLE_VALUE || target.is_invalid() {
        return Err("DuplicateHandle produced an invalid target handle".into());
    }

    Ok((target.0 as usize as u64, size))
}
