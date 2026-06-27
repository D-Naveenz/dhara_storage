use std::path::{Path, PathBuf};

use tracing::{info, warn};

use super::builder::{
    TridBuildProgress, build_trid_xml_package_with_progress, inspect_package, load_bundled_package,
    normalize_package, packages_match, sync_embedded_package, write_package,
};
use super::logging::{log_build_progress, log_task_step, log_transform_statistics};
use super::output::emit_stdout_line;

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
    info!(
        target: "dhara_tool::ops::runner",
        action = ?action,
        log_path = %log_path.display(),
        "executing builder action"
    );

    let report = match action {
        BuilderAction::Pack { output } => {
            log_task_step("load bundled runtime package", "started", None);
            let package = load_bundled_package()?;
            log_task_step("load bundled runtime package", "success", None);

            log_task_step(
                "write bundled package",
                "started",
                Some(&output.display().to_string()),
            );
            let written = write_package(&package, &output)?;
            log_task_step(
                "write bundled package",
                "success",
                Some(&written.display().to_string()),
            );

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
            log_task_step(
                "build TrID XML package",
                "started",
                Some(&input.display().to_string()),
            );
            let build = build_trid_xml_package_with_progress(&input, |update| {
                log_build_progress(&update);
                progress(update);
            })?;
            log_transform_statistics(&build.report);
            log_task_step(
                "build TrID XML package",
                "success",
                Some(&output.display().to_string()),
            );

            log_task_step(
                "write definitions package",
                "started",
                Some(&output.display().to_string()),
            );
            let written = write_package(&build.package, &output)?;
            log_task_step(
                "write definitions package",
                "success",
                Some(&written.display().to_string()),
            );

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
            log_task_step(
                "inspect definitions package",
                "started",
                Some(&input.display().to_string()),
            );
            let summary = inspect_package(&input)?;
            log_task_step(
                "inspect definitions package",
                "success",
                Some(&input.display().to_string()),
            );
            CommandReport {
                title: "Package Summary".to_string(),
                status: ReportStatus::Success,
                fields: vec![
                    field("Package Id", summary.package_id),
                    field("Format", "FlatBuffers"),
                    field("Package Version", summary.package_version),
                    field("Source Version", summary.source_version),
                    field("Package Revision", summary.package_revision.to_string()),
                    field("Tags", summary.tags.to_string()),
                    field("Definitions", summary.definition_count.to_string()),
                    field("Log", log_path.display().to_string()),
                ],
                exit_code: 0,
            }
        }
        BuilderAction::InspectTridXml { input } => {
            log_task_step(
                "inspect TrID XML transformation",
                "started",
                Some(&input.display().to_string()),
            );
            let build = build_trid_xml_package_with_progress(&input, |update| {
                log_build_progress(&update);
                progress(update);
            })?;
            log_transform_statistics(&build.report);
            log_task_step(
                "inspect TrID XML transformation",
                "success",
                Some(&input.display().to_string()),
            );

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
            log_task_step(
                "normalize definitions package",
                "started",
                Some(&input.display().to_string()),
            );
            let written = normalize_package(&input, &output)?;
            log_task_step(
                "normalize definitions package",
                "success",
                Some(&written.display().to_string()),
            );
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
            log_task_step("verify package equivalence", "started", None);
            let matches = packages_match(&left, &right)?;
            if !matches {
                warn!(
                    target: "dhara_tool::ops::runner",
                    left = %left.display(),
                    right = %right.display(),
                    "package verification reported differences"
                );
                log_task_step("verify package equivalence", "warning", Some("packages differ"));
            } else {
                log_task_step("verify package equivalence", "success", Some("packages match"));
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
            log_task_step(
                "sync runtime definitions package",
                "started",
                Some(&format!(
                    "input={}; output={}; check={check}",
                    input.display(),
                    output.display()
                )),
            );
            let outcome = sync_embedded_package(&input, &output, check)?;
            let (status, exit_code, result) = match outcome.status {
                super::builder::SyncEmbeddedStatus::Skipped => {
                    log_task_step(
                        "sync runtime definitions package",
                        "skipped",
                        Some(outcome.detail.as_str()),
                    );
                    (ReportStatus::Success, 0, "skipped")
                }
                super::builder::SyncEmbeddedStatus::UpToDate => {
                    log_task_step(
                        "sync runtime definitions package",
                        "success",
                        Some(outcome.detail.as_str()),
                    );
                    (ReportStatus::Success, 0, "up-to-date")
                }
                super::builder::SyncEmbeddedStatus::Updated => {
                    log_task_step(
                        "sync runtime definitions package",
                        "success",
                        Some(outcome.detail.as_str()),
                    );
                    (ReportStatus::Success, 0, "updated")
                }
                super::builder::SyncEmbeddedStatus::NeedsUpdate => {
                    log_task_step(
                        "sync runtime definitions package",
                        "warning",
                        Some(outcome.detail.as_str()),
                    );
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

    log_runner_report(&report);
    Ok(report)
}

pub fn print_report(report: &CommandReport) {
    emit_stdout_line(report.title().to_owned());
    for entry in report.fields() {
        emit_stdout_line(format!("{:<20} {}", entry.label(), entry.value()));
    }
}

fn log_runner_report(report: &CommandReport) {
    let outcome = match report.status {
        ReportStatus::Success => "success",
        ReportStatus::Warning => "warning",
    };

    info!(
        target: "dhara_tool::audit",
        title = report.title.as_str(),
        outcome,
        exit_code = report.exit_code,
        field_count = report.fields.len(),
        "operation completed"
    );
}

fn extend_transform_report_fields(
    fields: &mut Vec<ReportField>,
    report: &super::builder::TridTransformReport,
) {
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
