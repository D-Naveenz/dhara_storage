use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

use anyhow::{Context, Result, bail};
use tracing::{debug, info};
use zip::ZipArchive;

use crate::output::{emit_stderr_line, emit_stdout_line, set_active_child};

fn command_display(program: &str, args: &[String]) -> String {
    if args.is_empty() {
        program.to_owned()
    } else {
        format!("{program} {}", args.join(" "))
    }
}

pub fn run_command(program: &str, args: &[String], cwd: &Path) -> Result<()> {
    emit_stdout_line(format!("> {}", command_display(program, args)));
    info!(
        target: "dhara_tool_dhara_storage::support",
        program,
        args = args.join(" "),
        cwd = %cwd.display(),
        "running command"
    );
    let status = run_command_streaming(program, args, cwd, &[])?;
    if !status.success() {
        bail!(
            "command failed with status {}: {}",
            status,
            command_display(program, args)
        );
    }
    debug!(
        target: "dhara_tool_dhara_storage::support",
        program,
        status = %status,
        "command completed successfully"
    );
    Ok(())
}

pub fn run_command_with_env(
    program: &str,
    args: &[String],
    cwd: &Path,
    envs: &[(&str, &str)],
) -> Result<()> {
    run_command_with_env_redacted(program, args, cwd, envs, &[])
}

pub fn run_command_with_env_redacted(
    program: &str,
    args: &[String],
    cwd: &Path,
    envs: &[(&str, &str)],
    redacted_values: &[&str],
) -> Result<()> {
    let display_args = redact_args(args, redacted_values);
    emit_stdout_line(format!("> {}", command_display(program, &display_args)));
    info!(
        target: "dhara_tool_dhara_storage::support",
        program,
        args = display_args.join(" "),
        cwd = %cwd.display(),
        env_count = envs.len(),
        "running command with environment overrides"
    );
    let status = run_command_streaming(program, args, cwd, envs)?;
    if !status.success() {
        bail!(
            "command failed with status {}: {}",
            status,
            command_display(program, &display_args)
        );
    }
    debug!(
        target: "dhara_tool_dhara_storage::support",
        program,
        status = %status,
        "command completed successfully"
    );
    Ok(())
}

fn redact_args(args: &[String], redacted_values: &[&str]) -> Vec<String> {
    args.iter()
        .map(|arg| {
            if redacted_values
                .iter()
                .any(|value| !value.is_empty() && arg == value)
            {
                "<redacted>".to_owned()
            } else {
                arg.clone()
            }
        })
        .collect()
}

pub fn run_command_expect_failure(
    program: &str,
    args: &[String],
    cwd: &Path,
    expected_output: &str,
) -> Result<()> {
    emit_stdout_line(format!("> {}", command_display(program, args)));
    info!(
        target: "dhara_tool_dhara_storage::support",
        program,
        args = args.join(" "),
        cwd = %cwd.display(),
        expected_output,
        "running command that is expected to fail"
    );
    let output = run_command_capture(program, args, cwd, &[])?;
    if output.status.success() {
        bail!(
            "command unexpectedly succeeded: {}",
            command_display(program, args)
        );
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}\n{stderr}");
    if !combined.contains(expected_output) {
        bail!(
            "command failed without expected output '{}': {}",
            expected_output,
            command_display(program, args)
        );
    }

    debug!(
        target: "dhara_tool_dhara_storage::support",
        program,
        status = %output.status,
        expected_output,
        "command failed as expected"
    );
    Ok(())
}

pub fn write_nuget_config(path: &Path, sources: &[PathBuf]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let mut body = String::from(
        r#"<?xml version="1.0" encoding="utf-8"?>
<configuration>
  <packageSources>
    <clear />
"#,
    );
    for (index, source) in sources.iter().enumerate() {
        body.push_str(&format!(
            "    <add key=\"source{index}\" value=\"{}\" />\n",
            source.display()
        ));
    }
    body.push_str("  </packageSources>\n</configuration>\n");
    fs::write(path, body).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

pub fn inspect_package_entries(package_path: &Path) -> Result<Vec<String>> {
    debug!(
        target: "dhara_tool_dhara_storage::support",
        package_path = %package_path.display(),
        "reading package entries"
    );
    let file = fs::File::open(package_path)
        .with_context(|| format!("failed to open {}", package_path.display()))?;
    let mut archive = ZipArchive::new(file)
        .with_context(|| format!("failed to read {}", package_path.display()))?;

    let mut entries = Vec::with_capacity(archive.len());
    for index in 0..archive.len() {
        let file = archive.by_index(index)?;
        entries.push(file.name().replace('\\', "/"));
    }

    Ok(entries)
}

fn prepare_command<'a>(
    program: &str,
    args: &[String],
    cwd: &Path,
    envs: &[(&'a str, &'a str)],
) -> Result<Command> {
    let mut command = Command::new(program);
    command.args(args).current_dir(cwd);
    for (key, value) in envs {
        command.env(key, value);
    }

    if program.eq_ignore_ascii_case("dotnet")
        && std::env::var_os("DOTNET_CLI_HOME").is_none()
        && !envs.iter().any(|(key, _)| *key == "DOTNET_CLI_HOME")
    {
        let dotnet_home = cwd.join(".dotnet");
        fs::create_dir_all(&dotnet_home)
            .with_context(|| format!("failed to create {}", dotnet_home.display()))?;
        command.env("DOTNET_CLI_HOME", dotnet_home);
    }

    if program.eq_ignore_ascii_case("dotnet")
        && std::env::var_os("DOTNET_SKIP_FIRST_TIME_EXPERIENCE").is_none()
        && !envs
            .iter()
            .any(|(key, _)| *key == "DOTNET_SKIP_FIRST_TIME_EXPERIENCE")
    {
        command.env("DOTNET_SKIP_FIRST_TIME_EXPERIENCE", "1");
    }

    Ok(command)
}

fn run_command_streaming<'a>(
    program: &str,
    args: &[String],
    cwd: &Path,
    envs: &[(&'a str, &'a str)],
) -> Result<std::process::ExitStatus> {
    let mut command = prepare_command(program, args, cwd, envs)?;
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let child = command
        .spawn()
        .with_context(|| format!("failed to start '{program}'"))?;
    let child = Arc::new(Mutex::new(child));

    let stdout = child
        .lock()
        .expect("child mutex should not be poisoned")
        .stdout
        .take();
    let stderr = child
        .lock()
        .expect("child mutex should not be poisoned")
        .stderr
        .take();

    let stdout_reader = stdout.map(|pipe| spawn_reader(BufReader::new(pipe), true));
    let stderr_reader = stderr.map(|pipe| spawn_reader(BufReader::new(pipe), false));

    set_active_child(Some(child.clone()));
    let status = child
        .lock()
        .expect("child mutex should not be poisoned")
        .wait()
        .with_context(|| format!("failed to wait for '{program}'"))?;
    set_active_child(None);

    if let Some(reader) = stdout_reader {
        let _ = reader.join();
    }
    if let Some(reader) = stderr_reader {
        let _ = reader.join();
    }

    Ok(status)
}

fn run_command_capture<'a>(
    program: &str,
    args: &[String],
    cwd: &Path,
    envs: &[(&'a str, &'a str)],
) -> Result<std::process::Output> {
    let mut command = prepare_command(program, args, cwd, envs)?;
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = command
        .output()
        .with_context(|| format!("failed to start '{program}'"))?;

    for line in String::from_utf8_lossy(&output.stdout).lines() {
        emit_stdout_line(line.to_owned());
    }
    for line in String::from_utf8_lossy(&output.stderr).lines() {
        emit_stderr_line(line.to_owned());
    }

    Ok(output)
}

fn spawn_reader<R>(reader: BufReader<R>, stdout: bool) -> thread::JoinHandle<()>
where
    R: std::io::Read + Send + 'static,
{
    thread::spawn(move || {
        for line in reader.lines() {
            let Ok(line) = line else {
                break;
            };
            if stdout {
                emit_stdout_line(line);
            } else {
                emit_stderr_line(line);
            }
        }
    })
}
