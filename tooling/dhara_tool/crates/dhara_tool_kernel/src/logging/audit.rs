use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

use chrono::{Local, NaiveDate};
use tracing::{Level, debug, error, info, warn};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::Layer;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::context::{CommandResult, RunMode, ToolContext};
use crate::output::{emit_stderr_line, emit_warn_line};
use crate::paths::{resolve_defs_output_dir, resolve_logs_dir, resolve_output_dir};

use crate::filedefs::{TridBuildProgress, TridTransformReport};

static LOGGING: OnceLock<LoggingRuntime> = OnceLock::new();

const LOG_FILE_STEM: &str = "dhara_tool";
pub(crate) const AUDIT_TARGET: &str = "dhara_tool::audit";

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
            logs_dir: resolve_logs_dir(&context.tool_root, context.logs_dir.as_deref()),
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
    if LOGGING.set(runtime).is_err() {
        return LOGGING
            .get()
            .ok_or_else(|| std::io::Error::other("logging runtime was not initialized"));
    }
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

    let (console_max_level, file_max_level) =
        resolve_log_levels(options.min, options.trace, options.run_mode);

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

    let init_result = if options.run_mode == RunMode::Interactive {
        tracing_subscriber::registry()
            .with(console_layer)
            .with(file_layer)
            .with(InteractiveDiagnosticLayer)
            .try_init()
    } else {
        tracing_subscriber::registry()
            .with(console_layer)
            .with(file_layer)
            .try_init()
    };

    if let Err(error) = init_result
        && !tracing::dispatcher::has_been_set()
    {
        return Err(io_error_from_set_global_default(error));
    }

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

/// Console TRACE in direct mode; OFF in interactive (TUI diagnostic layer).
/// File: INFO default; WARN with `--min`; DEBUG with `--trace`.
fn resolve_log_levels(min: bool, trace: bool, run_mode: RunMode) -> (LevelFilter, LevelFilter) {
    let console = if run_mode == RunMode::Interactive {
        LevelFilter::OFF
    } else {
        LevelFilter::TRACE
    };
    let file = if trace {
        LevelFilter::DEBUG
    } else if min {
        LevelFilter::WARN
    } else {
        LevelFilter::INFO
    };
    (console, file)
}

struct InteractiveDiagnosticLayer;

struct DiagnosticMessage {
    text: String,
}

impl Default for DiagnosticMessage {
    fn default() -> Self {
        Self {
            text: String::new(),
        }
    }
}

impl tracing::field::Visit for DiagnosticMessage {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.text = value.to_owned();
        }
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" && self.text.is_empty() {
            self.text = format!("{value:?}");
            if self.text.len() >= 2
                && self.text.starts_with('"')
                && self.text.ends_with('"')
            {
                self.text = self.text[1..self.text.len() - 1].to_owned();
            }
        }
    }
}

impl<S> Layer<S> for InteractiveDiagnosticLayer
where
    S: tracing::Subscriber,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let level = event.metadata().level();
        if !matches!(level, &Level::WARN | &Level::ERROR) {
            return;
        }

        let mut message = DiagnosticMessage::default();
        event.record(&mut message);
        if message.text.is_empty() {
            return;
        }

        match level {
            &Level::WARN => emit_warn_line(message.text),
            &Level::ERROR => emit_stderr_line(message.text),
            _ => {}
        }
    }
}

pub fn log_session_begin(log_path: &Path, options: &LoggingOptions) {
    let version = env!("CARGO_PKG_VERSION");
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
        &options.context.tool_root,
        options.context.output_dir.as_deref(),
    );
    let defs_output_dir = resolve_defs_output_dir(
        &options.context.repo_root,
        options.context.output_dir.as_deref(),
    );
    debug!(
        target: AUDIT_TARGET,
        repo_root = %options.context.repo_root.display(),
        tool_root = %options.context.tool_root.display(),
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
    write_session_record(exit_code, module_id, error);
}

/// File-only separator between process invocations in the daily log.
pub fn write_session_record(exit_code: i32, module_id: Option<&str>, error: Option<&str>) {
    let Some(path) = current_log_path() else {
        return;
    };
    let record = format_session_record(exit_code, module_id, error);
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        use std::io::Write;
        let _ = file.write_all(record.as_bytes());
    }
}

fn format_session_record(exit_code: i32, module_id: Option<&str>, error: Option<&str>) -> String {
    let module = module_id.unwrap_or("—");
    let error_suffix = error
        .map(|value| format!("  error={value}"))
        .unwrap_or_default();
    format!(
        "================================================================================\n\
         session end  exit={exit_code}  module={module}{error_suffix}\n\
         ================================================================================\n"
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

fn summarize_defs_report(report: &crate::context::StructuredReport) -> String {
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
    debug!(
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
            logs_dir.join(log_file_name_for(today))
        })
    })
}

fn allocate_log_path(logs_dir: &Path) -> std::io::Result<PathBuf> {
    fs::create_dir_all(logs_dir)?;
    let today = Local::now().date_naive();
    Ok(logs_dir.join(log_file_name_for(today)))
}

fn log_file_name_for(date: NaiveDate) -> String {
    format!("{}_{LOG_FILE_STEM}.log", date.format("%Y-%m-%d"))
}

pub(crate) fn format_duration(duration: Duration) -> String {
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

    use super::{format_duration, format_session_record, log_file_name_for, resolve_log_levels};
    use crate::context::RunMode;
    use std::io::Write;
    use std::time::Duration;

    #[test]
    fn log_file_name_uses_daily_stem() {
        let date = NaiveDate::from_ymd_opt(2026, 4, 10).unwrap();
        assert_eq!(log_file_name_for(date), "2026-04-10_dhara_tool.log");
    }

    #[test]
    fn daily_log_appends_to_same_file() {
        let temp = tempdir().unwrap();
        let logs_dir = temp.path();
        let date = NaiveDate::from_ymd_opt(2026, 4, 10).unwrap();
        let path = logs_dir.join(log_file_name_for(date));
        std::fs::write(&path, "first\n").unwrap();
        std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap()
            .write_all(b"second\n")
            .unwrap();
        let content = std::fs::read_to_string(path).unwrap();
        assert!(content.contains("first"));
        assert!(content.contains("second"));
    }

    #[test]
    fn session_record_format() {
        let record = format_session_record(0, Some("defs.build-trid-xml"), None);
        assert!(record.contains("session end"));
        assert!(record.contains("exit=0"));
        assert!(record.contains("defs.build-trid-xml"));
    }

    #[test]
    fn format_duration_scales_units() {
        assert_eq!(format_duration(Duration::from_millis(450)), "450ms");
        assert_eq!(format_duration(Duration::from_secs(65)), "1m5s");
    }

    #[test]
    fn direct_mode_uses_trace_on_console() {
        let (console, file) = resolve_log_levels(false, false, RunMode::Direct);
        assert_eq!(console, LevelFilter::TRACE);
        assert_eq!(file, LevelFilter::INFO);
    }

    #[test]
    fn interactive_mode_suppresses_console_logging() {
        let (console, file) = resolve_log_levels(false, false, RunMode::Interactive);
        assert_eq!(console, LevelFilter::OFF);
        assert_eq!(file, LevelFilter::INFO);
    }

    #[test]
    fn min_lowers_file_log_to_warn_only() {
        let (console, file) = resolve_log_levels(true, false, RunMode::Direct);
        assert_eq!(console, LevelFilter::TRACE);
        assert_eq!(file, LevelFilter::WARN);
    }

    #[test]
    fn trace_raises_file_log_to_debug() {
        let (console, file) = resolve_log_levels(false, true, RunMode::Direct);
        assert_eq!(console, LevelFilter::TRACE);
        assert_eq!(file, LevelFilter::DEBUG);
    }
}
