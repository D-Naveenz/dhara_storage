use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::repo_config::CONFIG_PATH;

const OUTPUT: &str = "output";
const ARTIFACTS: &str = "artifacts";
const LOGS: &str = "logs";

/// Relative path to the operator crate manifest (monorepo layout marker).
pub const TOOL_CRATE_MANIFEST_RELATIVE: &str = "tooling/dhara_tool/Cargo.toml";

/// Relative path from the repository root to the embedded defs output directory.
pub const EMBEDDED_DEFS_DIR_RELATIVE: &str = "src/core/dhara_storage_dal/resources";

/// Relative path from the repository root to the embedded runtime defs package.
pub const EMBEDDED_DEFS_RELATIVE: &str = "src/core/dhara_storage_dal/resources/filedefs.dat";

/// Relative path from the repository root to the embedded runtime defs package.
pub const RUNTIME_DEFS_RELATIVE: &str = EMBEDDED_DEFS_RELATIVE;

/// Returns true when `path` contains a Dhara Storage operator config (`dhara.config.toml`).
pub fn is_repo_root(path: &Path) -> bool {
    path.join(CONFIG_PATH).is_file()
}

/// Canonical directory containing the running `dhara_tool` executable (`exe_path`).
pub fn resolve_exe_root(current_exe: PathBuf) -> Result<PathBuf> {
    let parent = current_exe
        .parent()
        .with_context(|| format!("failed to resolve directory for '{}'", current_exe.display()))?;
    Ok(canonicalize_path(parent))
}

/// Normalizes operator input to the repository root directory.
///
/// Accepts either a directory containing `dhara.config.toml` or a direct path to that file.
pub fn normalize_repository_input(path: PathBuf) -> Result<PathBuf> {
    if path.is_file() {
        let is_config = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == CONFIG_PATH);
        if !is_config {
            bail!(
                "expected a repository directory or {CONFIG_PATH} file, got '{}'",
                path.display()
            );
        }
        let parent = path
            .parent()
            .with_context(|| format!("'{CONFIG_PATH}' has no parent directory"))?;
        let root = canonicalize_path(parent);
        if !is_repo_root(&root) {
            bail!(
                "parent of '{}' is not a repository root (missing {CONFIG_PATH})",
                path.display()
            );
        }
        return Ok(root);
    }

    let root = canonicalize_path(&path);
    if !is_repo_root(&root) {
        bail!(
            "'{}' is not a repository root (expected {CONFIG_PATH})",
            root.display()
        );
    }
    Ok(root)
}

pub(crate) fn canonicalize_path(path: &Path) -> PathBuf {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

    #[cfg(windows)]
    {
        const VERBATIM_PREFIX: &str = r"\\?\";
        let canonical_text = canonical.to_string_lossy();
        if let Some(stripped) = canonical_text.strip_prefix(VERBATIM_PREFIX) {
            return PathBuf::from(stripped);
        }
    }

    canonical
}

/// Joins a relative override against `base`; absolute overrides are unchanged.
pub fn resolve_path_against_base(base: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }
}

/// Joins a relative override against `repo_root`; absolute overrides are unchanged.
pub fn resolve_path_against_repo(repo_root: &Path, path: &Path) -> PathBuf {
    resolve_path_against_base(repo_root, path)
}

/// Default TrID/package input directory (`tooling/dhara_tool/package`).
pub fn default_package_dir(repo_root: &Path) -> PathBuf {
    repo_root.join("tooling").join("dhara_tool").join("package")
}

/// Default directory for NuGet and other operator artifacts (`{tool_root}/output`).
pub fn default_output_dir(tool_root: &Path) -> PathBuf {
    tool_root.join(OUTPUT)
}

/// Default directory for operator logs (`{tool_root}/logs`).
pub fn default_logs_dir(tool_root: &Path) -> PathBuf {
    tool_root.join(LOGS)
}

/// Default directory for generated `filedefs.dat` artifacts (under the workspace).
pub fn default_defs_output_dir(repo_root: &Path) -> PathBuf {
    repo_root.join(EMBEDDED_DEFS_DIR_RELATIVE)
}

/// Default directory for temporary assembly and verification staging (`{tool_root}/artifacts`).
pub fn default_artifacts_dir(tool_root: &Path) -> PathBuf {
    tool_root.join(ARTIFACTS)
}

/// Default directory for packed NuGet packages (`{tool_root}/output/nuget`).
pub fn default_nuget_dir(tool_root: &Path) -> PathBuf {
    default_output_dir(tool_root).join("nuget")
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
pub fn resolve_output_dir(tool_root: &Path, override_value: Option<&Path>) -> PathBuf {
    override_value
        .map(|path| resolve_path_against_base(tool_root, path))
        .unwrap_or_else(|| default_output_dir(tool_root))
}

/// Resolves the effective defs output directory, honoring an optional CLI override.
pub fn resolve_defs_output_dir(repo_root: &Path, override_value: Option<&Path>) -> PathBuf {
    override_value
        .map(|path| resolve_path_against_repo(repo_root, path))
        .unwrap_or_else(|| default_defs_output_dir(repo_root))
}

/// Resolves the effective logs directory, honoring an optional CLI override.
pub fn resolve_logs_dir(tool_root: &Path, logs_override: Option<&Path>) -> PathBuf {
    logs_override
        .map(|path| resolve_path_against_base(tool_root, path))
        .unwrap_or_else(|| default_logs_dir(tool_root))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_defaults_are_under_tool_root() {
        let tool = PathBuf::from("/exe");
        assert_eq!(default_output_dir(&tool), PathBuf::from("/exe/output"));
        assert_eq!(default_logs_dir(&tool), PathBuf::from("/exe/logs"));
        assert_eq!(
            default_artifacts_dir(&tool),
            PathBuf::from("/exe/artifacts")
        );
        assert_eq!(
            default_nuget_dir(&tool),
            PathBuf::from("/exe/output/nuget")
        );
    }

    #[test]
    fn workspace_defaults_stay_under_repo_root() {
        let root = PathBuf::from("/repo");
        assert_eq!(
            default_defs_output_dir(&root),
            PathBuf::from("/repo/src/core/dhara_storage_dal/resources")
        );
        assert_eq!(
            default_package_dir(&root),
            PathBuf::from("/repo/tooling/dhara_tool/package")
        );
        assert_eq!(
            default_defs_package_path(&root),
            PathBuf::from("/repo/src/core/dhara_storage_dal/resources/filedefs.dat")
        );
    }

    #[test]
    fn logs_ignore_output_override() {
        let tool = PathBuf::from("/exe");
        let custom_output = PathBuf::from("/exe/custom-output");
        assert_eq!(resolve_logs_dir(&tool, None), PathBuf::from("/exe/logs"));
        assert_eq!(
            resolve_logs_dir(&tool, Some(Path::new("/exe/custom-logs"))),
            PathBuf::from("/exe/custom-logs")
        );
        assert_eq!(
            resolve_logs_dir(&tool, Some(Path::new("logs"))),
            PathBuf::from("/exe/logs")
        );
        let _ = custom_output;
    }

    #[test]
    fn is_repo_root_requires_dhara_config() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("repo");
        std::fs::create_dir_all(&root).unwrap();
        assert!(!is_repo_root(&root));

        std::fs::write(root.join(CONFIG_PATH), "[versions]\n").unwrap();
        assert!(is_repo_root(&root));
    }

    #[test]
    fn normalize_accepts_directory_or_config_file() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("repo");
        std::fs::create_dir_all(&root).unwrap();
        let config = root.join(CONFIG_PATH);
        std::fs::write(&config, "[versions]\n").unwrap();

        let from_dir = normalize_repository_input(root.clone()).unwrap();
        let from_file = normalize_repository_input(config).unwrap();
        assert!(is_repo_root(&from_dir));
        assert_eq!(from_dir, from_file);
    }

    #[test]
    fn normalize_rejects_missing_config() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("repo");
        std::fs::create_dir_all(&root).unwrap();
        assert!(normalize_repository_input(root).is_err());
    }
}
