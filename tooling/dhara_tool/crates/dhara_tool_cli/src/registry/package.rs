use crate::command::SectionSpec;
use crate::commands::{
    native_merge_command, package_pack_command, package_publish_command,
    package_stage_native_command, verify_package_command,
};

use super::{command, RegisteredCommand};

pub fn native_section() -> SectionSpec {
    SectionSpec {
        name: "native",
        prompt: "dhara:native> ",
        summary: "Native asset staging helpers",
    }
}

pub fn verify_section() -> SectionSpec {
    SectionSpec {
        name: "verify",
        prompt: "dhara:verify> ",
        summary: "Verification commands",
    }
}

pub fn section() -> SectionSpec {
    SectionSpec {
        name: "package",
        prompt: "dhara:package> ",
        summary: "NuGet packaging commands",
    }
}

pub fn commands() -> Vec<RegisteredCommand> {
    vec![
        command(
            "native.merge",
            &["native", "merge"],
            "Merge per-OS native stage trees",
            "--output <path> --input <path>...",
            "native",
            native_merge_command,
        ),
        command(
            "verify.package",
            &["verify", "package"],
            "Pack and verify the NuGet package",
            "[--configuration <name>] [--version <semver>] [--native-stage <path>]",
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
            "package.stage-native",
            &["package", "stage-native"],
            "Stage host-buildable native libraries",
            "[--configuration <name>] [--msvc-env]",
            "package",
            package_stage_native_command,
        ),
        command(
            "package.publish",
            &["package", "publish"],
            "Verify and optionally publish the NuGet package",
            "[--configuration <name>] [--version <semver>] [--source <url>] [--api-key-env <name>] [--dry-run|--execute]",
            "package",
            package_publish_command,
        ),
    ]
}
