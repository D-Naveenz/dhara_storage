use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use semver::Version;
use serde::{Deserialize, Serialize};
use toml_edit::{DocumentMut, value};
use xmltree::{Element, XMLNode};

pub const CONFIG_PATH: &str = "dhara.config.toml";
pub const ENV_EXAMPLE_PATH: &str = ".env.example";
pub const ENV_LOCAL_PATH: &str = ".env.local";
pub const ROOT_CARGO_TOML_PATH: &str = "Cargo.toml";
pub const TOOL_CARGO_TOML_PATH: &str = "tooling/dhara_tool/Cargo.toml";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DharaRepoConfig {
    pub versions: VersionConfig,
    pub tool: ToolConfig,
    pub nuget: NuGetConfig,
    pub ci: CiConfig,
    pub publish: PublishConfig,
    pub targets: TargetsConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionConfig {
    #[serde(alias = "rust_workspace", alias = "nuget_package")]
    pub workspace: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolConfig {
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NuGetConfig {
    pub package_id: String,
    pub source: String,
    pub authors: Vec<String>,
    pub description: String,
    pub tags: Vec<String>,
    pub readme: String,
    #[serde(default)]
    pub icon: Option<String>,
    pub repository_url: String,
    pub project_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CiConfig {
    pub smoke_project: String,
    pub package_project: String,
    pub tests_project: String,
    pub native_runtimes: Vec<String>,
    pub host_runtime_smoke: String,
    pub aot_runtime_smoke: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublishConfig {
    pub environment: String,
    pub api_key_env: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetsConfig {
    pub rust_targets: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ShowOutput {
    pub config: DharaRepoConfig,
    pub env: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionPart {
    Major,
    Minor,
    Patch,
}

pub fn load_config(repo_root: &Path) -> Result<DharaRepoConfig> {
    let config_path = repo_root.join(CONFIG_PATH);
    let content = fs::read_to_string(&config_path)
        .with_context(|| format!("failed to read {}", config_path.display()))?;
    toml::from_str(&content).with_context(|| format!("failed to parse {}", config_path.display()))
}

pub fn load_env(repo_root: &Path) -> Result<BTreeMap<String, String>> {
    let env_path = repo_root.join(ENV_LOCAL_PATH);
    if !env_path.exists() {
        return Ok(BTreeMap::new());
    }

    let content = fs::read_to_string(&env_path)
        .with_context(|| format!("failed to read {}", env_path.display()))?;
    parse_env_content(&content)
}

pub fn show(repo_root: &Path) -> Result<String> {
    let output = ShowOutput {
        config: load_config(repo_root)?,
        env: masked_env(load_env(repo_root)?),
    };
    toml::to_string_pretty(&output).context("failed to serialize configuration")
}

pub fn init_env(repo_root: &Path) -> Result<bool> {
    let example_path = repo_root.join(ENV_EXAMPLE_PATH);
    let local_path = repo_root.join(ENV_LOCAL_PATH);
    if local_path.exists() {
        return Ok(false);
    }

    let content = fs::read_to_string(&example_path)
        .with_context(|| format!("failed to read {}", example_path.display()))?;
    fs::write(&local_path, content)
        .with_context(|| format!("failed to write {}", local_path.display()))?;
    Ok(true)
}

pub fn verify_release(repo_root: &Path) -> Result<()> {
    let config = load_config(repo_root)?;
    validate_config(repo_root, &config)
}

/// Kind of manifest drift relative to `dhara.config.toml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConfigDriftKind {
    ToolCrateVersion,
    WorkspaceCargoToml,
    NuGetCsproj,
}

/// One detected drift item shown in activation prompts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigDriftItem {
    pub kind: ConfigDriftKind,
    pub summary: String,
}

/// Returns manifest fields that differ from `dhara.config.toml` (config is truth).
pub fn detect_config_drift(repo_root: &Path) -> Result<Vec<ConfigDriftItem>> {
    let config = load_config(repo_root)?;
    let mut drifts = Vec::new();

    let manifest_tool_version = read_tool_crate_version(repo_root)?;
    if manifest_tool_version != config.tool.version {
        drifts.push(ConfigDriftItem {
            kind: ConfigDriftKind::ToolCrateVersion,
            summary: format!(
                "{TOOL_CARGO_TOML_PATH} package.version: {manifest_tool_version} -> {}",
                config.tool.version
            ),
        });
    }

    let cargo_path = repo_root.join(ROOT_CARGO_TOML_PATH);
    let cargo_content = fs::read_to_string(&cargo_path)
        .with_context(|| format!("failed to read {}", cargo_path.display()))?;
    let updated_cargo = sync_cargo_toml(&cargo_content, &config.versions.workspace)?;
    if updated_cargo != cargo_content {
        drifts.push(ConfigDriftItem {
            kind: ConfigDriftKind::WorkspaceCargoToml,
            summary: format!(
                "{ROOT_CARGO_TOML_PATH} workspace version -> {}",
                config.versions.workspace
            ),
        });
    }

    let csproj_path = repo_root.join(&config.ci.package_project);
    let csproj_content = fs::read_to_string(&csproj_path)
        .with_context(|| format!("failed to read {}", csproj_path.display()))?;
    if csproj_needs_sync(&csproj_content, &config)? {
        drifts.push(ConfigDriftItem {
            kind: ConfigDriftKind::NuGetCsproj,
            summary: format!(
                "dhara.config.toml NuGet metadata -> {}",
                config.ci.package_project
            ),
        });
    }

    Ok(drifts)
}

/// Writes manifest updates for the given drift items (config is truth).
pub fn apply_config_drift(repo_root: &Path, items: &[ConfigDriftItem]) -> Result<()> {
    if items.is_empty() {
        return Ok(());
    }

    let config = load_config(repo_root)?;
    let kinds: std::collections::HashSet<_> = items.iter().map(|item| item.kind).collect();

    if kinds.contains(&ConfigDriftKind::ToolCrateVersion) {
        let path = repo_root.join(TOOL_CARGO_TOML_PATH);
        let content = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let updated = sync_tool_cargo_toml(&content, &config.tool.version)?;
        if updated != content {
            fs::write(&path, updated)
                .with_context(|| format!("failed to write {}", path.display()))?;
        }
    }

    if kinds.contains(&ConfigDriftKind::WorkspaceCargoToml) {
        let path = repo_root.join(ROOT_CARGO_TOML_PATH);
        let content = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let updated = sync_cargo_toml(&content, &config.versions.workspace)?;
        if updated != content {
            fs::write(&path, updated)
                .with_context(|| format!("failed to write {}", path.display()))?;
        }
    }

    if kinds.contains(&ConfigDriftKind::NuGetCsproj) {
        let path = repo_root.join(&config.ci.package_project);
        let content = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let updated = sync_csproj(&content, &config)?;
        if updated != content {
            fs::write(&path, updated)
                .with_context(|| format!("failed to write {}", path.display()))?;
        }
    }

    Ok(())
}

pub fn set_version(repo_root: &Path, version: &str) -> Result<()> {
    let parsed = Version::parse(version).with_context(|| format!("invalid semver: {version}"))?;
    let mut config = load_config(repo_root)?;
    config.versions.workspace = parsed.to_string();
    write_config(repo_root, &config)
}

pub fn bump_version(repo_root: &Path, part: VersionPart) -> Result<String> {
    let mut config = load_config(repo_root)?;
    let current = &config.versions.workspace;
    let mut version =
        Version::parse(current).with_context(|| format!("invalid configured semver: {current}"))?;
    match part {
        VersionPart::Major => {
            version.major += 1;
            version.minor = 0;
            version.patch = 0;
        }
        VersionPart::Minor => {
            version.minor += 1;
            version.patch = 0;
        }
        VersionPart::Patch => {
            version.patch += 1;
        }
    }

    let next = version.to_string();
    config.versions.workspace = next.clone();
    write_config(repo_root, &config)?;
    Ok(next)
}

pub fn sync_cargo_toml(content: &str, version: &str) -> Result<String> {
    Version::parse(version)
        .with_context(|| format!("invalid rust workspace version: {version}"))?;
    let mut document = content
        .parse::<DocumentMut>()
        .context("failed to parse Cargo.toml")?;
    document["workspace"]["package"]["version"] = value(version);
    document["workspace"]["dependencies"]["dhara_storage_dal"]["version"] = value(version);
    document["workspace"]["dependencies"]["dhara_storage"]["version"] = value(version);
    Ok(document.to_string())
}

pub fn sync_tool_cargo_toml(content: &str, version: &str) -> Result<String> {
    Version::parse(version).with_context(|| format!("invalid tool version: {version}"))?;
    let mut document = content
        .parse::<DocumentMut>()
        .context("failed to parse tooling/dhara_tool/Cargo.toml")?;
    document["package"]["version"] = value(version);
    Ok(document.to_string())
}

pub fn read_tool_crate_version(repo_root: &Path) -> Result<String> {
    let path = repo_root.join(TOOL_CARGO_TOML_PATH);
    let content =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let document = content
        .parse::<DocumentMut>()
        .with_context(|| format!("failed to parse {}", path.display()))?;
    let version = document["package"]["version"]
        .as_str()
        .with_context(|| format!("{} is missing package.version", path.display()))?;
    Version::parse(version).with_context(|| format!("invalid tool semver: {version}"))?;
    Ok(version.to_owned())
}

/// NuGet fields owned by `dhara.config.toml` (semantic compare; not full-file text).
#[derive(Debug, Clone, PartialEq, Eq)]
struct ManagedCsprojSnapshot {
    package_id: String,
    version: String,
    description: String,
    package_readme_file: String,
    repository_url: String,
    package_project_url: String,
    authors: String,
    package_tags: String,
    package_icon: Option<String>,
    readme_include: String,
    icon_include: Option<String>,
}

pub fn csproj_needs_sync(content: &str, config: &DharaRepoConfig) -> Result<bool> {
    Ok(managed_csproj_snapshot_from_content(content, config)?
        != managed_csproj_snapshot_from_config(config)?)
}

pub fn sync_csproj(content: &str, config: &DharaRepoConfig) -> Result<String> {
    let expected = managed_csproj_snapshot_from_config(config)?;
    let current = managed_csproj_snapshot_from_content(content, config)?;
    if current == expected {
        return Ok(content.to_owned());
    }

    let mut updated = content.to_owned();
    upsert_property_element(&mut updated, "PackageId", &expected.package_id)?;
    upsert_property_element(&mut updated, "Version", &expected.version)?;
    upsert_property_element(&mut updated, "Description", &expected.description)?;
    upsert_property_element(
        &mut updated,
        "PackageReadmeFile",
        &expected.package_readme_file,
    )?;
    upsert_property_element(&mut updated, "RepositoryUrl", &expected.repository_url)?;
    upsert_property_element(
        &mut updated,
        "PackageProjectUrl",
        &expected.package_project_url,
    )?;
    upsert_property_element(&mut updated, "Authors", &expected.authors)?;
    upsert_property_element(&mut updated, "PackageTags", &expected.package_tags)?;

    match (&expected.package_icon, current.package_icon.as_ref()) {
        (Some(icon), _) => upsert_property_element(&mut updated, "PackageIcon", icon)?,
        (None, Some(_)) => remove_property_element(&mut updated, "PackageIcon"),
        (None, None) => {}
    }

    let readme_file = file_name(&config.nuget.readme)?;
    patch_pack_none_include(
        &mut updated,
        &[config.nuget.readme.as_str(), readme_file],
        &expected.readme_include,
        current.readme_include.as_str(),
    )?;

    if let Some(icon_include) = &expected.icon_include {
        let icon_file = file_name(config.nuget.icon.as_deref().unwrap_or(icon_include))?;
        patch_pack_none_include(
            &mut updated,
            &[
                config.nuget.icon.as_deref().unwrap_or(icon_include),
                icon_file,
            ],
            icon_include,
            current.icon_include.as_deref().unwrap_or(""),
        )?;
    }

    Ok(updated)
}

fn managed_csproj_snapshot_from_config(config: &DharaRepoConfig) -> Result<ManagedCsprojSnapshot> {
    let readme_file = file_name(&config.nuget.readme)?;
    let readme_include =
        project_relative_include(&config.nuget.readme, &config.ci.package_project)?;
    let icon_include = config
        .nuget
        .icon
        .as_deref()
        .map(|icon| project_relative_include(icon, &config.ci.package_project))
        .transpose()?;

    Ok(ManagedCsprojSnapshot {
        package_id: config.nuget.package_id.clone(),
        version: config.versions.workspace.clone(),
        description: config.nuget.description.clone(),
        package_readme_file: readme_file.to_owned(),
        repository_url: config.nuget.repository_url.clone(),
        package_project_url: config.nuget.project_url.clone(),
        authors: config.nuget.authors.join(";"),
        package_tags: config.nuget.tags.join(";"),
        package_icon: config
            .nuget
            .icon
            .as_deref()
            .map(file_name)
            .transpose()?
            .map(str::to_owned),
        readme_include,
        icon_include,
    })
}

fn managed_csproj_snapshot_from_content(
    content: &str,
    config: &DharaRepoConfig,
) -> Result<ManagedCsprojSnapshot> {
    let project =
        Element::parse(content.as_bytes()).context("failed to parse Dhara.Storage.csproj")?;
    let readme_file = file_name(&config.nuget.readme)?;
    let readme_include = find_pack_none_include(
        &project,
        &[config.nuget.readme.as_str(), readme_file],
    )
    .map(|path| normalize_include_path(&path))
    .unwrap_or_default();
    let icon_include = config.nuget.icon.as_deref().and_then(|icon| {
        let icon_file = file_name(icon).ok()?;
        find_pack_none_include(&project, &[icon, icon_file])
            .map(|path| normalize_include_path(&path))
    });

    Ok(ManagedCsprojSnapshot {
        package_id: find_property_text(&project, "PackageId").unwrap_or_default(),
        version: find_property_text(&project, "Version").unwrap_or_default(),
        description: find_property_text(&project, "Description").unwrap_or_default(),
        package_readme_file: find_property_text(&project, "PackageReadmeFile").unwrap_or_default(),
        repository_url: find_property_text(&project, "RepositoryUrl").unwrap_or_default(),
        package_project_url: find_property_text(&project, "PackageProjectUrl").unwrap_or_default(),
        authors: find_property_text(&project, "Authors").unwrap_or_default(),
        package_tags: find_property_text(&project, "PackageTags").unwrap_or_default(),
        package_icon: find_property_text(&project, "PackageIcon"),
        readme_include,
        icon_include,
    })
}

fn normalize_include_path(path: &str) -> String {
    path.replace('/', "\\")
}

fn find_property_text(project: &Element, name: &str) -> Option<String> {
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
                return property
                    .get_text()
                    .map(|value| value.trim().to_owned())
                    .filter(|value| !value.is_empty());
            }
        }
    }

    None
}

fn find_pack_none_include(project: &Element, aliases: &[&str]) -> Option<String> {
    for child in &project.children {
        let XMLNode::Element(group) = child else {
            continue;
        };
        if group.name != "ItemGroup" {
            continue;
        }

        for item in &group.children {
            let XMLNode::Element(entry) = item else {
                continue;
            };
            if entry.name != "None" || !none_pack_enabled(entry) {
                continue;
            }

            let include = entry.attributes.get("Include")?;
            if aliases.contains(&include.as_str()) {
                return Some(include.clone());
            }

            let include_file_name = csproj_include_file_name(include)?;
            if aliases
                .iter()
                .any(|alias| csproj_include_file_name(alias) == Some(include_file_name))
            {
                return Some(include.clone());
            }
        }
    }

    None
}

fn none_pack_enabled(entry: &Element) -> bool {
    if entry
        .attributes
        .get("Pack")
        .is_some_and(|value| value.eq_ignore_ascii_case("true"))
    {
        return true;
    }

    entry.children.iter().any(|child| {
        let XMLNode::Element(element) = child else {
            return false;
        };
        element.name == "Pack"
            && element
                .get_text()
                .is_some_and(|value| value.eq_ignore_ascii_case("true"))
    })
}

fn upsert_property_element(content: &mut String, name: &str, value: &str) -> Result<()> {
    let open = format!("<{name}>");
    let close = format!("</{name}>");
    if let Some(start) = content.find(&open) {
        let value_start = start + open.len();
        let rel_end = content[value_start..]
            .find(&close)
            .with_context(|| format!("malformed csproj element <{name}>"))?;
        let value_end = value_start + rel_end;
        if &content[value_start..value_end] == value {
            return Ok(());
        }
        content.replace_range(value_start..value_end, value);
        return Ok(());
    }

    let updated = insert_property_in_first_group(content, name, value)?;
    *content = updated;
    Ok(())
}

fn remove_property_element(content: &mut String, name: &str) {
    let open = format!("<{name}>");
    let close = format!("</{name}>");
    let Some(start) = content.find(&open) else {
        return;
    };
    let Some(rel_end) = content[start..].find(&close) else {
        return;
    };
    let end = start + rel_end + close.len();
    content.replace_range(start..end, "");
}

fn insert_property_in_first_group(content: &str, name: &str, value: &str) -> Result<String> {
    if let Some(index) = content.find("<PropertyGroup>") {
        let insert_at = index + "<PropertyGroup>".len();
        let insertion = format!("\n    <{name}>{value}</{name}>");
        let mut updated = String::with_capacity(content.len() + insertion.len());
        updated.push_str(&content[..insert_at]);
        updated.push_str(&insertion);
        updated.push_str(&content[insert_at..]);
        return Ok(updated);
    }

    let project_open = content
        .find("<Project")
        .context("csproj is missing a Project root element")?;
    let rel_close = content[project_open..]
        .find('>')
        .context("csproj has a malformed Project opening tag")?;
    let insert_at = project_open + rel_close + 1;
    let insertion = format!("\n  <PropertyGroup>\n    <{name}>{value}</{name}>\n  </PropertyGroup>");
    let mut updated = String::with_capacity(content.len() + insertion.len());
    updated.push_str(&content[..insert_at]);
    updated.push_str(&insertion);
    updated.push_str(&content[insert_at..]);
    Ok(updated)
}

fn patch_pack_none_include(
    content: &mut String,
    _aliases: &[&str],
    expected_include: &str,
    current_include: &str,
) -> Result<()> {
    let expected_include = normalize_include_path(expected_include);
    if !current_include.is_empty() && normalize_include_path(current_include) == expected_include
    {
        return Ok(());
    }

    if !current_include.is_empty() {
        for candidate in [
            current_include.to_owned(),
            normalize_include_path(current_include),
        ] {
            let quoted = format!(r#"Include="{candidate}""#);
            if let Some(start) = content.find(&quoted) {
                content.replace_range(
                    start..start + quoted.len(),
                    &format!(r#"Include="{expected_include}""#),
                );
                return Ok(());
            }
        }
    }

    append_pack_none_item(content, &expected_include);
    Ok(())
}

fn append_pack_none_item(content: &mut String, include: &str) {
    let item = format!(
        "  <ItemGroup>\n    <None Include=\"{include}\" Pack=\"true\" PackagePath=\"\\\" />\n  </ItemGroup>\n"
    );
    if let Some(index) = content.rfind("</Project>") {
        content.insert_str(index, &item);
    }
}

pub fn parse_env_content(content: &str) -> Result<BTreeMap<String, String>> {
    let mut values = BTreeMap::new();
    for (line_number, raw_line) in content.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let (key, value) = line.split_once('=').with_context(|| {
            format!(
                "invalid env entry on line {}: expected KEY=VALUE",
                line_number + 1
            )
        })?;
        values.insert(key.trim().to_owned(), value.trim().to_owned());
    }
    Ok(values)
}

fn masked_env(values: BTreeMap<String, String>) -> BTreeMap<String, String> {
    values
        .into_iter()
        .map(|(key, value)| {
            let upper_key = key.to_ascii_uppercase();
            if upper_key.contains("KEY")
                || upper_key.contains("TOKEN")
                || upper_key.contains("SECRET")
                || upper_key.contains("PASSWORD")
            {
                (key, "<redacted>".to_owned())
            } else {
                (key, value)
            }
        })
        .collect()
}

pub fn validate_config(repo_root: &Path, config: &DharaRepoConfig) -> Result<()> {
    Version::parse(&config.versions.workspace)
        .with_context(|| format!("invalid workspace version: {}", config.versions.workspace))?;
    Version::parse(&config.tool.version)
        .with_context(|| format!("invalid tool version: {}", config.tool.version))?;

    if config.nuget.package_id.trim().is_empty() {
        bail!("nuget.package_id must not be empty");
    }
    if config.nuget.authors.is_empty() {
        bail!("nuget.authors must not be empty");
    }
    if config.nuget.tags.is_empty() {
        bail!("nuget.tags must not be empty");
    }
    if config.ci.native_runtimes.is_empty() {
        bail!("ci.native_runtimes must not be empty");
    }
    if config.publish.environment.trim().is_empty() {
        bail!("publish.environment must not be empty");
    }
    if config.publish.api_key_env.trim().is_empty() {
        bail!("publish.api_key_env must not be empty");
    }
    for runtime in &config.ci.native_runtimes {
        if !config.targets.rust_targets.contains_key(runtime) {
            bail!("targets.rust_targets is missing an entry for runtime '{runtime}'");
        }
    }
    if !config
        .ci
        .native_runtimes
        .contains(&config.ci.host_runtime_smoke)
    {
        bail!(
            "ci.host_runtime_smoke '{}' must be present in ci.native_runtimes",
            config.ci.host_runtime_smoke
        );
    }
    if !config
        .ci
        .native_runtimes
        .contains(&config.ci.aot_runtime_smoke)
    {
        bail!(
            "ci.aot_runtime_smoke '{}' must be present in ci.native_runtimes",
            config.ci.aot_runtime_smoke
        );
    }

    require_exists(repo_root, CONFIG_PATH)?;
    require_exists(repo_root, ROOT_CARGO_TOML_PATH)?;
    require_exists(repo_root, &config.ci.package_project)?;
    require_exists(repo_root, &config.ci.tests_project)?;
    require_exists(repo_root, &config.ci.smoke_project)?;
    require_exists(repo_root, &config.nuget.readme)?;
    if let Some(icon) = &config.nuget.icon {
        require_exists(repo_root, icon)?;
    }
    require_exists(repo_root, ENV_EXAMPLE_PATH)?;
    require_exists(repo_root, crate::paths::RUNTIME_DEFS_RELATIVE)?;

    Ok(())
}

fn file_name(path: &str) -> Result<&str> {
    Path::new(path)
        .file_name()
        .and_then(|value| value.to_str())
        .with_context(|| format!("path must end with a file name: {path}"))
}

fn project_relative_include(asset_path: &str, project_path: &str) -> Result<String> {
    let asset = Path::new(asset_path);
    if asset.is_absolute() {
        bail!("repo-managed package assets must use repository-relative paths: {asset_path}");
    }

    let project_dir = Path::new(project_path)
        .parent()
        .with_context(|| format!("package project path must have a parent: {project_path}"))?;
    let project_parts = path_parts(project_dir);
    let asset_parts = path_parts(asset);

    let common_len = project_parts
        .iter()
        .zip(asset_parts.iter())
        .take_while(|(left, right)| left.eq_ignore_ascii_case(right))
        .count();

    let mut relative = Vec::new();
    relative.extend(std::iter::repeat_n(
        "..".to_owned(),
        project_parts.len() - common_len,
    ));
    relative.extend(asset_parts[common_len..].iter().cloned());

    if relative.is_empty() {
        bail!("package asset path cannot point at the package project directory");
    }

    Ok(relative.join("\\"))
}

fn path_parts(path: &Path) -> Vec<String> {
    path.components()
        .filter_map(|component| match component {
            std::path::Component::Normal(value) => Some(value.to_string_lossy().to_string()),
            std::path::Component::ParentDir => Some("..".to_owned()),
            _ => None,
        })
        .collect()
}

fn write_config(repo_root: &Path, config: &DharaRepoConfig) -> Result<()> {
    validate_config(repo_root, config)?;
    let content = toml::to_string_pretty(config).context("failed to serialize config")?;
    let config_path = repo_root.join(CONFIG_PATH);
    fs::write(&config_path, content)
        .with_context(|| format!("failed to write {}", config_path.display()))
}

fn require_exists(repo_root: &Path, relative_path: &str) -> Result<PathBuf> {
    let path = repo_root.join(relative_path);
    if !path.exists() {
        bail!("required path does not exist: {}", path.display());
    }
    Ok(path)
}

fn csproj_include_file_name(include: &str) -> Option<&str> {
    include
        .rsplit(['\\', '/'])
        .next()
        .filter(|name| !name.is_empty())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    fn sample_config() -> DharaRepoConfig {
        let mut targets = BTreeMap::new();
        targets.insert("win-x64".to_owned(), "x86_64-pc-windows-msvc".to_owned());
        targets.insert("win-arm64".to_owned(), "aarch64-pc-windows-msvc".to_owned());
        targets.insert(
            "linux-x64".to_owned(),
            "x86_64-unknown-linux-gnu".to_owned(),
        );
        targets.insert(
            "linux-arm64".to_owned(),
            "aarch64-unknown-linux-gnu".to_owned(),
        );
        targets.insert("osx-arm64".to_owned(), "aarch64-apple-darwin".to_owned());

        DharaRepoConfig {
            versions: VersionConfig {
                workspace: "0.2.0".to_owned(),
            },
            tool: ToolConfig {
                version: "0.8.1".to_owned(),
            },
            nuget: NuGetConfig {
                package_id: "Dhara.Storage".to_owned(),
                source: "https://api.nuget.org/v3/index.json".to_owned(),
                authors: vec!["Naveen Dharmathunga".to_owned()],
                description: "High-level .NET bindings for the native Dhara Storage Rust runtime."
                    .to_owned(),
                tags: vec!["storage".to_owned(), "ffi".to_owned()],
                readme: "src/bindings/Dhara.Storage/README.md".to_owned(),
                icon: Some("src/bindings/Dhara.Storage/icon-small.png".to_owned()),
                repository_url: "https://github.com/D-Naveenz/rheo_storage".to_owned(),
                project_url: "https://github.com/D-Naveenz/rheo_storage".to_owned(),
            },
            ci: CiConfig {
                smoke_project:
                    "src/bindings/Dhara.Storage.ConsumerSmoke/Dhara.Storage.ConsumerSmoke.csproj"
                        .to_owned(),
                package_project: "src/bindings/Dhara.Storage/Dhara.Storage.csproj".to_owned(),
                tests_project: "src/bindings/Dhara.Storage.Tests/Dhara.Storage.Tests.csproj"
                    .to_owned(),
                native_runtimes: vec![
                    "win-x64".to_owned(),
                    "win-arm64".to_owned(),
                    "linux-x64".to_owned(),
                    "linux-arm64".to_owned(),
                    "osx-arm64".to_owned(),
                ],
                host_runtime_smoke: "win-x64".to_owned(),
                aot_runtime_smoke: "win-x64".to_owned(),
            },
            publish: PublishConfig {
                environment: "nuget-production".to_owned(),
                api_key_env: "NUGET_API_KEY".to_owned(),
            },
            targets: TargetsConfig {
                rust_targets: targets,
            },
        }
    }

    fn write_required_files(repo_root: &Path) {
        fs::create_dir_all(repo_root.join("src/bindings/Dhara.Storage")).unwrap();
        fs::create_dir_all(repo_root.join("src/bindings/Dhara.Storage.Tests")).unwrap();
        fs::create_dir_all(repo_root.join("src/bindings/Dhara.Storage.ConsumerSmoke")).unwrap();
        fs::write(repo_root.join(CONFIG_PATH), "placeholder").unwrap();
        fs::write(repo_root.join(ROOT_CARGO_TOML_PATH), "[workspace]\n").unwrap();
        fs::create_dir_all(repo_root.join("tooling/dhara_tool")).unwrap();
        fs::write(
            repo_root.join(TOOL_CARGO_TOML_PATH),
            "[package]\nversion = \"0.8.1\"\n",
        )
        .unwrap();
        fs::write(repo_root.join(ENV_EXAMPLE_PATH), "NUGET_API_KEY=\n").unwrap();
        fs::write(
            repo_root.join("src/bindings/Dhara.Storage/Dhara.Storage.csproj"),
            "<Project />",
        )
        .unwrap();
        fs::write(
            repo_root.join("src/bindings/Dhara.Storage.Tests/Dhara.Storage.Tests.csproj"),
            "<Project />",
        )
        .unwrap();
        fs::write(
            repo_root.join(
                "src/bindings/Dhara.Storage.ConsumerSmoke/Dhara.Storage.ConsumerSmoke.csproj",
            ),
            "<Project />",
        )
        .unwrap();
        fs::write(
            repo_root.join("src/bindings/Dhara.Storage/README.md"),
            "# Dhara.Storage",
        )
        .unwrap();
        fs::write(
            repo_root.join("src/bindings/Dhara.Storage/icon-small.png"),
            "png",
        )
        .unwrap();
        fs::create_dir_all(repo_root.join("src/core/dhara_storage_dal/resources")).unwrap();
        fs::write(
            repo_root.join(crate::paths::RUNTIME_DEFS_RELATIVE),
            "placeholder",
        )
        .unwrap();
    }

    #[test]
    fn parse_env_content_ignores_comments_and_blank_lines() {
        let parsed = parse_env_content(
            r#"
            # comment
            NUGET_API_KEY=test-key

            NUGET_SOURCE=https://api.nuget.org/v3/index.json
            "#,
        )
        .unwrap();

        assert_eq!(parsed.get("NUGET_API_KEY"), Some(&"test-key".to_owned()));
        assert_eq!(
            parsed.get("NUGET_SOURCE"),
            Some(&"https://api.nuget.org/v3/index.json".to_owned())
        );
    }

    #[test]
    fn masked_env_redacts_secret_like_keys() {
        let mut values = BTreeMap::new();
        values.insert("NUGET_API_KEY".to_owned(), "secret".to_owned());
        values.insert(
            "NUGET_SOURCE".to_owned(),
            "https://api.nuget.org/v3/index.json".to_owned(),
        );

        let masked = masked_env(values);

        assert_eq!(masked.get("NUGET_API_KEY"), Some(&"<redacted>".to_owned()));
        assert_eq!(
            masked.get("NUGET_SOURCE"),
            Some(&"https://api.nuget.org/v3/index.json".to_owned())
        );
    }

    #[test]
    fn sync_cargo_toml_updates_workspace_version() {
        let updated = sync_cargo_toml(
            "[workspace]\n[workspace.package]\nversion = \"0.1.0\"\n[workspace.dependencies]\ndhara_storage_dal = { version = \"0.1.0\", path = \"src/core/dhara_storage_dal\" }\ndhara_storage = { version = \"0.1.0\", path = \"src/core/dhara_storage\" }\n",
            "0.2.0",
        )
        .unwrap();

        assert!(updated.contains("version = \"0.2.0\""));
        assert!(updated.contains(
            "dhara_storage_dal = { version = \"0.2.0\", path = \"src/core/dhara_storage_dal\" }"
        ));
        assert!(updated.contains(
            "dhara_storage = { version = \"0.2.0\", path = \"src/core/dhara_storage\" }"
        ));
    }

    #[test]
    fn sync_csproj_updates_package_metadata() {
        let config = sample_config();
        let updated = sync_csproj(
            r#"<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><Version>1.0.0</Version></PropertyGroup></Project>"#,
            &config,
        )
        .unwrap();

        assert!(updated.contains("<PackageId>Dhara.Storage</PackageId>"));
        assert!(updated.contains("<Version>0.2.0</Version>"));
        assert!(updated.contains("<PackageTags>storage;ffi</PackageTags>"));
        assert!(updated.contains("<PackageReadmeFile>README.md</PackageReadmeFile>"));
    }

    #[test]
    fn sync_csproj_uses_project_relative_root_assets() {
        let mut config = sample_config();
        config.nuget.icon = Some("assets/branding/dhara-logo-colored_sm.png".to_owned());
        let updated = sync_csproj(
            r#"<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <PackageIcon>old.png</PackageIcon>
  </PropertyGroup>
  <ItemGroup>
    <None Include="..\..\..\..\..\Dhara.AI\assets\branding\dhara-logo-colored_sm.png">
      <Pack>True</Pack>
      <PackagePath>\</PackagePath>
    </None>
  </ItemGroup>
</Project>"#,
            &config,
        )
        .unwrap();

        assert!(updated.contains("..\\..\\..\\assets\\branding\\dhara-logo-colored_sm.png"));
        assert!(!updated.contains("Dhara.AI"));
    }

    #[test]
    fn csproj_needs_sync_ignores_msbuild_xml_formatting() {
        let config = sample_config();
        let formatted = r#"<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <StagedNativeRoot Condition="&apos;$(StagedNativeRoot)&apos; == &apos;&apos;" />
    <PackageId>Dhara.Storage</PackageId>
    <Version>0.2.0</Version>
    <Description>High-level .NET bindings for the native Dhara Storage Rust runtime.</Description>
    <PackageReadmeFile>README.md</PackageReadmeFile>
    <RepositoryUrl>https://github.com/D-Naveenz/rheo_storage</RepositoryUrl>
    <PackageProjectUrl>https://github.com/D-Naveenz/rheo_storage</PackageProjectUrl>
    <Authors>Naveen Dharmathunga</Authors>
    <PackageTags>storage;ffi</PackageTags>
    <PackageIcon>icon-small.png</PackageIcon>
  </PropertyGroup>
  <ItemGroup>
    <None Pack="true" Include="README.md" PackagePath="\" />
  </ItemGroup>
  <ItemGroup>
    <None Include="icon-small.png" Pack="true" PackagePath="\" />
  </ItemGroup>
</Project>"#;

        assert!(!csproj_needs_sync(formatted, &config).unwrap());
        assert_eq!(sync_csproj(formatted, &config).unwrap(), formatted);
    }

    #[test]
    fn validate_config_accepts_complete_repo_layout() {
        let temp = tempdir().unwrap();
        write_required_files(temp.path());
        let config = sample_config();

        validate_config(temp.path(), &config).unwrap();
    }

    #[test]
    fn sync_tool_cargo_toml_updates_package_version() {
        let updated = sync_tool_cargo_toml("[package]\nversion = \"0.8.1\"\n", "0.8.4").unwrap();
        assert!(updated.contains("version = \"0.8.4\""));
    }

    #[test]
    fn detect_config_drift_reports_tool_manifest_mismatch() {
        let temp = tempdir().unwrap();
        write_required_files(temp.path());
        let mut config = sample_config();
        config.tool.version = "0.9.0".to_owned();
        fs::write(
            temp.path().join(CONFIG_PATH),
            toml::to_string_pretty(&config).unwrap(),
        )
        .unwrap();

        let drifts = detect_config_drift(temp.path()).unwrap();
        assert!(drifts.iter().any(|item| {
            item.kind == ConfigDriftKind::ToolCrateVersion
                && item.summary.contains("0.8.1")
                && item.summary.contains("0.9.0")
        }));
    }

    #[test]
    fn apply_config_drift_writes_tool_crate_from_config() {
        let temp = tempdir().unwrap();
        write_required_files(temp.path());
        let mut config = sample_config();
        config.tool.version = "0.9.0".to_owned();
        fs::write(
            temp.path().join(CONFIG_PATH),
            toml::to_string_pretty(&config).unwrap(),
        )
        .unwrap();

        let drifts = detect_config_drift(temp.path()).unwrap();
        apply_config_drift(temp.path(), &drifts).unwrap();
        assert_eq!(read_tool_crate_version(temp.path()).unwrap(), "0.9.0");
        assert!(
            !detect_config_drift(temp.path())
                .unwrap()
                .iter()
                .any(|item| item.kind == ConfigDriftKind::ToolCrateVersion)
        );
    }

    #[test]
    fn bump_version_updates_workspace_version() {
        let temp = tempdir().unwrap();
        write_required_files(temp.path());
        let config = sample_config();
        fs::write(
            temp.path().join(CONFIG_PATH),
            toml::to_string_pretty(&config).unwrap(),
        )
        .unwrap();

        let bumped = bump_version(temp.path(), VersionPart::Major).unwrap();
        let reloaded = load_config(temp.path()).unwrap();

        assert_eq!(bumped, "1.0.0");
        assert_eq!(reloaded.versions.workspace, "1.0.0");
    }
}
