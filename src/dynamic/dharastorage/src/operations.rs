use std::ffi::c_char;
use std::io::Cursor;
use std::sync::Arc;

use dhara_storage::{
    FileStorage, ReadOptions, copy_directory_with_options, copy_file_with_options,
    create_directory, create_directory_all, delete_directory_with_options, delete_file,
    move_directory_with_options, move_file_with_options, rename_directory, rename_file,
};

use crate::abi::{
    DharaOperationSnapshot, DharaStatus, NativeOperationHandle, clone_operation_handle,
    fail_if_cancelled, progress_reporter, spawn_path_operation, transfer_options, write_options,
};
use crate::errors::FfiFailure;
use crate::marshal::{
    execute_bytes, execute_json, execute_operation_handle_creation, execute_string, execute_unit,
    parse_bytes_arg, parse_path_arg, parse_string_arg, reset_buffer_out, validate_buffer_out,
    write_error_only, write_error_payload,
};
use crate::models::path_to_string;

#[unsafe(no_mangle)]
/// Starts an asynchronous file copy operation and returns a native operation handle.
///
/// # Safety
///
/// `source`, `destination`, `out_handle`, `out_error_ptr`, and `out_error_len` must follow the
/// Dhara Storage FFI pointer contracts. The input strings must be valid null-terminated UTF-8 strings.
pub unsafe extern "C" fn dhara_operation_start_copy_file(
    source: *const c_char,
    destination: *const c_char,
    overwrite: u8,
    out_handle: *mut *mut NativeOperationHandle,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> DharaStatus {
    ffi_fn!(execute_operation_handle_creation(
        out_handle,
        out_error_ptr,
        out_error_len,
        || {
            let source = parse_path_arg(source, "source")?;
            let destination = parse_path_arg(destination, "destination")?;
            Ok(spawn_path_operation(move |handle| {
                copy_file_with_options(
                    &source,
                    &destination,
                    transfer_options(&handle, overwrite != 0),
                )
                .map(|path| path_to_string(&path))
                .map(crate::abi::OperationResult::String)
                .map_err(FfiFailure::from)
            }))
        }
    ))
}

#[unsafe(no_mangle)]
/// Starts an asynchronous file move operation and returns a native operation handle.
///
/// # Safety
///
/// `source`, `destination`, `out_handle`, `out_error_ptr`, and `out_error_len` must follow the
/// Dhara Storage FFI pointer contracts. The input strings must be valid null-terminated UTF-8 strings.
pub unsafe extern "C" fn dhara_operation_start_move_file(
    source: *const c_char,
    destination: *const c_char,
    overwrite: u8,
    out_handle: *mut *mut NativeOperationHandle,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> DharaStatus {
    ffi_fn!(execute_operation_handle_creation(
        out_handle,
        out_error_ptr,
        out_error_len,
        || {
            let source = parse_path_arg(source, "source")?;
            let destination = parse_path_arg(destination, "destination")?;
            Ok(spawn_path_operation(move |handle| {
                move_file_with_options(
                    &source,
                    &destination,
                    transfer_options(&handle, overwrite != 0),
                )
                .map(|path| path_to_string(&path))
                .map(crate::abi::OperationResult::String)
                .map_err(FfiFailure::from)
            }))
        }
    ))
}

#[unsafe(no_mangle)]
/// Starts an asynchronous file rename operation within the current parent directory.
///
/// # Safety
///
/// `source`, `new_name`, `out_handle`, `out_error_ptr`, and `out_error_len` must follow the Dhara
/// Storage FFI pointer contracts. String inputs must be valid null-terminated UTF-8 strings.
pub unsafe extern "C" fn dhara_operation_start_rename_file(
    source: *const c_char,
    new_name: *const c_char,
    out_handle: *mut *mut NativeOperationHandle,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> DharaStatus {
    ffi_fn!(execute_operation_handle_creation(
        out_handle,
        out_error_ptr,
        out_error_len,
        || {
            let source = parse_path_arg(source, "source")?;
            let new_name = parse_string_arg(new_name, "new_name")?;
            Ok(spawn_path_operation(move |handle| {
                fail_if_cancelled(&handle)?;
                rename_file(&source, &new_name)
                    .map(|path| path_to_string(&path))
                    .map(crate::abi::OperationResult::String)
                    .map_err(FfiFailure::from)
            }))
        }
    ))
}

#[unsafe(no_mangle)]
/// Starts an asynchronous file delete operation.
///
/// # Safety
///
/// `path`, `out_handle`, `out_error_ptr`, and `out_error_len` must follow the Dhara Storage FFI
/// pointer contracts. `path` must be a valid null-terminated UTF-8 string.
pub unsafe extern "C" fn dhara_operation_start_delete_file(
    path: *const c_char,
    out_handle: *mut *mut NativeOperationHandle,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> DharaStatus {
    ffi_fn!(execute_operation_handle_creation(
        out_handle,
        out_error_ptr,
        out_error_len,
        || {
            let path = parse_path_arg(path, "path")?;
            Ok(spawn_path_operation(move |handle| {
                fail_if_cancelled(&handle)?;
                delete_file(&path)
                    .map(|_| crate::abi::OperationResult::None)
                    .map_err(FfiFailure::from)
            }))
        }
    ))
}

#[unsafe(no_mangle)]
/// Starts an asynchronous raw-byte file read operation.
///
/// # Safety
///
/// `path`, `out_handle`, `out_error_ptr`, and `out_error_len` must follow the Dhara Storage FFI
/// pointer contracts. `path` must be a valid null-terminated UTF-8 string.
pub unsafe extern "C" fn dhara_operation_start_read_file(
    path: *const c_char,
    out_handle: *mut *mut NativeOperationHandle,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> DharaStatus {
    ffi_fn!(execute_operation_handle_creation(
        out_handle,
        out_error_ptr,
        out_error_len,
        || {
            let path = parse_path_arg(path, "path")?;
            Ok(spawn_path_operation(move |handle| {
                FileStorage::from_existing(&path)
                    .map_err(FfiFailure::from)?
                    .read_with_options(ReadOptions {
                        buffer_size: None,
                        progress: Some(progress_reporter(&handle)),
                        cancellation_token: Some(handle.cancellation_token.clone()),
                    })
                    .map(crate::abi::OperationResult::Bytes)
                    .map_err(FfiFailure::from)
            }))
        }
    ))
}

#[unsafe(no_mangle)]
/// Starts an asynchronous UTF-8 file read operation.
///
/// # Safety
///
/// `path`, `out_handle`, `out_error_ptr`, and `out_error_len` must follow the Dhara Storage FFI
/// pointer contracts. `path` must be a valid null-terminated UTF-8 string.
pub unsafe extern "C" fn dhara_operation_start_read_file_text(
    path: *const c_char,
    out_handle: *mut *mut NativeOperationHandle,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> DharaStatus {
    ffi_fn!(execute_operation_handle_creation(
        out_handle,
        out_error_ptr,
        out_error_len,
        || {
            let path = parse_path_arg(path, "path")?;
            Ok(spawn_path_operation(move |handle| {
                let bytes = FileStorage::from_existing(&path)
                    .map_err(FfiFailure::from)?
                    .read_with_options(ReadOptions {
                        buffer_size: None,
                        progress: Some(progress_reporter(&handle)),
                        cancellation_token: Some(handle.cancellation_token.clone()),
                    })
                    .map_err(FfiFailure::from)?;

                let text = String::from_utf8(bytes)
                    .map_err(|_| FfiFailure::error("file contents were not valid UTF-8"))?;

                Ok(crate::abi::OperationResult::String(text))
            }))
        }
    ))
}

#[unsafe(no_mangle)]
/// Starts an asynchronous raw-byte file write operation.
///
/// # Safety
///
/// `path`, `data_ptr`, `out_handle`, `out_error_ptr`, and `out_error_len` must follow the Dhara
/// Storage FFI pointer contracts. `path` must be valid null-terminated UTF-8, and `data_ptr`
/// must reference `data_len` readable bytes when `data_len` is non-zero.
pub unsafe extern "C" fn dhara_operation_start_write_file(
    path: *const c_char,
    data_ptr: *const u8,
    data_len: usize,
    overwrite: u8,
    create_parent_directories: u8,
    out_handle: *mut *mut NativeOperationHandle,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> DharaStatus {
    ffi_fn!(execute_operation_handle_creation(
        out_handle,
        out_error_ptr,
        out_error_len,
        || {
            let path = parse_path_arg(path, "path")?;
            let bytes = parse_bytes_arg(data_ptr, data_len, "data")?.to_vec();
            Ok(spawn_path_operation(move |handle| {
                let file = FileStorage::new(&path).map_err(FfiFailure::from)?;
                let mut cursor = Cursor::new(bytes);
                file.write_from_reader(
                    &mut cursor,
                    write_options(&handle, overwrite != 0, create_parent_directories != 0),
                )
                .map(|file| crate::abi::OperationResult::String(path_to_string(file.path())))
                .map_err(FfiFailure::from)
            }))
        }
    ))
}

#[unsafe(no_mangle)]
/// Starts an asynchronous UTF-8 text file write operation.
///
/// # Safety
///
/// `path`, `text`, `out_handle`, `out_error_ptr`, and `out_error_len` must follow the Dhara
/// Storage FFI pointer contracts. String inputs must be valid null-terminated UTF-8 strings.
pub unsafe extern "C" fn dhara_operation_start_write_file_text(
    path: *const c_char,
    text: *const c_char,
    overwrite: u8,
    create_parent_directories: u8,
    out_handle: *mut *mut NativeOperationHandle,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> DharaStatus {
    ffi_fn!(execute_operation_handle_creation(
        out_handle,
        out_error_ptr,
        out_error_len,
        || {
            let path = parse_path_arg(path, "path")?;
            let text = parse_string_arg(text, "text")?;
            Ok(spawn_path_operation(move |handle| {
                let file = FileStorage::new(&path).map_err(FfiFailure::from)?;
                let mut cursor = Cursor::new(text.into_bytes());
                file.write_from_reader(
                    &mut cursor,
                    write_options(&handle, overwrite != 0, create_parent_directories != 0),
                )
                .map(|file| crate::abi::OperationResult::String(path_to_string(file.path())))
                .map_err(FfiFailure::from)
            }))
        }
    ))
}

#[unsafe(no_mangle)]
/// Starts an asynchronous single-directory create operation.
///
/// # Safety
///
/// `path`, `out_handle`, `out_error_ptr`, and `out_error_len` must follow the Dhara Storage FFI
/// pointer contracts. `path` must be a valid null-terminated UTF-8 string.
pub unsafe extern "C" fn dhara_operation_start_create_directory(
    path: *const c_char,
    out_handle: *mut *mut NativeOperationHandle,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> DharaStatus {
    ffi_fn!(execute_operation_handle_creation(
        out_handle,
        out_error_ptr,
        out_error_len,
        || {
            let path = parse_path_arg(path, "path")?;
            Ok(spawn_path_operation(move |handle| {
                fail_if_cancelled(&handle)?;
                create_directory(&path)
                    .map(|path| crate::abi::OperationResult::String(path_to_string(&path)))
                    .map_err(FfiFailure::from)
            }))
        }
    ))
}

#[unsafe(no_mangle)]
/// Starts an asynchronous recursive directory create operation.
///
/// # Safety
///
/// `path`, `out_handle`, `out_error_ptr`, and `out_error_len` must follow the Dhara Storage FFI
/// pointer contracts. `path` must be a valid null-terminated UTF-8 string.
pub unsafe extern "C" fn dhara_operation_start_create_directory_all(
    path: *const c_char,
    out_handle: *mut *mut NativeOperationHandle,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> DharaStatus {
    ffi_fn!(execute_operation_handle_creation(
        out_handle,
        out_error_ptr,
        out_error_len,
        || {
            let path = parse_path_arg(path, "path")?;
            Ok(spawn_path_operation(move |handle| {
                fail_if_cancelled(&handle)?;
                create_directory_all(&path)
                    .map(|path| crate::abi::OperationResult::String(path_to_string(&path)))
                    .map_err(FfiFailure::from)
            }))
        }
    ))
}

#[unsafe(no_mangle)]
/// Starts an asynchronous directory-tree copy operation.
///
/// # Safety
///
/// `source`, `destination`, `out_handle`, `out_error_ptr`, and `out_error_len` must follow the
/// Dhara Storage FFI pointer contracts. The input strings must be valid null-terminated UTF-8 strings.
pub unsafe extern "C" fn dhara_operation_start_copy_directory(
    source: *const c_char,
    destination: *const c_char,
    overwrite: u8,
    out_handle: *mut *mut NativeOperationHandle,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> DharaStatus {
    ffi_fn!(execute_operation_handle_creation(
        out_handle,
        out_error_ptr,
        out_error_len,
        || {
            let source = parse_path_arg(source, "source")?;
            let destination = parse_path_arg(destination, "destination")?;
            Ok(spawn_path_operation(move |handle| {
                copy_directory_with_options(
                    &source,
                    &destination,
                    transfer_options(&handle, overwrite != 0),
                )
                .map(|path| crate::abi::OperationResult::String(path_to_string(&path)))
                .map_err(FfiFailure::from)
            }))
        }
    ))
}

#[unsafe(no_mangle)]
/// Starts an asynchronous directory-tree move operation.
///
/// # Safety
///
/// `source`, `destination`, `out_handle`, `out_error_ptr`, and `out_error_len` must follow the
/// Dhara Storage FFI pointer contracts. The input strings must be valid null-terminated UTF-8 strings.
pub unsafe extern "C" fn dhara_operation_start_move_directory(
    source: *const c_char,
    destination: *const c_char,
    overwrite: u8,
    out_handle: *mut *mut NativeOperationHandle,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> DharaStatus {
    ffi_fn!(execute_operation_handle_creation(
        out_handle,
        out_error_ptr,
        out_error_len,
        || {
            let source = parse_path_arg(source, "source")?;
            let destination = parse_path_arg(destination, "destination")?;
            Ok(spawn_path_operation(move |handle| {
                move_directory_with_options(
                    &source,
                    &destination,
                    transfer_options(&handle, overwrite != 0),
                )
                .map(|path| crate::abi::OperationResult::String(path_to_string(&path)))
                .map_err(FfiFailure::from)
            }))
        }
    ))
}

#[unsafe(no_mangle)]
/// Starts an asynchronous directory rename operation within the current parent directory.
///
/// # Safety
///
/// `source`, `new_name`, `out_handle`, `out_error_ptr`, and `out_error_len` must follow the Dhara
/// Storage FFI pointer contracts. String inputs must be valid null-terminated UTF-8 strings.
pub unsafe extern "C" fn dhara_operation_start_rename_directory(
    source: *const c_char,
    new_name: *const c_char,
    out_handle: *mut *mut NativeOperationHandle,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> DharaStatus {
    ffi_fn!(execute_operation_handle_creation(
        out_handle,
        out_error_ptr,
        out_error_len,
        || {
            let source = parse_path_arg(source, "source")?;
            let new_name = parse_string_arg(new_name, "new_name")?;
            Ok(spawn_path_operation(move |handle| {
                fail_if_cancelled(&handle)?;
                rename_directory(&source, &new_name)
                    .map(|path| crate::abi::OperationResult::String(path_to_string(&path)))
                    .map_err(FfiFailure::from)
            }))
        }
    ))
}

#[unsafe(no_mangle)]
/// Starts an asynchronous directory delete operation.
///
/// # Safety
///
/// `path`, `out_handle`, `out_error_ptr`, and `out_error_len` must follow the Dhara Storage FFI
/// pointer contracts. `path` must be a valid null-terminated UTF-8 string.
pub unsafe extern "C" fn dhara_operation_start_delete_directory(
    path: *const c_char,
    recursive: u8,
    out_handle: *mut *mut NativeOperationHandle,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> DharaStatus {
    ffi_fn!(execute_operation_handle_creation(
        out_handle,
        out_error_ptr,
        out_error_len,
        || {
            let path = parse_path_arg(path, "path")?;
            Ok(spawn_path_operation(move |handle| {
                delete_directory_with_options(
                    &path,
                    crate::abi::delete_options(&handle, recursive != 0),
                )
                .map(|_| crate::abi::OperationResult::None)
                .map_err(FfiFailure::from)
            }))
        }
    ))
}

#[unsafe(no_mangle)]
/// Reads the latest progress snapshot from a background native operation.
///
/// # Safety
///
/// `handle`, `out_snapshot`, `out_error_ptr`, and `out_error_len` must follow the Dhara Storage
/// FFI pointer contracts. `out_snapshot` must point to writable memory for one snapshot value.
pub unsafe extern "C" fn dhara_operation_get_snapshot(
    handle: *mut NativeOperationHandle,
    out_snapshot: *mut DharaOperationSnapshot,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> DharaStatus {
    ffi_fn!({
        if out_snapshot.is_null() {
            return write_error_only(
                out_error_ptr,
                out_error_len,
                FfiFailure::invalid_argument(
                    "out_snapshot",
                    "snapshot output pointer must not be null",
                ),
            );
        }
        if let Err(failure) = validate_buffer_out(out_error_ptr, out_error_len) {
            return failure.status;
        }
        reset_buffer_out(out_error_ptr, out_error_len);

        match clone_operation_handle(handle) {
            Ok(handle) => {
                *out_snapshot = handle.snapshot();
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
/// Requests cooperative cancellation for a background native operation.
///
/// # Safety
///
/// `handle`, `out_error_ptr`, and `out_error_len` must follow the Dhara Storage FFI pointer contracts.
pub unsafe extern "C" fn dhara_operation_cancel(
    handle: *mut NativeOperationHandle,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> DharaStatus {
    ffi_fn!(execute_unit(out_error_ptr, out_error_len, || {
        let handle = clone_operation_handle(handle)?;
        handle.cancellation_token.cancel();
        Ok(())
    }))
}

#[unsafe(no_mangle)]
/// Takes the string result produced by a completed background operation.
///
/// # Safety
///
/// `handle`, `out_string_ptr`, `out_string_len`, `out_error_ptr`, and `out_error_len` must follow
/// the Dhara Storage FFI pointer contracts.
pub unsafe extern "C" fn dhara_operation_take_string_result(
    handle: *mut NativeOperationHandle,
    out_string_ptr: *mut *mut u8,
    out_string_len: *mut usize,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> DharaStatus {
    ffi_fn!(execute_string(
        out_string_ptr,
        out_string_len,
        out_error_ptr,
        out_error_len,
        || {
            let handle = clone_operation_handle(handle)?;
            if let Some(failure) = handle.clone_error() {
                return Err(failure);
            }
            Ok(handle.take_string_result()?.unwrap_or_default())
        }
    ))
}

#[unsafe(no_mangle)]
/// Takes the byte result produced by a completed background operation.
///
/// # Safety
///
/// `handle`, `out_bytes_ptr`, `out_bytes_len`, `out_error_ptr`, and `out_error_len` must follow
/// the Dhara Storage FFI pointer contracts.
pub unsafe extern "C" fn dhara_operation_take_bytes_result(
    handle: *mut NativeOperationHandle,
    out_bytes_ptr: *mut *mut u8,
    out_bytes_len: *mut usize,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> DharaStatus {
    ffi_fn!(execute_bytes(
        out_bytes_ptr,
        out_bytes_len,
        out_error_ptr,
        out_error_len,
        || {
            let handle = clone_operation_handle(handle)?;
            if let Some(failure) = handle.clone_error() {
                return Err(failure);
            }
            Ok(handle.take_bytes_result()?.unwrap_or_default())
        }
    ))
}

#[unsafe(no_mangle)]
/// Retrieves the last error payload recorded for a background operation.
///
/// # Safety
///
/// `handle`, `out_json_ptr`, `out_json_len`, `out_error_ptr`, and `out_error_len` must follow
/// the Dhara Storage FFI pointer contracts.
pub unsafe extern "C" fn dhara_operation_get_error(
    handle: *mut NativeOperationHandle,
    out_json_ptr: *mut *mut u8,
    out_json_len: *mut usize,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> DharaStatus {
    ffi_fn!(execute_json(
        out_json_ptr,
        out_json_len,
        out_error_ptr,
        out_error_len,
        || {
            let handle = clone_operation_handle(handle)?;
            Ok(handle.clone_error().map(|failure| failure.payload))
        }
    ))
}

#[unsafe(no_mangle)]
/// Frees a background native operation handle and joins its worker thread if necessary.
///
/// # Safety
///
/// `handle` must either be null or a pointer previously returned by a Dhara Storage operation-start
/// function. The pointer must not be freed more than once.
pub unsafe extern "C" fn dhara_operation_free(handle: *mut NativeOperationHandle) {
    if handle.is_null() {
        return;
    }

    let handle = Arc::from_raw(handle);
    handle.cancellation_token.cancel();
    if let Ok(mut slot) = handle.worker.lock()
        && let Some(worker) = slot.take()
    {
        let _ = worker.join();
    }
}
