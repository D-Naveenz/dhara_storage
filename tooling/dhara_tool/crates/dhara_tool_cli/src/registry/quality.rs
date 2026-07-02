use crate::command::SectionSpec;
use crate::commands::{
    quality_clippy_command, quality_doc_command, quality_fmt_command, quality_run_command,
    quality_test_dotnet_command, quality_test_rust_command,
};

use super::{command, RegisteredCommand};

pub fn section() -> SectionSpec {
    SectionSpec {
        name: "quality",
        prompt: "dhara:quality> ",
        summary: "Repository quality gate commands",
    }
}

pub fn commands() -> Vec<RegisteredCommand> {
    vec![
        command(
            "quality.fmt",
            &["quality", "fmt"],
            "Run rustfmt on workspace crates",
            "[--check]",
            "quality",
            quality_fmt_command,
        ),
        command(
            "quality.clippy",
            &["quality", "clippy"],
            "Run clippy on workspace crates",
            "",
            "quality",
            quality_clippy_command,
        ),
        command(
            "quality.doc",
            &["quality", "doc"],
            "Build Rust API documentation",
            "",
            "quality",
            quality_doc_command,
        ),
        command(
            "quality.test-rust",
            &["quality", "test-rust"],
            "Run Rust crate tests",
            "",
            "quality",
            quality_test_rust_command,
        ),
        command(
            "quality.test-dotnet",
            &["quality", "test-dotnet"],
            "Run .NET binding tests",
            "",
            "quality",
            quality_test_dotnet_command,
        ),
        command(
            "quality.run",
            &["quality", "run"],
            "Run local CI parity checks",
            "[--skip-docs] [--skip-dotnet]",
            "quality",
            quality_run_command,
        ),
    ]
}
