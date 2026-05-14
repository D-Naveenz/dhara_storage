pub mod builder;
pub mod capability;
pub mod config;
pub mod defs;
pub mod interface;
pub mod logging;
pub mod output;
pub mod package_flow;
pub mod release_flow;
pub mod runner;
pub mod support;
pub mod verify;

pub use builder::{
    BuilderError, FiledefsPackageMetadata, LoadedPackage, PackageSummary, SyncEmbeddedOutcome,
    SyncEmbeddedStatus, TridBuildProgress, TridBuildStage, TridTransformReport, inspect_package,
    load_bundled_package, load_package, normalize_package, packages_match, sync_embedded_package,
    write_package, write_package_with_purpose,
};
pub use capability::DharaStorageCapability;
pub use config::{
    CONFIG_PATH, CiConfig, DharaRepoConfig, ENV_EXAMPLE_PATH, ENV_LOCAL_PATH, NuGetConfig,
    PublishConfig, ROOT_CARGO_TOML_PATH, ShowOutput, TargetsConfig, VersionConfig, VersionPart,
    bump_version, init_env, load_config, load_env, parse_env_content, set_version, show, sync,
    sync_cargo_toml, sync_csproj, validate_config, verify_release,
};
pub use defs::{
    DefsCommand, DefsPaths, default_embedded_sync_paths, execute as execute_defs, print_defs_help,
};
pub use interface::{
    CommandHandler, CommandResult, CommandSpec, ReportField, SectionSpec, StructuredReport,
    ToolContext,
};
pub use logging::{LoggingOptions, LoggingRuntime, init_logging, log_file_path};
pub use output::{
    OutputCaptureGuard, OutputEvent, OutputStream, cancel_active_subprocess, emit_stderr_line,
    emit_stdout_line,
};
pub use package_flow::{PackageOptions, pack as pack_package, publish as publish_package};
pub use release_flow::{ReleaseOptions, run as run_release};
pub use runner::{
    BuilderAction, CommandReport, ReportField as BuilderReportField, ReportStatus, execute_action,
    print_report,
};
pub use support::{
    inspect_package_entries, run_command, run_command_expect_failure, run_command_with_env,
    run_command_with_env_redacted, write_nuget_config,
};
