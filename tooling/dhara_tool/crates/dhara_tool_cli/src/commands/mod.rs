mod config;
mod defs;
mod package;
mod quality;

use std::path::PathBuf;

use anyhow::{Result, bail};
use clap::{ArgAction, Parser, ValueEnum};

use crate::command::ToolContext;
use dhara_tool_kernel::repo_config::{VersionPart, load_config};
use dhara_tool_ops::nuget::PackageOptions;

pub(crate) use config::{config_env_init, config_show, version_bump, version_set};
pub(crate) use defs::{
    defs_build_trid_xml, defs_inspect, defs_inspect_trid_xml, defs_normalize, defs_pack,
    defs_sync_embedded, defs_verify,
};
pub(crate) use package::{
    native_merge_command, package_pack_command, package_publish_command,
    package_stage_native_command, release_run_command, verify_package_command,
};
pub(crate) use quality::{
    quality_clippy_command, quality_doc_command, quality_fmt_command, quality_run_command,
    quality_test_dotnet_command, quality_test_rust_command,
};

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum PartArg {
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
pub(crate) struct NoArgs {}

#[derive(Debug, Parser)]
pub(crate) struct VersionSetArgs {
    pub version: String,
}

#[derive(Debug, Parser)]
pub(crate) struct VersionBumpArgs {
    #[arg(long)]
    pub part: PartArg,
}

#[derive(Debug, Parser)]
pub(crate) struct StageNativeArgs {
    #[arg(long, default_value = "Release")]
    pub configuration: String,
    #[arg(long, action = ArgAction::SetTrue)]
    pub msvc_env: bool,
}

#[derive(Debug, Parser)]
pub(crate) struct NativeMergeArgs {
    #[arg(long)]
    pub output: PathBuf,
    #[arg(long)]
    pub input: Vec<PathBuf>,
}

#[derive(Debug, Parser)]
pub(crate) struct QualityFmtArgs {
    #[arg(long, action = ArgAction::SetTrue)]
    pub check: bool,
}

#[derive(Debug, Parser)]
pub(crate) struct QualityRunArgs {
    #[arg(long, action = ArgAction::SetTrue)]
    pub skip_docs: bool,
    #[arg(long, action = ArgAction::SetTrue)]
    pub skip_dotnet: bool,
}

#[derive(Debug, Parser)]
pub(crate) struct PackageArgs {
    #[arg(long, default_value = "Release")]
    pub configuration: String,
    #[arg(long)]
    pub version: Option<String>,
    #[arg(long)]
    pub native_stage: Option<PathBuf>,
}

#[derive(Debug, Parser)]
pub(crate) struct PublishArgs {
    #[arg(long, default_value = "Release")]
    pub configuration: String,
    #[arg(long)]
    pub version: Option<String>,
    #[arg(long)]
    pub source: Option<String>,
    #[arg(long)]
    pub api_key_env: Option<String>,
    #[arg(long, action = ArgAction::SetTrue)]
    pub dry_run: bool,
    #[arg(long, action = ArgAction::SetTrue)]
    pub execute: bool,
}

#[derive(Debug, Parser)]
pub(crate) struct ReleaseRunArgs {
    #[arg(long, default_value = "Release")]
    pub configuration: String,
    #[arg(long)]
    pub source: Option<String>,
    #[arg(long)]
    pub api_key_env: Option<String>,
    #[arg(long, action = ArgAction::SetTrue)]
    pub dry_run: bool,
    #[arg(long, action = ArgAction::SetTrue)]
    pub skip_cargo: bool,
    #[arg(long, action = ArgAction::SetTrue)]
    pub skip_nuget: bool,
    #[arg(long)]
    pub native_stage: Option<PathBuf>,
    #[arg(long)]
    pub prepacked_nuget: Option<PathBuf>,
    #[arg(long, action = ArgAction::SetTrue)]
    pub verify_package: bool,
}

#[derive(Debug, Parser)]
pub(crate) struct OutputArg {
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Parser)]
pub(crate) struct InputArg {
    #[arg(short, long)]
    pub input: Option<PathBuf>,
}

#[derive(Debug, Parser)]
pub(crate) struct InputOutputArgs {
    #[arg(short, long)]
    pub input: Option<PathBuf>,
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Parser)]
pub(crate) struct VerifyDefsArgs {
    #[arg(long)]
    pub left: PathBuf,
    #[arg(long)]
    pub right: PathBuf,
}

#[derive(Debug, Parser)]
pub(crate) struct SyncEmbeddedArgs {
    #[arg(short, long)]
    pub input: Option<PathBuf>,
    #[arg(short, long)]
    pub output: Option<PathBuf>,
    #[arg(long)]
    pub check: bool,
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

pub(crate) fn current_config(context: &ToolContext) -> Result<dhara_tool_kernel::DharaRepoConfig> {
    load_config(&context.repo_root)
}

pub(crate) fn package_options(args: PackageArgs, context: &ToolContext) -> PackageOptions {
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

pub(crate) fn publish_options(args: PublishArgs, context: &ToolContext) -> Result<PackageOptions> {
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
) -> dhara_tool_ops::ReleaseOptions {
    dhara_tool_ops::ReleaseOptions {
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::command::{RunMode, ToolContext};

    use super::{ReleaseRunArgs, parse_args, release_options};

    fn test_context() -> ToolContext {
        ToolContext {
            repo_root: PathBuf::from("."),
            tool_root: PathBuf::from("."),
            run_mode: RunMode::Direct,
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
