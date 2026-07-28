//! Win32 DuplicateHandle helpers for the daemon data plane.

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use windows::Win32::Foundation::{
    CloseHandle, DUPLICATE_SAME_ACCESS, DuplicateHandle, GENERIC_READ, GENERIC_WRITE, HANDLE,
    INVALID_HANDLE_VALUE,
};
use windows::Win32::Storage::FileSystem::{
    CREATE_ALWAYS, CREATE_NEW, CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_DELETE,
    FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_ALWAYS, OPEN_EXISTING,
};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcess, PROCESS_DUP_HANDLE};

/// Open `path` for reading and duplicate the handle into `parent_pid`.
pub fn open_read_and_duplicate(path: &Path, parent_pid: u32) -> Result<(u64, u64), String> {
    let source = open_file(
        path,
        GENERIC_READ.0,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
        OPEN_EXISTING,
    )?;
    let size = std::fs::metadata(path)
        .map(|meta| meta.len())
        .map_err(|err| format!("metadata failed: {err}"))?;
    let handle = duplicate_into_parent(source, parent_pid)?;
    Ok((handle, size))
}

/// Create or open `path` for writing and duplicate the handle into `parent_pid`.
pub fn open_write_and_duplicate(
    path: &Path,
    parent_pid: u32,
    overwrite: bool,
    create_parents: bool,
) -> Result<u64, String> {
    if create_parents {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|err| format!("create_dir_all failed: {err}"))?;
        }
    }

    let disposition = if overwrite {
        CREATE_ALWAYS
    } else if path.exists() {
        OPEN_ALWAYS
    } else {
        CREATE_NEW
    };

    let source = open_file(
        path,
        GENERIC_WRITE.0,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
        disposition,
    )?;
    duplicate_into_parent(source, parent_pid)
}

fn open_file(
    path: &Path,
    access: u32,
    share: windows::Win32::Storage::FileSystem::FILE_SHARE_MODE,
    disposition: windows::Win32::Storage::FileSystem::FILE_CREATION_DISPOSITION,
) -> Result<HANDLE, String> {
    let wide: Vec<u16> = OsStr::new(path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let source = unsafe {
        CreateFileW(
            windows::core::PCWSTR(wide.as_ptr()),
            access,
            share,
            None,
            disposition,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )
    }
    .map_err(|err| format!("CreateFileW failed: {err}"))?;

    if source.is_invalid() {
        return Err("CreateFileW returned INVALID_HANDLE_VALUE".into());
    }

    Ok(source)
}

fn duplicate_into_parent(source: HANDLE, parent_pid: u32) -> Result<u64, String> {
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

    Ok(target.0 as usize as u64)
}
