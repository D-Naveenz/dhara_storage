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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DharaRepoConfig {
    pub versions: VersionConfig,
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

pub fn sync(repo_root: &Path) -> Result<()> {
    let config = load_config(repo_root)?;
    validate_config(repo_root, &config)?;

    let cargo_path = repo_root.join(ROOT_CARGO_TOML_PATH);
    let cargo_content = fs::read_to_string(&cargo_path)
        .with_context(|| format!("failed to read {}", cargo_path.display()))?;
    let updated_cargo = sync_cargo_toml(&cargo_content, &config.versions.workspace)?;
    if updated_cargo != cargo_content {
        fs::write(&cargo_path, updated_cargo)
            .with_context(|| format!("failed to write {}", cargo_path.display()))?;
    }

    let csproj_path = repo_root.join(&config.ci.package_project);
    let csproj_content = fs::read_to_string(&csproj_path)
        .with_context(|| format!("failed to read {}", csproj_path.display()))?;
    let updated_csproj = sync_csproj(&csproj_content, &config)?;
    if updated_csproj != csproj_content {
        fs::write(&csproj_path, updated_csproj)
            .with_context(|| format!("failed to write {}", csproj_path.display()))?;
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

pub fn sync_csproj(content: &str, config: &DharaRepoConfig) -> Result<String> {
    let mut project =
        Element::parse(content.as_bytes()).context("failed to parse Dhara.Storage.csproj")?;
    let property_group = get_or_add_property_group(&mut project);

    set_or_add_property(property_group, "PackageId", &config.nuget.package_id);
    set_or_add_property(property_group, "Version", &config.versions.workspace);
    set_or_add_property(property_group, "Description", &config.nuget.description);
    set_or_add_property(
        property_group,
        "PackageReadmeFile",
        file_name(&config.nuget.readme)?,
    );
    set_or_add_property(
        property_group,
        "RepositoryUrl",
        &config.nuget.repository_url,
    );
    set_or_add_property(
        property_group,
        "PackageProjectUrl",
        &config.nuget.project_url,
    );
    set_or_add_property(property_group, "Authors", &config.nuget.authors.join(";"));
    set_or_add_property(property_group, "PackageTags", &config.nuget.tags.join(";"));

    if let Some(icon) = &config.nuget.icon {
        set_or_add_property(property_group, "PackageIcon", file_name(icon)?);
    }
    dedupe_managed_package_properties(&mut project);

    let readme_file = file_name(&config.nuget.readme)?.to_owned();
    let readme_include =
        project_relative_include(&config.nuget.readme, &config.ci.package_project)?;
    normalize_pack_none_item(
        &mut project,
        &[config.nuget.readme.as_str(), readme_file.as_str()],
        &readme_include,
        "\\",
    );
    if let Some(icon) = &config.nuget.icon {
        let icon_file = file_name(icon)?.to_owned();
        let icon_include = project_relative_include(icon, &config.ci.package_project)?;
        normalize_pack_none_item(
            &mut project,
            &[icon.as_str(), icon_file.as_str()],
            &icon_include,
            "\\",
        );
    }
    prune_empty_item_groups(&mut project);

    let mut output = Vec::new();
    project
        .write_with_config(
            &mut output,
            xmltree::EmitterConfig::new()
                .perform_indent(true)
                .write_document_declaration(false),
        )
        .context("failed to render Dhara.Storage.csproj")?;
    String::from_utf8(output).context("generated csproj was not valid utf-8")
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

fn get_or_add_property_group(project: &mut Element) -> &mut Element {
    let index = project
        .children
        .iter()
        .position(
            |child| matches!(child, XMLNode::Element(element) if element.name == "PropertyGroup"),
        )
        .unwrap_or_else(|| {
            project
                .children
                .insert(0, XMLNode::Element(Element::new("PropertyGroup")));
            0
        });

    match project.children.get_mut(index) {
        Some(XMLNode::Element(element)) => element,
        _ => unreachable!("property group index always points to an element"),
    }
}

fn set_or_add_property(group: &mut Element, name: &str, value_text: &str) {
    if let Some(element) = group.children.iter_mut().find_map(|child| match child {
        XMLNode::Element(element) if element.name == name => Some(element),
        _ => None,
    }) {
        element.children.clear();
        element.children.push(XMLNode::Text(value_text.to_owned()));
        return;
    }

    let mut element = Element::new(name);
    element.children.push(XMLNode::Text(value_text.to_owned()));
    group.children.push(XMLNode::Element(element));
}

fn normalize_pack_none_item(
    project: &mut Element,
    aliases: &[&str],
    include: &str,
    package_path: &str,
) {
    let include_file_name = Path::new(include)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(include);

    for child in &mut project.children {
        let XMLNode::Element(group) = child else {
            continue;
        };
        if group.name != "ItemGroup" {
            continue;
        }

        group
            .children
            .retain(|item| !is_pack_none_alias(item, aliases, include_file_name, package_path));
    }

    let mut none = Element::new("None");
    none.attributes
        .insert("Include".to_owned(), include.to_owned());
    none.attributes.insert("Pack".to_owned(), "true".to_owned());
    none.attributes
        .insert("PackagePath".to_owned(), package_path.to_owned());

    let mut group = Element::new("ItemGroup");
    group.children.push(XMLNode::Element(none));
    project.children.push(XMLNode::Element(group));
}

fn is_pack_none_alias(
    item: &XMLNode,
    aliases: &[&str],
    include_file_name: &str,
    package_path: &str,
) -> bool {
    let XMLNode::Element(entry) = item else {
        return false;
    };
    if entry.name != "None" {
        return false;
    }

    let Some(candidate) = entry.attributes.get("Include") else {
        return false;
    };
    if aliases.contains(&candidate.as_str()) {
        return true;
    }

    let candidate_file_name = Path::new(candidate)
        .file_name()
        .and_then(|value| value.to_str());
    candidate_file_name == Some(include_file_name)
        && item_metadata(entry, "PackagePath").is_some_and(|candidate| candidate == package_path)
}

fn item_metadata(entry: &Element, name: &str) -> Option<String> {
    if let Some(value) = entry.attributes.get(name) {
        return Some(value.clone());
    }

    entry.children.iter().find_map(|child| match child {
        XMLNode::Element(element) if element.name == name => {
            element.get_text().map(|value| value.into_owned())
        }
        _ => None,
    })
}

fn dedupe_managed_package_properties(project: &mut Element) {
    const MANAGED: &[&str] = &[
        "PackageId",
        "Version",
        "Description",
        "PackageReadmeFile",
        "PackageIcon",
        "RepositoryUrl",
        "PackageProjectUrl",
        "Authors",
        "PackageTags",
    ];

    let mut found_first_group = false;
    for child in &mut project.children {
        let XMLNode::Element(group) = child else {
            continue;
        };
        if group.name != "PropertyGroup" {
            continue;
        }
        if !found_first_group {
            found_first_group = true;
            continue;
        }

        group.children.retain(|item| {
            !matches!(
                item,
                XMLNode::Element(entry) if MANAGED.contains(&entry.name.as_str())
            )
        });
    }
}

fn prune_empty_item_groups(project: &mut Element) {
    project.children.retain(|child| {
        !matches!(
            child,
            XMLNode::Element(group) if group.name == "ItemGroup" && group.children.is_empty()
        )
    });
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

        DharaRepoConfig {
            versions: VersionConfig {
                workspace: "0.2.0".to_owned(),
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
                native_runtimes: vec!["win-x64".to_owned(), "win-arm64".to_owned()],
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
            "[workspace]\n[workspace.package]\nversion = \"0.1.0\"\n[workspace.dependencies]\ndhara_storage_dal = { version = \"0.1.0\", path = \"src/static/dhara_storage_dal\" }\ndhara_storage = { version = \"0.1.0\", path = \"src/static/dhara_storage\" }\n",
            "0.2.0",
        )
        .unwrap();

        assert!(updated.contains("version = \"0.2.0\""));
        assert!(updated.contains(
            "dhara_storage_dal = { version = \"0.2.0\", path = \"src/static/dhara_storage_dal\" }"
        ));
        assert!(updated.contains(
            "dhara_storage = { version = \"0.2.0\", path = \"src/static/dhara_storage\" }"
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
    fn validate_config_accepts_complete_repo_layout() {
        let temp = tempdir().unwrap();
        write_required_files(temp.path());
        let config = sample_config();

        validate_config(temp.path(), &config).unwrap();
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
