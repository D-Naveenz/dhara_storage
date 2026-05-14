use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use toml_edit::DocumentMut;
use tracing::info;
use xmltree::{Element, XMLNode};

use crate::{
    CommandResult, DharaRepoConfig, PackageOptions, load_env, package_flow, run_command,
    verify_release,
};

const CARGO_REGISTRY_TOKEN_ENV: &str = "CARGO_REGISTRY_TOKEN";

#[derive(Debug, Clone)]
pub struct ReleaseOptions {
    pub configuration: String,
    pub source_override: Option<String>,
    pub api_key_env_override: Option<String>,
    pub output_dir: Option<PathBuf>,
    pub dry_run: bool,
    pub publish_nuget: bool,
}

pub fn run(
    repo_root: &Path,
    config: &DharaRepoConfig,
    options: &ReleaseOptions,
) -> Result<CommandResult> {
    info!(
        target: "dhara_tool_dhara_storage::release_flow",
        dry_run = options.dry_run,
        publish_nuget = options.publish_nuget,
        "running release flow"
    );

    verify_release(repo_root)?;
    validate_versions_synced(repo_root, config)?;
    ensure_release_secrets(repo_root, config, options)?;

    run_cargo_release(repo_root, options.dry_run)?;

    if !options.publish_nuget {
        return Ok(CommandResult::with_message(if options.dry_run {
            "Cargo release dry run completed. NuGet release was skipped."
        } else {
            "Cargo release completed. NuGet release was skipped."
        }));
    }

    let package_options = PackageOptions {
        configuration: options.configuration.clone(),
        version_override: None,
        source_override: options.source_override.clone(),
        api_key_env_override: options.api_key_env_override.clone(),
        output_dir: options.output_dir.clone(),
        execute_publish: false,
    };

    if options.dry_run {
        package_flow::publish(repo_root, config, &package_options)?;
        return Ok(CommandResult::with_message(
            "Cargo and NuGet release dry run completed.",
        ));
    }

    package_flow::pack(repo_root, config, &package_options)?;
    package_flow::publish_packed(repo_root, config, &package_options)?;

    Ok(CommandResult::with_message(
        "Cargo and NuGet release completed successfully.",
    ))
}

fn run_cargo_release(repo_root: &Path, dry_run: bool) -> Result<()> {
    let mut args = vec![
        "release".to_owned(),
        "--workspace".to_owned(),
        "--isolated".to_owned(),
        "--allow-branch".to_owned(),
        if dry_run { "HEAD" } else { "main" }.to_owned(),
        "--tag-name".to_owned(),
        "v{{version}}".to_owned(),
        "--no-confirm".to_owned(),
    ];

    if dry_run {
        args.push("--no-verify".to_owned());
    } else {
        args.push("--execute".to_owned());
    }

    run_command("cargo", &args, repo_root)
}

fn ensure_release_secrets(
    repo_root: &Path,
    config: &DharaRepoConfig,
    options: &ReleaseOptions,
) -> Result<()> {
    if options.dry_run {
        return Ok(());
    }

    ensure_secret(repo_root, CARGO_REGISTRY_TOKEN_ENV)?;
    if options.publish_nuget {
        let nuget_key = options
            .api_key_env_override
            .as_deref()
            .unwrap_or(&config.publish.api_key_env);
        ensure_secret(repo_root, nuget_key)?;
    }
    Ok(())
}

fn ensure_secret(repo_root: &Path, key: &str) -> Result<()> {
    if std::env::var(key)
        .ok()
        .is_some_and(|value| !value.trim().is_empty())
    {
        return Ok(());
    }

    if load_env(repo_root)?
        .get(key)
        .is_some_and(|value| !value.trim().is_empty())
    {
        return Ok(());
    }

    bail!("{key} is required for release execution");
}

fn validate_versions_synced(repo_root: &Path, config: &DharaRepoConfig) -> Result<()> {
    let expected = config.versions.workspace.as_str();
    let cargo_path = repo_root.join("Cargo.toml");
    let cargo_content = fs::read_to_string(&cargo_path)
        .with_context(|| format!("failed to read {}", cargo_path.display()))?;
    let cargo = cargo_content
        .parse::<DocumentMut>()
        .context("failed to parse Cargo.toml")?;

    require_toml_version(
        &cargo,
        &["workspace", "package", "version"],
        expected,
        "workspace.package.version",
    )?;
    for dependency in ["dhara_dhbin", "dhara_storage", "dhara_tool_dhara_storage"] {
        require_toml_version(
            &cargo,
            &["workspace", "dependencies", dependency, "version"],
            expected,
            &format!("workspace.dependencies.{dependency}.version"),
        )?;
    }

    let csproj_path = repo_root.join(&config.ci.package_project);
    let csproj_content = fs::read_to_string(&csproj_path)
        .with_context(|| format!("failed to read {}", csproj_path.display()))?;
    let project =
        Element::parse(csproj_content.as_bytes()).context("failed to parse package csproj")?;
    let actual_csproj_version = find_property(&project, "Version")
        .with_context(|| format!("Version property missing from {}", csproj_path.display()))?;
    if actual_csproj_version.trim() != expected {
        bail!(
            "package csproj Version is {}, expected {}",
            actual_csproj_version,
            expected
        );
    }

    Ok(())
}

fn require_toml_version(
    document: &DocumentMut,
    path: &[&str],
    expected: &str,
    label: &str,
) -> Result<()> {
    let mut item = document.as_item();
    for segment in path {
        item = item
            .get(segment)
            .with_context(|| format!("{label} is missing"))?;
    }

    let actual = item
        .as_str()
        .with_context(|| format!("{label} must be a string"))?;
    if actual != expected {
        bail!("{label} is {actual}, expected {expected}");
    }

    Ok(())
}

fn find_property(project: &Element, name: &str) -> Option<String> {
    for child in &project.children {
        let XMLNode::Element(group) = child else {
            continue;
        };
        if group.name != "PropertyGroup" {
            continue;
        }
        for item in &group.children {
            let XMLNode::Element(property) = item else {
                continue;
            };
            if property.name == name {
                return property.get_text().map(|value| value.into_owned());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;
    use crate::{
        CiConfig, NuGetConfig, PublishConfig, TargetsConfig, VersionConfig, config::DharaRepoConfig,
    };

    fn sample_config() -> DharaRepoConfig {
        DharaRepoConfig {
            versions: VersionConfig {
                workspace: "0.5.0".to_owned(),
            },
            nuget: NuGetConfig {
                package_id: "Dhara.Storage".to_owned(),
                source: "https://api.nuget.org/v3/index.json".to_owned(),
                authors: vec!["Naveen Dharmathunga".to_owned()],
                description: "Dhara Storage".to_owned(),
                tags: vec!["storage".to_owned()],
                readme: "bindings/dotnet/Dhara.Storage/README.md".to_owned(),
                icon: None,
                repository_url: "https://github.com/D-Naveenz/dhara_storage".to_owned(),
                project_url: "https://github.com/D-Naveenz/dhara_storage".to_owned(),
            },
            ci: CiConfig {
                smoke_project:
                    "bindings/dotnet/Dhara.Storage.ConsumerSmoke/Dhara.Storage.ConsumerSmoke.csproj"
                        .to_owned(),
                package_project: "bindings/dotnet/Dhara.Storage/Dhara.Storage.csproj".to_owned(),
                tests_project: "bindings/dotnet/Dhara.Storage.Tests/Dhara.Storage.Tests.csproj"
                    .to_owned(),
                native_runtimes: vec!["win-x64".to_owned()],
                host_runtime_smoke: "win-x64".to_owned(),
                aot_runtime_smoke: "win-x64".to_owned(),
            },
            publish: PublishConfig {
                environment: "nuget-production".to_owned(),
                api_key_env: "NUGET_API_KEY".to_owned(),
            },
            targets: TargetsConfig {
                rust_targets: [("win-x64".to_owned(), "x86_64-pc-windows-msvc".to_owned())]
                    .into_iter()
                    .collect(),
            },
        }
    }

    fn write_version_files(repo_root: &Path, cargo_version: &str, csproj_version: &str) {
        fs::write(
            repo_root.join("Cargo.toml"),
            format!(
                r#"[workspace]
[workspace.package]
version = "{cargo_version}"
[workspace.dependencies]
dhara_dhbin = {{ version = "{cargo_version}", path = "dhara_dhbin" }}
dhara_storage = {{ version = "{cargo_version}", path = "dhara_storage" }}
dhara_tool_dhara_storage = {{ version = "{cargo_version}", path = "tooling/dhara_tool_dhara_storage" }}
"#
            ),
        )
        .unwrap();
        fs::create_dir_all(repo_root.join("bindings/dotnet/Dhara.Storage")).unwrap();
        fs::write(
            repo_root.join("bindings/dotnet/Dhara.Storage/Dhara.Storage.csproj"),
            format!(
                r#"<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><Version>{csproj_version}</Version></PropertyGroup></Project>"#
            ),
        )
        .unwrap();
    }

    #[test]
    fn validate_versions_synced_accepts_matching_metadata() {
        let temp = tempdir().unwrap();
        let config = sample_config();
        write_version_files(temp.path(), "0.5.0", "0.5.0");

        validate_versions_synced(temp.path(), &config).unwrap();
    }

    #[test]
    fn validate_versions_synced_rejects_mismatched_metadata() {
        let temp = tempdir().unwrap();
        let config = sample_config();
        write_version_files(temp.path(), "0.5.0", "0.4.4");

        let error = validate_versions_synced(temp.path(), &config).unwrap_err();

        assert!(error.to_string().contains("package csproj Version"));
    }

    #[test]
    fn execute_requires_publish_secret() {
        let temp = tempdir().unwrap();
        let mut config = sample_config();
        config.publish.api_key_env = "DHARA_TOOL_TEST_MISSING_NUGET_KEY".to_owned();

        let error = ensure_secret(temp.path(), &config.publish.api_key_env).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("DHARA_TOOL_TEST_MISSING_NUGET_KEY")
        );
    }
}
