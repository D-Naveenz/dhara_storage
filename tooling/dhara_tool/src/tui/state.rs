use std::collections::BTreeMap;
use std::path::Path;

use dhara_storage_ops::OutputStream;

use crate::command::{CommandRegistry, CommandResult, CommandSpec, ToolContext};

use super::exec::{RunCompletion, RunHandle, cancel_run, start_run};
use super::schema::CommandForm;

const QUICK_ACTIONS: &[&str] = &["verify.ci", "verify.package", "config.show", "version.bump"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Sections,
    Commands,
    Main,
    Output,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainView {
    Dashboard,
    Form,
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

pub struct AppState {
    pub repository_label: String,
    pub focus: Focus,
    pub main_view: MainView,
    pub selected_section: usize,
    pub selected_command: usize,
    pub selected_quick_action: usize,
    pub forms: BTreeMap<&'static str, CommandForm>,
    pub editing_text: bool,
    pub output_scroll: usize,
    pub session_history: Vec<HistoryEntry>,
    pub active_run: Option<RunHandle>,
    pub active_output: Vec<OutputLine>,
    pub latest_result: Option<CommandResult>,
    pub status_message: String,
    pub should_quit: bool,
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

    pub fn with_repository_label(label: impl Into<String>) -> Self {
        Self {
            repository_label: label.into(),
            focus: Focus::Main,
            main_view: MainView::Dashboard,
            selected_section: 0,
            selected_command: 0,
            selected_quick_action: 0,
            forms: BTreeMap::new(),
            editing_text: false,
            output_scroll: 0,
            session_history: Vec::new(),
            active_run: None,
            active_output: Vec::new(),
            latest_result: None,
            status_message: "Ready.".to_owned(),
            should_quit: false,
        }
    }

    pub fn repository_label_from_path(path: &Path) -> String {
        path.file_name()
            .and_then(|value| value.to_str())
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| path.display().to_string())
    }

    pub fn current_section<'a>(&self, registry: &'a CommandRegistry) -> Option<&'a str> {
        registry
            .sections()
            .nth(self.selected_section)
            .map(|section| section.name)
    }

    pub fn commands_for_current_section<'a>(
        &self,
        registry: &'a CommandRegistry,
    ) -> Vec<&'a CommandSpec> {
        let Some(section) = self.current_section(registry) else {
            return Vec::new();
        };
        registry
            .commands()
            .filter(move |command| command.section == section)
            .collect()
    }

    pub fn current_command<'a>(&self, registry: &'a CommandRegistry) -> Option<&'a CommandSpec> {
        self.commands_for_current_section(registry)
            .get(self.selected_command)
            .copied()
    }

    pub fn selected_quick_command<'a>(
        &self,
        registry: &'a CommandRegistry,
    ) -> Option<&'a CommandSpec> {
        let command_id = QUICK_ACTIONS.get(self.selected_quick_action)?;
        registry
            .commands()
            .find(|command| &command.id == command_id)
    }

    pub fn ensure_form<'a>(&mut self, command: &'a CommandSpec) -> &'a CommandSpec {
        self.forms
            .entry(command.id)
            .or_insert_with(|| CommandForm::from_command(command));
        command
    }

    pub fn selected_form<'a>(
        &'a mut self,
        command: &'a CommandSpec,
    ) -> Option<&'a mut CommandForm> {
        self.forms.get_mut(command.id)
    }

    pub fn move_sections(&mut self, registry: &CommandRegistry, delta: isize) {
        let count = registry.sections().count();
        if count == 0 {
            return;
        }
        self.selected_section = wrap_index(self.selected_section, count, delta);
        self.selected_command = 0;
    }

    pub fn move_commands(&mut self, registry: &CommandRegistry, delta: isize) {
        let count = self.commands_for_current_section(registry).len();
        if count == 0 {
            return;
        }
        self.selected_command = wrap_index(self.selected_command, count, delta);
        self.main_view = MainView::Form;
    }

    pub fn move_quick_actions(&mut self, delta: isize) {
        if QUICK_ACTIONS.is_empty() {
            return;
        }
        self.selected_quick_action =
            wrap_index(self.selected_quick_action, QUICK_ACTIONS.len(), delta);
    }

    pub fn cycle_focus(&mut self, backwards: bool) {
        self.focus = match (self.focus, backwards) {
            (Focus::Sections, false) => Focus::Commands,
            (Focus::Commands, false) => Focus::Main,
            (Focus::Main, false) => Focus::Output,
            (Focus::Output, false) => Focus::Sections,
            (Focus::Sections, true) => Focus::Output,
            (Focus::Commands, true) => Focus::Sections,
            (Focus::Main, true) => Focus::Commands,
            (Focus::Output, true) => Focus::Main,
        };
        self.editing_text = false;
    }

    pub fn activate_command(&mut self, registry: &CommandRegistry) {
        if let Some(command) = self.current_command(registry).cloned() {
            self.ensure_form(&command);
            self.main_view = MainView::Form;
            self.focus = Focus::Main;
            self.status_message = format!("Selected {}", command.path_string());
        }
    }

    pub fn activate_quick_action(&mut self, registry: &CommandRegistry) {
        if let Some(command) = self.selected_quick_command(registry).cloned() {
            self.ensure_form(&command);
            self.main_view = MainView::Form;
            self.focus = Focus::Main;
            self.status_message = format!("Selected quick action {}", command.path_string());
        }
    }

    pub fn run_selected(&mut self, registry: &CommandRegistry, context: &ToolContext) {
        if self.active_run.is_some() {
            self.status_message = "A command is already running.".to_owned();
            return;
        }

        let command = match self.main_view {
            MainView::Dashboard => self.selected_quick_command(registry).cloned(),
            MainView::Form => self.current_command(registry).cloned(),
        };
        let Some(command) = command else {
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
        self.latest_result = None;
        self.output_scroll = 0;
        self.status_message = format!("Running {}...", command.path_string());
        self.active_run = Some(start_run(
            registry.clone(),
            context.clone(),
            command_path,
            command.path_string(),
            command.ui.supports_cancel,
        ));
        self.main_view = MainView::Form;
        self.focus = Focus::Output;
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
                    self.latest_result = Some(result.clone());
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
        }
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

    pub fn output_lines(&self) -> &[OutputLine] {
        if self.active_run.is_some() {
            &self.active_output
        } else if let Some(entry) = self.session_history.last() {
            &entry.output
        } else {
            &self.active_output
        }
    }
}

fn wrap_index(current: usize, count: usize, delta: isize) -> usize {
    if count == 0 {
        return 0;
    }
    let count = count as isize;
    let current = current as isize;
    ((current + delta).rem_euclid(count)) as usize
}
