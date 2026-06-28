use std::path::{Path, PathBuf};

const TOOLING: &str = "tooling";
const OUTPUT: &str = "output";
const ARTIFACTS: &str = "artifacts";

/// Default directory for generated operator artifacts (`tooling/output`).
pub fn default_output_dir(repo_root: &Path) -> PathBuf {
    repo_root.join(TOOLING).join(OUTPUT)
}

/// Default directory for operator logs (`tooling/output/logs`).
pub fn default_logs_dir(repo_root: &Path) -> PathBuf {
    default_output_dir(repo_root).join("logs")
}

/// Default directory for temporary assembly and verification staging (`tooling/artifacts`).
pub fn default_artifacts_dir(repo_root: &Path) -> PathBuf {
    repo_root.join(TOOLING).join(ARTIFACTS)
}

/// Relative path from the repository root to the canonical runtime defs package.
pub const RUNTIME_DEFS_RELATIVE: &str = "tooling/output/filedefs.dat";

/// Default directory for packed NuGet packages (`tooling/output/nuget`).
pub fn default_nuget_dir(repo_root: &Path) -> PathBuf {
    default_output_dir(repo_root).join("nuget")
}

/// Default path for generated defs packages (`tooling/output/filedefs.dat`).
pub fn default_defs_package_path(repo_root: &Path) -> PathBuf {
    default_output_dir(repo_root).join("filedefs.dat")
}

/// Resolves the effective output directory, honoring an optional CLI override.
pub fn resolve_output_dir(repo_root: &Path, override_value: Option<&Path>) -> PathBuf {
    override_value
        .map(Path::to_path_buf)
        .unwrap_or_else(|| default_output_dir(repo_root))
}

/// Resolves the effective logs directory, honoring optional CLI overrides.
pub fn resolve_logs_dir(
    repo_root: &Path,
    output_override: Option<&Path>,
    logs_override: Option<&Path>,
) -> PathBuf {
    logs_override
        .map(Path::to_path_buf)
        .unwrap_or_else(|| resolve_output_dir(repo_root, output_override).join("logs"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_under_tooling() {
        let root = PathBuf::from("/repo");
        assert_eq!(
            default_output_dir(&root),
            PathBuf::from("/repo/tooling/output")
        );
        assert_eq!(
            default_logs_dir(&root),
            PathBuf::from("/repo/tooling/output/logs")
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
            PathBuf::from("/repo/tooling/output/filedefs.dat")
        );
    }

    #[test]
    fn logs_follow_output_override() {
        let root = PathBuf::from("/repo");
        let custom_output = PathBuf::from("/repo/custom-output");
        assert_eq!(
            resolve_logs_dir(&root, Some(&custom_output), None),
            PathBuf::from("/repo/custom-output/logs")
        );
    }
}
