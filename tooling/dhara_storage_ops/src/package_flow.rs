use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use tracing::{debug, info};

use crate::{
    CommandResult, DharaRepoConfig, inspect_package_entries, load_env, run_command,
    run_command_expect_failure, run_command_with_env_redacted, sync, verify_release,
    write_nuget_config,
};

#[derive(Debug, Clone)]
pub struct PackageOptions {
    pub configuration: String,
    pub version_override: Option<String>,
    pub source_override: Option<String>,
    pub api_key_env_override: Option<String>,
    pub output_dir: Option<PathBuf>,
    pub execute_publish: bool,
}

pub fn pack(
    repo_root: &Path,
    config: &DharaRepoConfig,
    options: &PackageOptions,
) -> Result<CommandResult> {
    info!(
        target: "dhara_storage_ops::package_flow",
        configuration = %options.configuration,
        output_dir = options.output_dir.as_ref().map(|path| path.display().to_string()).unwrap_or_default(),
        version_override = options.version_override.as_deref().unwrap_or(""),
        "packing NuGet package"
    );
    verify_release(repo_root)?;
    sync(repo_root)?;

    let version = effective_version(config, &options.version_override);
    let working_root = working_root(repo_root, options.output_dir.as_ref())?;
    let native_stage_root = working_root.join("native-stage");
    let nuget_output = working_root.join("nuget");
    reset_directory(&native_stage_root)?;
    reset_directory(&nuget_output)?;

    stage_native_assets(repo_root, config, options, &native_stage_root)?;

    run_command(
        "dotnet",
        &[
            "pack".to_owned(),
            config.ci.package_project.clone(),
            "--configuration".to_owned(),
            options.configuration.clone(),
            "--include-symbols".to_owned(),
            "-p:ContinuousIntegrationBuild=true".to_owned(),
            "-p:Platform=AnyCPU".to_owned(),
            "-p:PlatformTarget=AnyCPU".to_owned(),
            format!("-p:Version={version}"),
            format!("-p:StagedNativeRoot={}", native_stage_root.display()),
            "--output".to_owned(),
            nuget_output.display().to_string(),
        ],
        repo_root,
    )?;

    let package_path = nuget_output.join(format!("{}.{}.nupkg", config.nuget.package_id, version));
    inspect_package_contents(&package_path, config)?;
    info!(
        target: "dhara_storage_ops::package_flow",
        package_path = %package_path.display(),
        "completed NuGet pack flow"
    );

    Ok(CommandResult::with_message(format!(
        "Packed {}",
        package_path.display()
    )))
}

pub fn verify(
    repo_root: &Path,
    config: &DharaRepoConfig,
    options: &PackageOptions,
) -> Result<CommandResult> {
    info!(
        target: "dhara_storage_ops::package_flow",
        configuration = %options.configuration,
        "verifying NuGet package"
    );
    pack(repo_root, config, options)?;

    let version = effective_version(config, &options.version_override);
    let working_root = working_root(repo_root, options.output_dir.as_ref())?;
    let package_path = working_root
        .join("nuget")
        .join(format!("{}.{}.nupkg", config.nuget.package_id, version));
    let local_config = working_root.join("local-package.nuget.config");
    let dependency_source = effective_source(repo_root, config, options)?;
    write_nuget_config(
        &local_config,
        &[
            package_path
                .parent()
                .context("package path should have a parent")?
                .to_path_buf(),
            PathBuf::from(&dependency_source),
        ],
    )?;

    restore_smoke_consumer(
        repo_root,
        config,
        &version,
        &local_config,
        Some(&config.ci.host_runtime_smoke),
        false,
    )?;
    run_smoke_consumer(repo_root, config, &version)?;
    verify_unsupported_runtime_rejected(repo_root, config, &version, &local_config)?;
    restore_smoke_consumer(
        repo_root,
        config,
        &version,
        &local_config,
        Some(&config.ci.aot_runtime_smoke),
        true,
    )?;
    publish_aot_smoke_consumer(
        repo_root,
        config,
        &version,
        &working_root.join("smoke-aot"),
        &config.ci.aot_runtime_smoke,
    )?;
    info!(
        target: "dhara_storage_ops::package_flow",
        package_path = %package_path.display(),
        "completed NuGet verification flow"
    );
    Ok(CommandResult::with_message(
        "Package verified successfully.",
    ))
}

pub fn publish(
    repo_root: &Path,
    config: &DharaRepoConfig,
    options: &PackageOptions,
) -> Result<CommandResult> {
    info!(
        target: "dhara_storage_ops::package_flow",
        execute_publish = options.execute_publish,
        "publishing NuGet package"
    );
    verify(repo_root, config, options)?;

    if !options.execute_publish {
        return Ok(CommandResult::with_message(
            "Dry run complete. Package was verified but not published.",
        ));
    }

    let version = effective_version(config, &options.version_override);
    let source = effective_source(repo_root, config, options)?;
    let api_key_env = options
        .api_key_env_override
        .clone()
        .unwrap_or_else(|| config.publish.api_key_env.clone());
    let api_key = secret_from_env(repo_root, &api_key_env)?;

    let working_root = working_root(repo_root, options.output_dir.as_ref())?;
    let package_path = working_root
        .join("nuget")
        .join(format!("{}.{}.nupkg", config.nuget.package_id, version));

    run_command_with_env_redacted(
        "dotnet",
        &[
            "nuget".to_owned(),
            "push".to_owned(),
            package_path.display().to_string(),
            "--api-key".to_owned(),
            api_key.clone(),
            "--source".to_owned(),
            source.clone(),
            "--skip-duplicate".to_owned(),
        ],
        repo_root,
        &[],
        &[api_key.as_str()],
    )?;

    info!(
        target: "dhara_storage_ops::package_flow",
        package_path = %package_path.display(),
        source = %source,
        "published NuGet package after local smoke verification"
    );

    Ok(CommandResult::with_message(
        "Published package successfully.",
    ))
}

pub fn publish_packed(
    repo_root: &Path,
    config: &DharaRepoConfig,
    options: &PackageOptions,
) -> Result<CommandResult> {
    info!(
        target: "dhara_storage_ops::package_flow",
        "publishing pre-packed NuGet package"
    );

    let version = effective_version(config, &options.version_override);
    let source = effective_source(repo_root, config, options)?;
    let api_key_env = options
        .api_key_env_override
        .clone()
        .unwrap_or_else(|| config.publish.api_key_env.clone());
    let api_key = secret_from_env(repo_root, &api_key_env)?;

    let working_root = working_root(repo_root, options.output_dir.as_ref())?;
    let package_path = working_root
        .join("nuget")
        .join(format!("{}.{}.nupkg", config.nuget.package_id, version));
    if !package_path.exists() {
        bail!("NuGet package does not exist: {}", package_path.display());
    }

    run_command_with_env_redacted(
        "dotnet",
        &[
            "nuget".to_owned(),
            "push".to_owned(),
            package_path.display().to_string(),
            "--api-key".to_owned(),
            api_key.clone(),
            "--source".to_owned(),
            source.clone(),
            "--skip-duplicate".to_owned(),
        ],
        repo_root,
        &[],
        &[api_key.as_str()],
    )?;

    info!(
        target: "dhara_storage_ops::package_flow",
        package_path = %package_path.display(),
        source = %source,
        "published pre-packed NuGet package"
    );

    Ok(CommandResult::with_message(
        "Published package successfully.",
    ))
}

fn stage_native_assets(
    repo_root: &Path,
    config: &DharaRepoConfig,
    options: &PackageOptions,
    stage_root: &Path,
) -> Result<()> {
    let profile_flag = if options.configuration.eq_ignore_ascii_case("Release") {
        "--release"
    } else {
        bail!("only Release packaging is currently supported");
    };

    for rid in &config.ci.native_runtimes {
        let target = config
            .targets
            .rust_targets
            .get(rid)
            .with_context(|| format!("missing rust target mapping for runtime '{rid}'"))?;
        debug!(
            target: "dhara_storage_ops::package_flow",
            runtime = %rid,
            rust_target = %target,
            stage_root = %stage_root.display(),
            "staging native asset"
        );
        run_command(
            "cargo",
            &[
                "build".to_owned(),
                "-p".to_owned(),
                "dharastorage".to_owned(),
                profile_flag.to_owned(),
                "--target".to_owned(),
                target.clone(),
            ],
            repo_root,
        )?;

        let source_path = repo_root
            .join("target")
            .join(target)
            .join("release")
            .join("dharastorage.dll");
        let destination_path = stage_root
            .join("runtimes")
            .join(rid)
            .join("native")
            .join("dharastorage.dll");
        if let Some(parent) = destination_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        fs::copy(&source_path, &destination_path).with_context(|| {
            format!(
                "failed to copy native asset from '{}' to '{}'",
                source_path.display(),
                destination_path.display()
            )
        })?;
    }

    Ok(())
}

fn inspect_package_contents(package_path: &Path, config: &DharaRepoConfig) -> Result<()> {
    let entries = inspect_package_entries(package_path)?;
    debug!(
        target: "dhara_storage_ops::package_flow",
        package_path = %package_path.display(),
        entry_count = entries.len(),
        "inspecting package contents"
    );
    if !entries
        .iter()
        .any(|entry| entry == "lib/net10.0/Dhara.Storage.dll")
    {
        bail!("managed assembly missing from package");
    }

    for rid in &config.ci.native_runtimes {
        let expected = format!("runtimes/{rid}/native/dharastorage.dll");
        if !entries.iter().any(|entry| entry == &expected) {
            bail!("native asset missing from package: {expected}");
        }
    }
    if !entries.iter().any(|entry| entry == "README.md") {
        bail!("README.md missing from package");
    }
    if let Some(icon) = &config.nuget.icon {
        let icon_name = Path::new(icon)
            .file_name()
            .and_then(|value| value.to_str())
            .with_context(|| format!("package icon path must end with a file name: {icon}"))?;
        if !entries.iter().any(|entry| entry == icon_name) {
            bail!("package icon missing from package: {icon_name}");
        }
    }
    if !entries
        .iter()
        .any(|entry| entry == "build/Dhara.Storage.targets")
    {
        bail!("build/Dhara.Storage.targets missing from package");
    }
    Ok(())
}

fn restore_smoke_consumer(
    repo_root: &Path,
    config: &DharaRepoConfig,
    version: &str,
    nuget_config: &Path,
    runtime: Option<&str>,
    publish_aot: bool,
) -> Result<()> {
    info!(
        target: "dhara_storage_ops::package_flow",
        project = %config.ci.smoke_project,
        version,
        runtime = runtime.unwrap_or("default"),
        publish_aot,
        "restoring smoke consumer"
    );
    remove_package_cache(repo_root, &config.nuget.package_id, version)?;
    reset_smoke_consumer_outputs(repo_root, config)?;
    let mut args = vec![
        "restore".to_owned(),
        config.ci.smoke_project.clone(),
        format!("-p:DharaStoragePackageVersion={version}"),
        format!("--configfile={}", nuget_config.display()),
        "--force-evaluate".to_owned(),
    ];
    if let Some(runtime) = runtime {
        args.push("--runtime".to_owned());
        args.push(runtime.to_owned());
        args.push(format!("-p:Platform={}", platform(runtime)?));
        args.push(format!("-p:PlatformTarget={}", platform_target(runtime)?));
    }
    if publish_aot {
        args.push("-p:PublishAot=true".to_owned());
    }
    run_command("dotnet", &args, repo_root)
}

fn run_smoke_consumer(repo_root: &Path, config: &DharaRepoConfig, version: &str) -> Result<()> {
    info!(
        target: "dhara_storage_ops::package_flow",
        project = %config.ci.smoke_project,
        version,
        "running smoke consumer"
    );
    run_command(
        "dotnet",
        &[
            "run".to_owned(),
            "--project".to_owned(),
            config.ci.smoke_project.clone(),
            "--configuration".to_owned(),
            "Release".to_owned(),
            "--runtime".to_owned(),
            config.ci.host_runtime_smoke.clone(),
            format!("-p:Platform={}", platform(&config.ci.host_runtime_smoke)?),
            format!(
                "-p:PlatformTarget={}",
                platform_target(&config.ci.host_runtime_smoke)?
            ),
            "--no-restore".to_owned(),
            format!("-p:DharaStoragePackageVersion={version}"),
        ],
        repo_root,
    )
}

fn verify_unsupported_runtime_rejected(
    repo_root: &Path,
    config: &DharaRepoConfig,
    version: &str,
    nuget_config: &Path,
) -> Result<()> {
    info!(
        target: "dhara_storage_ops::package_flow",
        project = %config.ci.smoke_project,
        version,
        unsupported_runtime = "win-x86",
        "verifying unsupported runtime rejection"
    );
    remove_package_cache(repo_root, &config.nuget.package_id, version)?;
    run_command_expect_failure(
        "dotnet",
        &[
            "build".to_owned(),
            config.ci.smoke_project.clone(),
            "--configuration".to_owned(),
            "Release".to_owned(),
            "--runtime".to_owned(),
            "win-x86".to_owned(),
            "-p:Platform=x86".to_owned(),
            "-p:PlatformTarget=x86".to_owned(),
            format!("--configfile={}", nuget_config.display()),
            format!("-p:DharaStoragePackageVersion={version}"),
        ],
        repo_root,
        "does not support 32-bit runtime identifier",
    )
}

fn publish_aot_smoke_consumer(
    repo_root: &Path,
    config: &DharaRepoConfig,
    version: &str,
    output_dir: &Path,
    runtime: &str,
) -> Result<()> {
    info!(
        target: "dhara_storage_ops::package_flow",
        project = %config.ci.smoke_project,
        version,
        runtime,
        output_dir = %output_dir.display(),
        "publishing AOT smoke consumer"
    );
    reset_directory(output_dir)?;
    run_command(
        "dotnet",
        &[
            "publish".to_owned(),
            config.ci.smoke_project.clone(),
            "--configuration".to_owned(),
            "Release".to_owned(),
            "--runtime".to_owned(),
            runtime.to_owned(),
            format!("-p:Platform={}", platform(runtime)?),
            format!("-p:PlatformTarget={}", platform_target(runtime)?),
            "--self-contained".to_owned(),
            "true".to_owned(),
            "--no-restore".to_owned(),
            "-p:PublishAot=true".to_owned(),
            format!("-p:DharaStoragePackageVersion={version}"),
            "--output".to_owned(),
            output_dir.display().to_string(),
        ],
        repo_root,
    )?;

    let executable = output_dir.join("Dhara.Storage.ConsumerSmoke.exe");
    run_command(
        executable
            .to_str()
            .context("published smoke consumer path was not valid utf-8")?,
        &[],
        repo_root,
    )
}

fn remove_package_cache(repo_root: &Path, package_id: &str, version: &str) -> Result<()> {
    let mut package_roots = Vec::new();
    if let Some(path) = std::env::var_os("NUGET_PACKAGES") {
        package_roots.push(PathBuf::from(path));
    }
    if let Some(path) = std::env::var_os("DOTNET_CLI_HOME") {
        package_roots.push(PathBuf::from(path).join(".nuget").join("packages"));
    }
    if let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) {
        package_roots.push(PathBuf::from(home).join(".nuget").join("packages"));
    }
    package_roots.push(repo_root.join(".dotnet").join(".nuget").join("packages"));

    package_roots.sort();
    package_roots.dedup();

    for package_root in package_roots {
        let package_path = package_root
            .join(package_id.to_ascii_lowercase())
            .join(version);
        if package_path.exists() {
            fs::remove_dir_all(&package_path).with_context(|| {
                format!(
                    "failed to remove stale package cache at {}",
                    package_path.display()
                )
            })?;
        }
    }
    Ok(())
}

fn reset_smoke_consumer_outputs(repo_root: &Path, config: &DharaRepoConfig) -> Result<()> {
    let project_path = repo_root.join(&config.ci.smoke_project);
    let project_dir = project_path.parent().with_context(|| {
        format!(
            "smoke project path must have a parent: {}",
            project_path.display()
        )
    })?;
    for directory in ["bin", "obj"] {
        let path = project_dir.join(directory);
        if path.exists() {
            fs::remove_dir_all(&path)
                .with_context(|| format!("failed to remove {}", path.display()))?;
        }
    }
    Ok(())
}

fn platform(runtime: &str) -> Result<&'static str> {
    match runtime {
        "win-x64" => Ok("x64"),
        "win-arm64" => Ok("ARM64"),
        "win-x86" => Ok("x86"),
        _ => bail!("unsupported runtime for Platform inference: {runtime}"),
    }
}

fn platform_target(runtime: &str) -> Result<&'static str> {
    match runtime {
        "win-x64" => Ok("x64"),
        "win-arm64" => Ok("arm64"),
        "win-x86" => Ok("x86"),
        _ => bail!("unsupported runtime for PlatformTarget inference: {runtime}"),
    }
}

fn effective_version(config: &DharaRepoConfig, override_value: &Option<String>) -> String {
    override_value
        .clone()
        .unwrap_or_else(|| config.versions.workspace.clone())
}

fn effective_source(
    repo_root: &Path,
    config: &DharaRepoConfig,
    options: &PackageOptions,
) -> Result<String> {
    if let Some(source) = options.source_override.clone().and_then(non_empty_option) {
        return Ok(source);
    }

    if let Some(source) = env_file_value(repo_root, "NUGET_SOURCE")? {
        return Ok(source);
    }

    if let Ok(source) = std::env::var("NUGET_SOURCE")
        && !source.trim().is_empty()
    {
        return Ok(source);
    }

    Ok(config.nuget.source.clone())
}

fn secret_from_env(repo_root: &Path, key: &str) -> Result<String> {
    if let Some(value) = env_file_value(repo_root, key)? {
        return Ok(value);
    }

    std::env::var(key)
        .ok()
        .and_then(non_empty_option)
        .with_context(|| format!("{key} is not set in the environment or .env.local"))
}

fn env_file_value(repo_root: &Path, key: &str) -> Result<Option<String>> {
    Ok(load_env(repo_root)?.remove(key).and_then(non_empty_option))
}

fn non_empty_option(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

fn working_root(repo_root: &Path, override_value: Option<&PathBuf>) -> Result<PathBuf> {
    let root = override_value
        .cloned()
        .unwrap_or_else(|| repo_root.join(".artifacts").join("dhara_tool"));
    fs::create_dir_all(&root).with_context(|| format!("failed to create {}", root.display()))?;
    Ok(root)
}

fn reset_directory(path: &Path) -> Result<()> {
    if path.exists() {
        fs::remove_dir_all(path).with_context(|| format!("failed to remove {}", path.display()))?;
    }
    fs::create_dir_all(path).with_context(|| format!("failed to create {}", path.display()))?;
    Ok(())
}
