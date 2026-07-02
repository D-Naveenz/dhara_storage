use crate::command::SectionSpec;
use crate::commands::{
    defs_build_trid_xml, defs_inspect, defs_inspect_trid_xml, defs_normalize, defs_pack,
    defs_sync_embedded, defs_verify,
};

use super::{command, RegisteredCommand};

pub fn section() -> SectionSpec {
    SectionSpec {
        name: "defs",
        prompt: "dhara:defs> ",
        summary: "Definitions package commands",
    }
}

pub fn commands() -> Vec<RegisteredCommand> {
    vec![
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
    ]
}
