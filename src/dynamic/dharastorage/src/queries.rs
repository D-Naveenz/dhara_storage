use std::ffi::c_char;
use std::path::{Path, PathBuf};

use dhara_storage::{
    DirectoryInfo, FileInfo, analyze_path, copy_directory, copy_file, create_directory,
    create_directory_all, delete_directory, delete_file, move_directory, move_file, read_file,
    read_file_to_string, rename_directory, rename_file, write_file, write_file_string,
};

use crate::abi::DharaStatus;
use crate::errors::FfiFailure;
use crate::marshal::{
    execute_bytes, execute_json, execute_string, execute_unit, parse_bytes_arg, parse_path_arg,
    parse_string_arg,
};
use crate::models::{
    AnalysisReportDto, DirectoryInfoDto, EntryKind, FileInfoDto, list_entries_json, path_to_string,
};

#[unsafe(no_mangle)]
/// Analyzes a file path immediately and returns a JSON report.
///
/// # Safety
///
/// `path`, `out_json_ptr`, `out_json_len`, `out_error_ptr`, and `out_error_len` must follow the
/// Dhara Storage FFI pointer contracts. `path` must be a valid null-terminated UTF-8 string.
pub unsafe extern "C" fn dhara_analyze_path(
    path: *const c_char,
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
            let path = parse_path_arg(path, "path")?;
            let report = analyze_path(&path).map_err(FfiFailure::from)?;
            Ok(AnalysisReportDto::from(report))
        }
    ))
}

#[unsafe(no_mangle)]
/// Reads file metadata and optionally content-analysis data, returning JSON.
///
/// # Safety
///
/// `path`, `out_json_ptr`, `out_json_len`, `out_error_ptr`, and `out_error_len` must follow the
/// Dhara Storage FFI pointer contracts. `path` must be a valid null-terminated UTF-8 string.
pub unsafe extern "C" fn dhara_get_file_info(
    path: *const c_char,
    include_analysis: u8,
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
            let path = parse_path_arg(path, "path")?;
            let info = if include_analysis != 0 {
                FileInfo::from_path_with_analysis(&path)
            } else {
                FileInfo::from_path(&path)
            }
            .map_err(FfiFailure::from)?;

            FileInfoDto::try_from_info(info, include_analysis != 0)
        }
    ))
}

#[unsafe(no_mangle)]
/// Reads directory metadata and optionally directory-summary data, returning JSON.
///
/// # Safety
///
/// `path`, `out_json_ptr`, `out_json_len`, `out_error_ptr`, and `out_error_len` must follow the
/// Dhara Storage FFI pointer contracts. `path` must be a valid null-terminated UTF-8 string.
pub unsafe extern "C" fn dhara_get_directory_info(
    path: *const c_char,
    include_summary: u8,
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
            let path = parse_path_arg(path, "path")?;
            let info = if include_summary != 0 {
                DirectoryInfo::from_path_with_summary(&path)
            } else {
                DirectoryInfo::from_path(&path)
            }
            .map_err(FfiFailure::from)?;

            DirectoryInfoDto::try_from_info(info, include_summary != 0)
        }
    ))
}

#[unsafe(no_mangle)]
/// Lists child files for a directory and returns the result as JSON.
///
/// # Safety
///
/// `path`, `out_json_ptr`, `out_json_len`, `out_error_ptr`, and `out_error_len` must follow the
/// Dhara Storage FFI pointer contracts. `path` must be a valid null-terminated UTF-8 string.
pub unsafe extern "C" fn dhara_list_files(
    path: *const c_char,
    recursive: u8,
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
            let path = parse_path_arg(path, "path")?;
            list_entries_json(&path, recursive != 0, EntryKind::Files)
        }
    ))
}

#[unsafe(no_mangle)]
/// Lists child directories for a directory and returns the result as JSON.
///
/// # Safety
///
/// `path`, `out_json_ptr`, `out_json_len`, `out_error_ptr`, and `out_error_len` must follow the
/// Dhara Storage FFI pointer contracts. `path` must be a valid null-terminated UTF-8 string.
pub unsafe extern "C" fn dhara_list_directories(
    path: *const c_char,
    recursive: u8,
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
            let path = parse_path_arg(path, "path")?;
            list_entries_json(&path, recursive != 0, EntryKind::Directories)
        }
    ))
}

#[unsafe(no_mangle)]
/// Lists both files and directories for a directory and returns the result as JSON.
///
/// # Safety
///
/// `path`, `out_json_ptr`, `out_json_len`, `out_error_ptr`, and `out_error_len` must follow the
/// Dhara Storage FFI pointer contracts. `path` must be a valid null-terminated UTF-8 string.
pub unsafe extern "C" fn dhara_list_entries(
    path: *const c_char,
    recursive: u8,
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
            let path = parse_path_arg(path, "path")?;
            list_entries_json(&path, recursive != 0, EntryKind::All)
        }
    ))
}

#[unsafe(no_mangle)]
/// Reads the full file contents into an owned byte buffer.
///
/// # Safety
///
/// `path`, `out_bytes_ptr`, `out_bytes_len`, `out_error_ptr`, and `out_error_len` must follow
/// the Dhara Storage FFI pointer contracts. `path` must be a valid null-terminated UTF-8 string.
pub unsafe extern "C" fn dhara_read_file(
    path: *const c_char,
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
            let path = parse_path_arg(path, "path")?;
            read_file(&path).map_err(FfiFailure::from)
        }
    ))
}

#[unsafe(no_mangle)]
/// Reads the full file contents into an owned UTF-8 string buffer.
///
/// # Safety
///
/// `path`, `out_string_ptr`, `out_string_len`, `out_error_ptr`, and `out_error_len` must follow
/// the Dhara Storage FFI pointer contracts. `path` must be a valid null-terminated UTF-8 string.
pub unsafe extern "C" fn dhara_read_file_text(
    path: *const c_char,
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
            let path = parse_path_arg(path, "path")?;
            read_file_to_string(&path).map_err(FfiFailure::from)
        },
    ))
}

#[unsafe(no_mangle)]
/// Writes an in-memory byte buffer to a file and returns the resulting path.
///
/// # Safety
///
/// `path`, `data_ptr`, `out_path_ptr`, `out_path_len`, `out_error_ptr`, and `out_error_len`
/// must follow the Dhara Storage FFI pointer contracts. `path` must be valid null-terminated UTF-8,
/// and `data_ptr` must reference `data_len` readable bytes when `data_len` is non-zero.
pub unsafe extern "C" fn dhara_write_file(
    path: *const c_char,
    data_ptr: *const u8,
    data_len: usize,
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
        || {
            let path = parse_path_arg(path, "path")?;
            let bytes = parse_bytes_arg(data_ptr, data_len, "data")?;
            let written = write_file(&path, bytes).map_err(FfiFailure::from)?;
            Ok(path_to_string(&written))
        }
    ))
}

#[unsafe(no_mangle)]
/// Writes UTF-8 text to a file and returns the resulting path.
///
/// # Safety
///
/// `path`, `text`, `out_path_ptr`, `out_path_len`, `out_error_ptr`, and `out_error_len` must
/// follow the Dhara Storage FFI pointer contracts. String inputs must be valid null-terminated UTF-8 strings.
pub unsafe extern "C" fn dhara_write_file_text(
    path: *const c_char,
    text: *const c_char,
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
        || {
            let path = parse_path_arg(path, "path")?;
            let text = parse_string_arg(text, "text")?;
            let written = write_file_string(&path, &text).map_err(FfiFailure::from)?;
            Ok(path_to_string(&written))
        }
    ))
}

#[unsafe(no_mangle)]
/// Copies a file synchronously and returns the destination path.
///
/// # Safety
///
/// `source`, `destination`, `out_path_ptr`, `out_path_len`, `out_error_ptr`, and `out_error_len`
/// must follow the Dhara Storage FFI pointer contracts. String inputs must be valid null-terminated UTF-8 strings.
pub unsafe extern "C" fn dhara_copy_file(
    source: *const c_char,
    destination: *const c_char,
    out_path_ptr: *mut *mut u8,
    out_path_len: *mut usize,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> DharaStatus {
    ffi_fn!(execute_path_operation(
        source,
        destination,
        out_path_ptr,
        out_path_len,
        out_error_ptr,
        out_error_len,
        |source, destination| copy_file(source, destination),
    ))
}

#[unsafe(no_mangle)]
/// Moves a file synchronously and returns the destination path.
///
/// # Safety
///
/// `source`, `destination`, `out_path_ptr`, `out_path_len`, `out_error_ptr`, and `out_error_len`
/// must follow the Dhara Storage FFI pointer contracts. String inputs must be valid null-terminated UTF-8 strings.
pub unsafe extern "C" fn dhara_move_file(
    source: *const c_char,
    destination: *const c_char,
    out_path_ptr: *mut *mut u8,
    out_path_len: *mut usize,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> DharaStatus {
    ffi_fn!(execute_path_operation(
        source,
        destination,
        out_path_ptr,
        out_path_len,
        out_error_ptr,
        out_error_len,
        |source, destination| move_file(source, destination),
    ))
}

#[unsafe(no_mangle)]
/// Renames a file synchronously and returns the resulting path.
///
/// # Safety
///
/// `source`, `new_name`, `out_path_ptr`, `out_path_len`, `out_error_ptr`, and `out_error_len`
/// must follow the Dhara Storage FFI pointer contracts. String inputs must be valid null-terminated UTF-8 strings.
pub unsafe extern "C" fn dhara_rename_file(
    source: *const c_char,
    new_name: *const c_char,
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
        || {
            let source = parse_path_arg(source, "source")?;
            let new_name = parse_string_arg(new_name, "new_name")?;
            let renamed = rename_file(&source, &new_name).map_err(FfiFailure::from)?;
            Ok(path_to_string(&renamed))
        }
    ))
}

#[unsafe(no_mangle)]
/// Deletes a file synchronously.
///
/// # Safety
///
/// `path`, `out_error_ptr`, and `out_error_len` must follow the Dhara Storage FFI pointer contracts.
/// `path` must be a valid null-terminated UTF-8 string.
pub unsafe extern "C" fn dhara_delete_file(
    path: *const c_char,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> DharaStatus {
    ffi_fn!(execute_unit(out_error_ptr, out_error_len, || {
        let path = parse_path_arg(path, "path")?;
        delete_file(&path).map_err(FfiFailure::from)
    }))
}

#[unsafe(no_mangle)]
/// Creates a directory synchronously and returns the resulting path.
///
/// # Safety
///
/// `path`, `out_path_ptr`, `out_path_len`, `out_error_ptr`, and `out_error_len` must follow the
/// Dhara Storage FFI pointer contracts. `path` must be a valid null-terminated UTF-8 string.
pub unsafe extern "C" fn dhara_create_directory(
    path: *const c_char,
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
        || {
            let path = parse_path_arg(path, "path")?;
            let created = create_directory(&path).map_err(FfiFailure::from)?;
            Ok(path_to_string(&created))
        }
    ))
}

#[unsafe(no_mangle)]
/// Creates a directory and any missing parents synchronously, returning the resulting path.
///
/// # Safety
///
/// `path`, `out_path_ptr`, `out_path_len`, `out_error_ptr`, and `out_error_len` must follow the
/// Dhara Storage FFI pointer contracts. `path` must be a valid null-terminated UTF-8 string.
pub unsafe extern "C" fn dhara_create_directory_all(
    path: *const c_char,
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
        || {
            let path = parse_path_arg(path, "path")?;
            let created = create_directory_all(&path).map_err(FfiFailure::from)?;
            Ok(path_to_string(&created))
        }
    ))
}

#[unsafe(no_mangle)]
/// Copies a directory tree synchronously and returns the destination path.
///
/// # Safety
///
/// `source`, `destination`, `out_path_ptr`, `out_path_len`, `out_error_ptr`, and `out_error_len`
/// must follow the Dhara Storage FFI pointer contracts. String inputs must be valid null-terminated UTF-8 strings.
pub unsafe extern "C" fn dhara_copy_directory(
    source: *const c_char,
    destination: *const c_char,
    out_path_ptr: *mut *mut u8,
    out_path_len: *mut usize,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> DharaStatus {
    ffi_fn!(execute_path_operation(
        source,
        destination,
        out_path_ptr,
        out_path_len,
        out_error_ptr,
        out_error_len,
        |source, destination| copy_directory(source, destination),
    ))
}

#[unsafe(no_mangle)]
/// Moves a directory tree synchronously and returns the destination path.
///
/// # Safety
///
/// `source`, `destination`, `out_path_ptr`, `out_path_len`, `out_error_ptr`, and `out_error_len`
/// must follow the Dhara Storage FFI pointer contracts. String inputs must be valid null-terminated UTF-8 strings.
pub unsafe extern "C" fn dhara_move_directory(
    source: *const c_char,
    destination: *const c_char,
    out_path_ptr: *mut *mut u8,
    out_path_len: *mut usize,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> DharaStatus {
    ffi_fn!(execute_path_operation(
        source,
        destination,
        out_path_ptr,
        out_path_len,
        out_error_ptr,
        out_error_len,
        |source, destination| move_directory(source, destination),
    ))
}

#[unsafe(no_mangle)]
/// Renames a directory synchronously and returns the resulting path.
///
/// # Safety
///
/// `source`, `new_name`, `out_path_ptr`, `out_path_len`, `out_error_ptr`, and `out_error_len`
/// must follow the Dhara Storage FFI pointer contracts. String inputs must be valid null-terminated UTF-8 strings.
pub unsafe extern "C" fn dhara_rename_directory(
    source: *const c_char,
    new_name: *const c_char,
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
        || {
            let source = parse_path_arg(source, "source")?;
            let new_name = parse_string_arg(new_name, "new_name")?;
            let renamed = rename_directory(&source, &new_name).map_err(FfiFailure::from)?;
            Ok(path_to_string(&renamed))
        }
    ))
}

#[unsafe(no_mangle)]
/// Deletes a directory synchronously.
///
/// # Safety
///
/// `path`, `out_error_ptr`, and `out_error_len` must follow the Dhara Storage FFI pointer contracts.
/// `path` must be a valid null-terminated UTF-8 string.
pub unsafe extern "C" fn dhara_delete_directory(
    path: *const c_char,
    recursive: u8,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> DharaStatus {
    ffi_fn!(execute_unit(out_error_ptr, out_error_len, || {
        let path = parse_path_arg(path, "path")?;
        if recursive == 0 {
            return Err(FfiFailure::invalid_argument(
                "recursive",
                "delete_directory requires recursive=1 in v1",
            ));
        }

        delete_directory(&path).map_err(FfiFailure::from)
    }))
}

unsafe fn execute_path_operation(
    source: *const c_char,
    destination: *const c_char,
    out_path_ptr: *mut *mut u8,
    out_path_len: *mut usize,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
    operation: impl FnOnce(&Path, &Path) -> Result<PathBuf, dhara_storage::StorageError>,
) -> DharaStatus {
    execute_string(
        out_path_ptr,
        out_path_len,
        out_error_ptr,
        out_error_len,
        || {
            let source = parse_path_arg(source, "source")?;
            let destination = parse_path_arg(destination, "destination")?;
            let result = operation(&source, &destination).map_err(FfiFailure::from)?;
            Ok(path_to_string(&result))
        },
    )
}
