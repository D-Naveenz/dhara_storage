use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use dhara_storage::{DirectoryStorage, SearchScope, StorageEntry};

use crate::errors::FfiFailure;

#[derive(Debug, Clone, Copy)]
pub(crate) enum EntryKind {
    Files,
    Directories,
    All,
}

#[derive(Debug, Clone)]
pub(crate) struct StorageEntryDto {
    pub(crate) kind: &'static str,
    pub(crate) path: String,
    pub(crate) name: String,
}

pub(crate) fn list_entries(
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
