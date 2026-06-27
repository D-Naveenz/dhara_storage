use std::cell::Cell;
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use chrono::{Local, NaiveDate};
use tracing::{debug, error, info, warn};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::Layer;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::command::{CommandResult, ToolContext};
use crate::paths::{resolve_logs_dir, resolve_output_dir};

use super::builder::{TridBuildProgress, TridBuildStage, TridTransformReport};

static LOGGING: OnceLock<LoggingRuntime> = OnceLock::new();

const LOG_FILE_STEM: &str = "dhara_tool";

#[derive(Debug, Clone)]
pub struct LoggingOptions {
    pub silent: bool,
    pub verbose: u8,
    pub logs_dir: PathBuf,
    pub interactive: bool,
}

pub struct LoggingRuntime {
    pub log_path: PathBuf,
    _guard: WorkerGuard,
}

impl LoggingOptions {
    pub fn from_context(context: &ToolContext, interactive: bool) -> Self {
        Self {
            silent: context.silent,
            verbose: context.verbose,
            logs_dir: resolve_logs_dir(
                &context.repo_root,
                context.output_dir.as_deref(),
                context.logs_dir.as_deref(),
            ),
            interactive,
        }
    }
}

/// Initializes file logging once per process and returns the active runtime.
pub fn ensure_logging(options: LoggingOptions) -> Result<&'static LoggingRuntime, std::io::Error> {
    if let Some(runtime) = LOGGING.get() {
        return Ok(runtime);
    }

    let runtime = init_logging(options)?;
    let _ = LOGGING.set(runtime);
    LOGGING
        .get()
        .ok_or_else(|| std::io::Error::other("logging runtime was not initialized"))
}

pub fn init_logging(options: LoggingOptions) -> Result<LoggingRuntime, std::io::Error> {
    let log_path = allocate_log_path(&options.logs_dir)?;
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;

    let (writer, guard) = tracing_appender::non_blocking(file);

    let console_max_level = if options.interactive || options.silent {
        LevelFilter::ERROR
    } else {
        match options.verbose {
            0 => LevelFilter::WARN,
            1 => LevelFilter::INFO,
            2 => LevelFilter::DEBUG,
            _ => LevelFilter::TRACE,
        }
    };

    let file_max_level = match options.verbose {
        0 => LevelFilter::INFO,
        1 => LevelFilter::DEBUG,
        _ => LevelFilter::TRACE,
    };

    let console_layer = fmt::layer()
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .without_time()
        .with_span_events(FmtSpan::NONE)
        .compact()
        .with_filter(console_max_level);

    let file_layer = fmt::layer()
        .with_writer(writer)
        .with_target(true)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_ansi(false)
        .with_span_events(FmtSpan::NONE)
        .compact()
        .with_filter(file_max_level);

    tracing_subscriber::registry()
        .with(console_layer)
        .with(file_layer)
        .try_init()
        .map_err(io_error_from_set_global_default)?;

    info!(
        target: "dhara_tool::audit",
        log_path = %log_path.display(),
        session = log_session_from_path(&log_path),
        verbose = options.verbose,
        silent = options.silent,
        interactive = options.interactive,
        "dhara_tool logging initialized"
    );

    Ok(LoggingRuntime {
        log_path,
        _guard: guard,
    })
}

fn io_error_from_set_global_default(
    error: impl std::error::Error + Send + Sync + 'static,
) -> std::io::Error {
    std::io::Error::other(error)
}

pub fn current_log_path() -> Option<PathBuf> {
    LOGGING.get().map(|runtime| runtime.log_path.clone())
}

/// Returns the path for the active log file, or allocates the next session path when logging
/// has not been initialized yet.
pub fn log_file_path(logs_dir: &Path) -> PathBuf {
    current_log_path().unwrap_or_else(|| {
        allocate_log_path(logs_dir).unwrap_or_else(|_| {
            let today = Local::now().date_naive();
            logs_dir.join(log_file_name_for(today, 0))
        })
    })
}

fn allocate_log_path(logs_dir: &Path) -> std::io::Result<PathBuf> {
    fs::create_dir_all(logs_dir)?;
    let today = Local::now().date_naive();
    let session = next_log_session(logs_dir, today)?;
    Ok(logs_dir.join(log_file_name_for(today, session)))
}

fn next_log_session(logs_dir: &Path, date: NaiveDate) -> std::io::Result<u32> {
    let mut highest: Option<u32> = None;

    if logs_dir.is_dir() {
        for entry in fs::read_dir(logs_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }

            let name = entry.file_name();
            let Some(session) = parse_log_session(name.to_string_lossy().as_ref(), date) else {
                continue;
            };

            highest = Some(highest.map_or(session, |current| current.max(session)));
        }
    }

    Ok(match highest {
        None => 0,
        Some(session) => session + 1,
    })
}

fn parse_log_session(file_name: &str, date: NaiveDate) -> Option<u32> {
    let date_prefix = date.format("%Y-%m-%d").to_string();
    if file_name == log_file_name_for(date, 0) {
        return Some(0);
    }

    let prefix = format!("{date_prefix}_{LOG_FILE_STEM}_");
    let suffix = ".log";
    if !file_name.starts_with(&prefix) || !file_name.ends_with(suffix) {
        return None;
    }

    let session_text = &file_name[prefix.len()..file_name.len() - suffix.len()];
    session_text.parse().ok()
}

fn log_file_name_for(date: NaiveDate, session: u32) -> String {
    let date_prefix = date.format("%Y-%m-%d");
    match session {
        0 => format!("{date_prefix}_{LOG_FILE_STEM}.log"),
        session => format!("{date_prefix}_{LOG_FILE_STEM}_{session}.log"),
    }
}

fn log_session_from_path(log_path: &Path) -> u32 {
    log_path
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| {
            let date = Local::now().date_naive();
            parse_log_session(name, date)
        })
        .unwrap_or(0)
}

pub fn log_command_begin(command_id: &str, command_line: &str, context: &ToolContext) {
    reset_build_progress_logging();
    let output_dir = resolve_output_dir(&context.repo_root, context.output_dir.as_deref());
    let logs_dir = resolve_logs_dir(
        &context.repo_root,
        context.output_dir.as_deref(),
        context.logs_dir.as_deref(),
    );

    info!(
        target: "dhara_tool::audit",
        command_id,
        command = command_line,
        repo_root = %context.repo_root.display(),
        output_dir = %output_dir.display(),
        logs_dir = %logs_dir.display(),
        package_dir = context
            .package_dir
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "default".to_owned()),
        silent = context.silent,
        verbose = context.verbose,
        "command started"
    );
}

pub fn log_command_end(command_id: &str, result: &CommandResult) {
    if let Some(message) = &result.message {
        info!(
            target: "dhara_tool::audit",
            command_id,
            message = message.as_str(),
            "command message"
        );
    }

    if let Some(report) = &result.report {
        log_structured_report(command_id, report.title.as_str(), &report.fields);
    }

    let outcome = if result.exit_code == 0 {
        "success"
    } else {
        "failed"
    };

    info!(
        target: "dhara_tool::audit",
        command_id,
        exit_code = result.exit_code,
        outcome,
        "command finished"
    );
}

pub fn log_task_step(task: &str, outcome: &str, detail: Option<&str>) {
    match outcome {
        "failed" | "error" => error!(
            target: "dhara_tool::audit",
            task,
            outcome,
            detail = detail.unwrap_or(""),
            "task step"
        ),
        "skipped" | "warning" | "bypassed" => warn!(
            target: "dhara_tool::audit",
            task,
            outcome,
            detail = detail.unwrap_or(""),
            "task step"
        ),
        _ => info!(
            target: "dhara_tool::audit",
            task,
            outcome,
            detail = detail.unwrap_or(""),
            "task step"
        ),
    }
}

pub fn log_build_progress(update: &TridBuildProgress) {
    let log_at_info = build_progress_should_log_at_info(update);
    if log_at_info {
        info!(target: "dhara_tool::ops::trid", stage = ?update.stage, message = update.message.as_str(), current = update.current, total = ?update.total, current_item = update.current_item.as_deref().unwrap_or(""), parsed = update.stats.parsed_count, accepted = update.stats.accepted_count, mime_corrected = update.stats.mime_corrected, mime_rejected = update.stats.mime_rejected, extension_rejected = update.stats.extension_rejected, signature_rejected = update.stats.signature_rejected, final_trimmed = update.stats.final_trimmed, "build progress");
    } else {
        debug!(target: "dhara_tool::ops::trid", stage = ?update.stage, message = update.message.as_str(), current = update.current, total = ?update.total, current_item = update.current_item.as_deref().unwrap_or(""), parsed = update.stats.parsed_count, accepted = update.stats.accepted_count, mime_corrected = update.stats.mime_corrected, mime_rejected = update.stats.mime_rejected, extension_rejected = update.stats.extension_rejected, signature_rejected = update.stats.signature_rejected, final_trimmed = update.stats.final_trimmed, "build progress");
    }
}

thread_local! {
    static LAST_BUILD_STAGE: Cell<Option<TridBuildStage>> = const { Cell::new(None) };
    static LAST_BUILD_MILESTONE: Cell<usize> = const { Cell::new(0) };
}

pub fn reset_build_progress_logging() {
    LAST_BUILD_STAGE.with(|stage| stage.set(None));
    LAST_BUILD_MILESTONE.with(|milestone| milestone.set(0));
}

fn build_progress_should_log_at_info(update: &TridBuildProgress) -> bool {
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

    if matches!(
        update.stage,
        TridBuildStage::ParseDefinitions | TridBuildStage::ReduceDefinitions
    ) {
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

pub fn log_transform_statistics(report: &TridTransformReport) {
    info!(
        target: "dhara_tool::audit",
        total_parsed = report.total_parsed,
        mime_corrected = report.mime_corrected,
        mime_rejected = report.mime_rejected,
        extension_rejected = report.extension_rejected,
        signature_rejected = report.signature_rejected,
        final_trimmed = report.final_trimmed,
        final_kept = report.final_kept,
        "TrID transformation statistics"
    );
}

fn log_structured_report(
    scope: &str,
    title: &str,
    fields: &[crate::command::ReportField],
) {
    info!(
        target: "dhara_tool::audit",
        scope,
        title,
        field_count = fields.len(),
        "structured report"
    );

    for field in fields {
        info!(
            target: "dhara_tool::audit",
            scope,
            label = field.label.as_str(),
            value = field.value.as_str(),
            "report field"
        );
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use tempfile::tempdir;

    use super::{log_file_name_for, next_log_session, parse_log_session};

    #[test]
    fn log_file_name_uses_dhara_tool_stem_for_session_zero() {
        let date = NaiveDate::from_ymd_opt(2026, 4, 10).unwrap();
        assert_eq!(log_file_name_for(date, 0), "2026-04-10_dhara_tool.log");
    }

    #[test]
    fn log_file_name_adds_session_suffix_for_positive_sessions() {
        let date = NaiveDate::from_ymd_opt(2026, 4, 10).unwrap();
        assert_eq!(log_file_name_for(date, 1), "2026-04-10_dhara_tool_1.log");
        assert_eq!(log_file_name_for(date, 3), "2026-04-10_dhara_tool_3.log");
    }

    #[test]
    fn parse_log_session_recognizes_session_zero_and_numbered_sessions() {
        let date = NaiveDate::from_ymd_opt(2026, 4, 10).unwrap();
        assert_eq!(parse_log_session("2026-04-10_dhara_tool.log", date), Some(0));
        assert_eq!(parse_log_session("2026-04-10_dhara_tool_1.log", date), Some(1));
        assert_eq!(parse_log_session("2026-04-09_dhara_tool.log", date), None);
    }

    #[test]
    fn next_log_session_starts_at_zero_and_increments() {
        let temp = tempdir().unwrap();
        let logs_dir = temp.path();
        let date = NaiveDate::from_ymd_opt(2026, 4, 10).unwrap();

        assert_eq!(next_log_session(logs_dir, date).unwrap(), 0);

        std::fs::write(logs_dir.join(log_file_name_for(date, 0)), "session 0").unwrap();
        assert_eq!(next_log_session(logs_dir, date).unwrap(), 1);

        std::fs::write(logs_dir.join(log_file_name_for(date, 1)), "session 1").unwrap();
        assert_eq!(next_log_session(logs_dir, date).unwrap(), 2);
    }
}
