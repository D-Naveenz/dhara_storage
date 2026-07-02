use std::collections::BTreeMap;
use std::path::Path;

use dhara_tool_cli::command::{CommandRegistry, CommandResult, CommandSpec, ToolContext};
use dhara_tool_cli::forms::CommandForm;
use dhara_tool_kernel::{
    OutputStream, WorkspaceSnapshot,
    filedefs::TridBuildProgress,
    repo_config::{ConfigDriftItem, apply_config_drift},
    workspace::DefsPackageStatus,
};
use dhara_tool_cli::runner::{RunCompletion, RunHandle, cancel_run, start_run};

use super::tree::{NavTree, TreeViewState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainTab {
    Options,
    Terminal,
    History,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputLine {
    pub is_error: bool,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
    pub label: String,
    pub status: String,
    pub output: Vec<OutputLine>,
    pub result: Option<CommandResult>,
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

#[derive(Debug, Clone, Default)]
pub struct ProgressState {
    pub value: f32,
    pub label: String,
}

pub struct AppState {
    pub repository_label: String,
    pub workspace: WorkspaceSnapshot,
    pub nav_tree: NavTree,
    pub tree_view: TreeViewState,
    pub main_tab: MainTab,
    pub forms: BTreeMap<&'static str, CommandForm>,
    pub session_history: Vec<HistoryEntry>,
    pub selected_history_index: Option<usize>,
    pub active_run: Option<RunHandle>,
    pub active_output: Vec<OutputLine>,
    pub progress: Option<ProgressState>,
    pub status_message: String,
    pub should_quit: bool,
    pub activation_prompt: Option<ActivationPrompt>,
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
            main_tab: MainTab::Options,
            forms: BTreeMap::new(),
            session_history: Vec::new(),
            selected_history_index: None,
            active_run: None,
            active_output: Vec::new(),
            progress: None,
            status_message: "Ready.".to_owned(),
            should_quit: false,
            activation_prompt: None,
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
    }

    pub fn select_command(&mut self, registry: &CommandRegistry, command_id: &'static str) {
        let Some(command) = registry.commands().find(|command| command.id == command_id) else {
            return;
        };
        self.tree_view.select_command(command_id);
        self.ensure_form(command);
        self.main_tab = MainTab::Options;
        self.status_message = format!("Selected {}", command.path_string());
    }

    pub fn run_selected(&mut self, registry: &CommandRegistry, context: &ToolContext) {
        if self.active_run.is_some() {
            self.status_message = "A command is already running.".to_owned();
            return;
        }

        let Some(command) = self.selected_command(registry).cloned() else {
            self.status_message = "No command selected.".to_owned();
            return;
        };

        self.ensure_form(&command);
        let Some(form) = self.forms.get(command.id) else {
            self.status_message = "Unable to initialize command form.".to_owned();
            return;
        };
        let args = match form.build_args(&command) {
            Ok(args) => args,
            Err(error) => {
                self.status_message = error.to_string();
                return;
            }
        };

        let mut command_path = command
            .path
            .iter()
            .map(|part| (*part).to_owned())
            .collect::<Vec<_>>();
        command_path.extend(args);

        self.active_output.clear();
        self.progress = Some(ProgressState {
            value: 0.0,
            label: format!("Running {}...", command.path_string()),
        });
        self.status_message = format!("Running {}...", command.path_string());
        self.active_run = Some(start_run(
            registry.clone(),
            context.clone(),
            command_path,
            command.path_string(),
            command.ui.supports_cancel,
        ));
        self.main_tab = MainTab::Terminal;
    }

    pub fn poll_active_run(&mut self) {
        let mut completed = None;
        if let Some(run) = &mut self.active_run {
            while let Ok(event) = run.output_rx.try_recv() {
                self.active_output.push(OutputLine {
                    is_error: matches!(event.stream, OutputStream::Stderr),
                    text: event.line,
                });
            }

            if let Some(result) = run.try_take_completion() {
                completed = Some((run.label.clone(), result));
            }
        }

        if let Some((label, completion)) = completed {
            match completion {
                RunCompletion::Succeeded(result) => {
                    let status = if result.exit_code == 0 {
                        "success"
                    } else {
                        "failed"
                    };
                    self.status_message = format!("{label} completed with status {status}.");
                    self.session_history.push(HistoryEntry {
                        label,
                        status: status.to_owned(),
                        output: self.active_output.clone(),
                        result: Some(result),
                    });
                }
                RunCompletion::Failed(error) => {
                    self.status_message = error.clone();
                    self.active_output.push(OutputLine {
                        is_error: true,
                        text: error.clone(),
                    });
                    self.session_history.push(HistoryEntry {
                        label,
                        status: "failed".to_owned(),
                        output: self.active_output.clone(),
                        result: None,
                    });
                }
            }
            self.active_run = None;
            self.progress = None;
        }
    }

    pub fn apply_progress_update(&mut self, update: TridBuildProgress) {
        if self.active_run.is_none() {
            return;
        }

        let label = progress_label(&update);
        let value = match update.total.filter(|total| *total > 0) {
            Some(total) => (update.current as f32 / total as f32).clamp(0.0, 1.0),
            None => self.progress.as_ref().map(|state| state.value).unwrap_or(0.0),
        };

        self.progress = Some(ProgressState { value, label });
    }

    pub fn cancel_active(&mut self) {
        let Some(run) = &self.active_run else {
            self.status_message = "No active command to cancel.".to_owned();
            return;
        };
        if !run.cancelable {
            self.status_message = "The active command cannot be canceled safely.".to_owned();
            return;
        }
        if cancel_run() {
            self.status_message = "Sent cancellation request to the active subprocess.".to_owned();
        } else {
            self.status_message =
                "The active command is running, but no cancelable subprocess is active yet."
                    .to_owned();
        }
    }

    pub fn terminal_lines(&self) -> &[OutputLine] {
        &self.active_output
    }

    pub fn history_preview_lines(&self) -> &[OutputLine] {
        let Some(index) = self.selected_history_index else {
            return &[];
        };
        self.session_history
            .get(index)
            .map(|entry| entry.output.as_slice())
            .unwrap_or(&[])
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
        Ok(())
    }

    pub fn decline_activation(&mut self) {
        self.activation_prompt = None;
        self.should_quit = true;
        self.status_message =
            "Activation declined. Update manifests manually or relaunch with --yes.".to_owned();
    }
}

fn progress_label(update: &TridBuildProgress) -> String {
    use dhara_tool_kernel::filedefs::TridBuildStage;

    let stage = match update.stage {
        TridBuildStage::LoadSource => "load",
        TridBuildStage::ExtractArchive => "extract",
        TridBuildStage::ParseDefinitions => "parse",
        TridBuildStage::ReduceDefinitions => "reduce",
        TridBuildStage::FinalizePackage => "finalize",
    };

    if let Some(total) = update.total.filter(|total| *total > 0) {
        format!("{stage}: {}/{} — {}", update.current, total, update.message)
    } else if update.message.is_empty() {
        stage.to_owned()
    } else {
        format!("{stage}: {}", update.message)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use anyhow::Result;

    use dhara_tool_cli::command::{
        CommandRegistry, CommandResult, CommandSpec, CommandUi, RunMode, SectionSpec, ToolContext,
    };
    use dhara_tool_kernel::filedefs::{
        TridBuildProgress, TridBuildStage, TridBuildStats,
    };
    use dhara_tool_kernel::{WorkspaceSnapshot, workspace::DefsPackageStatus};
    use dhara_tool_cli::runner::start_run;

    use super::{AppState, MainTab};

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
    fn poll_active_run_records_history_on_completion() {
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
        state.main_tab = MainTab::Terminal;

        loop {
            state.poll_active_run();
            if state.active_run.is_none() {
                break;
            }
        }

        assert_eq!(state.session_history.len(), 1);
        assert_eq!(state.session_history[0].status, "success");
        assert!(state.progress.is_none());
    }

    #[test]
    fn apply_progress_update_sets_bar_value() {
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

        state.apply_progress_update(TridBuildProgress {
            stage: TridBuildStage::ParseDefinitions,
            message: "Parsing".to_owned(),
            current: 50,
            total: Some(100),
            current_item: None,
            stats: TridBuildStats::default(),
            trace_detail: None,
        });

        let progress = state.progress.expect("progress set");
        assert!((progress.value - 0.5).abs() < f32::EPSILON);
        assert!(progress.label.contains("parse"));
    }
}
