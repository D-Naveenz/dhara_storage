use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

pub fn run_with_msvc_env(command: &str) -> Result<()> {
    let vs_install = locate_visual_studio_install()?;
    let vcvars = vs_install
        .join("VC")
        .join("Auxiliary")
        .join("Build")
        .join("vcvarsall.bat");
    if !vcvars.is_file() {
        bail!(
            "Visual Studio vcvarsall.bat was not found at {}",
            vcvars.display()
        );
    }

    let status = Command::new("cmd.exe")
        .arg("/d")
        .arg("/c")
        .arg(format!(
            "call \"{}\" x64_arm64 && {}",
            vcvars.display(),
            command
        ))
        .status()
        .context("failed to spawn cmd.exe for MSVC environment")?;

    if !status.success() {
        bail!("MSVC command failed with status {status}: {command}");
    }

    Ok(())
}

fn locate_visual_studio_install() -> Result<std::path::PathBuf> {
    let program_files_x86 = std::env::var("ProgramFiles(x86)")
        .context("ProgramFiles(x86) is not set; MSVC tooling requires Windows")?;
    let vswhere = Path::new(&program_files_x86)
        .join("Microsoft Visual Studio")
        .join("Installer")
        .join("vswhere.exe");
    if !vswhere.is_file() {
        bail!(
            "vswhere.exe was not found at {}; install Visual Studio build tools",
            vswhere.display()
        );
    }

    let output = Command::new(&vswhere)
        .args([
            "-latest",
            "-products",
            "*",
            "-requires",
            "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
            "Microsoft.VisualStudio.Component.VC.Tools.ARM64",
            "-property",
            "installationPath",
        ])
        .output()
        .with_context(|| format!("failed to run {}", vswhere.display()))?;

    if !output.status.success() {
        bail!("vswhere failed with status {}", output.status);
    }

    let install = String::from_utf8(output.stdout)
        .context("vswhere output was not valid UTF-8")?
        .trim()
        .to_owned();
    if install.is_empty() {
        bail!("Visual Studio with x64 and ARM64 MSVC build tools was not found");
    }

    Ok(Path::new(&install).to_path_buf())
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(not(windows))]
    fn run_with_msvc_env_is_windows_only() {
        let error = super::run_with_msvc_env("echo test").unwrap_err();
        assert!(error.to_string().contains("MSVC"));
    }
}
