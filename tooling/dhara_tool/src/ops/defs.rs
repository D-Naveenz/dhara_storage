use std::path::{Path, PathBuf};

use anyhow::Result;
use tracing::{error, info};

use crate::command::{CommandResult, ReportField, StructuredReport, ToolContext};
use crate::paths::{default_defs_package_path, resolve_logs_dir, resolve_output_dir};

use super::{BuilderAction, current_log_path, execute_action, log_file_path};

/// Repo-relative working paths used by defs commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefsPaths {
    /// Repository root used to resolve canonical tooling paths.
    pub repo_root: PathBuf,
    /// Source directory or archive root used to discover TrID XML inputs.
    pub package_dir: PathBuf,
    /// Output directory used for generated `filedefs.dat` artifacts.
    pub output_dir: PathBuf,
    /// Directory where defs command log files are written.
    pub logs_dir: PathBuf,
}

impl DefsPaths {
    /// Resolves defs working paths from the current tool context.
    pub fn from_context(context: &ToolContext) -> Self {
        Self::from_repo_root(
            &context.repo_root,
            context.package_dir.clone(),
            context.output_dir.clone(),
            context.logs_dir.clone(),
        )
    }

    /// Resolves defs working paths from a repo root plus optional overrides.
    pub fn from_repo_root(
        repo_root: &Path,
        package_dir: Option<PathBuf>,
        output_dir: Option<PathBuf>,
        logs_dir: Option<PathBuf>,
    ) -> Self {
        Self {
            repo_root: repo_root.to_path_buf(),
            package_dir: package_dir
                .unwrap_or_else(|| repo_root.join("tooling").join("dhara_tool").join("package")),
            output_dir: resolve_output_dir(repo_root, output_dir.as_deref()),
            logs_dir: resolve_logs_dir(repo_root, output_dir.as_deref(), logs_dir.as_deref()),
        }
    }

    /// Returns the preferred default input path for TrID XML ingestion.
    pub fn default_trid_input_path(&self) -> PathBuf {
        let preferred_archive = self.package_dir.join("triddefs_xml.7z");
        if preferred_archive.exists() {
            return preferred_archive;
        }

        let preferred_directory = self.package_dir.join("triddefs_xml");
        if preferred_directory.exists() {
            return preferred_directory;
        }

        self.package_dir.clone()
    }

    /// Returns the default output path for generated `filedefs.dat` files.
    pub fn default_package_output_path(&self) -> PathBuf {
        default_defs_package_path(&self.repo_root)
    }
}

/// Supported defs subcommands exposed through `dhara_tool`.
#[derive(Debug, Clone)]
pub enum DefsCommand {
    Pack {
        output: Option<PathBuf>,
    },
    BuildTridXml {
        input: Option<PathBuf>,
        output: Option<PathBuf>,
    },
    Inspect {
        input: Option<PathBuf>,
    },
    InspectTridXml {
        input: Option<PathBuf>,
    },
    Normalize {
        input: Option<PathBuf>,
        output: Option<PathBuf>,
    },
    Verify {
        left: PathBuf,
        right: PathBuf,
    },
    SyncEmbedded {
        input: Option<PathBuf>,
        output: Option<PathBuf>,
        check: bool,
    },
}

/// Executes a defs command using repository-relative defaults and structured logging.
pub fn execute(command: DefsCommand, context: &ToolContext) -> Result<CommandResult> {
    let paths = DefsPaths::from_context(context);
    info!(
        target: "dhara_tool::ops::defs",
        command = ?command,
        repo_root = %context.repo_root.display(),
        package_dir = %paths.package_dir.display(),
        output_dir = %paths.output_dir.display(),
        logs_dir = %paths.logs_dir.display(),
        verbose = context.verbose,
        silent = context.silent,
        "starting defs command"
    );
    let log_path = current_log_path().unwrap_or_else(|| log_file_path(&paths.logs_dir));
    let action = resolve_action(command, &paths);
    let report = execute_action(action, &log_path, |_| {}).map_err(|error| {
        error!(
            target: "dhara_tool::ops::defs",
            log_path = %log_path.display(),
            error = %error,
            "defs command failed"
        );
        anyhow::anyhow!(error.to_string())
    })?;
    info!(
        target: "dhara_tool::ops::defs",
        title = report.title(),
        exit_code = report.exit_code(),
        log_path = %log_path.display(),
        "defs command completed"
    );

    Ok(CommandResult {
        exit_code: report.exit_code(),
        report: Some(StructuredReport {
            title: report.title().to_owned(),
            fields: report
                .fields()
                .iter()
                .map(|field| ReportField {
                    label: field.label().to_owned(),
                    value: field.value().to_owned(),
                })
                .collect(),
        }),
        message: None,
    })
}

/// Returns help text for the defs command group.
pub fn print_defs_help() -> String {
    [
        "Defs commands:",
        "  defs pack [--output <path>]",
        "  defs build-trid-xml [--input <path>] [--output <path>]",
        "  defs inspect [--input <path>]",
        "  defs inspect-trid-xml [--input <path>]",
        "  defs normalize [--input <path>] [--output <path>]",
        "  defs verify --left <path> --right <path>",
        "  defs sync-embedded [--input <path>] [--output <path>] [--check]",
    ]
    .join("\n")
}

fn resolve_action(command: DefsCommand, paths: &DefsPaths) -> BuilderAction {
    match command {
        DefsCommand::Pack { output } => BuilderAction::Pack {
            output: output.unwrap_or_else(|| paths.default_package_output_path()),
        },
        DefsCommand::BuildTridXml { input, output } => BuilderAction::BuildTridXml {
            input: input.unwrap_or_else(|| paths.default_trid_input_path()),
            output: output.unwrap_or_else(|| paths.default_package_output_path()),
        },
        DefsCommand::Inspect { input } => BuilderAction::Inspect {
            input: input.unwrap_or_else(|| paths.default_package_output_path()),
        },
        DefsCommand::InspectTridXml { input } => BuilderAction::InspectTridXml {
            input: input.unwrap_or_else(|| paths.default_trid_input_path()),
        },
        DefsCommand::Normalize { input, output } => BuilderAction::Normalize {
            input: input.unwrap_or_else(|| paths.default_package_output_path()),
            output: output.unwrap_or_else(|| paths.default_package_output_path()),
        },
        DefsCommand::Verify { left, right } => BuilderAction::Verify { left, right },
        DefsCommand::SyncEmbedded {
            input,
            output,
            check,
        } => BuilderAction::SyncEmbedded {
            input: input.unwrap_or_else(|| paths.default_trid_input_path()),
            output: output.unwrap_or_else(|| paths.default_package_output_path()),
            check,
        },
    }
}
