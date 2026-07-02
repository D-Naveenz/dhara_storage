use anyhow::Result;

use crate::command::{CommandResult, ToolContext};

use super::{NoArgs, QualityFmtArgs, QualityRunArgs, current_config, parse_args};

pub(crate) fn quality_fmt_command(context: &ToolContext, args: &[String]) -> Result<CommandResult> {
    let Some(args) = parse_args::<QualityFmtArgs>("quality fmt", args)? else {
        return Ok(CommandResult::success());
    };
    dhara_tool_ops::quality::run_fmt(&context.repo_root, args.check)?;
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
    dhara_tool_ops::quality::run_clippy(&context.repo_root)?;
    Ok(CommandResult::with_message("Clippy checks passed."))
}

pub(crate) fn quality_doc_command(context: &ToolContext, args: &[String]) -> Result<CommandResult> {
    if parse_args::<NoArgs>("quality doc", args)?.is_none() {
        return Ok(CommandResult::success());
    }
    dhara_tool_ops::quality::run_doc(&context.repo_root)?;
    Ok(CommandResult::with_message("Documentation build passed."))
}

pub(crate) fn quality_test_rust_command(
    context: &ToolContext,
    args: &[String],
) -> Result<CommandResult> {
    if parse_args::<NoArgs>("quality test-rust", args)?.is_none() {
        return Ok(CommandResult::success());
    }
    dhara_tool_ops::quality::run_test_rust(&context.repo_root)?;
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
    dhara_tool_ops::quality::ensure_dotnet_available()?;
    dhara_tool_ops::quality::run_test_dotnet(&context.repo_root, &config)?;
    Ok(CommandResult::with_message(".NET tests passed."))
}

pub(crate) fn quality_run_command(context: &ToolContext, args: &[String]) -> Result<CommandResult> {
    let Some(args) = parse_args::<QualityRunArgs>("quality run", args)? else {
        return Ok(CommandResult::success());
    };
    let config = current_config(context)?;
    dhara_tool_ops::quality::run_all(
        &context.repo_root,
        &config,
        args.skip_docs,
        args.skip_dotnet,
    )?;
    Ok(CommandResult::with_message("Local CI checks passed."))
}
