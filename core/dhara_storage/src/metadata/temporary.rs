//! Temporary-entry detection helpers.
//!
//! # Platform behavior
//!
//! - **Windows:** `FILE_ATTRIBUTE_TEMPORARY` and/or path under the process temp directory.
//! - **Unix:** path under `std::env::temp_dir()` / `$TMPDIR`; unlinked nodes (`nlink == 0`)
//!   when observable. Linux `O_TMPFILE` and tmpfs mounts are documented alternatives used by
//!   crates such as `tempfile`; creation APIs on storage handles delegate to that crate.

use std::fs;
use std::path::Path;

use crate::error::StorageError;
use crate::operations::common::resolve_storage_paths;

/// Returns whether `path` looks temporary by OS attribute and/or temp-directory location.
pub fn is_temporary_path(path: &Path) -> Result<bool, StorageError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|err| StorageError::io("read metadata for", path, err))?;
    Ok(is_temporary_from_metadata(path, &metadata))
}

pub(crate) fn is_temporary_from_metadata(path: &Path, metadata: &fs::Metadata) -> bool {
    if has_temporary_attribute(metadata) {
        return true;
    }
    if is_under_temp_dir(path) {
        return true;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.nlink() == 0 {
            return true;
        }
    }
    false
}

#[cfg(windows)]
fn has_temporary_attribute(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    // FILE_ATTRIBUTE_TEMPORARY = 0x100
    metadata.file_attributes() & 0x100 != 0
}

#[cfg(not(windows))]
fn has_temporary_attribute(_metadata: &fs::Metadata) -> bool {
    false
}

fn is_under_temp_dir(path: &Path) -> bool {
    let temp = match std::env::temp_dir().canonicalize() {
        Ok(path) => path,
        Err(_) => std::env::temp_dir(),
    };
    let absolute = match path.canonicalize() {
        Ok(path) => path,
        Err(_) => match resolve_storage_paths(path) {
            Ok(resolved) => resolved.absolute,
            Err(_) => return false,
        },
    };
    absolute.starts_with(&temp)
}
