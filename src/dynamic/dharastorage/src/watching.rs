use std::ffi::c_char;
use std::ptr;
use std::time::Duration;

use dhara_storage::{DirectoryStorage, StorageChangeEvent, StorageError, StorageWatchConfig};

use crate::abi::{DharaStatus, NativeWatchHandle, with_watch_handle};
use crate::errors::FfiFailure;
use crate::marshal::{
    execute_json, execute_unit, parse_path_arg, reset_buffer_out, validate_buffer_out,
    write_error_only, write_error_payload,
};
use crate::models::StorageChangeEventDto;

#[unsafe(no_mangle)]
/// Creates a directory-watch handle for debounced filesystem notifications.
///
/// # Safety
///
/// `path`, `out_handle`, `out_error_ptr`, and `out_error_len` must follow the Dhara Storage FFI
/// pointer contracts. `path` must be a valid null-terminated UTF-8 string.
pub unsafe extern "C" fn dhara_watch_create(
    path: *const c_char,
    recursive: u8,
    debounce_window_ms: u64,
    out_handle: *mut *mut NativeWatchHandle,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> DharaStatus {
    ffi_fn!({
        if out_handle.is_null() {
            return write_error_only(
                out_error_ptr,
                out_error_len,
                FfiFailure::invalid_argument("out_handle", "watch handle output must not be null"),
            );
        }
        if let Err(failure) = validate_buffer_out(out_error_ptr, out_error_len) {
            return failure.status;
        }
        *out_handle = ptr::null_mut();
        reset_buffer_out(out_error_ptr, out_error_len);

        let result = (|| {
            let path = parse_path_arg(path, "path")?;
            let config = StorageWatchConfig {
                recursive: recursive != 0,
                debounce_window: Duration::from_millis(debounce_window_ms.max(1)),
            };
            let watch = DirectoryStorage::from_existing(&path)
                .map_err(FfiFailure::from)?
                .watch(config)
                .map_err(FfiFailure::from)?;
            Ok(Box::into_raw(Box::new(NativeWatchHandle {
                handle: std::sync::Mutex::new(Some(watch)),
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
/// Attempts to receive the next watch event without blocking and returns JSON when one is available.
///
/// # Safety
///
/// `handle`, `out_json_ptr`, `out_json_len`, `out_error_ptr`, and `out_error_len` must follow
/// the Dhara Storage FFI pointer contracts.
pub unsafe extern "C" fn dhara_watch_try_recv_json(
    handle: *mut NativeWatchHandle,
    out_json_ptr: *mut *mut u8,
    out_json_len: *mut usize,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> DharaStatus {
    ffi_fn!(execute_watch_receive(
        handle,
        out_json_ptr,
        out_json_len,
        out_error_ptr,
        out_error_len,
        |watch| watch.try_recv(),
    ))
}

#[unsafe(no_mangle)]
/// Blocks until the next watch event is available and returns it as JSON.
///
/// # Safety
///
/// `handle`, `out_json_ptr`, `out_json_len`, `out_error_ptr`, and `out_error_len` must follow
/// the Dhara Storage FFI pointer contracts.
pub unsafe extern "C" fn dhara_watch_recv_json(
    handle: *mut NativeWatchHandle,
    out_json_ptr: *mut *mut u8,
    out_json_len: *mut usize,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> DharaStatus {
    ffi_fn!(execute_watch_receive(
        handle,
        out_json_ptr,
        out_json_len,
        out_error_ptr,
        out_error_len,
        |watch| watch.recv().map(Some),
    ))
}

#[unsafe(no_mangle)]
/// Waits up to `timeout_ms` for the next watch event and returns it as JSON when available.
///
/// # Safety
///
/// `handle`, `out_json_ptr`, `out_json_len`, `out_error_ptr`, and `out_error_len` must follow
/// the Dhara Storage FFI pointer contracts.
pub unsafe extern "C" fn dhara_watch_recv_json_timeout(
    handle: *mut NativeWatchHandle,
    timeout_ms: u64,
    out_json_ptr: *mut *mut u8,
    out_json_len: *mut usize,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> DharaStatus {
    ffi_fn!(execute_watch_receive(
        handle,
        out_json_ptr,
        out_json_len,
        out_error_ptr,
        out_error_len,
        |watch| watch.recv_timeout(Duration::from_millis(timeout_ms)),
    ))
}

#[unsafe(no_mangle)]
/// Stops an active directory-watch handle.
///
/// # Safety
///
/// `handle`, `out_error_ptr`, and `out_error_len` must follow the Dhara Storage FFI pointer contracts.
pub unsafe extern "C" fn dhara_watch_stop(
    handle: *mut NativeWatchHandle,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> DharaStatus {
    ffi_fn!(execute_unit(out_error_ptr, out_error_len, || {
        with_watch_handle(handle, |watch_handle| {
            let mut slot = watch_handle
                .handle
                .lock()
                .unwrap_or_else(|p| p.into_inner());
            slot.take();
            Ok(())
        })
    }))
}

#[unsafe(no_mangle)]
/// Frees a native watch handle.
///
/// # Safety
///
/// `handle` must either be null or a pointer previously returned by `dhara_watch_create`. The
/// pointer must not be freed more than once.
pub unsafe extern "C" fn dhara_watch_free(handle: *mut NativeWatchHandle) {
    if handle.is_null() {
        return;
    }

    let handle = Box::from_raw(handle);
    if let Ok(mut slot) = handle.handle.lock() {
        slot.take();
    }
}

unsafe fn execute_watch_receive(
    handle: *mut NativeWatchHandle,
    out_json_ptr: *mut *mut u8,
    out_json_len: *mut usize,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
    receive: impl FnOnce(
        &dhara_storage::DirectoryWatchHandle,
    ) -> Result<Option<StorageChangeEvent>, StorageError>,
) -> DharaStatus {
    execute_json(
        out_json_ptr,
        out_json_len,
        out_error_ptr,
        out_error_len,
        || {
            with_watch_handle(handle, |watch_handle| {
                let slot = watch_handle
                    .handle
                    .lock()
                    .unwrap_or_else(|p| p.into_inner());
                let watch = slot
                    .as_ref()
                    .ok_or_else(|| FfiFailure::error("watch handle has already been stopped"))?;
                Ok(receive(watch)
                    .map_err(FfiFailure::from)?
                    .map(StorageChangeEventDto::from))
            })
        },
    )
}
