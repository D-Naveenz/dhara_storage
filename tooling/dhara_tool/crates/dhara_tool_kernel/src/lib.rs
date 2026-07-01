pub mod activation;
pub mod context;
pub mod filedefs;
pub mod logging;
pub mod msvc;
pub mod output;
pub mod paths;
pub mod repo_config;
pub mod subprocess;
pub mod workers;
pub mod workspace;

pub use context::{
    CommandResult, ReportField, RunMode, StructuredReport, ToolContext,
};
pub use filedefs::{
    BuilderError, DefsCommand, DefsPaths, LoadedPackage, PackageSummary, ReduceTraceDetail,
    SyncEmbeddedOutcome, SyncEmbeddedStatus, TridBuildProgress, TridBuildStage,
    TridTransformReport, execute as execute_defs, inspect_package, load_bundled_package,
    load_package, normalize_package, packages_match, print_defs_help, sync_embedded_package,
    write_package,
};
pub use logging::{
    LoggingOptions, LoggingRuntime, current_log_path, ensure_logging, format_command_args,
    init_logging, is_long_running_module, log_build_progress, log_file_path, log_module_begin,
    log_module_begin_debug, log_module_compact_finish, log_module_end, log_module_failed,
    log_module_step_debug, log_module_step_error, log_module_step_warn, log_session_begin,
    log_session_end, log_transform_statistics, summarize_command_result,
};
pub use output::{
    OutputCaptureGuard, OutputEvent, OutputStream, cancel_active_subprocess, emit_stderr_line,
    emit_stdout_line,
};
pub use repo_config::{
    CONFIG_PATH, CiConfig, ConfigDriftItem, ConfigDriftKind, DharaRepoConfig, ENV_EXAMPLE_PATH,
    ENV_LOCAL_PATH, NuGetConfig, PublishConfig, ROOT_CARGO_TOML_PATH, ShowOutput,
    TOOL_CARGO_TOML_PATH, TargetsConfig, ToolConfig, VersionConfig, VersionPart,
    apply_config_drift, bump_version, detect_config_drift, init_env, load_config, load_env,
    parse_env_content, read_tool_crate_version, set_version, show, sync_cargo_toml, sync_csproj,
    sync_tool_cargo_toml, validate_config, verify_release,
};
pub use subprocess::{
    inspect_package_entries, run_command, run_command_expect_failure, run_command_with_env,
    run_command_with_env_redacted, write_nuget_config,
};
pub use workspace::{
    DefsPackageStatus, WorkspaceSnapshot, ensure_workspace_state, next_package_revision,
    next_package_revision_for_build, record_package_written, refresh_workspace_state,
    workspace_snapshot,
};

pub fn defs_package_version() -> &'static str {
    dhara_storage_dal::PACKAGE_VERSION
}
