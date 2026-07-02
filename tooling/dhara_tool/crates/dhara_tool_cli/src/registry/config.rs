use crate::command::SectionSpec;
use crate::commands::{config_env_init, config_show, version_bump, version_set};

use super::{command, RegisteredCommand};

pub fn section() -> SectionSpec {
    SectionSpec {
        name: "config",
        prompt: "dhara:config> ",
        summary: "Repository configuration commands",
    }
}

pub fn version_section() -> SectionSpec {
    SectionSpec {
        name: "version",
        prompt: "dhara:version> ",
        summary: "Versioning commands",
    }
}

pub fn commands() -> Vec<RegisteredCommand> {
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
    ]
}
