use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use chrono::{Local, NaiveDate};
use tracing::{debug, error, info, warn};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::Layer;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::command::{CommandResult, RunMode, ToolContext};
use crate::paths::{resolve_defs_output_dir, resolve_logs_dir, resolve_output_dir};

use crate::filedefs::{TridBuildProgress, TridTransformReport};

static LOGGING: OnceLock<LoggingRuntime> = OnceLock::new();

const LOG_FILE_STEM: &str = "dhara_tool";
const AUDIT_TARGET: &str = "dhara_tool::audit";

#[derive(Debug, Clone)]
pub struct LoggingOptions {
    pub run_mode: RunMode,
    pub min: bool,
    pub trace: bool,
    pub logs_dir: PathBuf,
    pub context: ToolContext,
}

pub struct LoggingRuntime {
    pub log_path: PathBuf,
    _guard: WorkerGuard,
}

impl LoggingOptions {
    pub fn from_context(context: &ToolContext) -> Self {
        Self {
            run_mode: context.run_mode,
            min: context.min,
            trace: context.trace,
            logs_dir: resolve_logs_dir(&context.repo_root, context.logs_dir.as_deref()),
            context: context.clone(),
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

    let (console_max_level, file_max_level) = resolve_log_levels(options.min, options.trace);

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

    log_session_begin(&log_path, &options);
    crate::logging::progress::init_progress_settings(&options.context);

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

/// Console stays at INFO (level 3). File level depends on `--min` / `--trace`.
fn resolve_log_levels(min: bool, trace: bool) -> (LevelFilter, LevelFilter) {
    let console = LevelFilter::INFO;
    let file = if trace {
        LevelFilter::DEBUG
    } else if min {
        LevelFilter::WARN
    } else {
        LevelFilter::INFO
    };
    (console, file)
}

pub fn log_session_begin(log_path: &Path, options: &LoggingOptions) {
    let version = crate::version();
    let mode = options.run_mode.as_str();
    let workers = options.context.workers;

    info!(
        target: AUDIT_TARGET,
        "dhara_tool {version} started — mode={mode}, workers={workers}"
    );

    let min = if options.min { "yes" } else { "no" };
    let trace = if options.trace { "yes" } else { "no" };
    debug!(
        target: AUDIT_TARGET,
        "flags min={min}, trace={trace}, log={}",
        log_path.display()
    );

    let output_dir = resolve_output_dir(
        &options.context.repo_root,
        options.context.output_dir.as_deref(),
    );
    let defs_output_dir = resolve_defs_output_dir(
        &options.context.repo_root,
        options.context.output_dir.as_deref(),
    );
    debug!(
        target: AUDIT_TARGET,
        repo_root = %options.context.repo_root.display(),
        output_dir = %output_dir.display(),
        defs_output_dir = %defs_output_dir.display(),
        logs_dir = %options.logs_dir.display(),
        package_dir = options
            .context
            .package_dir
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "default".to_owned()),
        "session paths resolved"
    );

    let workspace = crate::workspace::workspace_snapshot(&options.context);
    debug!(
        target: AUDIT_TARGET,
        defs_path = %workspace.defs_path.display(),
        defs_status = workspace.status_label(),
        package_version = workspace.package_version.as_deref().unwrap_or("—"),
        package_revision = workspace.package_revision.map(|value| value.to_string()).unwrap_or_else(|| "—".to_owned()),
        definitions_release = workspace.definitions_release.as_deref().unwrap_or("—"),
        definition_count = workspace.definition_count.map(|value| value.to_string()).unwrap_or_else(|| "—".to_owned()),
        "workspace definitions package snapshot"
    );
}

pub fn log_session_end(exit_code: i32, module_id: Option<&str>, error: Option<&str>) {
    let timestamp = Local::now().format("%Y-%m-%dT%H:%M:%S%.3fZ");
    match (exit_code, module_id, error) {
        (0, Some(module), None) => info!(
            target: AUDIT_TARGET,
            "dhara_tool exiting 0 at {timestamp} — completed {module}"
        ),
        (code, Some(module), Some(err)) => info!(
            target: AUDIT_TARGET,
            "dhara_tool exiting {code} at {timestamp} — {module} failed: {err}"
        ),
        (code, _, Some(err)) => info!(
            target: AUDIT_TARGET,
            "dhara_tool exiting {code} at {timestamp} — {err}"
        ),
        (code, Some(module), None) => info!(
            target: AUDIT_TARGET,
            "dhara_tool exiting {code} at {timestamp} — {module}"
        ),
        (code, None, None) => info!(
            target: AUDIT_TARGET,
            "dhara_tool exiting {code} at {timestamp}"
        ),
    }
}

pub fn is_long_running_module(command_id: &str) -> bool {
    matches!(
        command_id,
        "defs.build-trid-xml"
            | "defs.inspect-trid-xml"
            | "defs.sync-embedded"
            | "verify.ci"
            | "verify.package"
            | "package.pack"
            | "package.publish"
            | "release.run"
    )
}

pub fn format_command_args(args: &[String]) -> String {
    if args.is_empty() {
        "defaults".to_owned()
    } else {
        args.join(" ")
    }
}

pub fn summarize_command_result(command_id: &str, result: &CommandResult) -> String {
    if let Some(message) = &result.message {
        return message.clone();
    }

    if let Some(report) = &result.report {
        if command_id.starts_with("defs.") {
            return summarize_defs_report(report);
        }
        let highlights: Vec<String> = report
            .fields
            .iter()
            .take(4)
            .map(|field| format!("{}={}", field.label, field.value))
            .collect();
        if highlights.is_empty() {
            return report.title.clone();
        }
        return format!("{} — {}", report.title, highlights.join(", "));
    }

    if result.exit_code == 0 {
        "completed".to_owned()
    } else {
        format!("exit code {}", result.exit_code)
    }
}

fn summarize_defs_report(report: &crate::command::StructuredReport) -> String {
    let mut parts = Vec::new();
    for field in &report.fields {
        if matches!(
            field.label.as_str(),
            "Definitions" | "Total Parsed" | "Final Kept" | "Package Version" | "Result" | "Output"
        ) {
            parts.push(format!("{}={}", field.label, field.value));
        }
    }
    if parts.is_empty() {
        report.title.clone()
    } else {
        parts.join(", ")
    }
}

pub fn log_module_begin(module_id: &str, config_summary: &str) {
    crate::logging::progress::reset_build_progress_logging();
    info!(
        target: AUDIT_TARGET,
        "{module_id} started — {config_summary}"
    );
}

pub fn log_module_begin_debug(module_id: &str, config_summary: &str) {
    crate::logging::progress::reset_build_progress_logging();
    debug!(
        target: AUDIT_TARGET,
        "{module_id} — {config_summary}"
    );
}

pub fn log_module_end(module_id: &str, exit_code: i32, summary: &str, started: Instant) {
    let duration = format_duration(started.elapsed());
    let timestamp = Local::now().format("%Y-%m-%dT%H:%M:%S%.3fZ");
    if exit_code == 0 {
        info!(
            target: AUDIT_TARGET,
            "{module_id} finished in {duration} at {timestamp} — {summary}"
        );
    } else {
        warn!(
            target: AUDIT_TARGET,
            "{module_id} finished in {duration} at {timestamp} with exit {exit_code} — {summary}"
        );
    }
}

pub fn log_module_compact_finish(module_id: &str, exit_code: i32, summary: &str, started: Instant) {
    let duration = format_duration(started.elapsed());
    let timestamp = Local::now().format("%Y-%m-%dT%H:%M:%S%.3fZ");
    if exit_code == 0 {
        info!(
            target: AUDIT_TARGET,
            "{module_id} finished in {duration} at {timestamp} — {summary}"
        );
    } else {
        warn!(
            target: AUDIT_TARGET,
            "{module_id} finished in {duration} at {timestamp} with exit {exit_code} — {summary}"
        );
    }
}

pub fn log_module_failed(module_id: &str, error: &str, started: Instant) {
    let duration = format_duration(started.elapsed());
    let timestamp = Local::now().format("%Y-%m-%dT%H:%M:%S%.3fZ");
    error!(
        target: AUDIT_TARGET,
        "{module_id} failed in {duration} at {timestamp} — {error}"
    );
}

pub fn log_module_step_debug(message: &str) {
    debug!(target: AUDIT_TARGET, "{message}");
}

pub fn log_module_step_warn(message: &str) {
    warn!(target: AUDIT_TARGET, "{message}");
}

pub fn log_module_step_error(message: &str) {
    error!(target: AUDIT_TARGET, "{message}");
}

pub fn log_transform_statistics(report: &TridTransformReport) {
    info!(
        target: AUDIT_TARGET,
        "TrID transform — parsed={}, kept={}, mime_corrected={}, mime_rejected={}, ext_rejected={}, sig_rejected={}, trimmed={}",
        report.total_parsed,
        report.final_kept,
        report.mime_corrected,
        report.mime_rejected,
        report.extension_rejected,
        report.signature_rejected,
        report.final_trimmed,
    );
}

pub fn current_log_path() -> Option<PathBuf> {
    LOGGING.get().map(|runtime| runtime.log_path.clone())
}

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

fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    if secs >= 3600 {
        format!("{}h{}m", secs / 3600, (secs % 3600) / 60)
    } else if secs >= 60 {
        format!("{}m{}s", secs / 60, secs % 60)
    } else if duration.as_millis() >= 1000 {
        format!("{:.1}s", duration.as_secs_f64())
    } else {
        format!("{}ms", duration.as_millis())
    }
}

pub fn log_build_progress(update: &TridBuildProgress) {
    crate::logging::progress::dispatch_trid_progress(update);
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use tempfile::tempdir;
    use tracing_subscriber::filter::LevelFilter;

    use super::{
        format_duration, log_file_name_for, next_log_session, parse_log_session, resolve_log_levels,
    };
    use std::time::Duration;

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
        assert_eq!(
            parse_log_session("2026-04-10_dhara_tool.log", date),
            Some(0)
        );
        assert_eq!(
            parse_log_session("2026-04-10_dhara_tool_1.log", date),
            Some(1)
        );
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

    #[test]
    fn format_duration_scales_units() {
        assert_eq!(format_duration(Duration::from_millis(450)), "450ms");
        assert_eq!(format_duration(Duration::from_secs(65)), "1m5s");
    }

    #[test]
    fn default_log_levels_use_info_on_console_and_file() {
        let (console, file) = resolve_log_levels(false, false);
        assert_eq!(console, LevelFilter::INFO);
        assert_eq!(file, LevelFilter::INFO);
    }

    #[test]
    fn min_lowers_file_log_to_warn_only() {
        let (console, file) = resolve_log_levels(true, false);
        assert_eq!(console, LevelFilter::INFO);
        assert_eq!(file, LevelFilter::WARN);
    }

    #[test]
    fn trace_raises_file_log_to_debug() {
        let (console, file) = resolve_log_levels(false, true);
        assert_eq!(console, LevelFilter::INFO);
        assert_eq!(file, LevelFilter::DEBUG);
    }
}
