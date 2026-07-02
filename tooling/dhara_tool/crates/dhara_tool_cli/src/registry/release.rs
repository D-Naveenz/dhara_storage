use crate::command::SectionSpec;
use crate::commands::release_run_command;

use super::{command, RegisteredCommand};

pub fn section() -> SectionSpec {
    SectionSpec {
        name: "release",
        prompt: "dhara:release> ",
        summary: "Release commands",
    }
}

pub fn commands() -> Vec<RegisteredCommand> {
    vec![command(
        "release.run",
        &["release", "run"],
        "Run the Cargo-first release workflow",
        "[--configuration <name>] [--source <url>] [--api-key-env <name>] [--dry-run] [--skip-cargo] [--skip-nuget] [--native-stage <path>] [--prepacked-nuget <path>] [--verify-package]",
        "release",
        release_run_command,
    )]
}
