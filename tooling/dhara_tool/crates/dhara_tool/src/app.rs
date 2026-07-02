use std::env;
use std::io::{self, BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use dhara_tool_cli::{
    CommandRegistry, DharaStorageCapability, RunMode, ToolCapability, ToolContext,
};
use dhara_tool_gui::{GuiBootParams, can_launch_gui, run_gui};
use dhara_tool_kernel::{
    activation::run_activation, ensure_workspace_state, log_session_end, paths::resolve_exe_root,
    resolve_and_persist_repository, stale_cached_repository, try_cached_repository, workers,
};

pub fn run() -> Result<()> {
    let cli = parse_root_args(env::args().skip(1).collect())?;

    let mut registry = CommandRegistry::new();
    DharaStorageCapability.register(&mut registry);

    if cli.show_version {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    if cli.show_help {
        print!("{}", help_text(&registry));
        return Ok(());
    }

    let exe_root =
        resolve_exe_root(env::current_exe().context("failed to resolve current executable")?)?;

    let run_mode = if !cli.command.is_empty() {
        RunMode::Direct
    } else if can_launch_gui() {
        RunMode::Interactive
    } else {
        RunMode::Direct
    };

    let effective_workers = workers::init_global_thread_pool(cli.workers)?;

    let boot = GuiBootParams {
        min: cli.min,
        trace: cli.trace,
        workers: effective_workers,
        yes: cli.yes,
        package_dir: cli.package_dir.clone(),
        output_dir: cli.output_dir.clone(),
        logs_dir: cli.logs_dir.clone(),
    };

    let launch = determine_launch_mode(!cli.command.is_empty(), can_launch_gui());

    match launch {
        LaunchMode::InteractiveGui => {
            if let Some(repo_root) = try_early_repository(&exe_root, cli.repository.clone())? {
                let pending_activation =
                    run_activation(&repo_root, cli.yes, run_mode)?.unwrap_or_default();
                let context = build_context(
                    repo_root,
                    exe_root.clone(),
                    run_mode,
                    &cli,
                    effective_workers,
                );
                ensure_workspace_state(&context);
                run_gui(
                    &registry,
                    exe_root,
                    boot,
                    Some(context),
                    pending_activation,
                    None,
                )?;
            } else {
                let stale_hint = stale_cached_repository(&exe_root);
                run_gui(&registry, exe_root, boot, None, Vec::new(), stale_hint)?;
            }
        }
        LaunchMode::PlainHelp => print!("{}", help_text(&registry)),
        LaunchMode::DirectCommand => {
            let repo_root = resolve_repository_for_direct(&exe_root, cli.repository.clone())?;
            let pending_activation =
                run_activation(&repo_root, cli.yes, run_mode)?.unwrap_or_default();
            let _ = pending_activation;
            let context = build_context(
                repo_root,
                exe_root.clone(),
                run_mode,
                &cli,
                effective_workers,
            );
            ensure_workspace_state(&context);

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

fn build_context(
    repo_root: PathBuf,
    exe_root: PathBuf,
    run_mode: RunMode,
    cli: &RootArgs,
    workers: usize,
) -> ToolContext {
    ToolContext {
        repo_root,
        tool_root: exe_root,
        run_mode,
        min: cli.min,
        trace: cli.trace,
        workers,
        package_dir: cli.package_dir.clone(),
        output_dir: cli.output_dir.clone(),
        logs_dir: cli.logs_dir.clone(),
    }
}

/// Resolves from `-r` or valid cache only (no prompt).
fn try_early_repository(exe_root: &Path, cli_override: Option<PathBuf>) -> Result<Option<PathBuf>> {
    if let Some(path) = cli_override {
        return Ok(Some(resolve_and_persist_repository(exe_root, path, true)?));
    }

    Ok(try_cached_repository(exe_root))
}

fn resolve_repository_for_direct(
    exe_root: &Path,
    cli_override: Option<PathBuf>,
) -> Result<PathBuf> {
    if let Some(repo) = try_early_repository(exe_root, cli_override)? {
        return Ok(repo);
    }

    if io::stdin().is_terminal() {
        let path = prompt_repository_path()?;
        return resolve_and_persist_repository(exe_root, path, true);
    }

    bail!(
        "repository path is required; pass -r/--repository <path> or run interactively to create {}/runtime.toml",
        exe_root.display()
    );
}

fn prompt_repository_path() -> Result<PathBuf> {
    let mut stderr = io::stderr();
    write!(stderr, "Repository path (folder or dhara.config.toml): ")?;
    stderr.flush()?;

    let mut line = String::new();
    io::stdin()
        .lock()
        .read_line(&mut line)
        .context("failed to read repository path from stdin")?;
    let trimmed = line.trim();
    if trimmed.is_empty() {
        bail!("repository path is required");
    }
    Ok(PathBuf::from(trimmed))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LaunchMode {
    InteractiveGui,
    PlainHelp,
    DirectCommand,
}

fn determine_launch_mode(has_command: bool, interactive_gui: bool) -> LaunchMode {
    if has_command {
        LaunchMode::DirectCommand
    } else if interactive_gui {
        LaunchMode::InteractiveGui
    } else {
        LaunchMode::PlainHelp
    }
}

#[derive(Debug, Clone)]
struct RootArgs {
    repository: Option<PathBuf>,
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
        repository: None,
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
            "-r" | "--repository" => {
                parsed.repository = Some(PathBuf::from(next_value(&args, index, "--repository")?));
                index += 2;
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
            _ if token.starts_with("--repository=") => {
                parsed.repository = Some(PathBuf::from(token.trim_start_matches("--repository=")));
                index += 1;
            }
            _ if token.starts_with("-r=") => {
                parsed.repository = Some(PathBuf::from(token.trim_start_matches("-r=")));
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
           interactive  no subcommand with a graphical display — opens the operator GUI\n\
           direct       subcommand present — runs immediately (CI, agents, scripts)\n\n\
         Global options (may appear before or after the command):\n\
           -r, --repository <path>  repository directory or dhara.config.toml (overrides runtime cache)\n\
           --package-dir <path>\n\
           --output-dir <path>\n\
           --logs-dir <path>\n\
           -m, --min         file log WARN only (console stays INFO)\n\
           -t, --trace       file log DEBUG (console stays INFO)\n\
           -w, --workers <n>  cap Rayon worker threads (default 4; env TOOL_MAX_WORKERS)\n\
           -y, --yes         apply configuration drift without prompting\n\
           -h, --help\n\
           --version\n\n\
         Repository resolution:\n\
           1. -r/--repository when provided\n\
           2. exe_path/runtime.toml when valid\n\
           3. interactive prompt (TTY) or GUI repository picker\n\n\
         {}",
        registry.help_text()
    )
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use dhara_tool_kernel::{CONFIG_PATH, resolve_and_persist_repository, try_cached_repository};

    use super::{LaunchMode, determine_launch_mode, parse_root_args, try_early_repository};

    #[test]
    fn no_command_with_gui_uses_interactive() {
        assert_eq!(
            determine_launch_mode(false, true),
            LaunchMode::InteractiveGui
        );
    }

    #[test]
    fn no_command_without_gui_uses_plain_help() {
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
    fn repository_flag_parsed() {
        let parsed = parse_root_args(vec![
            "-r".to_owned(),
            "/repo".to_owned(),
            "config".to_owned(),
            "show".to_owned(),
        ])
        .unwrap();
        assert_eq!(parsed.repository, Some("/repo".into()));
    }

    #[test]
    fn repository_long_flag_parsed() {
        let parsed = parse_root_args(vec![
            "--repository=/repo".to_owned(),
            "config".to_owned(),
            "show".to_owned(),
        ])
        .unwrap();
        assert_eq!(parsed.repository, Some("/repo".into()));
    }

    #[test]
    fn explicit_repository_wins_and_persists_cache() {
        let temp = tempdir().unwrap();
        let root = temp.path().join("repo");
        let exe = temp.path().join("bin");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&exe).unwrap();
        fs::write(root.join(CONFIG_PATH), "[versions]\n").unwrap();

        let resolved = try_early_repository(&exe, Some(root.clone()))
            .unwrap()
            .unwrap();
        assert!(try_cached_repository(&exe).is_some());
        assert_eq!(
            resolved,
            resolve_and_persist_repository(&exe, root, false).unwrap()
        );
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
}
