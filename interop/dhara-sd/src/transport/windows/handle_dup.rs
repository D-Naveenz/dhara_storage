//! Win32 DuplicateHandle helpers for the daemon data plane.
//!
//! Opens use [`dhara_storage::open_for_read`] / [`dhara_storage::open_for_write`], then
//! duplicate the resulting handle into the host process.

use std::fs::File;
use std::os::windows::io::AsRawHandle;
use std::path::Path;
use std::time::Duration;

use dhara_storage::{
    FileShareMode, OpenReadOptions, OpenWriteOptions, open_for_read, open_for_write,
};
use windows::Win32::Foundation::{
    CloseHandle, DUPLICATE_SAME_ACCESS, DuplicateHandle, HANDLE, INVALID_HANDLE_VALUE,
};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcess, PROCESS_DUP_HANDLE};

/// Open `path` for reading and duplicate the handle into `parent_pid`.
pub fn open_read_and_duplicate(
    path: &Path,
    parent_pid: u32,
    share: FileShareMode,
    lock_timeout: Option<Duration>,
) -> Result<(u64, u64), String> {
    let file = open_for_read(
        path,
        OpenReadOptions {
            share,
            lock_timeout,
        },
    )
    .map_err(|err| err.to_string())?;
    let size = file
        .metadata()
        .map(|meta| meta.len())
        .map_err(|err| format!("metadata failed: {err}"))?;
    let handle = duplicate_file_into_parent(file, parent_pid)?;
    Ok((handle, size))
}

/// Create or open `path` for writing and duplicate the handle into `parent_pid`.
pub fn open_write_and_duplicate(
    path: &Path,
    parent_pid: u32,
    overwrite: bool,
    create_parents: bool,
    share: FileShareMode,
    lock_timeout: Option<Duration>,
) -> Result<u64, String> {
    let file = open_for_write(
        path,
        OpenWriteOptions {
            share,
            overwrite,
            create_parent_directories: create_parents,
            lock_timeout,
        },
    )
    .map_err(|err| err.to_string())?;
    duplicate_file_into_parent(file, parent_pid)
}

fn duplicate_file_into_parent(file: File, parent_pid: u32) -> Result<u64, String> {
    let source = HANDLE(file.as_raw_handle());
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

    drop(file);
    let _ = unsafe { CloseHandle(parent) };

    duplicated.map_err(|err| format!("DuplicateHandle failed: {err}"))?;

    if target == INVALID_HANDLE_VALUE || target.is_invalid() {
        return Err("DuplicateHandle produced an invalid target handle".into());
    }

    Ok(target.0 as usize as u64)
}
