use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

pub fn merge_native_stages(output: &Path, inputs: &[PathBuf]) -> Result<()> {
    if output.exists() {
        fs::remove_dir_all(output)
            .with_context(|| format!("failed to remove {}", output.display()))?;
    }
    fs::create_dir_all(output).with_context(|| format!("failed to create {}", output.display()))?;

    for stage in inputs {
        let runtimes = stage.join("runtimes");
        if !runtimes.is_dir() {
            bail!(
                "native stage input '{}' is missing a runtimes directory",
                stage.display()
            );
        }

        for entry in fs::read_dir(&runtimes)
            .with_context(|| format!("failed to read {}", runtimes.display()))?
        {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let destination = output.join("runtimes").join(entry.file_name());
            copy_dir_recursive(&entry.path(), &destination)?;
        }
    }

    Ok(())
}

fn copy_dir_recursive(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination)
        .with_context(|| format!("failed to create directory {}", destination.display()))?;

    for entry in fs::read_dir(source)
        .with_context(|| format!("failed to read directory {}", source.display()))?
    {
        let entry = entry?;
        let target = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target).with_context(|| {
                format!(
                    "failed to copy {} to {}",
                    entry.path().display(),
                    target.display()
                )
            })?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    fn write_native_asset(stage: &Path, rid: &str, file_name: &str) {
        let native_dir = stage.join("runtimes").join(rid).join("native");
        fs::create_dir_all(&native_dir).unwrap();
        fs::write(native_dir.join(file_name), b"native").unwrap();
    }

    #[test]
    fn merge_native_stages_combines_runtime_trees() {
        let temp = tempdir().unwrap();
        let win = temp.path().join("win-stage");
        let linux = temp.path().join("linux-stage");
        write_native_asset(&win, "win-x64", "dharastorage.dll");
        write_native_asset(&linux, "linux-x64", "libdharastorage.so");

        let output = temp.path().join("merged");
        merge_native_stages(&output, &[win.clone(), linux.clone()]).unwrap();

        assert!(
            output
                .join("runtimes/win-x64/native/dharastorage.dll")
                .is_file()
        );
        assert!(
            output
                .join("runtimes/linux-x64/native/libdharastorage.so")
                .is_file()
        );
    }

    #[test]
    fn merge_native_stages_rejects_missing_runtimes_directory() {
        let temp = tempdir().unwrap();
        let invalid = temp.path().join("invalid");
        fs::create_dir_all(&invalid).unwrap();

        let error = merge_native_stages(&temp.path().join("out"), &[invalid]).unwrap_err();

        assert!(error.to_string().contains("missing a runtimes directory"));
    }
}
