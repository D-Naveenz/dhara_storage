use std::path::{Path, PathBuf};

use crate::filedefs::{
    SyncEmbeddedStatus, TridBuildProgress, TridTransformReport,
    build_trid_xml_package_with_progress, inspect_package, load_bundled_package, normalize_package,
    packages_match, sync_embedded_package, write_package,
};
use crate::logging::{
    log_build_progress, log_module_step_debug, log_module_step_warn, log_transform_statistics,
};
use crate::output::emit_stdout_line;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuilderAction {
    Pack {
        output: PathBuf,
    },
    BuildTridXml {
        input: PathBuf,
        output: PathBuf,
    },
    Inspect {
        input: PathBuf,
    },
    InspectTridXml {
        input: PathBuf,
    },
    Normalize {
        input: PathBuf,
        output: PathBuf,
    },
    Verify {
        left: PathBuf,
        right: PathBuf,
    },
    SyncEmbedded {
        input: PathBuf,
        output: PathBuf,
        check: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReportStatus {
    Success,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportField {
    pub(crate) label: String,
    pub(crate) value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandReport {
    pub(crate) title: String,
    pub(crate) status: ReportStatus,
    pub(crate) fields: Vec<ReportField>,
    pub(crate) exit_code: i32,
}

impl BuilderAction {
    pub fn title(&self) -> &'static str {
        match self {
            Self::Pack { .. } => "Bundled Package",
            Self::BuildTridXml { .. } => "Build Complete",
            Self::Inspect { .. } => "Package Summary",
            Self::InspectTridXml { .. } => "Transformation Preview",
            Self::Normalize { .. } => "Normalized Package",
            Self::Verify { .. } => "Verification",
            Self::SyncEmbedded { .. } => "Embedded Package Sync",
        }
    }

    pub fn is_long_running(&self) -> bool {
        matches!(
            self,
            Self::BuildTridXml { .. } | Self::InspectTridXml { .. } | Self::SyncEmbedded { .. }
        )
    }
}

impl ReportField {
    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn value(&self) -> &str {
        &self.value
    }
}

impl CommandReport {
    pub fn title(&self) -> &str {
        &self.title
    }

    #[allow(dead_code)]
    pub fn status(&self) -> &ReportStatus {
        &self.status
    }

    pub fn fields(&self) -> &[ReportField] {
        &self.fields
    }

    #[allow(dead_code)]
    pub fn exit_code(&self) -> i32 {
        self.exit_code
    }
}

pub fn execute_action<F>(
    action: BuilderAction,
    log_path: &Path,
    mut progress: F,
) -> Result<CommandReport, Box<dyn std::error::Error + Send + Sync>>
where
    F: FnMut(TridBuildProgress),
{
    let long_running = action.is_long_running();

    let report = match action {
        BuilderAction::Pack { output } => {
            log_module_step_debug("loading bundled runtime package");
            let package = load_bundled_package()?;
            let written = write_package(&package, &output)?;
            log_module_step_debug(&format!("wrote bundled package to {}", written.display()));
            CommandReport {
                title: "Bundled Package".to_string(),
                status: ReportStatus::Success,
                fields: vec![
                    field("Output", written.display().to_string()),
                    field("Log", log_path.display().to_string()),
                ],
                exit_code: 0,
            }
        }
        BuilderAction::BuildTridXml { input, output } => {
            if long_running {
                log_module_step_debug(&format!(
                    "building TrID XML package from {}",
                    input.display()
                ));
            }
            let build = build_trid_xml_package_with_progress(&input, |update| {
                log_build_progress(&update);
                progress(update);
            })?;
            log_transform_statistics(&build.report);
            let written = write_package(&build.package, &output)?;
            log_module_step_debug(&format!(
                "wrote definitions package to {}",
                written.display()
            ));
            let mut fields = vec![
                field("Input", input.display().to_string()),
                field("Output", written.display().to_string()),
                field("Log", log_path.display().to_string()),
            ];
            extend_transform_report_fields(&mut fields, &build.report);
            CommandReport {
                title: "Build Complete".to_string(),
                status: ReportStatus::Success,
                fields,
                exit_code: 0,
            }
        }
        BuilderAction::Inspect { input } => {
            let summary = inspect_package(&input)?;
            CommandReport {
                title: "Package Summary".to_string(),
                status: ReportStatus::Success,
                fields: vec![
                    field("Signature", summary.signature),
                    field("Package Id", summary.package_id),
                    field("Format", "DSFD"),
                    field("Package Version", summary.package_version),
                    field("Definitions Release", summary.definitions_release),
                    field("Package Revision", summary.package_revision.to_string()),
                    field("Tags", summary.tags.to_string()),
                    field("Definitions", summary.definition_count.to_string()),
                    field("Log", log_path.display().to_string()),
                ],
                exit_code: 0,
            }
        }
        BuilderAction::InspectTridXml { input } => {
            if long_running {
                log_module_step_debug(&format!(
                    "previewing TrID XML transformation from {}",
                    input.display()
                ));
            }
            let build = build_trid_xml_package_with_progress(&input, |update| {
                log_build_progress(&update);
                progress(update);
            })?;
            log_transform_statistics(&build.report);
            let mut fields = vec![field("Log", log_path.display().to_string())];
            extend_transform_report_fields(&mut fields, &build.report);
            CommandReport {
                title: "Transformation Preview".to_string(),
                status: ReportStatus::Success,
                fields,
                exit_code: 0,
            }
        }
        BuilderAction::Normalize { input, output } => {
            let written = normalize_package(&input, &output)?;
            log_module_step_debug(&format!(
                "normalized package written to {}",
                written.display()
            ));
            CommandReport {
                title: "Normalized Package".to_string(),
                status: ReportStatus::Success,
                fields: vec![
                    field("Input", input.display().to_string()),
                    field("Output", written.display().to_string()),
                    field("Log", log_path.display().to_string()),
                ],
                exit_code: 0,
            }
        }
        BuilderAction::Verify { left, right } => {
            let matches = packages_match(&left, &right)?;
            if !matches {
                log_module_step_warn(&format!(
                    "package verification differed: {} vs {}",
                    left.display(),
                    right.display()
                ));
            }
            CommandReport {
                title: "Verification".to_string(),
                status: if matches {
                    ReportStatus::Success
                } else {
                    ReportStatus::Warning
                },
                fields: vec![
                    field("Left", left.display().to_string()),
                    field("Right", right.display().to_string()),
                    field("Result", if matches { "match" } else { "different" }),
                    field("Log", log_path.display().to_string()),
                ],
                exit_code: if matches { 0 } else { 1 },
            }
        }
        BuilderAction::SyncEmbedded {
            input,
            output,
            check,
        } => {
            if long_running {
                log_module_step_debug(&format!(
                    "syncing embedded definitions package from {}",
                    input.display()
                ));
            }
            let outcome = sync_embedded_package(&input, &output, check, |update| {
                log_build_progress(&update);
                progress(update);
            })?;
            let (status, exit_code, result) = match outcome.status {
                SyncEmbeddedStatus::Skipped => {
                    log_module_step_warn(&outcome.detail);
                    (ReportStatus::Success, 0, "skipped")
                }
                SyncEmbeddedStatus::UpToDate => {
                    log_module_step_debug(&outcome.detail);
                    (ReportStatus::Success, 0, "up-to-date")
                }
                SyncEmbeddedStatus::Updated => {
                    log_module_step_debug(&outcome.detail);
                    (ReportStatus::Success, 0, "updated")
                }
                SyncEmbeddedStatus::NeedsUpdate => {
                    log_module_step_warn(&outcome.detail);
                    (ReportStatus::Warning, 1, "update required")
                }
            };
            let mut fields = vec![
                field("Input", outcome.input.display().to_string()),
                field("Output", outcome.output.display().to_string()),
                field("Result", result),
            ];
            if let Some(package_version) = outcome.package_version {
                fields.push(field("Package Version", package_version));
            }
            fields.push(field("Detail", outcome.detail));
            fields.push(field("Log", log_path.display().to_string()));

            CommandReport {
                title: "Embedded Package Sync".to_string(),
                status,
                fields,
                exit_code,
            }
        }
    };

    Ok(report)
}

pub fn print_report(report: &CommandReport) {
    emit_stdout_line(report.title().to_owned());
    for entry in report.fields() {
        emit_stdout_line(format!("{:<20} {}", entry.label(), entry.value()));
    }
}

fn extend_transform_report_fields(fields: &mut Vec<ReportField>, report: &TridTransformReport) {
    fields.push(field("Total Parsed", report.total_parsed.to_string()));
    fields.push(field("MIME Corrected", report.mime_corrected.to_string()));
    fields.push(field("MIME Rejected", report.mime_rejected.to_string()));
    fields.push(field(
        "Extension Rejected",
        report.extension_rejected.to_string(),
    ));
    fields.push(field(
        "Signature Rejected",
        report.signature_rejected.to_string(),
    ));
    fields.push(field("Final Trimmed", report.final_trimmed.to_string()));
    fields.push(field("Final Kept", report.final_kept.to_string()));
}

fn field(label: impl Into<String>, value: impl Into<String>) -> ReportField {
    ReportField {
        label: label.into(),
        value: value.into(),
    }
}
