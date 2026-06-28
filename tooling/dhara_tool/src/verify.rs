use std::path::Path;

use anyhow::Result;

use crate::command::CommandResult;

use super::{DharaRepoConfig, PackageOptions, run_command, verify_release};

pub fn verify_release_config(repo_root: &Path) -> Result<CommandResult> {
    verify_release(repo_root)?;
    Ok(CommandResult::success())
}

pub fn verify_ci(repo_root: &Path, config: &DharaRepoConfig) -> Result<CommandResult> {
    verify_release_config(repo_root)?;

    for package in [
        "dhara_storage_dal",
        "dhara_storage",
        "dharastorage",
        "dhara_tool",
    ] {
        run_command(
            "cargo",
            &[
                "fmt".to_owned(),
                "-p".to_owned(),
                package.to_owned(),
                "--check".to_owned(),
            ],
            repo_root,
        )?;
    }

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
    for package in ["dhara_storage_dal", "dharastorage", "dhara_tool"] {
        run_command(
            "cargo",
            &[
                "clippy".to_owned(),
                "-p".to_owned(),
                package.to_owned(),
                "--all-targets".to_owned(),
                "--".to_owned(),
                "-D".to_owned(),
                "warnings".to_owned(),
            ],
            repo_root,
        )?;
    }

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
    for package in ["dhara_storage_dal", "dharastorage", "dhara_tool"] {
        run_command(
            "cargo",
            &["test".to_owned(), "-p".to_owned(), package.to_owned()],
            repo_root,
        )?;
    }

    run_command(
        "dotnet",
        &["test".to_owned(), config.ci.tests_project.clone()],
        repo_root,
    )?;

    Ok(CommandResult::success())
}

pub fn verify_docs(repo_root: &Path) -> Result<CommandResult> {
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
    for package in ["dhara_storage_dal", "dharastorage", "dhara_tool"] {
        run_command(
            "cargo",
            &[
                "doc".to_owned(),
                "-p".to_owned(),
                package.to_owned(),
                "--no-deps".to_owned(),
            ],
            repo_root,
        )?;
    }

    Ok(CommandResult::success())
}

pub fn verify_package(
    repo_root: &Path,
    config: &DharaRepoConfig,
    options: &PackageOptions,
) -> Result<CommandResult> {
    crate::nuget::verify(repo_root, config, options)
}
