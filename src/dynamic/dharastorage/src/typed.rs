use std::ptr;

use dhara_storage::{
    AnalysisReport, ContentKind, DetectedDefinition, DirectoryInfo, FileInfo, StorageChangeEvent,
    StorageChangeType,
};

use crate::errors::FfiFailure;
use crate::models::{StorageEntryDto, path_to_string, system_time_to_unix_millis};

/// Borrowed UTF-8 string slice inside an owned native result.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NativeUtf8 {
    /// Pointer to UTF-8 bytes, or null when `len` is zero.
    pub ptr: *const u8,
    /// Byte length of the UTF-8 string.
    pub len: usize,
}

/// Optional borrowed UTF-8 string slice inside an owned native result.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NativeOptionalUtf8 {
    /// Non-zero when `value` contains a meaningful string.
    pub has_value: u8,
    /// UTF-8 string value when `has_value` is non-zero.
    pub value: NativeUtf8,
}

/// File-system metadata returned through the typed native ABI.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NativeStorageMetadata {
    /// Full path.
    pub path: NativeUtf8,
    /// File or directory name.
    pub name: NativeUtf8,
    /// Non-zero when the item is read-only.
    pub is_read_only: u8,
    /// Non-zero when the item is hidden.
    pub is_hidden: u8,
    /// Non-zero when the item is marked as a system item.
    pub is_system: u8,
    /// Non-zero when the item is marked temporary.
    pub is_temporary: u8,
    /// Non-zero when the item is a symbolic link.
    pub is_symbolic_link: u8,
    /// Link target path when the item is a symbolic link and the target is available.
    pub link_target: NativeOptionalUtf8,
    /// Non-zero when `created_at_utc_ms` is meaningful.
    pub has_created_at_utc_ms: u8,
    /// Unix timestamp in milliseconds for creation time.
    pub created_at_utc_ms: u64,
    /// Non-zero when `modified_at_utc_ms` is meaningful.
    pub has_modified_at_utc_ms: u8,
    /// Unix timestamp in milliseconds for modification time.
    pub modified_at_utc_ms: u64,
    /// Non-zero when `accessed_at_utc_ms` is meaningful.
    pub has_accessed_at_utc_ms: u8,
    /// Unix timestamp in milliseconds for last access time.
    pub accessed_at_utc_ms: u64,
}

/// Directory summary returned through the typed native ABI.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NativeDirectorySummary {
    /// Total byte size of the directory tree.
    pub total_size: u64,
    /// Number of files in the directory tree.
    pub file_count: u64,
    /// Number of directories in the directory tree.
    pub directory_count: u64,
    /// Human-readable formatted size.
    pub formatted_size: NativeUtf8,
}

/// One detected file definition returned through the typed native ABI.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NativeDetectedDefinition {
    /// Human-readable file type label.
    pub file_type_label: NativeUtf8,
    /// MIME type.
    pub mime_type: NativeUtf8,
    /// Pointer to an array of extension strings.
    pub extensions_ptr: *const NativeUtf8,
    /// Number of extension strings.
    pub extensions_len: usize,
    /// Match score.
    pub score: u64,
    /// Match confidence.
    pub confidence: f64,
}

/// Analysis report returned through the typed native ABI.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NativeAnalysisReport {
    /// Pointer to an array of detected definitions.
    pub matches_ptr: *const NativeDetectedDefinition,
    /// Number of detected definitions.
    pub matches_len: usize,
    /// Top MIME type when available.
    pub top_mime_type: NativeOptionalUtf8,
    /// Top detected extension when available.
    pub top_detected_extension: NativeOptionalUtf8,
    /// Content kind: 0 text, 1 binary, 2 unknown.
    pub content_kind: u32,
    /// Number of bytes scanned during analysis.
    pub bytes_scanned: usize,
    /// Full file size in bytes.
    pub file_size: u64,
    /// Source extension when available.
    pub source_extension: NativeOptionalUtf8,
}

/// File information returned through the typed native ABI.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NativeFileInformation {
    /// Shared file-system metadata.
    pub metadata: NativeStorageMetadata,
    /// Display name.
    pub display_name: NativeUtf8,
    /// File size in bytes.
    pub size: u64,
    /// Human-readable formatted size.
    pub formatted_size: NativeUtf8,
    /// Filename extension when available.
    pub filename_extension: NativeOptionalUtf8,
    /// Analysis report pointer when analysis was requested and available.
    pub analysis: *const NativeAnalysisReport,
}

/// Directory information returned through the typed native ABI.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NativeDirectoryInformation {
    /// Shared file-system metadata.
    pub metadata: NativeStorageMetadata,
    /// Display name.
    pub display_name: NativeUtf8,
    /// Non-zero when `summary` is meaningful.
    pub has_summary: u8,
    /// Directory summary when `has_summary` is non-zero.
    pub summary: NativeDirectorySummary,
}

/// One listed storage entry returned through the typed native ABI.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NativeStorageEntry {
    /// Entry kind: 0 file, 1 directory.
    pub kind: u32,
    /// Full path.
    pub path: NativeUtf8,
    /// File or directory name.
    pub name: NativeUtf8,
}

/// Storage entry list returned through the typed native ABI.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NativeStorageEntryList {
    /// Pointer to an array of storage entries.
    pub entries_ptr: *const NativeStorageEntry,
    /// Number of storage entries.
    pub entries_len: usize,
}

/// Watch event returned through the typed native ABI.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NativeWatchEvent {
    /// Change type: 0 created, 1 deleted, 2 modified, 3 relocated.
    pub change_type: u32,
    /// Current path.
    pub path: NativeUtf8,
    /// Previous path for relocation events.
    pub previous_path: NativeOptionalUtf8,
    /// Observation time as a Unix timestamp in milliseconds.
    pub observed_at_utc_ms: u64,
}

#[derive(Default)]
struct StringStore {
    buffers: Vec<Box<[u8]>>,
}

impl StringStore {
    fn push(&mut self, value: impl AsRef<str>) -> NativeUtf8 {
        let bytes = value.as_ref().as_bytes();
        if bytes.is_empty() {
            return NativeUtf8 {
                ptr: ptr::null(),
                len: 0,
            };
        }

        let boxed = bytes.to_vec().into_boxed_slice();
        let value = NativeUtf8 {
            ptr: boxed.as_ptr(),
            len: boxed.len(),
        };
        self.buffers.push(boxed);
        value
    }

    fn push_optional(&mut self, value: Option<impl AsRef<str>>) -> NativeOptionalUtf8 {
        match value {
            Some(value) => NativeOptionalUtf8 {
                has_value: 1,
                value: self.push(value),
            },
            None => NativeOptionalUtf8 {
                has_value: 0,
                value: NativeUtf8 {
                    ptr: ptr::null(),
                    len: 0,
                },
            },
        }
    }
}

#[repr(C)]
struct AnalysisReportOwner {
    abi: NativeAnalysisReport,
    matches: Box<[NativeDetectedDefinition]>,
    extension_arrays: Vec<Box<[NativeUtf8]>>,
    strings: StringStore,
}

#[repr(C)]
struct FileInformationOwner {
    abi: NativeFileInformation,
    analysis: Option<Box<AnalysisReportOwner>>,
    strings: StringStore,
}

#[repr(C)]
struct DirectoryInformationOwner {
    abi: NativeDirectoryInformation,
    strings: StringStore,
}

#[repr(C)]
struct StorageEntryListOwner {
    abi: NativeStorageEntryList,
    entries: Box<[NativeStorageEntry]>,
    strings: StringStore,
}

#[repr(C)]
struct WatchEventOwner {
    abi: NativeWatchEvent,
    strings: StringStore,
}

impl AnalysisReportOwner {
    fn from_report(report: AnalysisReport) -> Box<Self> {
        let mut strings = StringStore::default();
        let mut extension_arrays = Vec::new();
        let matches = report
            .matches
            .into_iter()
            .map(|value| native_detected_definition(value, &mut strings, &mut extension_arrays))
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let matches_ptr = slice_ptr(&matches);
        let matches_len = matches.len();
        Box::new(Self {
            abi: NativeAnalysisReport {
                matches_ptr,
                matches_len,
                top_mime_type: strings.push_optional(report.top_mime_type.as_deref()),
                top_detected_extension: strings
                    .push_optional(report.top_detected_extension.as_deref()),
                content_kind: native_content_kind(report.content_kind),
                bytes_scanned: report.bytes_scanned,
                file_size: report.file_size,
                source_extension: strings.push_optional(report.source_extension.as_deref()),
            },
            matches,
            extension_arrays,
            strings,
        })
    }

    fn as_abi_ptr(&self) -> *const NativeAnalysisReport {
        &self.abi
    }

    fn into_raw_abi(mut owner: Box<Self>) -> *mut NativeAnalysisReport {
        let ptr = &mut owner.abi as *mut NativeAnalysisReport;
        let _ = Box::into_raw(owner);
        ptr
    }
}

impl FileInformationOwner {
    fn from_info(info: FileInfo, include_analysis: bool) -> Result<Box<Self>, FfiFailure> {
        let mut strings = StringStore::default();
        let analysis = if include_analysis {
            Some(AnalysisReportOwner::from_report(
                info.analysis().map_err(FfiFailure::from)?.clone(),
            ))
        } else {
            None
        };
        let analysis_ptr = analysis
            .as_ref()
            .map(|value| value.as_abi_ptr())
            .unwrap_or_else(ptr::null);

        Ok(Box::new(Self {
            abi: NativeFileInformation {
                metadata: native_metadata(info.metadata(), &mut strings),
                display_name: strings.push(info.display_name()),
                size: info.size(),
                formatted_size: strings.push(info.formatted_size()),
                filename_extension: strings.push_optional(info.filename_extension()),
                analysis: analysis_ptr,
            },
            analysis,
            strings,
        }))
    }

    fn into_raw_abi(mut owner: Box<Self>) -> *mut NativeFileInformation {
        let ptr = &mut owner.abi as *mut NativeFileInformation;
        let _ = Box::into_raw(owner);
        ptr
    }
}

impl DirectoryInformationOwner {
    fn from_info(info: DirectoryInfo, include_summary: bool) -> Result<Box<Self>, FfiFailure> {
        let mut strings = StringStore::default();
        let (has_summary, summary) = if include_summary {
            let summary = *info.summary().map_err(FfiFailure::from)?;
            (
                1,
                NativeDirectorySummary {
                    total_size: summary.total_size,
                    file_count: summary.file_count,
                    directory_count: summary.directory_count,
                    formatted_size: strings.push(summary.formatted_size()),
                },
            )
        } else {
            (
                0,
                NativeDirectorySummary {
                    total_size: 0,
                    file_count: 0,
                    directory_count: 0,
                    formatted_size: strings.push(""),
                },
            )
        };

        Ok(Box::new(Self {
            abi: NativeDirectoryInformation {
                metadata: native_metadata(info.metadata(), &mut strings),
                display_name: strings.push(info.display_name()),
                has_summary,
                summary,
            },
            strings,
        }))
    }

    fn into_raw_abi(mut owner: Box<Self>) -> *mut NativeDirectoryInformation {
        let ptr = &mut owner.abi as *mut NativeDirectoryInformation;
        let _ = Box::into_raw(owner);
        ptr
    }
}

impl StorageEntryListOwner {
    fn from_entries(entries: Vec<StorageEntryDto>) -> Box<Self> {
        let mut strings = StringStore::default();
        let entries = entries
            .into_iter()
            .map(|entry| NativeStorageEntry {
                kind: if entry.kind == "directory" { 1 } else { 0 },
                path: strings.push(entry.path),
                name: strings.push(entry.name),
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let entries_ptr = slice_ptr(&entries);
        let entries_len = entries.len();

        Box::new(Self {
            abi: NativeStorageEntryList {
                entries_ptr,
                entries_len,
            },
            entries,
            strings,
        })
    }

    fn into_raw_abi(mut owner: Box<Self>) -> *mut NativeStorageEntryList {
        let ptr = &mut owner.abi as *mut NativeStorageEntryList;
        let _ = Box::into_raw(owner);
        ptr
    }
}

impl WatchEventOwner {
    fn from_event(event: StorageChangeEvent) -> Box<Self> {
        let mut strings = StringStore::default();
        Box::new(Self {
            abi: NativeWatchEvent {
                change_type: match event.change_type {
                    StorageChangeType::Created => 0,
                    StorageChangeType::Deleted => 1,
                    StorageChangeType::Modified => 2,
                    StorageChangeType::Relocated => 3,
                },
                path: strings.push(path_to_string(&event.path)),
                previous_path: strings
                    .push_optional(event.previous_path.as_deref().map(path_to_string)),
                observed_at_utc_ms: system_time_to_unix_millis(Some(event.observed_at))
                    .unwrap_or(0),
            },
            strings,
        })
    }

    fn into_raw_abi(mut owner: Box<Self>) -> *mut NativeWatchEvent {
        let ptr = &mut owner.abi as *mut NativeWatchEvent;
        let _ = Box::into_raw(owner);
        ptr
    }
}

pub(crate) fn analysis_report_to_native(report: AnalysisReport) -> *mut NativeAnalysisReport {
    AnalysisReportOwner::into_raw_abi(AnalysisReportOwner::from_report(report))
}

pub(crate) fn file_info_to_native(
    info: FileInfo,
    include_analysis: bool,
) -> Result<*mut NativeFileInformation, FfiFailure> {
    FileInformationOwner::from_info(info, include_analysis).map(FileInformationOwner::into_raw_abi)
}

pub(crate) fn directory_info_to_native(
    info: DirectoryInfo,
    include_summary: bool,
) -> Result<*mut NativeDirectoryInformation, FfiFailure> {
    DirectoryInformationOwner::from_info(info, include_summary)
        .map(DirectoryInformationOwner::into_raw_abi)
}

pub(crate) fn storage_entries_to_native(
    entries: Vec<StorageEntryDto>,
) -> *mut NativeStorageEntryList {
    StorageEntryListOwner::into_raw_abi(StorageEntryListOwner::from_entries(entries))
}

pub(crate) fn watch_event_to_native(event: StorageChangeEvent) -> *mut NativeWatchEvent {
    WatchEventOwner::into_raw_abi(WatchEventOwner::from_event(event))
}

fn native_metadata(
    metadata: &dhara_storage::StorageMetadata,
    strings: &mut StringStore,
) -> NativeStorageMetadata {
    let created_at_utc_ms = system_time_to_unix_millis(metadata.created_at());
    let modified_at_utc_ms = system_time_to_unix_millis(metadata.modified_at());
    let accessed_at_utc_ms = system_time_to_unix_millis(metadata.accessed_at());
    NativeStorageMetadata {
        path: strings.push(path_to_string(metadata.path())),
        name: strings.push(metadata.name()),
        is_read_only: u8::from(metadata.is_read_only()),
        is_hidden: u8::from(metadata.is_hidden()),
        is_system: u8::from(metadata.is_system()),
        is_temporary: u8::from(metadata.is_temporary()),
        is_symbolic_link: u8::from(metadata.is_symbolic_link()),
        link_target: strings.push_optional(metadata.link_target().map(path_to_string)),
        has_created_at_utc_ms: u8::from(created_at_utc_ms.is_some()),
        created_at_utc_ms: created_at_utc_ms.unwrap_or(0),
        has_modified_at_utc_ms: u8::from(modified_at_utc_ms.is_some()),
        modified_at_utc_ms: modified_at_utc_ms.unwrap_or(0),
        has_accessed_at_utc_ms: u8::from(accessed_at_utc_ms.is_some()),
        accessed_at_utc_ms: accessed_at_utc_ms.unwrap_or(0),
    }
}

fn native_detected_definition(
    value: DetectedDefinition,
    strings: &mut StringStore,
    extension_arrays: &mut Vec<Box<[NativeUtf8]>>,
) -> NativeDetectedDefinition {
    let extensions = value
        .extensions
        .into_iter()
        .map(|extension| strings.push(extension))
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let extensions_ptr = slice_ptr(&extensions);
    let extensions_len = extensions.len();
    extension_arrays.push(extensions);

    NativeDetectedDefinition {
        file_type_label: strings.push(value.file_type_label),
        mime_type: strings.push(value.mime_type),
        extensions_ptr,
        extensions_len,
        score: value.score,
        confidence: value.confidence,
    }
}

fn native_content_kind(value: ContentKind) -> u32 {
    match value {
        ContentKind::Text => 0,
        ContentKind::Binary => 1,
        ContentKind::Unknown => 2,
    }
}

fn slice_ptr<T>(slice: &[T]) -> *const T {
    if slice.is_empty() {
        ptr::null()
    } else {
        slice.as_ptr()
    }
}

/// Frees an analysis report returned by the typed native ABI.
///
/// # Safety
///
/// `report` must be null or a pointer returned by `dhara_analyze_path_v2` or another typed
/// Dhara Storage ABI function that transfers ownership of a `NativeAnalysisReport`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhara_analysis_report_free(report: *mut NativeAnalysisReport) {
    if !report.is_null() {
        drop(Box::from_raw(report as *mut AnalysisReportOwner));
    }
}

/// Frees file information returned by the typed native ABI.
///
/// # Safety
///
/// `info` must be null or a pointer returned by `dhara_get_file_info_v2`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhara_file_info_free(info: *mut NativeFileInformation) {
    if !info.is_null() {
        drop(Box::from_raw(info as *mut FileInformationOwner));
    }
}

/// Frees directory information returned by the typed native ABI.
///
/// # Safety
///
/// `info` must be null or a pointer returned by `dhara_get_directory_info_v2`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhara_directory_info_free(info: *mut NativeDirectoryInformation) {
    if !info.is_null() {
        drop(Box::from_raw(info as *mut DirectoryInformationOwner));
    }
}

/// Frees a storage entry list returned by the typed native ABI.
///
/// # Safety
///
/// `entries` must be null or a pointer returned by a typed storage-entry listing function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhara_storage_entry_list_free(entries: *mut NativeStorageEntryList) {
    if !entries.is_null() {
        drop(Box::from_raw(entries as *mut StorageEntryListOwner));
    }
}

/// Frees a watch event returned by the typed native ABI.
///
/// # Safety
///
/// `event` must be null or a pointer returned by a typed watch receive function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhara_watch_event_free(event: *mut NativeWatchEvent) {
    if !event.is_null() {
        drop(Box::from_raw(event as *mut WatchEventOwner));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_abi_strings_are_pointer_and_length_pairs() {
        assert_eq!(
            std::mem::size_of::<NativeUtf8>(),
            std::mem::size_of::<usize>() * 2
        );
        assert_eq!(
            std::mem::align_of::<NativeUtf8>(),
            std::mem::align_of::<usize>()
        );
    }

    #[test]
    fn typed_abi_structs_keep_c_compatible_alignment() {
        assert_eq!(
            std::mem::align_of::<NativeAnalysisReport>(),
            std::mem::align_of::<usize>()
        );
        assert_eq!(
            std::mem::align_of::<NativeStorageEntryList>(),
            std::mem::align_of::<usize>()
        );
    }
}
