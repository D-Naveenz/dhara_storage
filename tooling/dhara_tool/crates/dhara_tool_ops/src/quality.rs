use std::path::Path;
use std::process::Command;

use anyhow::{Result, bail};

use dhara_tool_kernel::{repo_config::DharaRepoConfig, subprocess::run_command};

use crate::workflow_progress::{begin_workflow, plan_unit_step, run_planned_step, run_workflow_step};

const WORKSPACE_CRATES: &[&str] = &[
    "dhara_storage_dal",
    "dhara_storage",
    "dharastorage-ffi",
    "dhara_tool",
];

const OTHER_CLIPPY_CRATES: &[&str] = &["dhara_storage_dal", "dharastorage-ffi", "dhara_tool"];

pub fn run_fmt(repo_root: &Path, check: bool) -> Result<()> {
    run_workflow_step("fmt", "Formatting Rust", "Running cargo fmt", || run_fmt_inner(repo_root, check))
}

fn run_fmt_inner(repo_root: &Path, check: bool) -> Result<()> {
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
    run_workflow_step("clippy", "Running Clippy", "Running cargo clippy", || {
        run_clippy_inner(repo_root)
    })
}

fn run_clippy_inner(repo_root: &Path) -> Result<()> {
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
    run_workflow_step("doc", "Building docs", "Running cargo doc", || run_doc_inner(repo_root))
}

fn run_doc_inner(repo_root: &Path) -> Result<()> {
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
    run_workflow_step(
        "test-rust",
        "Running Rust tests",
        "Running cargo test",
        || run_test_rust_inner(repo_root),
    )
}

fn run_test_rust_inner(repo_root: &Path) -> Result<()> {
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
            "dharastorage-ffi".to_owned(),
        ],
        repo_root,
    )
}

pub fn run_test_dotnet(repo_root: &Path, config: &DharaRepoConfig) -> Result<()> {
    if !dotnet_available() {
        dhara_tool_kernel::log_module_step_debug("dotnet not found; skipping .NET tests");
        return Ok(());
    }
    run_workflow_step(
        "test-dotnet",
        "Running .NET tests",
        "Running dotnet test",
        || {
            run_command(
                "dotnet",
                &["test".to_owned(), config.ci.tests_project.clone()],
                repo_root,
            )
        },
    )
}

pub fn run_all(
    repo_root: &Path,
    config: &DharaRepoConfig,
    skip_docs: bool,
    skip_dotnet: bool,
) -> Result<()> {
    let include_dotnet = !skip_dotnet && dotnet_available();
    if let Some(session) = begin_workflow("Planning quality checks…") {
        plan_unit_step(&session, "fmt", "Formatting Rust");
        plan_unit_step(&session, "clippy", "Running Clippy");
        if !skip_docs {
            plan_unit_step(&session, "doc", "Building docs");
        }
        plan_unit_step(&session, "test-rust", "Running Rust tests");
        if include_dotnet {
            plan_unit_step(&session, "test-dotnet", "Running .NET tests");
        }
        session.commit();
    }

    run_planned_step("fmt", "Formatting Rust", "Running cargo fmt", || {
        run_fmt_inner(repo_root, true)
    })?;
    run_planned_step("clippy", "Running Clippy", "Running cargo clippy", || {
        run_clippy_inner(repo_root)
    })?;
    if !skip_docs {
        run_planned_step("doc", "Building docs", "Running cargo doc", || run_doc_inner(repo_root))?;
    }
    run_planned_step("test-rust", "Running Rust tests", "Running cargo test", || {
        run_test_rust_inner(repo_root)
    })?;
    if include_dotnet {
        run_planned_step("test-dotnet", "Running .NET tests", "Running dotnet test", || {
            run_command(
                "dotnet",
                &["test".to_owned(), config.ci.tests_project.clone()],
                repo_root,
            )
        })?;
    } else if !skip_dotnet {
        dhara_tool_kernel::log_module_step_debug("dotnet not found; skipping .NET tests");
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
        bail!("dotnet SDK is required for this command but was not found on PATH");
    }
}
