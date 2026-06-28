use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use crate::command::ToolContext;
use crate::paths::resolve_output_dir;

use dhara_storage_dal::DefinitionPackage;

use crate::filedefs::load_package;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DefsPackageStatus {
    Missing,
    Present,
    Invalid { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceSnapshot {
    pub defs_path: PathBuf,
    pub defs_status: DefsPackageStatus,
    pub package_revision: Option<u16>,
    pub definitions_release: Option<String>,
    pub package_version: Option<String>,
    pub definition_count: Option<usize>,
}

impl WorkspaceSnapshot {
    pub fn next_package_revision(&self, tool_version: &str) -> Result<u16, String> {
        if self.defs_status != DefsPackageStatus::Present {
            return Ok(1);
        }

        if self.package_version.as_deref() != Some(tool_version) {
            return Ok(1);
        }

        let current = self.package_revision.unwrap_or(0);
        current
            .checked_add(1)
            .ok_or_else(|| "package revision overflowed u16".to_owned())
    }

    pub fn status_label(&self) -> &'static str {
        match self.defs_status {
            DefsPackageStatus::Missing => "missing",
            DefsPackageStatus::Present => "present",
            DefsPackageStatus::Invalid { .. } => "invalid",
        }
    }

    pub fn version_match_label(&self, tool_version: &str) -> &'static str {
        match self.package_version.as_deref() {
            Some(version) if version == tool_version => "match",
            Some(_) => "mismatch",
            None => "—",
        }
    }
}

struct WorkspaceState {
    snapshot: WorkspaceSnapshot,
}

static WORKSPACE: OnceLock<Mutex<WorkspaceState>> = OnceLock::new();

fn defs_path_for_context(context: &ToolContext) -> PathBuf {
    resolve_output_dir(&context.repo_root, context.output_dir.as_deref()).join("filedefs.dat")
}

fn analyze_defs_package(defs_path: &Path) -> WorkspaceSnapshot {
    if !defs_path.is_file() {
        return WorkspaceSnapshot {
            defs_path: defs_path.to_path_buf(),
            defs_status: DefsPackageStatus::Missing,
            package_revision: None,
            definitions_release: None,
            package_version: None,
            definition_count: None,
        };
    }

    match load_package(defs_path) {
        Ok(loaded) => WorkspaceSnapshot {
            defs_path: defs_path.to_path_buf(),
            defs_status: DefsPackageStatus::Present,
            package_revision: Some(loaded.package.package_revision),
            definitions_release: Some(loaded.package.definitions_release.clone()),
            package_version: Some(loaded.package.package_version.clone()),
            definition_count: Some(loaded.package.definitions.len()),
        },
        Err(error) => WorkspaceSnapshot {
            defs_path: defs_path.to_path_buf(),
            defs_status: DefsPackageStatus::Invalid {
                reason: error.to_string(),
            },
            package_revision: None,
            definitions_release: None,
            package_version: None,
            definition_count: None,
        },
    }
}

/// Analyze the workspace once per process and cache the definitions package snapshot.
pub fn ensure_workspace_state(context: &ToolContext) -> WorkspaceSnapshot {
    let defs_path = defs_path_for_context(context);
    let mutex = WORKSPACE.get_or_init(|| {
        Mutex::new(WorkspaceState {
            snapshot: analyze_defs_package(&defs_path),
        })
    });

    let mut state = mutex.lock().expect("workspace state lock poisoned");
    if state.snapshot.defs_path != defs_path {
        state.snapshot = analyze_defs_package(&defs_path);
    }
    state.snapshot.clone()
}

/// Return the cached workspace snapshot, analyzing first when needed.
pub fn workspace_snapshot(context: &ToolContext) -> WorkspaceSnapshot {
    ensure_workspace_state(context)
}

/// Allocate the next package revision for a build at `tool_version`.
pub fn next_package_revision(context: &ToolContext, tool_version: &str) -> Result<u16, String> {
    let snapshot = ensure_workspace_state(context);
    snapshot.next_package_revision(tool_version)
}

/// Allocate the next package revision using the cached workspace snapshot.
///
/// Falls back to revision `1` when workspace analysis has not run yet.
pub fn next_package_revision_for_build(tool_version: &str) -> Result<u16, String> {
    let Some(mutex) = WORKSPACE.get() else {
        return Ok(1);
    };

    let state = mutex.lock().expect("workspace state lock poisoned");
    state.snapshot.next_package_revision(tool_version)
}

/// Refresh the cached snapshot after writing a definitions package.
pub fn record_package_written(path: &Path, package: &DefinitionPackage) {
    let Some(mutex) = WORKSPACE.get() else {
        return;
    };

    let mut state = mutex.lock().expect("workspace state lock poisoned");
    if state.snapshot.defs_path != path {
        return;
    }

    state.snapshot.defs_status = DefsPackageStatus::Present;
    state.snapshot.package_revision = Some(package.package_revision);
    state.snapshot.package_version = Some(package.package_version.clone());
    state.snapshot.definitions_release = Some(package.definitions_release.clone());
    state.snapshot.definition_count = Some(package.definitions.len());
}

/// Re-read the definitions package from disk into the cached snapshot.
pub fn refresh_workspace_state(context: &ToolContext) -> WorkspaceSnapshot {
    let defs_path = defs_path_for_context(context);
    let snapshot = analyze_defs_package(&defs_path);
    let mutex = WORKSPACE.get_or_init(|| {
        Mutex::new(WorkspaceState {
            snapshot: snapshot.clone(),
        })
    });
    mutex
        .lock()
        .expect("workspace state lock poisoned")
        .snapshot = snapshot.clone();
    snapshot
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use dhara_storage_dal::{DefinitionPackage, DefinitionRecord, encode_definition_package};
    use tempfile::tempdir;

    use super::{
        DefsPackageStatus, WorkspaceSnapshot, analyze_defs_package, next_package_revision,
        record_package_written,
    };
    use crate::command::{RunMode, ToolContext};
    use crate::paths::default_defs_package_path;

    fn sample_package(version: &str, revision: u16) -> DefinitionPackage {
        DefinitionPackage {
            package_version: version.to_owned(),
            definitions_release: "2026-06-24".to_owned(),
            package_revision: revision,
            tags: 48,
            definitions: vec![DefinitionRecord::default()],
        }
    }

    fn test_context(root: &Path) -> ToolContext {
        ToolContext {
            repo_root: root.to_path_buf(),
            run_mode: RunMode::Direct,
            min: false,
            trace: false,
            workers: 4,
            package_dir: None,
            output_dir: None,
            logs_dir: None,
        }
    }

    #[test]
    fn missing_dat_yields_revision_one() {
        let temp = tempdir().unwrap();
        let snapshot = analyze_defs_package(&default_defs_package_path(temp.path()));

        assert_eq!(snapshot.defs_status, DefsPackageStatus::Missing);
        assert_eq!(snapshot.next_package_revision("0.6.0").unwrap(), 1);
    }

    #[test]
    fn matching_version_increments_revision() {
        let snapshot = WorkspaceSnapshot {
            defs_path: PathBuf::from("filedefs.dat"),
            defs_status: DefsPackageStatus::Present,
            package_revision: Some(5),
            definitions_release: Some("2026-06-24".to_owned()),
            package_version: Some("0.6.0".to_owned()),
            definition_count: Some(1),
        };

        assert_eq!(snapshot.next_package_revision("0.6.0").unwrap(), 6);
    }

    #[test]
    fn mismatched_version_resets_revision() {
        let snapshot = WorkspaceSnapshot {
            defs_path: PathBuf::from("filedefs.dat"),
            defs_status: DefsPackageStatus::Present,
            package_revision: Some(5),
            definitions_release: Some("2026-06-24".to_owned()),
            package_version: Some("0.5.0".to_owned()),
            definition_count: Some(1),
        };

        assert_eq!(snapshot.next_package_revision("0.6.0").unwrap(), 1);
    }

    #[test]
    fn record_package_written_updates_cached_revision() {
        let temp = tempdir().unwrap();
        let defs_path = default_defs_package_path(temp.path());
        fs::create_dir_all(defs_path.parent().unwrap()).unwrap();
        fs::write(
            &defs_path,
            encode_definition_package(&sample_package("0.6.0", 2)),
        )
        .unwrap();

        let context = test_context(temp.path());
        assert_eq!(
            next_package_revision(&context, "0.6.0").expect("revision"),
            3
        );

        record_package_written(&defs_path, &sample_package("0.6.0", 3));
        assert_eq!(
            next_package_revision(&context, "0.6.0").expect("revision"),
            4
        );
    }
}
