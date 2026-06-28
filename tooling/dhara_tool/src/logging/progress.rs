use std::cell::Cell;
use std::io::{IsTerminal, Write};
use std::sync::Mutex;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use tracing::{debug, info};

use crate::command::{RunMode, ToolContext};

use crate::filedefs::{TridBuildProgress, TridBuildStage};

static PROGRESS_SETTINGS: OnceLock<ProgressSettings> = OnceLock::new();

const AUDIT_TARGET: &str = "dhara_tool::audit";

#[derive(Debug, Clone, Copy)]
pub struct ProgressSettings {
    pub minimal: bool,
    pub trace: bool,
    pub run_mode: RunMode,
}

impl ProgressSettings {
    pub fn from_context(context: &ToolContext) -> Self {
        Self {
            minimal: context.minimal,
            trace: context.trace,
            run_mode: context.run_mode,
        }
    }

    pub fn console_enabled(self) -> bool {
        self.run_mode == RunMode::Direct && !self.minimal && std::io::stderr().is_terminal()
    }
}

pub fn init_progress_settings(context: &ToolContext) {
    let _ = PROGRESS_SETTINGS.set(ProgressSettings::from_context(context));
}

fn settings() -> ProgressSettings {
    PROGRESS_SETTINGS
        .get()
        .copied()
        .unwrap_or(ProgressSettings {
            minimal: false,
            trace: false,
            run_mode: RunMode::Direct,
        })
}

thread_local! {
    static LAST_BUILD_STAGE: Cell<Option<TridBuildStage>> = const { Cell::new(None) };
    static LAST_BUILD_MILESTONE: Cell<usize> = const { Cell::new(0) };
    static LAST_CONSOLE_UPDATE: Cell<Option<Instant>> = const { Cell::new(None) };
}

pub fn reset_build_progress_logging() {
    LAST_BUILD_STAGE.with(|stage| stage.set(None));
    LAST_BUILD_MILESTONE.with(|milestone| milestone.set(0));
    LAST_CONSOLE_UPDATE.with(|last| last.set(None));
}

static PROGRESS_DISPATCH_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn with_progress_lock<R>(operation: impl FnOnce() -> R) -> R {
    let lock = PROGRESS_DISPATCH_LOCK.get_or_init(|| Mutex::new(()));
    let guard = lock.lock().expect("progress dispatch lock poisoned");
    let result = operation();
    drop(guard);
    result
}

pub fn emit_trid_progress(update: TridBuildProgress) {
    dispatch_trid_progress(&update);
}

pub fn dispatch_trid_progress(update: &TridBuildProgress) {
    with_progress_lock(|| {
        write_console_progress(update);
        log_audit_progress(update);
    });
}

fn write_console_progress(update: &TridBuildProgress) {
    if !settings().console_enabled() {
        return;
    }

    let Some(label) = console_stage_label(update.stage) else {
        return;
    };

    let total = update.total.filter(|total| *total > 0);
    let current = update.current;
    if total.is_none() && current == 0 {
        let _ = writeln!(std::io::stderr(), "{label}: {}", update.message);
        return;
    }

    let Some(total) = total else {
        return;
    };

    if !should_refresh_console(current, total) {
        return;
    }

    let line = format!("{label}: {current}/{total}");
    if std::io::stderr().is_terminal() {
        let _ = write!(std::io::stderr(), "\r{line:<40}");
        if current >= total {
            let _ = writeln!(std::io::stderr());
        }
    } else {
        let _ = writeln!(std::io::stderr(), "{line}");
    }
}

fn should_refresh_console(current: usize, total: usize) -> bool {
    if current == 0 || current >= total {
        return true;
    }

    LAST_CONSOLE_UPDATE.with(|last| {
        let now = Instant::now();
        let refresh = match last.get() {
            None => true,
            Some(previous) => now.duration_since(previous) >= Duration::from_millis(250),
        };
        if refresh {
            last.set(Some(now));
        }
        refresh
    })
}

fn console_stage_label(stage: TridBuildStage) -> Option<&'static str> {
    match stage {
        TridBuildStage::LoadSource => Some("load"),
        TridBuildStage::ExtractArchive => Some("extract"),
        TridBuildStage::ParseDefinitions => Some("parse"),
        TridBuildStage::ReduceDefinitions => Some("reduce"),
        TridBuildStage::FinalizePackage => Some("finalize"),
    }
}

fn log_audit_progress(update: &TridBuildProgress) {
    let Some(line) = format_audit_line(update) else {
        return;
    };

    if audit_should_log_at_info(update) {
        info!(target: AUDIT_TARGET, "{line}");
    } else if settings().trace {
        debug!(target: AUDIT_TARGET, "{line}");
    }
}

fn format_audit_line(update: &TridBuildProgress) -> Option<String> {
    match update.stage {
        TridBuildStage::LoadSource => Some(format!("stage: load source — {}", update.message)),
        TridBuildStage::ExtractArchive => {
            Some(format!("stage: extract archive — {}", update.message))
        }
        TridBuildStage::FinalizePackage => {
            Some(format!("stage: finalize package — {}", update.message))
        }
        TridBuildStage::ParseDefinitions => {
            if update.current == 0 {
                return Some(format!("stage: parse definitions — {}", update.message));
            }
            if update.message.starts_with("Parsed ") {
                return Some(format!("stage: parse definitions — {}", update.message));
            }
            None
        }
        TridBuildStage::ReduceDefinitions => {
            if update.current == 0 {
                return Some(format!("stage: reduce definitions — {}", update.message));
            }
            let total = update.total?;
            if settings().trace {
                let item = update.current_item.as_deref()?;
                let outcome = humanize_reduce_message(&update.message);
                return Some(format!("({}/{}) {item} — {outcome}", update.current, total));
            }
            if update.current >= total || update.current.is_multiple_of(1_000) {
                return Some(format!("({}/{}) reduce in progress", update.current, total));
            }
            None
        }
    }
}

fn humanize_reduce_message(message: &str) -> &str {
    match message {
        "Accepting validated definition" => "accepted",
        "Rejecting definition without patterns" => "rejected: no patterns",
        "Rejecting definition by extension floodgate" => "rejected: extension floodgate",
        "Rejecting definition by MIME validation" => "rejected: invalid MIME",
        other => other,
    }
}

fn audit_should_log_at_info(update: &TridBuildProgress) -> bool {
    let stage_changed = LAST_BUILD_STAGE.with(|last| {
        let previous = last.get();
        if previous != Some(update.stage) {
            last.set(Some(update.stage));
            true
        } else {
            false
        }
    });

    if stage_changed || update.current == 0 {
        return true;
    }

    if update.stage == TridBuildStage::ParseDefinitions {
        return update.message.starts_with("Parsed ");
    }

    if update.stage == TridBuildStage::ReduceDefinitions {
        let at_total = update
            .total
            .is_some_and(|total| total > 0 && update.current == total);
        let at_milestone = update.current > 0
            && LAST_BUILD_MILESTONE.with(|last| {
                let previous = last.get();
                let milestone = update.current / 1_000;
                let previous_milestone = previous / 1_000;
                if milestone > previous_milestone {
                    last.set(update.current);
                    true
                } else {
                    false
                }
            });
        return at_total || at_milestone;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::{format_audit_line, humanize_reduce_message};
    use crate::filedefs::{TridBuildProgress, TridBuildStage, TridBuildStats};

    #[test]
    fn parse_stage_skips_per_file_audit_lines() {
        assert!(
            format_audit_line(&TridBuildProgress {
                stage: TridBuildStage::ParseDefinitions,
                message: "Parsing XML definition".to_owned(),
                current: 42,
                total: Some(100),
                current_item: Some("sample.trid.xml".to_owned()),
                stats: TridBuildStats::default(),
            })
            .is_none()
        );
    }

    #[test]
    fn parse_stage_keeps_completion_summary() {
        let line = format_audit_line(&TridBuildProgress {
            stage: TridBuildStage::ParseDefinitions,
            message: "Parsed 100 definitions in 3.2s".to_owned(),
            current: 100,
            total: Some(100),
            current_item: None,
            stats: TridBuildStats::default(),
        })
        .unwrap();
        assert!(line.contains("Parsed 100 definitions"));
    }

    #[test]
    fn humanize_reduce_message_maps_known_outcomes() {
        assert_eq!(
            humanize_reduce_message("Rejecting definition by extension floodgate"),
            "rejected: extension floodgate"
        );
    }
}
