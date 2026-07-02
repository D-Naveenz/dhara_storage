use std::path::PathBuf;

#[cfg(windows)]
use anyhow::Context;
use anyhow::{Result, bail};

use crate::command::{CommandResult, ToolContext};
use dhara_tool_kernel::paths::resolve_path_against_repo;
use dhara_tool_ops::nuget::{
    PackageOptions, pack as pack_package, publish as publish_package, stage_native_for_host,
};
use dhara_tool_ops::release::run as run_release;

use super::{
    NativeMergeArgs, PackageArgs, PublishArgs, ReleaseRunArgs, StageNativeArgs, current_config,
    package_options, parse_args, publish_options, release_options,
};

pub(crate) fn verify_package_command(
    context: &ToolContext,
    args: &[String],
) -> Result<CommandResult> {
    let Some(args) = parse_args::<PackageArgs>("verify package", args)? else {
        return Ok(CommandResult::success());
    };
    let config = current_config(context)?;
    dhara_tool_ops::verify::verify_package(
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
        dhara_tool_kernel::msvc::run_with_msvc_env(&command)?;
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
    dhara_tool_ops::native_merge::merge_native_stages(&output, &inputs)?;
    Ok(CommandResult::with_message(format!(
        "Merged native stages into {}.",
        output.display()
    )))
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
        dhara_tool_kernel::msvc::run_with_msvc_env(&command)?;
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
