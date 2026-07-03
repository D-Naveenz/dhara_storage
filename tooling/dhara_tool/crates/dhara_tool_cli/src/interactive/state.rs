use std::collections::BTreeMap;
use std::path::Path;

use dhara_tool_kernel::{
    OutputStream, ProgressSnapshot, WorkspaceSnapshot,
    repo_config::{ConfigDriftItem, apply_config_drift},
    workspace::DefsPackageStatus,
};

use crate::command::{CommandRegistry, CommandSpec, ToolContext};
use crate::forms::CommandForm;
use crate::runner::{RunCompletion, RunHandle, cancel_run, start_run};

use super::tree::{NavTree, TreeViewState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainTab {
    Info,
    Options,
    Troubleshooting,
    SystemConfigs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusTone {
    Ready,
    Running,
    Success,
    Failed,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Warn,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticLine {
    pub severity: DiagnosticSeverity,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct ActivationPrompt {
    pub drifts: Vec<ConfigDriftItem>,
}

impl ActivationPrompt {
    pub fn new(drifts: Vec<ConfigDriftItem>) -> Self {
        Self { drifts }
    }
}

pub struct AppState {
    pub repository_label: String,
    pub workspace: WorkspaceSnapshot,
    pub nav_tree: NavTree,
    pub tree_view: TreeViewState,
    pub main_tab: MainTab,
    pub forms: BTreeMap<&'static str, CommandForm>,
    pub active_run: Option<RunHandle>,
    pub troubleshooting_lines: Vec<DiagnosticLine>,
    pub progress: Option<ProgressSnapshot>,
    pub status_message: String,
    pub status_tone: StatusTone,
    pub should_quit: bool,
    pub activation_prompt: Option<ActivationPrompt>,
    pub system_configs_text: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        Self::with_repository_label("workspace")
    }

    pub fn with_workspace(
        label: impl Into<String>,
        workspace: WorkspaceSnapshot,
        registry: &CommandRegistry,
    ) -> Self {
        Self {
            repository_label: label.into(),
            workspace,
            nav_tree: NavTree::from_registry(registry),
            tree_view: TreeViewState::new(registry),
            main_tab: MainTab::Info,
            forms: BTreeMap::new(),
            active_run: None,
            troubleshooting_lines: Vec::new(),
            progress: None,
            status_message: "Ready.".to_owned(),
            status_tone: StatusTone::Ready,
            should_quit: false,
            activation_prompt: None,
            system_configs_text: None,
        }
    }

    pub fn with_repository_label(label: impl Into<String>) -> Self {
        let registry = CommandRegistry::new();
        Self::with_workspace(
            label,
            WorkspaceSnapshot {
                defs_path: Path::new("src/core/dhara_storage_dal/resources/filedefs.dat")
                    .to_path_buf(),
                defs_status: DefsPackageStatus::Missing,
                package_revision: None,
                definitions_release: None,
                package_version: None,
                definition_count: None,
            },
            &registry,
        )
    }

    pub fn repository_label_from_path(path: &Path) -> String {
        path.file_name()
            .and_then(|value| value.to_str())
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| path.display().to_string())
    }

    pub fn selected_command<'a>(&self, registry: &'a CommandRegistry) -> Option<&'a CommandSpec> {
        let command_id = self.tree_view.selected_command_id?;
        registry.commands().find(|command| command.id == command_id)
    }

    pub fn ensure_form(&mut self, command: &CommandSpec) {
        self.forms
            .entry(command.id)
            .or_insert_with(|| CommandForm::from_command(command));
    }

    pub fn reset_form(&mut self, command: &CommandSpec) {
        self.forms
            .insert(command.id, CommandForm::from_command(command));
        self.status_message = format!("Reset options for {}", command.path_string());
        self.status_tone = StatusTone::Ready;
    }

    pub fn select_command(&mut self, registry: &CommandRegistry, command_id: &'static str) {
        let Some(command) = registry.commands().find(|command| command.id == command_id) else {
            return;
        };
        self.tree_view.select_command(command_id);
        self.ensure_form(command);
        self.main_tab = MainTab::Info;
        self.status_message = format!("Selected {}", command.path_string());
        self.status_tone = StatusTone::Ready;
    }

    pub fn run_selected(&mut self, registry: &CommandRegistry, context: &ToolContext) {
        if self.active_run.is_some() {
            self.status_message = "A command is already running.".to_owned();
            self.status_tone = StatusTone::Warning;
            return;
        }

        let Some(command) = self.selected_command(registry).cloned() else {
            self.status_message = "No command selected.".to_owned();
            self.status_tone = StatusTone::Warning;
            return;
        };

        self.ensure_form(&command);
        let Some(form) = self.forms.get(command.id) else {
            self.status_message = "Unable to initialize command form.".to_owned();
            self.status_tone = StatusTone::Warning;
            return;
        };
        let args = match form.build_args(&command) {
            Ok(args) => args,
            Err(error) => {
                self.status_message = error.to_string();
                self.status_tone = StatusTone::Warning;
                return;
            }
        };

        let mut command_path = command
            .path
            .iter()
            .map(|part| (*part).to_owned())
            .collect::<Vec<_>>();
        command_path.extend(args);

        self.troubleshooting_lines.clear();
        self.progress = None;
        self.status_message = format!("Running {}...", command.path_string());
        self.status_tone = StatusTone::Running;
        self.active_run = Some(start_run(
            registry.clone(),
            context.clone(),
            command_path,
            command.path_string(),
            command.ui.supports_cancel,
        ));
        self.main_tab = MainTab::Troubleshooting;
    }

    pub fn poll_active_run(&mut self) {
        let mut completed = None;
        if let Some(run) = &mut self.active_run {
            while let Ok(event) = run.output_rx.try_recv() {
                match event.stream {
                    OutputStream::Stderr => {
                        self.troubleshooting_lines.push(DiagnosticLine {
                            severity: DiagnosticSeverity::Error,
                            text: event.line,
                        });
                    }
                    OutputStream::Warn => {
                        self.troubleshooting_lines.push(DiagnosticLine {
                            severity: DiagnosticSeverity::Warn,
                            text: event.line,
                        });
                    }
                    OutputStream::Stdout => {}
                }
            }

            if let Some(result) = run.try_take_completion() {
                completed = Some((run.label.clone(), result));
            }
        }

        if let Some((label, completion)) = completed {
            match completion {
                RunCompletion::Succeeded(result) => {
                    let success = result.exit_code == 0;
                    self.status_tone = if success {
                        StatusTone::Success
                    } else {
                        StatusTone::Failed
                    };
                    let status = if success { "success" } else { "failed" };
                    self.status_message = format!("{label} completed with status {status}.");
                }
                RunCompletion::Failed(error) => {
                    self.status_tone = StatusTone::Failed;
                    self.status_message = error.clone();
                    self.troubleshooting_lines.push(DiagnosticLine {
                        severity: DiagnosticSeverity::Error,
                        text: error,
                    });
                }
            }
            self.active_run = None;
            self.progress = None;
        }
    }

    pub fn apply_progress_snapshot(&mut self, snapshot: ProgressSnapshot) {
        if self.active_run.is_none() && snapshot.phase != dhara_tool_kernel::RunPhase::Complete {
            return;
        }
        self.progress = Some(snapshot);
    }

    pub fn cancel_active(&mut self) {
        let Some(run) = &self.active_run else {
            self.status_message = "No active command to cancel.".to_owned();
            self.status_tone = StatusTone::Warning;
            return;
        };
        if !run.cancelable {
            self.status_message = "The active command cannot be canceled safely.".to_owned();
            self.status_tone = StatusTone::Warning;
            return;
        }
        if cancel_run() {
            self.status_message = "Sent cancellation request to the active subprocess.".to_owned();
            self.status_tone = StatusTone::Warning;
        } else {
            self.status_message =
                "The active command is running, but no cancelable subprocess is active yet."
                    .to_owned();
            self.status_tone = StatusTone::Warning;
        }
    }

    pub fn apply_activation_confirm(
        &mut self,
        repo_root: &std::path::Path,
    ) -> anyhow::Result<()> {
        let Some(prompt) = self.activation_prompt.take() else {
            return Ok(());
        };
        apply_config_drift(repo_root, &prompt.drifts)?;
        self.status_message =
            "Configuration drift applied from dhara.config.toml.".to_owned();
        self.status_tone = StatusTone::Success;
        Ok(())
    }

    pub fn decline_activation(&mut self) {
        self.activation_prompt = None;
        self.should_quit = true;
        self.status_message =
            "Activation declined. Update manifests manually or relaunch with --yes.".to_owned();
        self.status_tone = StatusTone::Warning;
    }

    pub fn invalidate_system_configs(&mut self) {
        self.system_configs_text = None;
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use anyhow::Result;

    use dhara_tool_kernel::{
        ProgressSnapshot, RunPhase, WorkspaceSnapshot, workspace::DefsPackageStatus,
    };

    use crate::command::{
        CommandRegistry, CommandResult, CommandSpec, CommandUi, RunMode, SectionSpec, ToolContext,
    };
    use crate::runner::start_run;

    use super::{AppState, MainTab, StatusTone};

    fn noop(_: &ToolContext, _: &[String]) -> Result<CommandResult> {
        Ok(CommandResult::with_message("done"))
    }

    fn registry_with_show() -> CommandRegistry {
        let mut registry = CommandRegistry::new();
        registry.add_section(SectionSpec {
            name: "config",
            prompt: "cfg> ",
            summary: "Config",
        });
        registry.add_command(CommandSpec {
            id: "config.show",
            path: &["config", "show"],
            summary: "Show",
            args_summary: "",
            section: "config",
            ui: CommandUi::empty("Show"),
            handler: Arc::new(noop),
        });
        registry
    }

    fn test_workspace() -> WorkspaceSnapshot {
        WorkspaceSnapshot {
            defs_path: std::path::Path::new("filedefs.dat").to_path_buf(),
            defs_status: DefsPackageStatus::Missing,
            package_revision: None,
            definitions_release: None,
            package_version: None,
            definition_count: None,
        }
    }

    #[test]
    fn poll_active_run_sets_success_tone() {
        let registry = registry_with_show();
        let mut state = AppState::with_workspace("repo", test_workspace(), &registry);
        state.select_command(&registry, "config.show");

        let context = ToolContext {
            repo_root: ".".into(),
            tool_root: ".".into(),
            run_mode: RunMode::Interactive,
            min: false,
            trace: false,
            workers: 4,
            package_dir: None,
            output_dir: None,
            logs_dir: None,
        };

        state.active_run = Some(start_run(
            registry.clone(),
            context,
            vec!["config".to_owned(), "show".to_owned()],
            "config show".to_owned(),
            false,
        ));
        state.main_tab = MainTab::Troubleshooting;

        loop {
            state.poll_active_run();
            if state.active_run.is_none() {
                break;
            }
        }

        assert_eq!(state.status_tone, StatusTone::Success);
        assert!(state.progress.is_none());
    }

    #[test]
    fn apply_progress_snapshot_updates_bar() {
        let registry = registry_with_show();
        let mut state = AppState::with_workspace("repo", test_workspace(), &registry);
        state.active_run = Some(start_run(
            registry.clone(),
            ToolContext {
                repo_root: ".".into(),
                tool_root: ".".into(),
                run_mode: RunMode::Interactive,
                min: false,
                trace: false,
                workers: 4,
                package_dir: None,
                output_dir: None,
                logs_dir: None,
            },
            vec!["config".to_owned(), "show".to_owned()],
            "config show".to_owned(),
            false,
        ));

        state.apply_progress_snapshot(ProgressSnapshot {
            phase: RunPhase::Running,
            overall: 0.42,
            percent: 42,
            active_step: Some("parse"),
            step_label: "parse: 42/100".to_owned(),
            analyzing_message: String::new(),
        });

        let progress = state.progress.expect("progress set");
        assert_eq!(progress.percent, 42);
        assert!(progress.step_label.contains("parse"));
    }
}
