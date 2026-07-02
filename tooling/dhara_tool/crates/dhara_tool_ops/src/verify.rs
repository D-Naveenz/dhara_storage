use std::path::Path;

use anyhow::Result;

use dhara_tool_kernel::CommandResult;
use dhara_tool_kernel::repo_config::DharaRepoConfig;

use crate::nuget::PackageOptions;

pub fn verify_package(
    repo_root: &Path,
    tool_root: &Path,
    config: &DharaRepoConfig,
    options: &PackageOptions,
) -> Result<CommandResult> {
    crate::nuget::verify(repo_root, tool_root, config, options)
}
