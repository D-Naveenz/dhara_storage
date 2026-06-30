use std::path::Path;
use std::process::Command;

use anyhow::{Result, bail};

use crate::repo_config::DharaRepoConfig;
use crate::subprocess::run_command;

const WORKSPACE_CRATES: &[&str] = &[
    "dhara_storage_dal",
    "dhara_storage",
    "dharastorage",
    "dhara_tool",
];

const OTHER_CLIPPY_CRATES: &[&str] = &["dhara_storage_dal", "dharastorage", "dhara_tool"];

pub fn run_fmt(repo_root: &Path, check: bool) -> Result<()> {
    let mut args = vec!["fmt".to_owned()];
    for crate_name in WORKSPACE_CRATES {
        args.push("-p".to_owned());
        args.push((*crate_name).to_owned());
    }
    if check {
        args.push("--check".to_owned());
    }
    run_command("cargo", &args, repo_root)
}

pub fn run_clippy(repo_root: &Path) -> Result<()> {
    run_command(
        "cargo",
        &[
            "clippy".to_owned(),
            "-p".to_owned(),
            "dhara_storage".to_owned(),
            "--all-targets".to_owned(),
            "--all-features".to_owned(),
            "--".to_owned(),
            "-D".to_owned(),
            "warnings".to_owned(),
        ],
        repo_root,
    )?;
    let mut args = vec!["clippy".to_owned()];
    for crate_name in OTHER_CLIPPY_CRATES {
        args.push("-p".to_owned());
        args.push((*crate_name).to_owned());
    }
    args.extend([
        "--all-targets".to_owned(),
        "--".to_owned(),
        "-D".to_owned(),
        "warnings".to_owned(),
    ]);
    run_command("cargo", &args, repo_root)
}

pub fn run_doc(repo_root: &Path) -> Result<()> {
    run_command(
        "cargo",
        &[
            "doc".to_owned(),
            "-p".to_owned(),
            "dhara_storage".to_owned(),
            "--no-deps".to_owned(),
            "--all-features".to_owned(),
        ],
        repo_root,
    )?;
    let mut args = vec!["doc".to_owned(), "--no-deps".to_owned()];
    for crate_name in OTHER_CLIPPY_CRATES {
        args.push("-p".to_owned());
        args.push((*crate_name).to_owned());
    }
    run_command("cargo", &args, repo_root)
}

pub fn run_test_rust(repo_root: &Path) -> Result<()> {
    run_command(
        "cargo",
        &[
            "test".to_owned(),
            "-p".to_owned(),
            "dhara_storage".to_owned(),
            "--all-features".to_owned(),
        ],
        repo_root,
    )?;
    run_command(
        "cargo",
        &[
            "test".to_owned(),
            "-p".to_owned(),
            "dhara_storage_dal".to_owned(),
        ],
        repo_root,
    )?;
    run_command(
        "cargo",
        &[
            "test".to_owned(),
            "-p".to_owned(),
            "dharastorage".to_owned(),
        ],
        repo_root,
    )
}

pub fn run_test_dotnet(repo_root: &Path, config: &DharaRepoConfig) -> Result<()> {
    if !dotnet_available() {
        crate::log_module_step_debug("dotnet not found; skipping .NET tests");
        return Ok(());
    }
    run_command(
        "dotnet",
        &["test".to_owned(), config.ci.tests_project.clone()],
        repo_root,
    )
}

pub fn run_all(
    repo_root: &Path,
    config: &DharaRepoConfig,
    skip_docs: bool,
    skip_dotnet: bool,
) -> Result<()> {
    run_fmt(repo_root, true)?;
    run_clippy(repo_root)?;
    if !skip_docs {
        run_doc(repo_root)?;
    }
    run_test_rust(repo_root)?;
    if !skip_dotnet {
        run_test_dotnet(repo_root, config)?;
    }
    Ok(())
}

pub fn dotnet_available() -> bool {
    Command::new("dotnet")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub fn ensure_dotnet_available() -> Result<()> {
    if dotnet_available() {
        Ok(())
    } else {
        bail!("dotnet SDK was not found on PATH")
    }
}
