use std::path::PathBuf;

use anyhow::{Result, bail};
use clap::{ArgAction, Parser, ValueEnum};

use super::{
    CommandResult, CommandSpec, DefsCommand, PackageOptions, SectionSpec, ToolContext, VersionPart,
    bump_version, execute_defs, init_env, load_config, pack_package, print_defs_help,
    publish_package, run_release, set_version, show, sync,
};

pub struct DharaStorageCapability;

impl DharaStorageCapability {
    pub fn sections(&self) -> Vec<SectionSpec> {
        vec![
            SectionSpec {
                name: "config",
                prompt: "dhara:config> ",
                summary: "Repository configuration commands",
            },
            SectionSpec {
                name: "version",
                prompt: "dhara:version> ",
                summary: "Versioning commands",
            },
            SectionSpec {
                name: "defs",
                prompt: "dhara:defs> ",
                summary: "Definitions package commands",
            },
            SectionSpec {
                name: "verify",
                prompt: "dhara:verify> ",
                summary: "Verification commands",
            },
            SectionSpec {
                name: "package",
                prompt: "dhara:package> ",
                summary: "NuGet packaging commands",
            },
            SectionSpec {
                name: "release",
                prompt: "dhara:release> ",
                summary: "Release commands",
            },
        ]
    }

    pub fn commands(&self) -> Vec<CommandSpec> {
        vec![
            command(
                "config.show",
                &["config", "show"],
                "Show effective repo configuration",
                "",
                "config",
                config_show,
            ),
            command(
                "config.sync",
                &["config", "sync"],
                "Synchronize repo-managed metadata",
                "",
                "config",
                config_sync,
            ),
            command(
                "config.env.init",
                &["config", "env", "init"],
                "Create .env.local from .env.example",
                "",
                "config",
                config_env_init,
            ),
            command(
                "version.set",
                &["version", "set"],
                "Set the shared workspace version",
                "<version>",
                "version",
                version_set,
            ),
            command(
                "version.bump",
                &["version", "bump"],
                "Bump the shared workspace version",
                "--part <major|minor|patch>",
                "version",
                version_bump,
            ),
            command(
                "defs.pack",
                &["defs", "pack"],
                "Write the bundled definitions package",
                "[--output <path>]",
                "defs",
                defs_pack,
            ),
            command(
                "defs.build-trid-xml",
                &["defs", "build-trid-xml"],
                "Build a definitions package from TrID XML",
                "[--input <path>] [--output <path>]",
                "defs",
                defs_build_trid_xml,
            ),
            command(
                "defs.inspect",
                &["defs", "inspect"],
                "Inspect an encoded package",
                "[--input <path>]",
                "defs",
                defs_inspect,
            ),
            command(
                "defs.inspect-trid-xml",
                &["defs", "inspect-trid-xml"],
                "Inspect a TrID XML source without writing output",
                "[--input <path>]",
                "defs",
                defs_inspect_trid_xml,
            ),
            command(
                "defs.normalize",
                &["defs", "normalize"],
                "Normalize an existing package",
                "[--input <path>] [--output <path>]",
                "defs",
                defs_normalize,
            ),
            command(
                "defs.verify",
                &["defs", "verify"],
                "Compare two packages",
                "--left <path> --right <path>",
                "defs",
                defs_verify,
            ),
            command(
                "defs.sync-embedded",
                &["defs", "sync-embedded"],
                "Refresh the embedded runtime package",
                "[--input <path>] [--output <path>] [--check]",
                "defs",
                defs_sync_embedded,
            ),
            command(
                "verify.release-config",
                &["verify", "release-config"],
                "Validate release configuration",
                "",
                "verify",
                verify_release_config_command,
            ),
            command(
                "verify.ci",
                &["verify", "ci"],
                "Run local CI-equivalent checks",
                "",
                "verify",
                verify_ci_command,
            ),
            command(
                "verify.docs",
                &["verify", "docs"],
                "Build docs for core crates",
                "",
                "verify",
                verify_docs_command,
            ),
            command(
                "verify.package",
                &["verify", "package"],
                "Pack and verify the NuGet package",
                "[--configuration <name>] [--version <semver>]",
                "verify",
                verify_package_command,
            ),
            command(
                "package.pack",
                &["package", "pack"],
                "Pack the NuGet package",
                "[--configuration <name>] [--version <semver>]",
                "package",
                package_pack_command,
            ),
            command(
                "package.publish",
                &["package", "publish"],
                "Verify and optionally publish the NuGet package",
                "[--configuration <name>] [--version <semver>] [--source <url>] [--api-key-env <name>] [--dry-run|--execute]",
                "package",
                package_publish_command,
            ),
            command(
                "release.publish",
                &["release", "publish"],
                "Release the NuGet package",
                "[--configuration <name>] [--version <semver>] [--source <url>] [--api-key-env <name>] [--dry-run|--execute]",
                "release",
                release_publish_command,
            ),
            command(
                "release.run",
                &["release", "run"],
                "Run the Cargo-first release workflow",
                "[--configuration <name>] [--source <url>] [--api-key-env <name>] [--dry-run] [--skip-cargo] [--skip-nuget]",
                "release",
                release_run_command,
            ),
        ]
    }
}

fn command(
    id: &'static str,
    path: &'static [&'static str],
    summary: &'static str,
    args_summary: &'static str,
    section: &'static str,
    handler: fn(&ToolContext, &[String]) -> Result<CommandResult>,
) -> CommandSpec {
    CommandSpec {
        id,
        path,
        summary,
        args_summary,
        section,
        handler,
    }
}

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
struct ReleaseRunArgs {
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

fn parse_args<T: Parser>(name: &str, args: &[String]) -> Result<Option<T>> {
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

fn current_config(context: &ToolContext) -> Result<super::DharaRepoConfig> {
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

fn release_options(args: ReleaseRunArgs, context: &ToolContext) -> super::ReleaseOptions {
    super::ReleaseOptions {
        configuration: args.configuration,
        source_override: args.source,
        api_key_env_override: args.api_key_env,
        output_dir: context.output_dir.clone(),
        dry_run: args.dry_run,
        publish_cargo: !args.skip_cargo,
        publish_nuget: !args.skip_nuget,
    }
}

fn config_show(context: &ToolContext, args: &[String]) -> Result<CommandResult> {
    if parse_args::<NoArgs>("config show", args)?.is_none() {
        return Ok(CommandResult::success());
    }
    Ok(CommandResult::with_message(show(&context.repo_root)?))
}

fn config_sync(context: &ToolContext, args: &[String]) -> Result<CommandResult> {
    if parse_args::<NoArgs>("config sync", args)?.is_none() {
        return Ok(CommandResult::success());
    }
    sync(&context.repo_root)?;
    Ok(CommandResult::with_message(
        "Synchronized repository configuration.",
    ))
}

fn config_env_init(context: &ToolContext, args: &[String]) -> Result<CommandResult> {
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

fn version_set(context: &ToolContext, args: &[String]) -> Result<CommandResult> {
    let Some(args) = parse_args::<VersionSetArgs>("version set", args)? else {
        return Ok(CommandResult::success());
    };
    set_version(&context.repo_root, &args.version)?;
    Ok(CommandResult::with_message(format!(
        "Updated version to {}.",
        args.version
    )))
}

fn version_bump(context: &ToolContext, args: &[String]) -> Result<CommandResult> {
    let Some(args) = parse_args::<VersionBumpArgs>("version bump", args)? else {
        return Ok(CommandResult::success());
    };
    let next = bump_version(&context.repo_root, args.part.into())?;
    Ok(CommandResult::with_message(next))
}

fn defs_pack(context: &ToolContext, args: &[String]) -> Result<CommandResult> {
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

fn defs_build_trid_xml(context: &ToolContext, args: &[String]) -> Result<CommandResult> {
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

fn defs_inspect(context: &ToolContext, args: &[String]) -> Result<CommandResult> {
    let Some(args) = parse_args::<InputArg>("defs inspect", args)? else {
        return Ok(CommandResult::success());
    };
    execute_defs(DefsCommand::Inspect { input: args.input }, context)
}

fn defs_inspect_trid_xml(context: &ToolContext, args: &[String]) -> Result<CommandResult> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        return Ok(CommandResult::with_message(print_defs_help()));
    }
    let Some(args) = parse_args::<InputArg>("defs inspect-trid-xml", args)? else {
        return Ok(CommandResult::success());
    };
    execute_defs(DefsCommand::InspectTridXml { input: args.input }, context)
}

fn defs_normalize(context: &ToolContext, args: &[String]) -> Result<CommandResult> {
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

fn defs_verify(context: &ToolContext, args: &[String]) -> Result<CommandResult> {
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

fn defs_sync_embedded(context: &ToolContext, args: &[String]) -> Result<CommandResult> {
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

fn verify_release_config_command(context: &ToolContext, args: &[String]) -> Result<CommandResult> {
    if parse_args::<NoArgs>("verify release-config", args)?.is_none() {
        return Ok(CommandResult::success());
    }
    super::verify::verify_release_config(&context.repo_root)
}

fn verify_ci_command(context: &ToolContext, args: &[String]) -> Result<CommandResult> {
    if parse_args::<NoArgs>("verify ci", args)?.is_none() {
        return Ok(CommandResult::success());
    }
    let config = current_config(context)?;
    super::verify::verify_ci(&context.repo_root, &config)
}

fn verify_docs_command(context: &ToolContext, args: &[String]) -> Result<CommandResult> {
    if parse_args::<NoArgs>("verify docs", args)?.is_none() {
        return Ok(CommandResult::success());
    }
    super::verify::verify_docs(&context.repo_root)
}

fn verify_package_command(context: &ToolContext, args: &[String]) -> Result<CommandResult> {
    let Some(args) = parse_args::<PackageArgs>("verify package", args)? else {
        return Ok(CommandResult::success());
    };
    let config = current_config(context)?;
    super::verify::verify_package(&context.repo_root, &config, &package_options(args, context))
}

fn package_pack_command(context: &ToolContext, args: &[String]) -> Result<CommandResult> {
    let Some(args) = parse_args::<PackageArgs>("package pack", args)? else {
        return Ok(CommandResult::success());
    };
    let config = current_config(context)?;
    pack_package(&context.repo_root, &config, &package_options(args, context))
}

fn package_publish_command(context: &ToolContext, args: &[String]) -> Result<CommandResult> {
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

fn release_publish_command(context: &ToolContext, args: &[String]) -> Result<CommandResult> {
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

fn release_run_command(context: &ToolContext, args: &[String]) -> Result<CommandResult> {
    let Some(args) = parse_args::<ReleaseRunArgs>("release run", args)? else {
        return Ok(CommandResult::success());
    };
    let config = current_config(context)?;
    run_release(&context.repo_root, &config, &release_options(args, context))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::ToolContext;
    use super::{DharaStorageCapability, ReleaseRunArgs, parse_args, release_options};

    #[test]
    fn registration_adds_expected_sections_and_commands() {
        let capability = DharaStorageCapability;

        let sections = capability
            .sections()
            .into_iter()
            .map(|section| section.name)
            .collect::<Vec<_>>();
        assert_eq!(
            sections,
            vec!["config", "version", "defs", "verify", "package", "release"]
        );

        let commands = capability
            .commands()
            .into_iter()
            .map(|command| command.id)
            .collect::<Vec<_>>();
        assert!(commands.contains(&"config.show"));
        assert!(commands.contains(&"defs.inspect-trid-xml"));
        assert!(commands.contains(&"verify.package"));
        assert!(commands.contains(&"release.publish"));
        assert!(commands.contains(&"release.run"));
    }

    fn test_context() -> ToolContext {
        ToolContext {
            repo_root: PathBuf::from("."),
            silent: false,
            verbose: 0,
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
