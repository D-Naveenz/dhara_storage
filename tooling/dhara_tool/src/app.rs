use std::env;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::command::{CommandRegistry, RunMode, ToolCapability, ToolContext};
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

    let repo_root = resolve_repo_root(
        cli.repo_root.clone(),
        env::current_dir().ok(),
        env::current_exe().ok(),
    )?;

    let run_mode = if !cli.command.is_empty() {
        RunMode::Direct
    } else if can_launch() {
        RunMode::Interactive
    } else {
        RunMode::Direct
    };

    let context = ToolContext {
        repo_root,
        run_mode,
        minimal: cli.minimal,
        trace: cli.trace,
        quiet: cli.quiet,
        package_dir: cli.package_dir,
        output_dir: cli.output_dir,
        logs_dir: cli.logs_dir,
    };

    ensure_workspace_state(&context);

    match determine_launch_mode(!cli.command.is_empty(), can_launch()) {
        LaunchMode::InteractiveTui => run_tui(&registry, &context)?,
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
        return normalize_repo_root(path.clone())
            .with_context(|| format!("failed to canonicalize repo root '{}'", path.display()));
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
        if looks_like_repo_root(candidate) {
            return normalize_repo_root(candidate.to_path_buf()).ok();
        }
    }

    None
}

fn looks_like_repo_root(path: &Path) -> bool {
    path.join("dhara.config.toml").is_file() && path.join("Cargo.toml").is_file()
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
    quiet: bool,
    minimal: bool,
    trace: bool,
    package_dir: Option<PathBuf>,
    output_dir: Option<PathBuf>,
    logs_dir: Option<PathBuf>,
    show_help: bool,
    show_version: bool,
    command: Vec<String>,
}

fn parse_root_args(args: Vec<String>) -> Result<RootArgs> {
    let mut parsed = RootArgs {
        repo_root: None,
        quiet: false,
        minimal: false,
        trace: false,
        package_dir: None,
        output_dir: None,
        logs_dir: None,
        show_help: false,
        show_version: false,
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
            "-q" | "--quiet" => {
                parsed.quiet = true;
                index += 1;
            }
            "--minimal" => {
                parsed.minimal = true;
                index += 1;
            }
            "--trace" => {
                parsed.trace = true;
                index += 1;
            }
            "-v" | "--verbose" => {
                parsed.trace = true;
                index += 1;
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
           -q, --quiet     suppress command stdout in direct mode\n\
           --minimal       quieter console output and no live progress\n\
           --trace         verbose audit trail (full reduce detail in log file)\n\
           -v, --verbose   deprecated alias for --trace\n\
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
    fn minimal_flag_parsed() {
        let parsed = parse_root_args(vec![
            "defs".to_owned(),
            "inspect".to_owned(),
            "--minimal".to_owned(),
        ])
        .unwrap();
        assert!(parsed.minimal);
    }

    #[test]
    fn verbose_flag_aliases_trace() {
        let parsed = parse_root_args(vec![
            "-v".to_owned(),
            "defs".to_owned(),
            "inspect".to_owned(),
        ])
        .unwrap();

        assert!(parsed.trace);
        assert_eq!(parsed.command, vec!["defs", "inspect"]);
    }

    #[test]
    fn quiet_flag_parsed() {
        let parsed = parse_root_args(vec![
            "defs".to_owned(),
            "inspect".to_owned(),
            "--quiet".to_owned(),
        ])
        .unwrap();
        assert!(parsed.quiet);
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

        let resolved =
            resolve_repo_root(None, Some(elsewhere), Some(nested.join("dhara_tool.exe"))).unwrap();

        assert_eq!(resolved, normalize_repo_root(root).unwrap());
    }
}
