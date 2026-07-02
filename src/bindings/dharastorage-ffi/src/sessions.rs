use std::ffi::c_char;
use std::fs::{self, OpenOptions};
use std::ptr;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;

use crate::abi::{DharaStatus, NativeWriteSession, with_write_session};
use crate::errors::FfiFailure;
use crate::marshal::{
    execute_string, execute_unit, parse_bytes_arg, parse_path_arg, reset_buffer_out,
    validate_buffer_out, write_error_only, write_error_payload,
};

#[unsafe(no_mangle)]
/// Creates a streaming native write session for incremental file uploads.
///
/// # Safety
///
/// `path`, `out_handle`, `out_error_ptr`, and `out_error_len` must follow the Dhara Storage FFI
/// pointer contracts. `path` must be a valid null-terminated UTF-8 string.
pub unsafe extern "C" fn dhara_write_session_create(
    path: *const c_char,
    overwrite: u8,
    create_parent_directories: u8,
    out_handle: *mut *mut NativeWriteSession,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> DharaStatus {
    ffi_fn!({
        if out_handle.is_null() {
            return write_error_only(
                out_error_ptr,
                out_error_len,
                FfiFailure::invalid_argument("out_handle", "write session output must not be null"),
            );
        }
        if let Err(failure) = validate_buffer_out(out_error_ptr, out_error_len) {
            return failure.status;
        }
        *out_handle = ptr::null_mut();
        reset_buffer_out(out_error_ptr, out_error_len);

        let result = (|| {
            let path = parse_path_arg(path, "path")?;
            if create_parent_directories != 0
                && let Some(parent) = path.parent()
                && !parent.as_os_str().is_empty()
            {
                fs::create_dir_all(parent)
                    .map_err(|err| FfiFailure::io("create parent directory for", parent, err))?;
            }

            let mut options = OpenOptions::new();
            options.write(true).create(true);
            if overwrite != 0 {
                options.truncate(true);
            } else {
                options.create_new(true);
            }

            let file = options
                .open(&path)
                .map_err(|err| FfiFailure::io("open write session", &path, err))?;

            Ok(Box::into_raw(Box::new(NativeWriteSession {
                path,
                file: Mutex::new(Some(file)),
                completed: AtomicBool::new(false),
            })))
        })();

        match result {
            Ok(handle) => {
                *out_handle = handle;
                DharaStatus::Ok
            }
            Err(failure) => {
                write_error_payload(out_error_ptr, out_error_len, &failure);
                failure.status
            }
        }
    })
}

#[unsafe(no_mangle)]
/// Appends a chunk of bytes to an open native write session.
///
/// # Safety
///
/// `handle`, `data_ptr`, `out_error_ptr`, and `out_error_len` must follow the Dhara Storage FFI
/// pointer contracts. `data_ptr` must reference `data_len` readable bytes when `data_len` is non-zero.
pub unsafe extern "C" fn dhara_write_session_write_chunk(
    handle: *mut NativeWriteSession,
    data_ptr: *const u8,
    data_len: usize,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> DharaStatus {
    ffi_fn!(execute_unit(out_error_ptr, out_error_len, || {
        let bytes = parse_bytes_arg(data_ptr, data_len, "data")?;
        with_write_session(handle, |session| session.write_chunk(bytes))
    }))
}

#[unsafe(no_mangle)]
/// Completes a native write session, flushes the file, and returns the resulting path.
///
/// # Safety
///
/// `handle`, `out_path_ptr`, `out_path_len`, `out_error_ptr`, and `out_error_len` must follow
/// the Dhara Storage FFI pointer contracts.
pub unsafe extern "C" fn dhara_write_session_complete(
    handle: *mut NativeWriteSession,
    out_path_ptr: *mut *mut u8,
    out_path_len: *mut usize,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> DharaStatus {
    ffi_fn!(execute_string(
        out_path_ptr,
        out_path_len,
        out_error_ptr,
        out_error_len,
        || with_write_session(handle, |session| session.complete()),
    ))
}

#[unsafe(no_mangle)]
/// Aborts a native write session and removes any partially written file when possible.
///
/// # Safety
///
/// `handle`, `out_error_ptr`, and `out_error_len` must follow the Dhara Storage FFI pointer contracts.
pub unsafe extern "C" fn dhara_write_session_abort(
    handle: *mut NativeWriteSession,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> DharaStatus {
    ffi_fn!(execute_unit(out_error_ptr, out_error_len, || {
        with_write_session(handle, |session| session.abort())
    }))
}

#[unsafe(no_mangle)]
/// Frees a native write-session handle.
///
/// # Safety
///
/// `handle` must either be null or a pointer previously returned by `dhara_write_session_create`.
/// The pointer must not be freed more than once.
pub unsafe extern "C" fn dhara_write_session_free(handle: *mut NativeWriteSession) {
    if handle.is_null() {
        return;
    }

    let session = Box::from_raw(handle);
    let _ = session.abort();
}
