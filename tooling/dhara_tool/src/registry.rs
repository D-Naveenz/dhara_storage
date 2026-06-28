use std::sync::Arc;

use anyhow::Result;

use crate::command::{
    ArgBinding, CommandRegistry, CommandResult, CommandSpec, CommandUi, FieldKind, FieldSpec,
    SectionSpec, ToolCapability, ToolContext,
};
use crate::commands::{
    config_env_init, config_show, config_sync, defs_build_trid_xml, defs_inspect,
    defs_inspect_trid_xml, defs_normalize, defs_pack, defs_sync_embedded, defs_verify,
    package_pack_command, package_publish_command, release_publish_command, release_run_command,
    verify_ci_command, verify_docs_command, verify_package_command, verify_release_config_command,
    version_bump, version_set,
};
const VERSION_PARTS: &[&str] = &["major", "minor", "patch"];
const CONFIGURATIONS: &[&str] = &["Release"];
const DRY_RUN_OPTIONS: &[&str] = &["dry-run", "execute"];

struct RegisteredCommand {
    id: &'static str,
    path: &'static [&'static str],
    summary: &'static str,
    args_summary: &'static str,
    section: &'static str,
    handler: fn(&ToolContext, &[String]) -> Result<CommandResult>,
}

pub struct DharaStorageCapability;

impl ToolCapability for DharaStorageCapability {
    fn register(&self, registry: &mut CommandRegistry) {
        for section in self.sections() {
            registry.add_section(section);
        }

        for command in self.commands() {
            let handler = command.handler;
            registry.add_command(CommandSpec {
                id: command.id,
                path: command.path,
                summary: command.summary,
                args_summary: command.args_summary,
                section: command.section,
                ui: ui_for_command(command.id, command.summary, command.args_summary),
                handler: Arc::new(handler),
            });
        }
    }
}

impl DharaStorageCapability {
    fn sections(&self) -> Vec<SectionSpec> {
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

    fn commands(&self) -> Vec<RegisteredCommand> {
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
) -> RegisteredCommand {
    RegisteredCommand {
        id,
        path,
        summary,
        args_summary,
        section,
        handler,
    }
}
fn ui_for_command(
    id: &'static str,
    summary: &'static str,
    args_summary: &'static str,
) -> CommandUi {
    match id {
        "config.show" => quick_command(
            "Inspect the effective Dhara repository configuration and resolved environment.",
            false,
        ),
        "config.sync" => quick_command(
            "Synchronize repo-managed metadata such as workspace version and NuGet package metadata.",
            false,
        ),
        "config.env.init" => quick_command(
            "Create .env.local from .env.example when the local file is missing.",
            false,
        ),
        "version.set" => CommandUi {
            description: "Set the shared workspace version used by both Cargo and NuGet metadata.",
            fields: vec![FieldSpec {
                key: "version",
                label: "Version",
                help: "Semantic version to write into dhara.config.toml and synchronized package metadata.",
                kind: FieldKind::Text,
                binding: ArgBinding::Positional,
                required: true,
                default_value: None,
            }],
            quick_run: true,
            supports_cancel: false,
        },
        "version.bump" => CommandUi {
            description: "Bump the shared workspace version using semantic-version part semantics.",
            fields: vec![FieldSpec {
                key: "part",
                label: "Part",
                help: "Which portion of the shared workspace version should be incremented.",
                kind: FieldKind::Select(VERSION_PARTS),
                binding: ArgBinding::FlagValue("--part"),
                required: true,
                default_value: Some("minor"),
            }],
            quick_run: true,
            supports_cancel: false,
        },
        "defs.pack" => CommandUi {
            description: "Write the bundled file definitions package from the embedded runtime asset.",
            fields: vec![optional_path(
                "output",
                "Output",
                "Optional output file path.",
                "--output",
            )],
            quick_run: false,
            supports_cancel: false,
        },
        "defs.build-trid-xml" => CommandUi {
            description: "Build a filedefs.dat package from TrID XML sources or archives.",
            fields: vec![
                optional_path(
                    "input",
                    "Input",
                    "Optional TrID XML input path or archive.",
                    "--input",
                ),
                optional_path(
                    "output",
                    "Output",
                    "Optional output package path.",
                    "--output",
                ),
            ],
            quick_run: false,
            supports_cancel: false,
        },
        "defs.inspect" => CommandUi {
            description: "Inspect an encoded FlatBuffers package and summarize its metadata and counts.",
            fields: vec![optional_path(
                "input",
                "Input",
                "Optional package path to inspect.",
                "--input",
            )],
            quick_run: false,
            supports_cancel: false,
        },
        "defs.inspect-trid-xml" => CommandUi {
            description: "Preview TrID XML transformation results without writing an output package.",
            fields: vec![optional_path(
                "input",
                "Input",
                "Optional TrID XML source path.",
                "--input",
            )],
            quick_run: false,
            supports_cancel: false,
        },
        "defs.normalize" => CommandUi {
            description: "Normalize an existing FlatBuffers package into the canonical builder format.",
            fields: vec![
                optional_path("input", "Input", "Optional source package path.", "--input"),
                optional_path(
                    "output",
                    "Output",
                    "Optional normalized output path.",
                    "--output",
                ),
            ],
            quick_run: false,
            supports_cancel: false,
        },
        "defs.verify" => CommandUi {
            description: "Compare two FlatBuffers packages for semantic equivalence.",
            fields: vec![
                required_path("left", "Left", "Left-hand package path.", "--left"),
                required_path("right", "Right", "Right-hand package path.", "--right"),
            ],
            quick_run: false,
            supports_cancel: false,
        },
        "defs.sync-embedded" => CommandUi {
            description: "Refresh the runtime filedefs.dat package at tooling/output from the builder source.",
            fields: vec![
                optional_path(
                    "input",
                    "Input",
                    "Optional TrID XML archive or directory path.",
                    "--input",
                ),
                optional_path(
                    "output",
                    "Output",
                    "Optional embedded package output path.",
                    "--output",
                ),
                FieldSpec {
                    key: "check",
                    label: "Check only",
                    help: "Validate whether the embedded package is up to date without writing changes.",
                    kind: FieldKind::Boolean,
                    binding: ArgBinding::Switch("--check"),
                    required: false,
                    default_value: Some("false"),
                },
            ],
            quick_run: false,
            supports_cancel: false,
        },
        "verify.release-config" => quick_command(
            "Validate the release configuration and required repo layout.",
            false,
        ),
        "verify.ci" => quick_command(
            "Run the repo's local CI-equivalent checks for formatting, linting, tests, and .NET verification.",
            true,
        ),
        "verify.docs" => quick_command(
            "Build documentation for the Dhara crates without dependencies.",
            true,
        ),
        "verify.package" => package_command(
            "Pack and verify the Dhara.Storage NuGet package, including smoke-consumer validation.",
        ),
        "package.pack" => {
            package_command("Pack the Dhara.Storage NuGet package with staged native assets.")
        }
        "package.publish" | "release.publish" => CommandUi {
            description: "Verify and optionally publish the Dhara.Storage NuGet package.",
            fields: vec![
                FieldSpec {
                    key: "configuration",
                    label: "Configuration",
                    help: "Build configuration used during package verification and packing.",
                    kind: FieldKind::Select(CONFIGURATIONS),
                    binding: ArgBinding::FlagValue("--configuration"),
                    required: true,
                    default_value: Some("Release"),
                },
                FieldSpec {
                    key: "version",
                    label: "Version override",
                    help: "Optional package version override. Leave empty to use dhara.config.toml.",
                    kind: FieldKind::Text,
                    binding: ArgBinding::FlagValue("--version"),
                    required: false,
                    default_value: None,
                },
                FieldSpec {
                    key: "source",
                    label: "Source",
                    help: "Optional NuGet source URL override.",
                    kind: FieldKind::Text,
                    binding: ArgBinding::FlagValue("--source"),
                    required: false,
                    default_value: None,
                },
                FieldSpec {
                    key: "api_key_env",
                    label: "API key env",
                    help: "Optional environment-variable name containing the NuGet API key.",
                    kind: FieldKind::Text,
                    binding: ArgBinding::FlagValue("--api-key-env"),
                    required: false,
                    default_value: None,
                },
                FieldSpec {
                    key: "mode",
                    label: "Mode",
                    help: "Choose whether to publish or perform a dry run only.",
                    kind: FieldKind::Select(DRY_RUN_OPTIONS),
                    binding: ArgBinding::FlagValue("__mode"),
                    required: true,
                    default_value: Some("dry-run"),
                },
            ],
            quick_run: false,
            supports_cancel: true,
        },
        "release.run" => CommandUi {
            description: "Run the Cargo-first release workflow, with optional NuGet publishing.",
            fields: vec![
                FieldSpec {
                    key: "configuration",
                    label: "Configuration",
                    help: "Build configuration used when NuGet packaging is enabled.",
                    kind: FieldKind::Select(CONFIGURATIONS),
                    binding: ArgBinding::FlagValue("--configuration"),
                    required: true,
                    default_value: Some("Release"),
                },
                FieldSpec {
                    key: "source",
                    label: "Source",
                    help: "Optional NuGet source URL override.",
                    kind: FieldKind::Text,
                    binding: ArgBinding::FlagValue("--source"),
                    required: false,
                    default_value: None,
                },
                FieldSpec {
                    key: "api_key_env",
                    label: "API key env",
                    help: "Optional environment-variable name containing the NuGet API key.",
                    kind: FieldKind::Text,
                    binding: ArgBinding::FlagValue("--api-key-env"),
                    required: false,
                    default_value: None,
                },
                FieldSpec {
                    key: "dry_run",
                    label: "Dry run",
                    help: "Run Cargo and NuGet release validation without publishing.",
                    kind: FieldKind::Boolean,
                    binding: ArgBinding::Switch("--dry-run"),
                    required: false,
                    default_value: Some("false"),
                },
                FieldSpec {
                    key: "skip_cargo",
                    label: "Skip Cargo",
                    help: "Skip the Cargo release phase when crates were already published.",
                    kind: FieldKind::Boolean,
                    binding: ArgBinding::Switch("--skip-cargo"),
                    required: false,
                    default_value: Some("false"),
                },
                FieldSpec {
                    key: "skip_nuget",
                    label: "Skip NuGet",
                    help: "Publish or dry-run only the Cargo release.",
                    kind: FieldKind::Boolean,
                    binding: ArgBinding::Switch("--skip-nuget"),
                    required: false,
                    default_value: Some("false"),
                },
            ],
            quick_run: false,
            supports_cancel: true,
        },
        _ => CommandUi {
            description: summary,
            fields: {
                let _ = args_summary;
                Vec::new()
            },
            quick_run: false,
            supports_cancel: false,
        },
    }
}

fn quick_command(description: &'static str, supports_cancel: bool) -> CommandUi {
    CommandUi {
        description,
        fields: Vec::new(),
        quick_run: true,
        supports_cancel,
    }
}

fn package_command(description: &'static str) -> CommandUi {
    CommandUi {
        description,
        fields: vec![
            FieldSpec {
                key: "configuration",
                label: "Configuration",
                help: "Build configuration used for verification and packing.",
                kind: FieldKind::Select(CONFIGURATIONS),
                binding: ArgBinding::FlagValue("--configuration"),
                required: true,
                default_value: Some("Release"),
            },
            FieldSpec {
                key: "version",
                label: "Version override",
                help: "Optional package version override. Leave empty to use dhara.config.toml.",
                kind: FieldKind::Text,
                binding: ArgBinding::FlagValue("--version"),
                required: false,
                default_value: None,
            },
        ],
        quick_run: true,
        supports_cancel: true,
    }
}

fn required_path(
    key: &'static str,
    label: &'static str,
    help: &'static str,
    flag: &'static str,
) -> FieldSpec {
    FieldSpec {
        key,
        label,
        help,
        kind: FieldKind::Path,
        binding: ArgBinding::FlagValue(flag),
        required: true,
        default_value: None,
    }
}

fn optional_path(
    key: &'static str,
    label: &'static str,
    help: &'static str,
    flag: &'static str,
) -> FieldSpec {
    FieldSpec {
        key,
        label,
        help,
        kind: FieldKind::Path,
        binding: ArgBinding::FlagValue(flag),
        required: false,
        default_value: None,
    }
}

#[cfg(test)]
mod tests {
    use crate::command::{CommandRegistry, ToolCapability};

    use super::DharaStorageCapability;

    #[test]
    fn registration_adds_expected_sections_and_commands() {
        let mut registry = CommandRegistry::new();
        DharaStorageCapability.register(&mut registry);

        let sections = registry
            .sections()
            .map(|section| section.name)
            .collect::<Vec<_>>();
        assert_eq!(
            sections,
            vec!["config", "defs", "package", "release", "verify", "version"]
        );

        let commands = registry
            .commands()
            .map(|command| command.id)
            .collect::<Vec<_>>();
        assert!(commands.contains(&"config.show"));
        assert!(commands.contains(&"defs.inspect-trid-xml"));
        assert!(commands.contains(&"verify.package"));
        assert!(commands.contains(&"release.publish"));
        assert!(commands.contains(&"release.run"));
        assert!(
            registry
                .commands()
                .all(|command| !command.ui.description.trim().is_empty())
        );
    }
}
