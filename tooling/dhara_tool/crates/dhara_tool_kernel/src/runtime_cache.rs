use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::paths::normalize_repository_input;
use crate::repo_config::CONFIG_PATH;

/// Filename for exe-local operator runtime preferences (`{exe_path}/runtime.toml`).
pub const RUNTIME_CACHE_FILE: &str = "runtime.toml";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeCache {
    pub repository: PathBuf,
}

pub fn runtime_cache_path(exe_root: &Path) -> PathBuf {
    exe_root.join(RUNTIME_CACHE_FILE)
}

pub fn load_runtime_cache(exe_root: &Path) -> Result<Option<RuntimeCache>> {
    let path = runtime_cache_path(exe_root);
    if !path.is_file() {
        return Ok(None);
    }

    let text = fs::read_to_string(&path)
        .with_context(|| format!("failed to read runtime cache '{}'", path.display()))?;
    let cache = toml::from_str(&text)
        .with_context(|| format!("failed to parse runtime cache '{}'", path.display()))?;
    Ok(Some(cache))
}

pub fn save_runtime_cache(exe_root: &Path, cache: &RuntimeCache) -> Result<()> {
    fs::create_dir_all(exe_root)
        .with_context(|| format!("failed to create exe directory '{}'", exe_root.display()))?;

    let path = runtime_cache_path(exe_root);
    let temp = exe_root.join(format!(".{RUNTIME_CACHE_FILE}.tmp"));
    let text = toml::to_string_pretty(cache).context("failed to serialize runtime cache")?;
    fs::write(&temp, &text)
        .with_context(|| format!("failed to write runtime cache '{}'", temp.display()))?;
    fs::rename(&temp, &path)
        .with_context(|| format!("failed to install runtime cache '{}'", path.display()))?;
    Ok(())
}

/// Returns a validated repository root from cache, or `None` when missing or stale.
pub fn try_cached_repository(exe_root: &Path) -> Option<PathBuf> {
    let cache = load_runtime_cache(exe_root).ok()??;
    normalize_repository_input(cache.repository).ok()
}

/// Returns the raw cached repository path even when validation fails (for GUI pre-fill).
pub fn stale_cached_repository(exe_root: &Path) -> Option<PathBuf> {
    load_runtime_cache(exe_root)
        .ok()
        .flatten()
        .map(|cache| cache.repository)
}

/// Normalizes `input`, optionally persists to cache, and returns the canonical repo root.
pub fn resolve_and_persist_repository(
    exe_root: &Path,
    input: PathBuf,
    persist: bool,
) -> Result<PathBuf> {
    let root = normalize_repository_input(input).with_context(|| {
        format!(
            "path is not a Dhara Storage repository (expected a directory containing {CONFIG_PATH})"
        )
    })?;

    if persist {
        save_runtime_cache(
            exe_root,
            &RuntimeCache {
                repository: root.clone(),
            },
        )?;
    }

    Ok(root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::is_repo_root;

    #[test]
    fn save_and_load_round_trip() {
        let temp = tempfile::tempdir().unwrap();
        let exe = temp.path().join("bin");
        fs::create_dir_all(&exe).unwrap();

        let repo = temp.path().join("repo");
        fs::create_dir_all(&repo).unwrap();
        fs::write(repo.join(CONFIG_PATH), "[versions]\n").unwrap();

        let root = normalize_repository_input(repo.clone()).unwrap();
        save_runtime_cache(
            &exe,
            &RuntimeCache {
                repository: root.clone(),
            },
        )
        .unwrap();

        let loaded = load_runtime_cache(&exe).unwrap().unwrap();
        assert_eq!(loaded.repository, root);
        assert!(try_cached_repository(&exe).is_some());
    }

    #[test]
    fn stale_cache_returns_none_for_validated_lookup() {
        let temp = tempfile::tempdir().unwrap();
        let exe = temp.path().join("bin");
        fs::create_dir_all(&exe).unwrap();

        save_runtime_cache(
            &exe,
            &RuntimeCache {
                repository: temp.path().join("missing"),
            },
        )
        .unwrap();

        assert!(try_cached_repository(&exe).is_none());
        assert_eq!(
            stale_cached_repository(&exe),
            Some(temp.path().join("missing"))
        );
    }

    #[test]
    fn cached_path_must_contain_config() {
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path().join("repo");
        fs::create_dir_all(&repo).unwrap();
        fs::write(repo.join(CONFIG_PATH), "[versions]\n").unwrap();
        assert!(is_repo_root(&repo));
    }
}
