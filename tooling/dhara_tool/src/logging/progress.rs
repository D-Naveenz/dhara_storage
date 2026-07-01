use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::io::{IsTerminal, Write};
use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use tracing::{debug, info};

use crate::command::{RunMode, ToolContext};

use crate::filedefs::{ReduceTraceDetail, TridBuildProgress, TridBuildStage};

static PROGRESS_SETTINGS: OnceLock<ProgressSettings> = OnceLock::new();

const AUDIT_TARGET: &str = "dhara_tool::audit";

#[derive(Debug, Clone, Copy)]
pub struct ProgressSettings {
    pub trace: bool,
    pub run_mode: RunMode,
}

impl ProgressSettings {
    pub fn from_context(context: &ToolContext) -> Self {
        Self {
            trace: context.trace,
            run_mode: context.run_mode,
        }
    }

    pub fn console_enabled(self) -> bool {
        self.run_mode == RunMode::Direct && std::io::stderr().is_terminal()
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
            trace: false,
            run_mode: RunMode::Direct,
        })
}

thread_local! {
    static LAST_BUILD_STAGE: Cell<Option<TridBuildStage>> = const { Cell::new(None) };
    static LAST_CONSOLE_UPDATE: Cell<Option<Instant>> = const { Cell::new(None) };
    static PHASE_STARTS: RefCell<HashMap<TridBuildStage, Instant>> = RefCell::new(HashMap::new());
}

pub fn reset_build_progress_logging() {
    LAST_BUILD_STAGE.with(|stage| stage.set(None));
    LAST_CONSOLE_UPDATE.with(|last| last.set(None));
    PHASE_STARTS.with(|starts| starts.borrow_mut().clear());
}

static PROGRESS_DISPATCH_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
static GUI_PROGRESS_TX: OnceLock<Mutex<Option<Sender<TridBuildProgress>>>> = OnceLock::new();

pub fn register_gui_progress_sender(sender: Sender<TridBuildProgress>) {
    let slot = GUI_PROGRESS_TX.get_or_init(|| Mutex::new(None));
    *slot.lock().expect("gui progress lock poisoned") = Some(sender);
}

pub fn unregister_gui_progress_sender() {
    if let Some(slot) = GUI_PROGRESS_TX.get() {
        *slot.lock().expect("gui progress lock poisoned") = None;
    }
}

fn forward_gui_progress(update: &TridBuildProgress) {
    if settings().run_mode != RunMode::Interactive {
        return;
    }
    let Some(slot) = GUI_PROGRESS_TX.get() else {
        return;
    };
    if let Some(sender) = slot.lock().expect("gui progress lock poisoned").as_ref() {
        let _ = sender.send(update.clone());
    }
}

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
        forward_gui_progress(update);
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
        TridBuildStage::LoadSource => None,
        TridBuildStage::ExtractArchive => Some("extract"),
        TridBuildStage::ParseDefinitions => Some("parse"),
        TridBuildStage::ReduceDefinitions => Some("reduce"),
        TridBuildStage::FinalizePackage => Some("finalize"),
    }
}

fn log_audit_progress(update: &TridBuildProgress) {
    maybe_log_reduce_trace(update);
    handle_phase_timing(update);
}

fn maybe_log_reduce_trace(update: &TridBuildProgress) {
    if !settings().trace {
        return;
    }
    if update.stage != TridBuildStage::ReduceDefinitions {
        return;
    }
    let Some(detail) = &update.trace_detail else {
        return;
    };
    let file_type = update.current_item.as_deref().unwrap_or("unknown");
    let Some(total) = update.total.filter(|total| *total > 0) else {
        return;
    };
    let line = format_reduce_trace_line(update.current, total, file_type, detail);
    debug!(target: AUDIT_TARGET, "{line}");
}

pub fn format_reduce_trace_line(
    current: usize,
    total: usize,
    file_type: &str,
    detail: &ReduceTraceDetail,
) -> String {
    let suffix = match detail {
        ReduceTraceDetail::RejectedNoPatterns => "rejected: no patterns".to_string(),
        ReduceTraceDetail::RejectedExtensionFloodgate => {
            "rejected: extension floodgate".to_string()
        }
        ReduceTraceDetail::RejectedInvalidMime { raw_mime } => {
            format!("rejected: invalid MIME: {raw_mime}")
        }
        ReduceTraceDetail::Accepted => "accepted".to_string(),
        ReduceTraceDetail::AcceptedMimeFix { from, to } => {
            format!("accepted: fix: ({from} -> {to})")
        }
    };
    format!("({current}/{total}) {file_type} — {suffix}")
}

fn handle_phase_timing(update: &TridBuildProgress) {
    let stage = update.stage;
    if stage == TridBuildStage::LoadSource {
        return;
    }

    let stage_changed = LAST_BUILD_STAGE.with(|last| {
        let previous = last.get();
        if previous != Some(stage) {
            last.set(Some(stage));
            true
        } else {
            false
        }
    });

    if stage_changed || update.current == 0 {
        let now = Instant::now();
        PHASE_STARTS.with(|starts| {
            starts.borrow_mut().insert(stage, now);
        });
        debug!(
            target: AUDIT_TARGET,
            "phase {} started",
            phase_name(stage)
        );
    }

    let Some(summary) = phase_finish_summary(update) else {
        return;
    };

    let duration = PHASE_STARTS.with(|starts| {
        starts
            .borrow()
            .get(&stage)
            .map(|start| format_phase_duration(start.elapsed()))
            .unwrap_or_else(|| "0ms".to_owned())
    });

    info!(
        target: AUDIT_TARGET,
        "phase {} finished in {duration} — {summary}",
        phase_name(stage)
    );
}

fn phase_name(stage: TridBuildStage) -> &'static str {
    match stage {
        TridBuildStage::LoadSource => "load",
        TridBuildStage::ExtractArchive => "extract",
        TridBuildStage::ParseDefinitions => "parse",
        TridBuildStage::ReduceDefinitions => "reduce",
        TridBuildStage::FinalizePackage => "finalize",
    }
}

pub fn phase_finish_summary(update: &TridBuildProgress) -> Option<&str> {
    match update.stage {
        TridBuildStage::ExtractArchive if update.message.starts_with("extracted") => {
            Some(update.message.as_str())
        }
        TridBuildStage::ParseDefinitions if update.message.starts_with("Parsed ") => {
            Some(update.message.as_str())
        }
        TridBuildStage::ReduceDefinitions if update.message.starts_with("kept ") => {
            Some(update.message.as_str())
        }
        TridBuildStage::FinalizePackage if update.message.starts_with("trimmed ") => {
            Some(update.message.as_str())
        }
        _ => None,
    }
}

fn format_phase_duration(duration: Duration) -> String {
    let secs = duration.as_secs_f64();
    if secs >= 1.0 {
        format!("{secs:.1}s")
    } else {
        format!("{}ms", duration.as_millis())
    }
}

#[cfg(test)]
mod tests {
    use super::{format_reduce_trace_line, phase_finish_summary};
    use crate::filedefs::{ReduceTraceDetail, TridBuildProgress, TridBuildStage, TridBuildStats};

    #[test]
    fn format_reduce_trace_line_rejects_invalid_mime_with_value() {
        let line = format_reduce_trace_line(
            5511,
            21692,
            "BrainSuite Surface File Format",
            &ReduceTraceDetail::RejectedInvalidMime {
                raw_mime: "application/x-foo".to_owned(),
            },
        );
        assert!(line.contains("rejected: invalid MIME: application/x-foo"));
    }

    #[test]
    fn format_reduce_trace_line_accepts_without_fix() {
        let line = format_reduce_trace_line(1768, 21692, "PNG Image", &ReduceTraceDetail::Accepted);
        assert_eq!(line, "(1768/21692) PNG Image — accepted");
    }

    #[test]
    fn format_reduce_trace_line_accepts_with_mime_fix() {
        let line = format_reduce_trace_line(
            1234,
            21692,
            "JPEG Image",
            &ReduceTraceDetail::AcceptedMimeFix {
                from: "application/octet-stream".to_owned(),
                to: "image/jpeg".to_owned(),
            },
        );
        assert_eq!(
            line,
            "(1234/21692) JPEG Image — accepted: fix: (application/octet-stream -> image/jpeg)"
        );
    }

    #[test]
    fn phase_finish_detects_parse_completion() {
        let update = TridBuildProgress {
            stage: TridBuildStage::ParseDefinitions,
            message: "Parsed 100 definitions in 3.2s".to_owned(),
            current: 100,
            total: Some(100),
            current_item: None,
            stats: TridBuildStats::default(),
            trace_detail: None,
        };
        assert_eq!(
            phase_finish_summary(&update),
            Some("Parsed 100 definitions in 3.2s")
        );
    }

    #[test]
    fn reduce_milestone_progress_has_no_phase_finish() {
        let update = TridBuildProgress {
            stage: TridBuildStage::ReduceDefinitions,
            message: String::new(),
            current: 1000,
            total: Some(21692),
            current_item: Some("Sample Format".to_owned()),
            stats: TridBuildStats::default(),
            trace_detail: Some(ReduceTraceDetail::Accepted),
        };
        assert!(phase_finish_summary(&update).is_none());
    }
}
