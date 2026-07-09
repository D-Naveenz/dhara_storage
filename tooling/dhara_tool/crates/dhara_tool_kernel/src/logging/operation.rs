use std::time::{Duration, Instant};

use tracing::{info, warn};

use crate::context::CommandResult;
use crate::operation_progress::{
    clear_run_activity, clear_run_clock, install_run_clock, set_command_activity,
    set_command_milestone,
};

use super::audit::{format_duration, summarize_command_result, AUDIT_TARGET};

pub const ELAPSED_UI_THRESHOLD: Duration = Duration::from_secs(4);

/// Human-readable present/past tense labels for operator audit prose.
#[derive(Debug, Clone)]
pub struct ActivityLabel {
    pub present: String,
    pub past: String,
    pub failure_subject: String,
}

/// Outcome passed to [`CommandRun::complete`].
#[derive(Debug, Clone)]
pub struct CommandOutcome {
    pub exit_code: i32,
    pub summary: Option<String>,
    pub error: Option<String>,
}

impl CommandOutcome {
    pub fn from_execute(command_id: &str, result: &Result<CommandResult, anyhow::Error>) -> Self {
        match result {
            Ok(command_result) => Self {
                exit_code: command_result.exit_code,
                summary: Some(summarize_command_result(command_id, command_result)),
                error: None,
            },
            Err(error) => Self {
                exit_code: 1,
                summary: None,
                error: Some(error.to_string()),
            },
        }
    }
}

/// RAII guard for one command execution: audit open on `begin`, close on `complete`.
pub struct CommandRun {
    activity: ActivityLabel,
    started: Instant,
}

impl CommandRun {
    pub fn begin(command_id: &str) -> Self {
        crate::logging::progress::reset_build_progress_logging();
        let activity = command_labels(command_id);
        info!(target: AUDIT_TARGET, "{}…", activity.present);

        let started = Instant::now();
        if crate::logging::interactive_mode_enabled() {
            install_run_clock(started);
            set_command_milestone(&activity.present);
            set_command_activity(&activity.present);
        }

        Self { activity, started }
    }

    pub fn complete(self, outcome: CommandOutcome) {
        if crate::logging::interactive_mode_enabled() {
            clear_run_activity();
            clear_run_clock();
        }

        let duration = format_duration(self.started.elapsed());

        if outcome.exit_code == 0 {
            let mut line = format!("{} in {duration}", self.activity.past);
            if let Some(summary) = outcome
                .summary
                .filter(|value| !value.is_empty() && value != "completed")
            {
                line.push_str(&format!(" — {summary}"));
            }
            info!(target: AUDIT_TARGET, "{line}");
            return;
        }

        if let Some(error) = outcome.error {
            warn!(
                target: AUDIT_TARGET,
                "{} failed after {duration} — {error}",
                self.activity.failure_subject
            );
        } else {
            warn!(
                target: AUDIT_TARGET,
                "{} failed after {duration} — exit {}",
                self.activity.failure_subject,
                outcome.exit_code
            );
        }
    }
}

/// Maps a command id to operator-facing activity labels.
pub fn command_labels(command_id: &str) -> ActivityLabel {
    let (present, past, failure_subject) = match command_id {
        "defs.build-trid-xml" => (
            "building definitions package",
            "built definitions package",
            "definitions package build",
        ),
        "defs.inspect-trid-xml" => (
            "inspecting definitions package",
            "inspected definitions package",
            "definitions package inspection",
        ),
        "defs.sync-embedded" => (
            "syncing embedded definitions",
            "synced embedded definitions",
            "embedded definitions sync",
        ),
        "defs.inspect" => (
            "inspecting definitions",
            "inspected definitions",
            "definitions inspection",
        ),
        "verify.package" => (
            "verifying package",
            "verified package",
            "package verification",
        ),
        "package.pack" => (
            "packaging release",
            "packaged release",
            "release packaging",
        ),
        "package.publish" => (
            "publishing package",
            "published package",
            "package publish",
        ),
        "release.run" => (
            "running release",
            "completed release",
            "release run",
        ),
        "quality.run" => (
            "running quality checks",
            "completed quality checks",
            "quality run",
        ),
        "config.show" => (
            "loading configuration",
            "loaded configuration",
            "configuration load",
        ),
        other => {
            let human = command_id_to_words(other);
            return ActivityLabel {
                present: format!("running {human}"),
                past: format!("completed {human}"),
                failure_subject: format!("{human} run"),
            };
        }
    };

    ActivityLabel {
        present: present.to_owned(),
        past: past.to_owned(),
        failure_subject: failure_subject.to_owned(),
    }
}

/// Present-tense label for a TrID pipeline phase (audit / direct-mode console).
pub fn phase_activity_label(stage: crate::filedefs::TridBuildStage) -> &'static str {
    use crate::filedefs::TridBuildStage;
    match stage {
        TridBuildStage::LoadSource => "loading TrID source",
        TridBuildStage::ExtractArchive => "extracting archive",
        TridBuildStage::ParseDefinitions => "parsing definitions",
        TridBuildStage::ReduceDefinitions => "reducing definitions",
        TridBuildStage::FinalizePackage => "finalizing package",
    }
}

fn command_id_to_words(command_id: &str) -> String {
    command_id.replace('.', " ").replace('-', " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_labels_defs_build() {
        let labels = command_labels("defs.build-trid-xml");
        assert_eq!(labels.present, "building definitions package");
        assert_eq!(labels.past, "built definitions package");
    }

    #[test]
    fn command_labels_fallback() {
        let labels = command_labels("foo.bar-baz");
        assert!(labels.present.contains("foo bar baz"));
    }
}
