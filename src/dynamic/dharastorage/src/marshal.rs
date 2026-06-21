use std::ffi::{CStr, c_char};
use std::path::PathBuf;
use std::ptr;
use std::slice;

use serde::Serialize;

use crate::abi::DharaStatus;
use crate::abi::NativeOperationHandle;
use crate::errors::FfiFailure;

pub(crate) unsafe fn execute_json<T: Serialize>(
    out_json_ptr: *mut *mut u8,
    out_json_len: *mut usize,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
    operation: impl FnOnce() -> Result<T, FfiFailure>,
) -> DharaStatus {
    execute_bytes_common(
        out_json_ptr,
        out_json_len,
        out_error_ptr,
        out_error_len,
        || {
            let value = operation()?;
            serde_json::to_vec(&value).map_err(|err| FfiFailure::error(err.to_string()))
        },
    )
}

pub(crate) unsafe fn execute_string(
    out_string_ptr: *mut *mut u8,
    out_string_len: *mut usize,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
    operation: impl FnOnce() -> Result<String, FfiFailure>,
) -> DharaStatus {
    execute_bytes_common(
        out_string_ptr,
        out_string_len,
        out_error_ptr,
        out_error_len,
        || operation().map(String::into_bytes),
    )
}

pub(crate) unsafe fn execute_bytes(
    out_bytes_ptr: *mut *mut u8,
    out_bytes_len: *mut usize,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
    operation: impl FnOnce() -> Result<Vec<u8>, FfiFailure>,
) -> DharaStatus {
    execute_bytes_common(
        out_bytes_ptr,
        out_bytes_len,
        out_error_ptr,
        out_error_len,
        operation,
    )
}

pub(crate) unsafe fn execute_bytes_common(
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
    operation: impl FnOnce() -> Result<Vec<u8>, FfiFailure>,
) -> DharaStatus {
    if let Err(failure) = validate_buffer_out(out_ptr, out_len) {
        return write_error_only(out_error_ptr, out_error_len, failure);
    }

    if let Err(failure) = validate_buffer_out(out_error_ptr, out_error_len) {
        return failure.status;
    }

    reset_buffer_out(out_ptr, out_len);
    reset_buffer_out(out_error_ptr, out_error_len);

    match operation() {
        Ok(bytes) => {
            write_buffer(out_ptr, out_len, bytes);
            DharaStatus::Ok
        }
        Err(failure) => {
            write_error_payload(out_error_ptr, out_error_len, &failure);
            failure.status
        }
    }
}

pub(crate) unsafe fn execute_unit(
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
    operation: impl FnOnce() -> Result<(), FfiFailure>,
) -> DharaStatus {
    if let Err(failure) = validate_buffer_out(out_error_ptr, out_error_len) {
        return failure.status;
    }

    reset_buffer_out(out_error_ptr, out_error_len);

    match operation() {
        Ok(()) => DharaStatus::Ok,
        Err(failure) => {
            write_error_payload(out_error_ptr, out_error_len, &failure);
            failure.status
        }
    }
}

pub(crate) unsafe fn execute_operation_handle_creation(
    out_handle: *mut *mut NativeOperationHandle,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
    operation: impl FnOnce() -> Result<*mut NativeOperationHandle, FfiFailure>,
) -> DharaStatus {
    if out_handle.is_null() {
        return write_error_only(
            out_error_ptr,
            out_error_len,
            FfiFailure::invalid_argument("out_handle", "output handle pointer must not be null"),
        );
    }

    if let Err(failure) = validate_buffer_out(out_error_ptr, out_error_len) {
        return failure.status;
    }

    *out_handle = ptr::null_mut();
    reset_buffer_out(out_error_ptr, out_error_len);

    match operation() {
        Ok(handle) => {
            *out_handle = handle;
            DharaStatus::Ok
        }
        Err(failure) => {
            write_error_payload(out_error_ptr, out_error_len, &failure);
            failure.status
        }
    }
}

pub(crate) unsafe fn validate_buffer_out(
    ptr_out: *mut *mut u8,
    len_out: *mut usize,
) -> Result<(), FfiFailure> {
    if ptr_out.is_null() {
        return Err(FfiFailure::invalid_argument(
            "out_ptr",
            "output pointer argument must not be null",
        ));
    }
    if len_out.is_null() {
        return Err(FfiFailure::invalid_argument(
            "out_len",
            "output length argument must not be null",
        ));
    }
    Ok(())
}

pub(crate) unsafe fn reset_buffer_out(ptr_out: *mut *mut u8, len_out: *mut usize) {
    *ptr_out = ptr::null_mut();
    *len_out = 0;
}

pub(crate) unsafe fn write_buffer(ptr_out: *mut *mut u8, len_out: *mut usize, bytes: Vec<u8>) {
    let boxed = bytes.into_boxed_slice();
    let len = boxed.len();
    let ptr = Box::into_raw(boxed) as *mut u8;
    *ptr_out = ptr;
    *len_out = len;
}

pub(crate) unsafe fn write_error_only(
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
    failure: FfiFailure,
) -> DharaStatus {
    if out_error_ptr.is_null() || out_error_len.is_null() {
        return failure.status;
    }

    reset_buffer_out(out_error_ptr, out_error_len);
    write_error_payload(out_error_ptr, out_error_len, &failure);
    failure.status
}

pub(crate) unsafe fn write_error_payload(
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
    failure: &FfiFailure,
) {
    let payload = serde_json::to_vec(&failure.payload).unwrap_or_else(|_| {
        br#"{"code":"ffi_error","message":"failed to serialize error"}"#.to_vec()
    });
    write_buffer(out_error_ptr, out_error_len, payload);
}

pub(crate) unsafe fn free_boxed_bytes(ptr: *mut u8, len: usize) {
    if ptr.is_null() {
        return;
    }

    let slice_ptr = ptr::slice_from_raw_parts_mut(ptr, len);
    drop(Box::from_raw(slice_ptr));
}

pub(crate) fn parse_path_arg(
    value: *const c_char,
    field: &'static str,
) -> Result<PathBuf, FfiFailure> {
    Ok(PathBuf::from(parse_string_arg(value, field)?))
}

pub(crate) fn parse_string_arg(
    value: *const c_char,
    field: &'static str,
) -> Result<String, FfiFailure> {
    if value.is_null() {
        return Err(FfiFailure::invalid_argument(
            field,
            "string pointer must not be null",
        ));
    }

    let value = unsafe { CStr::from_ptr(value) };
    let bytes = value.to_bytes();
    if bytes.is_empty() {
        return Err(FfiFailure::invalid_argument(
            field,
            "string value must not be empty",
        ));
    }

    String::from_utf8(bytes.to_vec())
        .map_err(|_| FfiFailure::invalid_argument(field, "string value must be valid UTF-8"))
}

pub(crate) fn parse_bytes_arg<'a>(
    data_ptr: *const u8,
    data_len: usize,
    field: &'static str,
) -> Result<&'a [u8], FfiFailure> {
    if data_len == 0 {
        return Ok(&[]);
    }
    if data_ptr.is_null() {
        return Err(FfiFailure::invalid_argument(
            field,
            "byte pointer must not be null",
        ));
    }

    Ok(unsafe { slice::from_raw_parts(data_ptr, data_len) })
}
