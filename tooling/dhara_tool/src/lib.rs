pub mod app;
pub mod command;
pub mod commands;
pub mod filedefs;
pub mod logging;
pub mod nuget;
pub mod output;
pub mod paths;
pub mod process;
pub mod registry;
pub mod release;
pub mod repo_config;
pub mod subprocess;
pub mod tui;
pub mod verify;
pub mod workspace;

pub use filedefs::{
    BuilderError, DefsCommand, DefsPaths, LoadedPackage, PackageSummary, SyncEmbeddedOutcome,
    SyncEmbeddedStatus, TridBuildProgress, TridBuildStage, TridTransformReport,
    execute as execute_defs, inspect_package, load_bundled_package, load_package,
    normalize_package, packages_match, print_defs_help, sync_embedded_package, write_package,
};
pub use logging::{
    LoggingOptions, LoggingRuntime, current_log_path, ensure_logging, format_command_args,
    init_logging, is_long_running_module, log_build_progress, log_file_path, log_module_begin,
    log_module_begin_debug, log_module_compact_finish, log_module_end, log_module_failed,
    log_module_step_debug, log_module_step_error, log_module_step_warn, log_session_begin,
    log_session_end, log_transform_statistics, summarize_command_result,
};
pub use nuget::{PackageOptions, pack as pack_package, publish as publish_package};
pub use output::{
    OutputCaptureGuard, OutputEvent, OutputStream, cancel_active_subprocess, emit_stderr_line,
    emit_stdout_line,
};
pub use registry::DharaStorageCapability;
pub use release::{ReleaseOptions, run as run_release};
pub use repo_config::{
    CONFIG_PATH, CiConfig, DharaRepoConfig, ENV_EXAMPLE_PATH, ENV_LOCAL_PATH, NuGetConfig,
    PublishConfig, ROOT_CARGO_TOML_PATH, ShowOutput, TargetsConfig, VersionConfig, VersionPart,
    bump_version, init_env, load_config, load_env, parse_env_content, set_version, show, sync,
    sync_cargo_toml, sync_csproj, validate_config, verify_release,
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

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
