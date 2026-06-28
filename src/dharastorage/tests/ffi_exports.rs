use std::ffi::CString;
use std::ptr;
use std::slice;

use dharastorage::{
    DharaStatus, NativeAnalysisReport, NativeFileInformation, NativeStorageEntryList,
    dhara_analysis_report_free, dhara_analyze_path, dhara_bytes_free, dhara_create_directory_all,
    dhara_delete_file, dhara_file_info_free, dhara_get_file_info, dhara_list_entries,
    dhara_read_file, dhara_rename_file, dhara_storage_entry_list_free, dhara_string_free,
    dhara_write_file_text,
};
#[allow(deprecated)]
use dharastorage::{
    dhara_analyze_path_json_old, dhara_get_directory_info_json_old, dhara_get_file_info_json_old,
    dhara_list_entries_json_old,
};
use tempfile::tempdir;

fn fixture_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("core")
        .join("dhara_storage")
        .join("tests")
        .join("fixtures")
        .join("sample-2.pdf")
}

fn string_from_output(ptr: *mut u8, len: usize) -> String {
    let bytes = unsafe { slice::from_raw_parts(ptr, len) };
    let value = String::from_utf8(bytes.to_vec()).expect("ffi output should be valid utf-8");
    unsafe { dhara_string_free(ptr, len) };
    value
}

fn bytes_from_output(ptr: *mut u8, len: usize) -> Vec<u8> {
    let bytes = unsafe { slice::from_raw_parts(ptr, len).to_vec() };
    unsafe { dhara_bytes_free(ptr, len) };
    bytes
}

#[test]
#[allow(deprecated)]
fn analyze_path_json_old_returns_legacy_json() {
    let fixture = std::fs::canonicalize(fixture_path()).unwrap();
    let fixture = CString::new(fixture.to_string_lossy().as_bytes()).unwrap();
    let mut out_ptr: *mut u8 = ptr::null_mut();
    let mut out_len = 0;
    let mut err_ptr: *mut u8 = ptr::null_mut();
    let mut err_len = 0;

    let status = unsafe {
        dhara_analyze_path_json_old(
            fixture.as_ptr(),
            &mut out_ptr,
            &mut out_len,
            &mut err_ptr,
            &mut err_len,
        )
    };

    assert_eq!(status, DharaStatus::Ok);
    assert!(err_ptr.is_null());
    let json = string_from_output(out_ptr, out_len);
    assert!(json.contains("\"top_mime_type\":\"application/pdf\""));
}

#[test]
fn analyze_path_returns_typed_report() {
    let fixture = std::fs::canonicalize(fixture_path()).unwrap();
    let fixture = CString::new(fixture.to_string_lossy().as_bytes()).unwrap();
    let mut report: *mut NativeAnalysisReport = ptr::null_mut();
    let mut err_ptr: *mut u8 = ptr::null_mut();
    let mut err_len = 0;

    let status =
        unsafe { dhara_analyze_path(fixture.as_ptr(), &mut report, &mut err_ptr, &mut err_len) };

    assert_eq!(status, DharaStatus::Ok);
    assert!(err_ptr.is_null());
    assert!(!report.is_null());
    let report_ref = unsafe { &*report };
    assert!(report_ref.matches_len > 0);
    assert!(report_ref.file_size > 0);
    assert!(!report_ref.matches_ptr.is_null());
    unsafe { dhara_analysis_report_free(report) };
}

#[test]
fn file_info_returns_optional_analysis_pointer() {
    let fixture = std::fs::canonicalize(fixture_path()).unwrap();
    let fixture = CString::new(fixture.to_string_lossy().as_bytes()).unwrap();
    let mut info: *mut NativeFileInformation = ptr::null_mut();
    let mut err_ptr: *mut u8 = ptr::null_mut();
    let mut err_len = 0;

    let status =
        unsafe { dhara_get_file_info(fixture.as_ptr(), 1, &mut info, &mut err_ptr, &mut err_len) };

    assert_eq!(status, DharaStatus::Ok);
    assert!(err_ptr.is_null());
    assert!(!info.is_null());
    let info_ref = unsafe { &*info };
    assert!(info_ref.size > 0);
    assert!(!info_ref.analysis.is_null());
    unsafe { dhara_file_info_free(info) };
}

#[test]
#[allow(deprecated)]
fn file_info_and_directory_info_include_optional_payloads() {
    let fixture = std::fs::canonicalize(fixture_path()).unwrap();
    let fixture = CString::new(fixture.to_string_lossy().as_bytes()).unwrap();
    let temp = tempdir().unwrap();
    let temp_c = CString::new(temp.path().to_string_lossy().as_bytes()).unwrap();

    let mut out_ptr: *mut u8 = ptr::null_mut();
    let mut out_len = 0;
    let mut err_ptr: *mut u8 = ptr::null_mut();
    let mut err_len = 0;

    let file_status = unsafe {
        dhara_get_file_info_json_old(
            fixture.as_ptr(),
            1,
            &mut out_ptr,
            &mut out_len,
            &mut err_ptr,
            &mut err_len,
        )
    };
    assert_eq!(file_status, DharaStatus::Ok);
    let json = string_from_output(out_ptr, out_len);
    assert!(json.contains("\"analysis\""));

    let directory_status = unsafe {
        dhara_get_directory_info_json_old(
            temp_c.as_ptr(),
            1,
            &mut out_ptr,
            &mut out_len,
            &mut err_ptr,
            &mut err_len,
        )
    };
    assert_eq!(directory_status, DharaStatus::Ok);
    let json = string_from_output(out_ptr, out_len);
    assert!(json.contains("\"summary\""));
}

#[test]
#[allow(deprecated)]
fn write_read_list_and_delete_round_trip_non_ascii_paths() {
    let temp = tempdir().unwrap();
    let nested = temp.path().join("nested").join("inner");
    let nested_c = CString::new(nested.to_string_lossy().as_bytes()).unwrap();
    let file = nested.join("unicodé.txt");
    let file_c = CString::new(file.to_string_lossy().as_bytes()).unwrap();
    let renamed = nested.join("renamed.txt");
    let renamed_name_c = CString::new("renamed.txt").unwrap();
    let mut out_ptr: *mut u8 = ptr::null_mut();
    let mut out_len = 0;
    let mut err_ptr: *mut u8 = ptr::null_mut();
    let mut err_len = 0;

    let status = unsafe {
        dhara_create_directory_all(
            nested_c.as_ptr(),
            &mut out_ptr,
            &mut out_len,
            &mut err_ptr,
            &mut err_len,
        )
    };
    assert_eq!(status, DharaStatus::Ok);
    let _ = string_from_output(out_ptr, out_len);

    let text = CString::new("hello from ffi").unwrap();
    let write_status = unsafe {
        dhara_write_file_text(
            file_c.as_ptr(),
            text.as_ptr(),
            &mut out_ptr,
            &mut out_len,
            &mut err_ptr,
            &mut err_len,
        )
    };
    assert_eq!(write_status, DharaStatus::Ok);
    let written_path = string_from_output(out_ptr, out_len);
    assert!(written_path.ends_with("unicodé.txt"));

    let read_status = unsafe {
        dhara_read_file(
            file_c.as_ptr(),
            &mut out_ptr,
            &mut out_len,
            &mut err_ptr,
            &mut err_len,
        )
    };
    assert_eq!(read_status, DharaStatus::Ok);
    let bytes = bytes_from_output(out_ptr, out_len);
    assert_eq!(bytes, b"hello from ffi");

    let list_status = unsafe {
        dhara_list_entries_json_old(
            nested_c.as_ptr(),
            0,
            &mut out_ptr,
            &mut out_len,
            &mut err_ptr,
            &mut err_len,
        )
    };
    assert_eq!(list_status, DharaStatus::Ok);
    let list_json = string_from_output(out_ptr, out_len);
    assert!(list_json.contains("unicod"));

    let rename_status = unsafe {
        dhara_rename_file(
            file_c.as_ptr(),
            renamed_name_c.as_ptr(),
            &mut out_ptr,
            &mut out_len,
            &mut err_ptr,
            &mut err_len,
        )
    };
    assert_eq!(rename_status, DharaStatus::Ok);
    let renamed_path = string_from_output(out_ptr, out_len);
    assert_eq!(renamed_path, renamed.to_string_lossy());

    let renamed_c = CString::new(renamed.to_string_lossy().as_bytes()).unwrap();
    let delete_status =
        unsafe { dhara_delete_file(renamed_c.as_ptr(), &mut err_ptr, &mut err_len) };
    assert_eq!(delete_status, DharaStatus::Ok);
}

#[test]
fn list_entries_returns_typed_entries() {
    let temp = tempdir().unwrap();
    std::fs::write(temp.path().join("entry.txt"), "typed").unwrap();
    let temp_c = CString::new(temp.path().to_string_lossy().as_bytes()).unwrap();
    let mut entries: *mut NativeStorageEntryList = ptr::null_mut();
    let mut err_ptr: *mut u8 = ptr::null_mut();
    let mut err_len = 0;

    let status =
        unsafe { dhara_list_entries(temp_c.as_ptr(), 0, &mut entries, &mut err_ptr, &mut err_len) };

    assert_eq!(status, DharaStatus::Ok);
    assert!(err_ptr.is_null());
    assert!(!entries.is_null());
    let entries_ref = unsafe { &*entries };
    assert!(entries_ref.entries_len >= 1);
    assert!(!entries_ref.entries_ptr.is_null());
    unsafe { dhara_storage_entry_list_free(entries) };
}

#[test]
fn invalid_arguments_produce_error_payload() {
    let mut err_ptr: *mut u8 = ptr::null_mut();
    let mut err_len = 0;
    let status = unsafe { dhara_delete_file(ptr::null(), &mut err_ptr, &mut err_len) };

    assert_eq!(status, DharaStatus::InvalidArgument);
    let json = string_from_output(err_ptr, err_len);
    assert!(json.contains("\"code\":\"invalid_argument\""));
}
