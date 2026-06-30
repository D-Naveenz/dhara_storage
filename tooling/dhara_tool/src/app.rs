use std::env;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::activation::run_activation;
use crate::command::{CommandRegistry, RunMode, ToolCapability, ToolContext};
use crate::paths::{is_repo_root, resolve_tool_root};
use crate::tui::{can_launch, run_tui};
use crate::{DharaStorageCapability, ensure_workspace_state, log_session_end};

pub fn run() -> Result<()> {
    let cli = parse_root_args(env::args().skip(1).collect())?;

    let mut registry = CommandRegistry::new();
    DharaStorageCapability.register(&mut registry);

    if cli.show_version {
        println!("{}", crate::version());
        return Ok(());
    }

    if cli.show_help {
        print!("{}", help_text(&registry));
        return Ok(());
    }

    let current_exe = env::current_exe().ok();
    let current_dir = env::current_dir().ok();

    let repo_root = resolve_repo_root(
        cli.repo_root.clone(),
        current_dir.clone(),
        current_exe.clone(),
    )?;

    let tool_root = resolve_tool_root(current_exe, current_dir);

    let run_mode = if !cli.command.is_empty() {
        RunMode::Direct
    } else if can_launch() {
        RunMode::Interactive
    } else {
        RunMode::Direct
    };

    let effective_workers = crate::workers::init_global_thread_pool(cli.workers)?;

    let context = ToolContext {
        repo_root: repo_root.clone(),
        tool_root,
        run_mode,
        min: cli.min,
        trace: cli.trace,
        workers: effective_workers,
        package_dir: cli.package_dir,
        output_dir: cli.output_dir,
        logs_dir: cli.logs_dir,
    };

    let pending_activation =
        run_activation(&repo_root, cli.yes, run_mode)?.unwrap_or_default();

    ensure_workspace_state(&context);

    match determine_launch_mode(!cli.command.is_empty(), can_launch()) {
        LaunchMode::InteractiveTui => run_tui(&registry, &context, pending_activation)?,
        LaunchMode::PlainHelp => print!("{}", help_text(&registry)),
        LaunchMode::DirectCommand => {
            let command_id = registry
                .resolve(&cli.command)
                .map(|(command, _)| command.id)
                .unwrap_or("unknown");
            let result = match registry.execute(&context, &cli.command) {
                Ok(result) => {
                    log_session_end(result.exit_code, Some(command_id), None);
                    result
                }
                Err(error) => {
                    log_session_end(1, Some(command_id), Some(&error.to_string()));
                    return Err(error);
                }
            };
            result.print(&context);
            if result.exit_code != 0 {
                std::process::exit(result.exit_code);
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LaunchMode {
    InteractiveTui,
    PlainHelp,
    DirectCommand,
}

fn determine_launch_mode(has_command: bool, interactive_terminal: bool) -> LaunchMode {
    if has_command {
        LaunchMode::DirectCommand
    } else if interactive_terminal {
        LaunchMode::InteractiveTui
    } else {
        LaunchMode::PlainHelp
    }
}

fn resolve_repo_root(
    requested_repo_root: Option<PathBuf>,
    current_dir: Option<PathBuf>,
    current_exe: Option<PathBuf>,
) -> Result<PathBuf> {
    if let Some(path) = requested_repo_root {
        let root = normalize_repo_root(path.clone())
            .with_context(|| format!("failed to canonicalize repo root '{}'", path.display()))?;
        if !is_repo_root(&root) {
            bail!(
                "--repo-root '{}' is not a Dhara Storage workspace root (expected dhara.config.toml and tooling/dhara_tool/Cargo.toml)",
                root.display()
            );
        }
        return Ok(root);
    }

    discover_repo_root(current_dir, current_exe).context(
        "failed to discover repo root; run dhara_tool from the workspace or pass --repo-root",
    )
}

fn discover_repo_root(
    current_dir: Option<PathBuf>,
    current_exe: Option<PathBuf>,
) -> Result<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(dir) = current_dir {
        candidates.push(dir);
    }
    if let Some(exe) = current_exe.and_then(|path| path.parent().map(Path::to_path_buf))
        && !candidates.iter().any(|candidate| candidate == &exe)
    {
        candidates.push(exe);
    }

    for candidate in candidates {
        if let Some(repo_root) = discover_repo_root_from(&candidate) {
            return Ok(repo_root);
        }
    }

    bail!("no workspace containing dhara.config.toml was found")
}

fn discover_repo_root_from(start: &Path) -> Option<PathBuf> {
    for candidate in start.ancestors() {
        if !is_repo_root(candidate) {
            continue;
        }
        if let Ok(root) = normalize_repo_root(candidate.to_path_buf()) {
            return Some(root);
        }
    }

    None
}

fn normalize_repo_root(path: PathBuf) -> Result<PathBuf> {
    let canonical = path.canonicalize()?;

    #[cfg(windows)]
    {
        const VERBATIM_PREFIX: &str = r"\\?\";
        let canonical_text = canonical.to_string_lossy();
        if let Some(stripped) = canonical_text.strip_prefix(VERBATIM_PREFIX) {
            return Ok(PathBuf::from(stripped));
        }
    }

    Ok(canonical)
}

#[derive(Debug, Clone)]
struct RootArgs {
    repo_root: Option<PathBuf>,
    min: bool,
    trace: bool,
    workers: Option<usize>,
    package_dir: Option<PathBuf>,
    output_dir: Option<PathBuf>,
    logs_dir: Option<PathBuf>,
    show_help: bool,
    show_version: bool,
    yes: bool,
    command: Vec<String>,
}

fn parse_root_args(args: Vec<String>) -> Result<RootArgs> {
    let mut parsed = RootArgs {
        repo_root: None,
        min: false,
        trace: false,
        workers: None,
        package_dir: None,
        output_dir: None,
        logs_dir: None,
        show_help: false,
        show_version: false,
        yes: false,
        command: Vec::new(),
    };

    let mut index = 0;
    while index < args.len() {
        let token = &args[index];
        match token.as_str() {
            "-h" | "--help" => {
                parsed.show_help = true;
                index += 1;
            }
            "--version" => {
                parsed.show_version = true;
                index += 1;
            }
            "-m" | "--min" => {
                parsed.min = true;
                index += 1;
            }
            "-t" | "--trace" => {
                parsed.trace = true;
                index += 1;
            }
            "-y" | "--yes" => {
                parsed.yes = true;
                index += 1;
            }
            "-w" | "--workers" => {
                let value = next_value(&args, index, "--workers")?;
                parsed.workers = Some(
                    value
                        .parse()
                        .with_context(|| format!("'{value}' is not a valid worker count"))?,
                );
                index += 2;
            }
            "--repo-root" => {
                parsed.repo_root = Some(PathBuf::from(next_value(&args, index, "--repo-root")?));
                index += 2;
            }
            "--package-dir" => {
                parsed.package_dir =
                    Some(PathBuf::from(next_value(&args, index, "--package-dir")?));
                index += 2;
            }
            "--output-dir" => {
                parsed.output_dir = Some(PathBuf::from(next_value(&args, index, "--output-dir")?));
                index += 2;
            }
            "--logs-dir" => {
                parsed.logs_dir = Some(PathBuf::from(next_value(&args, index, "--logs-dir")?));
                index += 2;
            }
            _ if token.starts_with("--repo-root=") => {
                parsed.repo_root = Some(PathBuf::from(token.trim_start_matches("--repo-root=")));
                index += 1;
            }
            _ if token.starts_with("--package-dir=") => {
                parsed.package_dir =
                    Some(PathBuf::from(token.trim_start_matches("--package-dir=")));
                index += 1;
            }
            _ if token.starts_with("--output-dir=") => {
                parsed.output_dir = Some(PathBuf::from(token.trim_start_matches("--output-dir=")));
                index += 1;
            }
            _ if token.starts_with("--logs-dir=") => {
                parsed.logs_dir = Some(PathBuf::from(token.trim_start_matches("--logs-dir=")));
                index += 1;
            }
            _ if token.starts_with("--workers=") => {
                let value = token.trim_start_matches("--workers=");
                parsed.workers = Some(
                    value
                        .parse()
                        .with_context(|| format!("'{value}' is not a valid worker count"))?,
                );
                index += 1;
            }
            _ => {
                parsed.command.push(token.clone());
                index += 1;
            }
        }
    }

    Ok(parsed)
}

fn next_value<'a>(args: &'a [String], index: usize, option: &str) -> Result<&'a str> {
    args.get(index + 1)
        .map(String::as_str)
        .with_context(|| format!("{option} requires a value"))
}

fn help_text(registry: &CommandRegistry) -> String {
    format!(
        "Usage: dhara_tool [global-options] <command> [command-options]\n\n\
         Launch modes:\n\
           interactive  no subcommand in a TTY — opens the guided TUI\n\
           direct       subcommand present — runs immediately (CI, agents, scripts)\n\n\
         Global options (may appear before or after the command):\n\
           --repo-root <path>\n\
           --package-dir <path>\n\
           --output-dir <path>\n\
           --logs-dir <path>\n\
           -m, --min         file log WARN only (console stays INFO)\n\
           -t, --trace       file log DEBUG (console stays INFO)\n\
           -w, --workers <n>  cap Rayon worker threads (default 4; env TOOL_MAX_WORKERS)\n\
           -y, --yes         apply configuration drift without prompting\n\
           -h, --help\n\
           --version\n\n\
         {}",
        registry.help_text()
    )
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::{
        LaunchMode, determine_launch_mode, discover_repo_root_from, normalize_repo_root,
        parse_root_args, resolve_repo_root,
    };

    #[test]
    fn no_command_in_tty_uses_tui() {
        assert_eq!(
            determine_launch_mode(false, true),
            LaunchMode::InteractiveTui
        );
    }

    #[test]
    fn no_command_without_tty_uses_plain_help() {
        assert_eq!(determine_launch_mode(false, false), LaunchMode::PlainHelp);
    }

    #[test]
    fn explicit_command_uses_direct_mode() {
        assert_eq!(determine_launch_mode(true, true), LaunchMode::DirectCommand);
        assert_eq!(
            determine_launch_mode(true, false),
            LaunchMode::DirectCommand
        );
    }

    #[test]
    fn explicit_repo_root_wins_over_discovery() {
        let temp = tempdir().unwrap();
        let root = temp.path().join("repo");
        let nested = root.join("target").join("debug");
        fs::create_dir_all(&nested).unwrap();
        fs::write(root.join("dhara.config.toml"), "placeholder").unwrap();
        fs::write(root.join("Cargo.toml"), "[workspace]\n").unwrap();
        fs::create_dir_all(root.join("tooling/dhara_tool")).unwrap();
        fs::write(root.join("tooling/dhara_tool/Cargo.toml"), "[package]\n").unwrap();

        let resolved = resolve_repo_root(
            Some(root.clone()),
            Some(nested.clone()),
            Some(nested.join("dhara_tool.exe")),
        )
        .unwrap();

        assert_eq!(resolved, normalize_repo_root(root).unwrap());
    }

    #[test]
    fn discovers_repo_root_from_nested_target_directory() {
        let temp = tempdir().unwrap();
        let root = temp.path().join("repo");
        let nested = root.join("target").join("debug");
        fs::create_dir_all(&nested).unwrap();
        fs::write(root.join("dhara.config.toml"), "placeholder").unwrap();
        fs::write(root.join("Cargo.toml"), "[workspace]\n").unwrap();
        fs::create_dir_all(root.join("tooling/dhara_tool")).unwrap();
        fs::write(root.join("tooling/dhara_tool/Cargo.toml"), "[package]\n").unwrap();

        let resolved = discover_repo_root_from(&nested).unwrap();

        assert_eq!(resolved, normalize_repo_root(root).unwrap());
    }

    #[test]
    fn trace_flag_may_follow_subcommand() {
        let parsed = parse_root_args(vec![
            "defs".to_owned(),
            "inspect-trid-xml".to_owned(),
            "--trace".to_owned(),
        ])
        .unwrap();

        assert!(parsed.trace);
        assert_eq!(parsed.command, vec!["defs", "inspect-trid-xml"]);
    }

    #[test]
    fn min_flag_parsed() {
        let parsed = parse_root_args(vec![
            "defs".to_owned(),
            "inspect".to_owned(),
            "--min".to_owned(),
        ])
        .unwrap();
        assert!(parsed.min);
    }

    #[test]
    fn min_short_flag_parsed() {
        let parsed = parse_root_args(vec![
            "-m".to_owned(),
            "defs".to_owned(),
            "inspect".to_owned(),
        ])
        .unwrap();
        assert!(parsed.min);
    }

    #[test]
    fn trace_short_flag_parsed() {
        let parsed = parse_root_args(vec![
            "-t".to_owned(),
            "defs".to_owned(),
            "inspect-trid-xml".to_owned(),
        ])
        .unwrap();
        assert!(parsed.trace);
    }

    #[test]
    fn workers_flag_parsed() {
        let parsed = parse_root_args(vec![
            "-w".to_owned(),
            "2".to_owned(),
            "defs".to_owned(),
            "inspect".to_owned(),
        ])
        .unwrap();
        assert_eq!(parsed.workers, Some(2));
    }

    #[test]
    fn yes_flag_parsed() {
        let parsed = parse_root_args(vec![
            "--yes".to_owned(),
            "config".to_owned(),
            "show".to_owned(),
        ])
        .unwrap();
        assert!(parsed.yes);
    }

    #[test]
    fn discovers_repo_root_from_current_executable_when_cwd_is_elsewhere() {
        let temp = tempdir().unwrap();
        let root = temp.path().join("repo");
        let nested = root.join("target").join("debug");
        let elsewhere = temp.path().join("elsewhere");
        fs::create_dir_all(&nested).unwrap();
        fs::create_dir_all(&elsewhere).unwrap();
        fs::write(root.join("dhara.config.toml"), "placeholder").unwrap();
        fs::write(root.join("Cargo.toml"), "[workspace]\n").unwrap();
        fs::create_dir_all(root.join("tooling/dhara_tool")).unwrap();
        fs::write(root.join("tooling/dhara_tool/Cargo.toml"), "[package]\n").unwrap();

        let resolved =
            resolve_repo_root(None, Some(elsewhere), Some(nested.join("dhara_tool.exe"))).unwrap();

        assert_eq!(resolved, normalize_repo_root(root).unwrap());
    }

    #[test]
    fn rejects_explicit_repo_root_outside_workspace_layout() {
        let temp = tempdir().unwrap();
        let crate_dir = temp.path().join("tooling").join("dhara_tool");
        fs::create_dir_all(&crate_dir).unwrap();
        fs::write(crate_dir.join("Cargo.toml"), "[package]\n").unwrap();

        let error = resolve_repo_root(Some(crate_dir.clone()), Some(crate_dir), None).unwrap_err();
        assert!(error.to_string().contains("--repo-root"));
    }
}
