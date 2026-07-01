use anyhow::Result;

use crate::command::{CommandResult, ToolContext};
use dhara_tool_kernel::{DefsCommand, execute_defs, print_defs_help};

use super::{
    InputArg, InputOutputArgs, OutputArg, SyncEmbeddedArgs, VerifyDefsArgs, parse_args,
};

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
