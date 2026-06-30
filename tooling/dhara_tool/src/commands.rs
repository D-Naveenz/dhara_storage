use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::{ArgAction, Parser, ValueEnum};

use crate::command::{CommandResult, ToolContext};
use crate::paths::resolve_path_against_repo;
use crate::filedefs::{DefsCommand, execute as execute_defs, print_defs_help};
use crate::nuget::{
    PackageOptions, pack as pack_package, publish as publish_package, stage_native_for_host,
};
use crate::release::run as run_release;
use crate::repo_config::{
    VersionPart, bump_version, init_env, load_config, set_version, show, sync,
};

#[derive(Debug, Clone, Copy, ValueEnum)]
enum PartArg {
    Major,
    Minor,
    Patch,
}

impl From<PartArg> for VersionPart {
    fn from(value: PartArg) -> Self {
        match value {
            PartArg::Major => VersionPart::Major,
            PartArg::Minor => VersionPart::Minor,
            PartArg::Patch => VersionPart::Patch,
        }
    }
}

#[derive(Debug, Parser)]
struct NoArgs {}

#[derive(Debug, Parser)]
struct VersionSetArgs {
    version: String,
}

#[derive(Debug, Parser)]
struct VersionBumpArgs {
    #[arg(long)]
    part: PartArg,
}

#[derive(Debug, Parser)]
struct StageNativeArgs {
    #[arg(long, default_value = "Release")]
    configuration: String,
    #[arg(long, action = ArgAction::SetTrue)]
    msvc_env: bool,
}

#[derive(Debug, Parser)]
struct NativeMergeArgs {
    #[arg(long)]
    output: PathBuf,
    #[arg(long)]
    input: Vec<PathBuf>,
}

#[derive(Debug, Parser)]
struct QualityFmtArgs {
    #[arg(long, action = ArgAction::SetTrue)]
    check: bool,
}

#[derive(Debug, Parser)]
struct QualityRunArgs {
    #[arg(long, action = ArgAction::SetTrue)]
    skip_docs: bool,
    #[arg(long, action = ArgAction::SetTrue)]
    skip_dotnet: bool,
}

#[derive(Debug, Parser)]
struct PackageArgs {
    #[arg(long, default_value = "Release")]
    configuration: String,
    #[arg(long)]
    version: Option<String>,
    #[arg(long)]
    native_stage: Option<PathBuf>,
}

#[derive(Debug, Parser)]
struct PublishArgs {
    #[arg(long, default_value = "Release")]
    configuration: String,
    #[arg(long)]
    version: Option<String>,
    #[arg(long)]
    source: Option<String>,
    #[arg(long)]
    api_key_env: Option<String>,
    #[arg(long, action = ArgAction::SetTrue)]
    dry_run: bool,
    #[arg(long, action = ArgAction::SetTrue)]
    execute: bool,
}

#[derive(Debug, Parser)]
pub(crate) struct ReleaseRunArgs {
    #[arg(long, default_value = "Release")]
    configuration: String,
    #[arg(long)]
    source: Option<String>,
    #[arg(long)]
    api_key_env: Option<String>,
    #[arg(long, action = ArgAction::SetTrue)]
    dry_run: bool,
    #[arg(long, action = ArgAction::SetTrue)]
    skip_cargo: bool,
    #[arg(long, action = ArgAction::SetTrue)]
    skip_nuget: bool,
    #[arg(long)]
    native_stage: Option<PathBuf>,
    #[arg(long)]
    prepacked_nuget: Option<PathBuf>,
    #[arg(long, action = ArgAction::SetTrue)]
    verify_package: bool,
}

#[derive(Debug, Parser)]
struct OutputArg {
    #[arg(short, long)]
    output: Option<PathBuf>,
}

#[derive(Debug, Parser)]
struct InputArg {
    #[arg(short, long)]
    input: Option<PathBuf>,
}

#[derive(Debug, Parser)]
struct InputOutputArgs {
    #[arg(short, long)]
    input: Option<PathBuf>,
    #[arg(short, long)]
    output: Option<PathBuf>,
}

#[derive(Debug, Parser)]
struct VerifyDefsArgs {
    #[arg(long)]
    left: PathBuf,
    #[arg(long)]
    right: PathBuf,
}

#[derive(Debug, Parser)]
struct SyncEmbeddedArgs {
    #[arg(short, long)]
    input: Option<PathBuf>,
    #[arg(short, long)]
    output: Option<PathBuf>,
    #[arg(long)]
    check: bool,
}

pub(crate) fn parse_args<T: Parser>(name: &str, args: &[String]) -> Result<Option<T>> {
    match T::try_parse_from(std::iter::once(name.to_owned()).chain(args.iter().cloned())) {
        Ok(parsed) => Ok(Some(parsed)),
        Err(error) => match error.kind() {
            clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion => {
                print!("{error}");
                Ok(None)
            }
            _ => bail!(error.to_string()),
        },
    }
}

pub(crate) fn current_config(context: &ToolContext) -> Result<crate::repo_config::DharaRepoConfig> {
    load_config(&context.repo_root)
}

fn package_options(args: PackageArgs, context: &ToolContext) -> PackageOptions {
    PackageOptions {
        configuration: args.configuration,
        version_override: args.version,
        source_override: None,
        api_key_env_override: None,
        output_dir: context.output_dir.clone(),
        execute_publish: false,
        native_stage_override: args.native_stage,
        prepacked_nuget_override: None,
    }
}

fn publish_options(args: PublishArgs, context: &ToolContext) -> Result<PackageOptions> {
    if args.dry_run && args.execute {
        bail!("--dry-run and --execute cannot be used together");
    }

    Ok(PackageOptions {
        configuration: args.configuration,
        version_override: args.version,
        source_override: args.source,
        api_key_env_override: args.api_key_env,
        output_dir: context.output_dir.clone(),
        execute_publish: args.execute && !args.dry_run,
        native_stage_override: None,
        prepacked_nuget_override: None,
    })
}

pub(crate) fn release_options(
    args: ReleaseRunArgs,
    context: &ToolContext,
) -> crate::release::ReleaseOptions {
    crate::release::ReleaseOptions {
        configuration: args.configuration,
        source_override: args.source,
        api_key_env_override: args.api_key_env,
        output_dir: context.output_dir.clone(),
        dry_run: args.dry_run,
        publish_cargo: !args.skip_cargo,
        publish_nuget: !args.skip_nuget,
        native_stage_override: args.native_stage,
        prepacked_nuget: args.prepacked_nuget,
        verify_package_on_dry_run: args.verify_package,
    }
}

pub(crate) fn config_show(context: &ToolContext, args: &[String]) -> Result<CommandResult> {
    if parse_args::<NoArgs>("config show", args)?.is_none() {
        return Ok(CommandResult::success());
    }
    Ok(CommandResult::with_message(show(&context.repo_root)?))
}

pub(crate) fn config_sync(context: &ToolContext, args: &[String]) -> Result<CommandResult> {
    if parse_args::<NoArgs>("config sync", args)?.is_none() {
        return Ok(CommandResult::success());
    }
    sync(&context.repo_root)?;
    Ok(CommandResult::with_message(
        "Synchronized repository configuration.",
    ))
}

pub(crate) fn config_env_init(context: &ToolContext, args: &[String]) -> Result<CommandResult> {
    if parse_args::<NoArgs>("config env init", args)?.is_none() {
        return Ok(CommandResult::success());
    }
    let created = init_env(&context.repo_root)?;
    Ok(CommandResult::with_message(if created {
        "Created .env.local from .env.example."
    } else {
        ".env.local already exists."
    }))
}

pub(crate) fn version_set(context: &ToolContext, args: &[String]) -> Result<CommandResult> {
    let Some(args) = parse_args::<VersionSetArgs>("version set", args)? else {
        return Ok(CommandResult::success());
    };
    set_version(&context.repo_root, &args.version)?;
    Ok(CommandResult::with_message(format!(
        "Updated version to {}.",
        args.version
    )))
}

pub(crate) fn version_bump(context: &ToolContext, args: &[String]) -> Result<CommandResult> {
    let Some(args) = parse_args::<VersionBumpArgs>("version bump", args)? else {
        return Ok(CommandResult::success());
    };
    let next = bump_version(&context.repo_root, args.part.into())?;
    Ok(CommandResult::with_message(next))
}

pub(crate) fn defs_pack(context: &ToolContext, args: &[String]) -> Result<CommandResult> {
    let Some(args) = parse_args::<OutputArg>("defs pack", args)? else {
        return Ok(CommandResult::success());
    };
    execute_defs(
        DefsCommand::Pack {
            output: args.output,
        },
        context,
    )
}

pub(crate) fn defs_build_trid_xml(context: &ToolContext, args: &[String]) -> Result<CommandResult> {
    let Some(args) = parse_args::<InputOutputArgs>("defs build-trid-xml", args)? else {
        return Ok(CommandResult::success());
    };
    execute_defs(
        DefsCommand::BuildTridXml {
            input: args.input,
            output: args.output,
        },
        context,
    )
}

pub(crate) fn defs_inspect(context: &ToolContext, args: &[String]) -> Result<CommandResult> {
    let Some(args) = parse_args::<InputArg>("defs inspect", args)? else {
        return Ok(CommandResult::success());
    };
    execute_defs(DefsCommand::Inspect { input: args.input }, context)
}

pub(crate) fn defs_inspect_trid_xml(
    context: &ToolContext,
    args: &[String],
) -> Result<CommandResult> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        return Ok(CommandResult::with_message(print_defs_help()));
    }
    let Some(args) = parse_args::<InputArg>("defs inspect-trid-xml", args)? else {
        return Ok(CommandResult::success());
    };
    execute_defs(DefsCommand::InspectTridXml { input: args.input }, context)
}

pub(crate) fn defs_normalize(context: &ToolContext, args: &[String]) -> Result<CommandResult> {
    let Some(args) = parse_args::<InputOutputArgs>("defs normalize", args)? else {
        return Ok(CommandResult::success());
    };
    execute_defs(
        DefsCommand::Normalize {
            input: args.input,
            output: args.output,
        },
        context,
    )
}

pub(crate) fn defs_verify(context: &ToolContext, args: &[String]) -> Result<CommandResult> {
    let Some(args) = parse_args::<VerifyDefsArgs>("defs verify", args)? else {
        return Ok(CommandResult::success());
    };
    execute_defs(
        DefsCommand::Verify {
            left: args.left,
            right: args.right,
        },
        context,
    )
}

pub(crate) fn defs_sync_embedded(context: &ToolContext, args: &[String]) -> Result<CommandResult> {
    let Some(args) = parse_args::<SyncEmbeddedArgs>("defs sync-embedded", args)? else {
        return Ok(CommandResult::success());
    };
    execute_defs(
        DefsCommand::SyncEmbedded {
            input: args.input,
            output: args.output,
            check: args.check,
        },
        context,
    )
}

pub(crate) fn verify_package_command(
    context: &ToolContext,
    args: &[String],
) -> Result<CommandResult> {
    let Some(args) = parse_args::<PackageArgs>("verify package", args)? else {
        return Ok(CommandResult::success());
    };
    let config = current_config(context)?;
    super::verify::verify_package(
        &context.repo_root,
        &context.tool_root,
        &config,
        &package_options(args, context),
    )
}

pub(crate) fn package_pack_command(
    context: &ToolContext,
    args: &[String],
) -> Result<CommandResult> {
    let Some(args) = parse_args::<PackageArgs>("package pack", args)? else {
        return Ok(CommandResult::success());
    };
    let config = current_config(context)?;
    pack_package(
        &context.repo_root,
        &context.tool_root,
        &config,
        &package_options(args, context),
    )
}

pub(crate) fn package_stage_native_command(
    context: &ToolContext,
    args: &[String],
) -> Result<CommandResult> {
    let Some(args) = parse_args::<StageNativeArgs>("package stage-native", args)? else {
        return Ok(CommandResult::success());
    };

    #[cfg(windows)]
    if args.msvc_env {
        let exe =
            std::env::current_exe().context("failed to resolve dhara_tool executable path")?;
        let mut command = format!("\"{}\" package stage-native", exe.display());
        if args.configuration != "Release" {
            command.push_str(&format!(" --configuration {}", args.configuration));
        }
        crate::msvc::run_with_msvc_env(&command)?;
        return Ok(CommandResult::with_message(
            "Staged host native assets under MSVC environment.",
        ));
    }

    if args.msvc_env {
        bail!("--msvc-env is only supported on Windows");
    }

    let config = current_config(context)?;
    stage_native_for_host(
        &context.repo_root,
        &context.tool_root,
        &config,
        &PackageOptions {
            configuration: args.configuration,
            version_override: None,
            source_override: None,
            api_key_env_override: None,
            output_dir: context.output_dir.clone(),
            execute_publish: false,
            native_stage_override: None,
            prepacked_nuget_override: None,
        },
    )
}

pub(crate) fn native_merge_command(
    context: &ToolContext,
    args: &[String],
) -> Result<CommandResult> {
    let Some(args) = parse_args::<NativeMergeArgs>("native merge", args)? else {
        return Ok(CommandResult::success());
    };
    if args.input.is_empty() {
        bail!("native merge requires at least one --input path");
    }
    let output = resolve_path_against_repo(&context.repo_root, &args.output);
    let inputs: Vec<PathBuf> = args
        .input
        .iter()
        .map(|path| resolve_path_against_repo(&context.repo_root, path))
        .collect();
    crate::native_merge::merge_native_stages(&output, &inputs)?;
    Ok(CommandResult::with_message(format!(
        "Merged native stages into {}.",
        output.display()
    )))
}

pub(crate) fn quality_fmt_command(context: &ToolContext, args: &[String]) -> Result<CommandResult> {
    let Some(args) = parse_args::<QualityFmtArgs>("quality fmt", args)? else {
        return Ok(CommandResult::success());
    };
    crate::quality::run_fmt(&context.repo_root, args.check)?;
    Ok(CommandResult::with_message(if args.check {
        "Formatting check passed."
    } else {
        "Formatted workspace crates."
    }))
}

pub(crate) fn quality_clippy_command(
    context: &ToolContext,
    args: &[String],
) -> Result<CommandResult> {
    if parse_args::<NoArgs>("quality clippy", args)?.is_none() {
        return Ok(CommandResult::success());
    }
    crate::quality::run_clippy(&context.repo_root)?;
    Ok(CommandResult::with_message("Clippy checks passed."))
}

pub(crate) fn quality_doc_command(context: &ToolContext, args: &[String]) -> Result<CommandResult> {
    if parse_args::<NoArgs>("quality doc", args)?.is_none() {
        return Ok(CommandResult::success());
    }
    crate::quality::run_doc(&context.repo_root)?;
    Ok(CommandResult::with_message("Documentation build passed."))
}

pub(crate) fn quality_test_rust_command(
    context: &ToolContext,
    args: &[String],
) -> Result<CommandResult> {
    if parse_args::<NoArgs>("quality test-rust", args)?.is_none() {
        return Ok(CommandResult::success());
    }
    crate::quality::run_test_rust(&context.repo_root)?;
    Ok(CommandResult::with_message("Rust tests passed."))
}

pub(crate) fn quality_test_dotnet_command(
    context: &ToolContext,
    args: &[String],
) -> Result<CommandResult> {
    if parse_args::<NoArgs>("quality test-dotnet", args)?.is_none() {
        return Ok(CommandResult::success());
    }
    let config = current_config(context)?;
    crate::quality::ensure_dotnet_available()?;
    crate::quality::run_test_dotnet(&context.repo_root, &config)?;
    Ok(CommandResult::with_message(".NET tests passed."))
}

pub(crate) fn quality_run_command(context: &ToolContext, args: &[String]) -> Result<CommandResult> {
    let Some(args) = parse_args::<QualityRunArgs>("quality run", args)? else {
        return Ok(CommandResult::success());
    };
    let config = current_config(context)?;
    crate::quality::run_all(
        &context.repo_root,
        &config,
        args.skip_docs,
        args.skip_dotnet,
    )?;
    Ok(CommandResult::with_message("Local CI checks passed."))
}

pub(crate) fn package_publish_command(
    context: &ToolContext,
    args: &[String],
) -> Result<CommandResult> {
    let Some(args) = parse_args::<PublishArgs>("package publish", args)? else {
        return Ok(CommandResult::success());
    };
    let config = current_config(context)?;
    publish_package(
        &context.repo_root,
        &context.tool_root,
        &config,
        &publish_options(args, context)?,
    )
}

pub(crate) fn release_run_command(context: &ToolContext, args: &[String]) -> Result<CommandResult> {
    #[cfg(windows)]
    if std::env::var_os("DHARA_TOOL_INSIDE_MSVC").is_none() {
        let exe =
            std::env::current_exe().context("failed to resolve dhara_tool executable path")?;
        let mut command = format!(
            "set DHARA_TOOL_INSIDE_MSVC=1&& \"{}\" release run",
            exe.display()
        );
        for arg in args {
            command.push(' ');
            if arg.contains(' ') {
                command.push('"');
                command.push_str(arg);
                command.push('"');
            } else {
                command.push_str(arg);
            }
        }
        crate::msvc::run_with_msvc_env(&command)?;
        return Ok(CommandResult::with_message(
            "Release flow completed under MSVC environment.",
        ));
    }

    let Some(args) = parse_args::<ReleaseRunArgs>("release run", args)? else {
        return Ok(CommandResult::success());
    };
    let config = current_config(context)?;
    run_release(
        &context.repo_root,
        &context.tool_root,
        &config,
        &release_options(args, context),
    )
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::command::ToolContext;

    use super::{ReleaseRunArgs, parse_args, release_options};

    fn test_context() -> ToolContext {
        ToolContext {
            repo_root: PathBuf::from("."),
            tool_root: PathBuf::from("."),
            run_mode: crate::command::RunMode::Direct,
            min: false,
            trace: false,
            workers: 4,
            package_dir: None,
            output_dir: None,
            logs_dir: None,
        }
    }

    #[test]
    fn release_run_defaults_to_execute_with_nuget() {
        let args = parse_args::<ReleaseRunArgs>("release run", &[])
            .unwrap()
            .unwrap();
        let options = release_options(args, &test_context());

        assert!(!options.dry_run);
        assert!(options.publish_cargo);
        assert!(options.publish_nuget);
        assert_eq!(options.configuration, "Release");
    }

    #[test]
    fn release_run_supports_dry_run_and_skips() {
        let args = parse_args::<ReleaseRunArgs>(
            "release run",
            &[
                "--dry-run".to_owned(),
                "--skip-cargo".to_owned(),
                "--skip-nuget".to_owned(),
            ],
        )
        .unwrap()
        .unwrap();
        let options = release_options(args, &test_context());

        assert!(options.dry_run);
        assert!(!options.publish_cargo);
        assert!(!options.publish_nuget);
    }
}
