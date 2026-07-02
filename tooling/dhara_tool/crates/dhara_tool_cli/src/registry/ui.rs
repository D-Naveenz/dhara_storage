use crate::command::{ArgBinding, CommandUi, FieldKind, FieldSpec};

pub(crate) const VERSION_PARTS: &[&str] = &["major", "minor", "patch"];
pub(crate) const CONFIGURATIONS: &[&str] = &["Release"];
pub(crate) const DRY_RUN_OPTIONS: &[&str] = &["dry-run", "execute"];

pub(crate) fn ui_for_command(
    id: &'static str,
    summary: &'static str,
    args_summary: &'static str,
) -> CommandUi {
    match id {
        "config.show" => quick_command(
            "Inspect the effective Dhara repository configuration and resolved environment.",
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
            description: "Refresh the runtime filedefs.dat package in dhara_storage_dal/resources from the builder source.",
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
        "verify.package" => package_command(
            "Pack and verify the Dhara.Storage NuGet package, including smoke-consumer validation.",
        ),
        "package.pack" => {
            package_command("Pack the Dhara.Storage NuGet package with staged native assets.")
        }
        "package.publish" => CommandUi {
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
