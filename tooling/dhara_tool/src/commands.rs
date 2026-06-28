use std::path::PathBuf;

use anyhow::{Result, bail};
use clap::{ArgAction, Parser, ValueEnum};

use crate::command::{CommandResult, ToolContext};
use crate::filedefs::{DefsCommand, execute as execute_defs, print_defs_help};
use crate::nuget::{PackageOptions, pack as pack_package, publish as publish_package};
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
struct PackageArgs {
    #[arg(long, default_value = "Release")]
    configuration: String,
    #[arg(long)]
    version: Option<String>,
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

pub(crate) fn verify_release_config_command(
    context: &ToolContext,
    args: &[String],
) -> Result<CommandResult> {
    if parse_args::<NoArgs>("verify release-config", args)?.is_none() {
        return Ok(CommandResult::success());
    }
    super::verify::verify_release_config(&context.repo_root)
}

pub(crate) fn verify_ci_command(context: &ToolContext, args: &[String]) -> Result<CommandResult> {
    if parse_args::<NoArgs>("verify ci", args)?.is_none() {
        return Ok(CommandResult::success());
    }
    let config = current_config(context)?;
    super::verify::verify_ci(&context.repo_root, &config)
}

pub(crate) fn verify_docs_command(context: &ToolContext, args: &[String]) -> Result<CommandResult> {
    if parse_args::<NoArgs>("verify docs", args)?.is_none() {
        return Ok(CommandResult::success());
    }
    super::verify::verify_docs(&context.repo_root)
}

pub(crate) fn verify_package_command(
    context: &ToolContext,
    args: &[String],
) -> Result<CommandResult> {
    let Some(args) = parse_args::<PackageArgs>("verify package", args)? else {
        return Ok(CommandResult::success());
    };
    let config = current_config(context)?;
    super::verify::verify_package(&context.repo_root, &config, &package_options(args, context))
}

pub(crate) fn package_pack_command(
    context: &ToolContext,
    args: &[String],
) -> Result<CommandResult> {
    let Some(args) = parse_args::<PackageArgs>("package pack", args)? else {
        return Ok(CommandResult::success());
    };
    let config = current_config(context)?;
    pack_package(&context.repo_root, &config, &package_options(args, context))
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
        &config,
        &publish_options(args, context)?,
    )
}

pub(crate) fn release_publish_command(
    context: &ToolContext,
    args: &[String],
) -> Result<CommandResult> {
    let Some(args) = parse_args::<PublishArgs>("release publish", args)? else {
        return Ok(CommandResult::success());
    };
    let config = current_config(context)?;
    publish_package(
        &context.repo_root,
        &config,
        &publish_options(args, context)?,
    )
}

pub(crate) fn release_run_command(context: &ToolContext, args: &[String]) -> Result<CommandResult> {
    let Some(args) = parse_args::<ReleaseRunArgs>("release run", args)? else {
        return Ok(CommandResult::success());
    };
    let config = current_config(context)?;
    run_release(&context.repo_root, &config, &release_options(args, context))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::command::ToolContext;

    use super::{ReleaseRunArgs, parse_args, release_options};

    fn test_context() -> ToolContext {
        ToolContext {
            repo_root: PathBuf::from("."),
            run_mode: crate::command::RunMode::Direct,
            minimal: false,
            trace: false,
            quiet: false,
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
