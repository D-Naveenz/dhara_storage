use anyhow::Result;

use crate::command::CommandResult;
use crate::command::ToolContext;
use dhara_tool_kernel::repo_config::{bump_version, init_env, set_version, show};

use super::{NoArgs, VersionBumpArgs, VersionSetArgs, parse_args};

pub(crate) fn config_show(context: &ToolContext, args: &[String]) -> Result<CommandResult> {
    if parse_args::<NoArgs>("config show", args)?.is_none() {
        return Ok(CommandResult::success());
    }
    Ok(CommandResult::with_message(show(&context.repo_root)?))
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
