use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use dhara_storage::{
    AnalysisReport, ContentKind, DetectedDefinition, DirectoryInfo, DirectoryStorage, FileInfo,
    SearchScope, StorageChangeEvent, StorageChangeType, StorageEntry,
};
use serde::Serialize;

use crate::errors::FfiFailure;

#[derive(Debug, Clone, Copy)]
pub(crate) enum EntryKind {
    Files,
    Directories,
    All,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) struct StorageMetadataDto {
    pub(crate) path: String,
    pub(crate) name: String,
    pub(crate) is_read_only: bool,
    pub(crate) is_hidden: bool,
    pub(crate) is_system: bool,
    pub(crate) is_temporary: bool,
    pub(crate) is_symbolic_link: bool,
    pub(crate) link_target: Option<String>,
    pub(crate) created_at_utc_ms: Option<u64>,
    pub(crate) modified_at_utc_ms: Option<u64>,
    pub(crate) accessed_at_utc_ms: Option<u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) struct FileInfoDto {
    pub(crate) metadata: StorageMetadataDto,
    pub(crate) display_name: String,
    pub(crate) size: u64,
    pub(crate) formatted_size: String,
    pub(crate) filename_extension: Option<String>,
    pub(crate) analysis: Option<AnalysisReportDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) struct DirectorySummaryDto {
    pub(crate) total_size: u64,
    pub(crate) file_count: u64,
    pub(crate) directory_count: u64,
    pub(crate) formatted_size: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) struct DirectoryInfoDto {
    pub(crate) metadata: StorageMetadataDto,
    pub(crate) display_name: String,
    pub(crate) summary: Option<DirectorySummaryDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) struct DetectedDefinitionDto {
    pub(crate) file_type_label: String,
    pub(crate) mime_type: String,
    pub(crate) extensions: Vec<String>,
    pub(crate) score: u64,
    pub(crate) confidence: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) struct AnalysisReportDto {
    pub(crate) matches: Vec<DetectedDefinitionDto>,
    pub(crate) top_mime_type: Option<String>,
    pub(crate) top_detected_extension: Option<String>,
    pub(crate) content_kind: ContentKindDto,
    pub(crate) bytes_scanned: usize,
    pub(crate) file_size: u64,
    pub(crate) source_extension: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ContentKindDto {
    Text,
    Binary,
    Unknown,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) struct StorageEntryDto {
    pub(crate) kind: &'static str,
    pub(crate) path: String,
    pub(crate) name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) struct StorageChangeEventDto {
    pub(crate) change_type: &'static str,
    pub(crate) path: String,
    pub(crate) previous_path: Option<String>,
    pub(crate) observed_at_utc_ms: u64,
}

pub(crate) fn list_entries_json(
    path: &Path,
    recursive: bool,
    kind: EntryKind,
) -> Result<Vec<StorageEntryDto>, FfiFailure> {
    let directory = DirectoryStorage::from_existing(path).map_err(FfiFailure::from)?;
    let scope = if recursive {
        SearchScope::AllDirectories
    } else {
        SearchScope::TopDirectoryOnly
    };

    match kind {
        EntryKind::Files => Ok(directory
            .files_matching("*", scope)
            .map_err(FfiFailure::from)?
            .into_iter()
            .map(|file| StorageEntryDto {
                kind: "file",
                path: path_to_string(file.path()),
                name: file.name().unwrap_or_default().to_owned(),
            })
            .collect()),
        EntryKind::Directories => Ok(directory
            .directories_matching("*", scope)
            .map_err(FfiFailure::from)?
            .into_iter()
            .map(|dir| StorageEntryDto {
                kind: "directory",
                path: path_to_string(dir.path()),
                name: dir.name().unwrap_or_default().to_owned(),
            })
            .collect()),
        EntryKind::All => Ok(directory
            .entries_matching("*", scope)
            .map_err(FfiFailure::from)?
            .into_iter()
            .map(StorageEntryDto::from_entry)
            .collect()),
    }
}

pub(crate) fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

pub(crate) fn system_time_to_unix_millis(value: Option<SystemTime>) -> Option<u64> {
    value
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
}

impl From<StorageChangeEvent> for StorageChangeEventDto {
    fn from(value: StorageChangeEvent) -> Self {
        Self {
            change_type: match value.change_type {
                StorageChangeType::Created => "created",
                StorageChangeType::Deleted => "deleted",
                StorageChangeType::Modified => "modified",
                StorageChangeType::Relocated => "relocated",
            },
            path: path_to_string(&value.path),
            previous_path: value.previous_path.as_deref().map(path_to_string),
            observed_at_utc_ms: system_time_to_unix_millis(Some(value.observed_at)).unwrap_or(0),
        }
    }
}

impl StorageMetadataDto {
    pub(crate) fn from_metadata(metadata: &dhara_storage::StorageMetadata) -> Self {
        Self {
            path: path_to_string(metadata.path()),
            name: metadata.name().to_owned(),
            is_read_only: metadata.is_read_only(),
            is_hidden: metadata.is_hidden(),
            is_system: metadata.is_system(),
            is_temporary: metadata.is_temporary(),
            is_symbolic_link: metadata.is_symbolic_link(),
            link_target: metadata.link_target().map(path_to_string),
            created_at_utc_ms: system_time_to_unix_millis(metadata.created_at()),
            modified_at_utc_ms: system_time_to_unix_millis(metadata.modified_at()),
            accessed_at_utc_ms: system_time_to_unix_millis(metadata.accessed_at()),
        }
    }
}

impl FileInfoDto {
    pub(crate) fn try_from_info(
        info: FileInfo,
        include_analysis: bool,
    ) -> Result<Self, FfiFailure> {
        let analysis = if include_analysis {
            Some(AnalysisReportDto::from(
                info.analysis().map_err(FfiFailure::from)?.clone(),
            ))
        } else {
            None
        };

        Ok(Self {
            metadata: StorageMetadataDto::from_metadata(info.metadata()),
            display_name: info.display_name().to_owned(),
            size: info.size(),
            formatted_size: info.formatted_size(),
            filename_extension: info.filename_extension().map(ToOwned::to_owned),
            analysis,
        })
    }
}

impl DirectoryInfoDto {
    pub(crate) fn try_from_info(
        info: DirectoryInfo,
        include_summary: bool,
    ) -> Result<Self, FfiFailure> {
        let summary = if include_summary {
            let summary = *info.summary().map_err(FfiFailure::from)?;
            Some(DirectorySummaryDto {
                total_size: summary.total_size,
                file_count: summary.file_count,
                directory_count: summary.directory_count,
                formatted_size: summary.formatted_size(),
            })
        } else {
            None
        };

        Ok(Self {
            metadata: StorageMetadataDto::from_metadata(info.metadata()),
            display_name: info.display_name().to_owned(),
            summary,
        })
    }
}

impl From<AnalysisReport> for AnalysisReportDto {
    fn from(value: AnalysisReport) -> Self {
        Self {
            matches: value
                .matches
                .into_iter()
                .map(DetectedDefinitionDto::from)
                .collect(),
            top_mime_type: value.top_mime_type,
            top_detected_extension: value.top_detected_extension,
            content_kind: ContentKindDto::from(value.content_kind),
            bytes_scanned: value.bytes_scanned,
            file_size: value.file_size,
            source_extension: value.source_extension,
        }
    }
}

impl From<DetectedDefinition> for DetectedDefinitionDto {
    fn from(value: DetectedDefinition) -> Self {
        Self {
            file_type_label: value.file_type_label,
            mime_type: value.mime_type,
            extensions: value.extensions,
            score: value.score,
            confidence: value.confidence,
        }
    }
}

impl From<ContentKind> for ContentKindDto {
    fn from(value: ContentKind) -> Self {
        match value {
            ContentKind::Text => Self::Text,
            ContentKind::Binary => Self::Binary,
            ContentKind::Unknown => Self::Unknown,
        }
    }
}

impl StorageEntryDto {
    pub(crate) fn from_entry(entry: StorageEntry) -> Self {
        match entry {
            StorageEntry::File(file) => Self {
                kind: "file",
                path: path_to_string(file.path()),
                name: file.name().unwrap_or_default().to_owned(),
            },
            StorageEntry::Directory(directory) => Self {
                kind: "directory",
                path: path_to_string(directory.path()),
                name: directory.name().unwrap_or_default().to_owned(),
            },
        }
    }
}
