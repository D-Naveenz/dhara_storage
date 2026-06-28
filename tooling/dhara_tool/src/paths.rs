use std::path::{Path, PathBuf};

const TOOLING: &str = "tooling";
const OUTPUT: &str = "output";
const ARTIFACTS: &str = "artifacts";
const LOGS: &str = "logs";

/// Relative path from the repository root to the embedded defs output directory.
pub const EMBEDDED_DEFS_DIR_RELATIVE: &str = "src/core/dhara_storage_dal/resources";

/// Relative path from the repository root to the embedded runtime defs package.
pub const EMBEDDED_DEFS_RELATIVE: &str = "src/core/dhara_storage_dal/resources/filedefs.dat";

/// Relative path from the repository root to the embedded runtime defs package.
pub const RUNTIME_DEFS_RELATIVE: &str = EMBEDDED_DEFS_RELATIVE;

/// Default directory for NuGet and other operator artifacts (`tooling/output`).
pub fn default_output_dir(repo_root: &Path) -> PathBuf {
    repo_root.join(TOOLING).join(OUTPUT)
}

/// Default directory for operator logs (`tooling/logs`).
pub fn default_logs_dir(repo_root: &Path) -> PathBuf {
    repo_root.join(TOOLING).join(LOGS)
}

/// Default directory for generated `filedefs.dat` artifacts.
pub fn default_defs_output_dir(repo_root: &Path) -> PathBuf {
    repo_root.join(EMBEDDED_DEFS_DIR_RELATIVE)
}

/// Default directory for temporary assembly and verification staging (`tooling/artifacts`).
pub fn default_artifacts_dir(repo_root: &Path) -> PathBuf {
    repo_root.join(TOOLING).join(ARTIFACTS)
}

/// Default directory for packed NuGet packages (`tooling/output/nuget`).
pub fn default_nuget_dir(repo_root: &Path) -> PathBuf {
    default_output_dir(repo_root).join("nuget")
}

/// Default path for generated defs packages.
pub fn default_defs_package_path(repo_root: &Path) -> PathBuf {
    embedded_defs_package_path(repo_root)
}

/// Path to the compile-time embedded defs package shipped inside `dhara_storage_dal`.
pub fn embedded_defs_package_path(repo_root: &Path) -> PathBuf {
    repo_root.join(EMBEDDED_DEFS_RELATIVE)
}

/// Resolves the effective NuGet/operator output directory, honoring an optional CLI override.
pub fn resolve_output_dir(repo_root: &Path, override_value: Option<&Path>) -> PathBuf {
    override_value
        .map(Path::to_path_buf)
        .unwrap_or_else(|| default_output_dir(repo_root))
}

/// Resolves the effective defs output directory, honoring an optional CLI override.
pub fn resolve_defs_output_dir(repo_root: &Path, override_value: Option<&Path>) -> PathBuf {
    override_value
        .map(Path::to_path_buf)
        .unwrap_or_else(|| default_defs_output_dir(repo_root))
}

/// Resolves the effective logs directory, honoring an optional CLI override.
pub fn resolve_logs_dir(repo_root: &Path, logs_override: Option<&Path>) -> PathBuf {
    logs_override
        .map(Path::to_path_buf)
        .unwrap_or_else(|| default_logs_dir(repo_root))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_under_expected_roots() {
        let root = PathBuf::from("/repo");
        assert_eq!(
            default_output_dir(&root),
            PathBuf::from("/repo/tooling/output")
        );
        assert_eq!(default_logs_dir(&root), PathBuf::from("/repo/tooling/logs"));
        assert_eq!(
            default_defs_output_dir(&root),
            PathBuf::from("/repo/src/core/dhara_storage_dal/resources")
        );
        assert_eq!(
            default_artifacts_dir(&root),
            PathBuf::from("/repo/tooling/artifacts")
        );
        assert_eq!(
            default_nuget_dir(&root),
            PathBuf::from("/repo/tooling/output/nuget")
        );
        assert_eq!(
            default_defs_package_path(&root),
            PathBuf::from("/repo/src/core/dhara_storage_dal/resources/filedefs.dat")
        );
    }

    #[test]
    fn logs_ignore_output_override() {
        let root = PathBuf::from("/repo");
        let custom_output = PathBuf::from("/repo/custom-output");
        assert_eq!(
            resolve_logs_dir(&root, None),
            PathBuf::from("/repo/tooling/logs")
        );
        assert_eq!(
            resolve_logs_dir(&root, Some(Path::new("/repo/custom-logs"))),
            PathBuf::from("/repo/custom-logs")
        );
        let _ = custom_output;
    }
}
