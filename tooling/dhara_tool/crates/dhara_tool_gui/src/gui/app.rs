use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{self, Receiver};

use anyhow::Result;
use iced::time;
use iced::widget::{column, container, row, stack};
use iced::{Length, Subscription, Task, Theme};

use dhara_tool_cli::command::{CommandRegistry, FieldKind, RunMode, ToolContext};
use dhara_tool_cli::forms::FormValue;
use dhara_tool_kernel::{
    activation::run_activation,
    ensure_workspace_state,
    filedefs::TridBuildProgress,
    logging::{register_gui_progress_sender, unregister_gui_progress_sender},
    repo_config::ConfigDriftItem,
    resolve_and_persist_repository,
};

use super::boot::GuiBootParams;
use super::panels::{
    view_action_bar, view_activation_overlay, view_tab_bar, view_tab_content, view_tree_nav,
};
use super::screens::{RepoSetupPrompt, view_repo_setup_overlay};
use super::state::{ActivationPrompt, AppState, MainTab};
use super::style::tab_content_panel;

pub struct DharaApp {
    pub state: AppState,
    pub registry: CommandRegistry,
    pub exe_root: PathBuf,
    pub boot: GuiBootParams,
    pub context: Option<ToolContext>,
    pub repo_setup: Option<RepoSetupPrompt>,
    pub progress_rx: Arc<Mutex<Receiver<TridBuildProgress>>>,
}

#[derive(Debug, Clone)]
pub enum Message {
    CommandSelected(&'static str),
    TreeToggleExpand(String),
    TabSelected(MainTab),
    FormTextChanged {
        command_id: &'static str,
        field_index: usize,
        value: String,
    },
    FormBooleanChanged {
        command_id: &'static str,
        field_index: usize,
        value: bool,
    },
    FormSelectChanged {
        command_id: &'static str,
        field_index: usize,
        value: String,
    },
    FormBrowsePressed {
        command_id: &'static str,
        field_index: usize,
    },
    FormBrowseResult {
        command_id: &'static str,
        field_index: usize,
        path: Option<PathBuf>,
    },
    RunPressed,
    CancelPressed,
    ResetFormPressed,
    Tick,
    ProgressUpdate(TridBuildProgress),
    HistorySelected(usize),
    ActivationConfirm,
    ActivationDecline,
    RepoPathChanged(String),
    RepoBrowsePressed,
    RepoBrowseResult(Option<PathBuf>),
    RepoConfirm,
    RepoCancel,
}

pub fn can_launch_gui() -> bool {
    #[cfg(windows)]
    {
        true
    }
    #[cfg(not(windows))]
    {
        std::env::var_os("DISPLAY").is_some() || std::env::var_os("WAYLAND_DISPLAY").is_some()
    }
}

pub fn run_gui(
    registry: &CommandRegistry,
    exe_root: PathBuf,
    boot: GuiBootParams,
    initial_context: Option<ToolContext>,
    pending_activation: Vec<ConfigDriftItem>,
    stale_repository_hint: Option<PathBuf>,
) -> Result<()> {
    let (progress_tx, progress_rx) = mpsc::channel();
    register_gui_progress_sender(progress_tx);
    let progress_rx = Arc::new(Mutex::new(progress_rx));
    let registry_boot = Arc::new(registry.clone());
    let exe_boot = Arc::new(exe_root);
    let boot_boot = Arc::new(boot);
    let context_boot = Arc::new(initial_context);
    let pending_boot = Arc::new(pending_activation);
    let stale_boot = Arc::new(stale_repository_hint);

    let boot_fn = {
        let registry_boot = Arc::clone(&registry_boot);
        let exe_boot = Arc::clone(&exe_boot);
        let boot_boot = Arc::clone(&boot_boot);
        let context_boot = Arc::clone(&context_boot);
        let pending_boot = Arc::clone(&pending_boot);
        let stale_boot = Arc::clone(&stale_boot);
        let progress_rx = Arc::clone(&progress_rx);
        move || {
            if let Some(context) = context_boot.as_ref().clone() {
                let workspace = ensure_workspace_state(&context);
                let mut state = AppState::with_workspace(
                    AppState::repository_label_from_path(&context.repo_root),
                    workspace,
                    &registry_boot,
                );
                if !pending_boot.is_empty() {
                    state.activation_prompt =
                        Some(ActivationPrompt::new(pending_boot.as_ref().clone()));
                    state.status_message =
                        "Configuration drift detected. Confirm activation to continue."
                            .to_owned();
                }

                (
                    DharaApp {
                        state,
                        registry: registry_boot.as_ref().clone(),
                        exe_root: exe_boot.as_ref().clone(),
                        boot: boot_boot.as_ref().clone(),
                        context: Some(context),
                        repo_setup: None,
                        progress_rx: Arc::clone(&progress_rx),
                    },
                    Task::none(),
                )
            } else {
                let initial = stale_boot
                    .as_ref()
                    .as_ref()
                    .map(|path| path.display().to_string());
                (
                    DharaApp {
                        state: AppState::with_repository_label("repository required"),
                        registry: registry_boot.as_ref().clone(),
                        exe_root: exe_boot.as_ref().clone(),
                        boot: boot_boot.as_ref().clone(),
                        context: None,
                        repo_setup: Some(RepoSetupPrompt::new(initial)),
                        progress_rx: Arc::clone(&progress_rx),
                    },
                    Task::none(),
                )
            }
        }
    };

    let run_result = iced::application(boot_fn, update, view)
        .title(|app: &DharaApp| {
            let label = app
                .context
                .as_ref()
                .map(|ctx| AppState::repository_label_from_path(&ctx.repo_root))
                .unwrap_or_else(|| "select repository".to_owned());
            format!("Dhara Tool - v{} | {}", env!("CARGO_PKG_VERSION"), label)
        })
        .subscription(subscription)
        .theme(Theme::Dark)
        .window(iced::window::Settings {
            size: iced::Size::new(960.0, 640.0),
            ..Default::default()
        })
        .exit_on_close_request(true)
        .run();

    unregister_gui_progress_sender();
    run_result.map_err(|error| anyhow::anyhow!("{error}"))
}

fn subscription(_app: &DharaApp) -> Subscription<Message> {
    time::every(time::Duration::from_millis(100)).map(|_| Message::Tick)
}

fn pick_config_file() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_title("Select dhara.config.toml")
        .add_filter("Dhara config", &["toml"])
        .pick_file()
        .map(|path| path)
}

fn update(app: &mut DharaApp, message: Message) -> Task<Message> {
    match message {
        Message::RepoPathChanged(value) => {
            if let Some(prompt) = app.repo_setup.as_mut() {
                prompt.path_text = value;
                prompt.error = None;
            }
        }
        Message::RepoBrowsePressed => {
            return Task::perform(async { pick_config_file() }, Message::RepoBrowseResult);
        }
        Message::RepoBrowseResult(path) => {
            if let (Some(prompt), Some(path)) = (app.repo_setup.as_mut(), path) {
                prompt.path_text = path.display().to_string();
                prompt.error = None;
            }
        }
        Message::RepoCancel => {
            app.state.should_quit = true;
        }
        Message::RepoConfirm => {
            let Some(prompt) = app.repo_setup.as_ref() else {
                return Task::none();
            };
            if prompt.path_text.trim().is_empty() {
                if let Some(prompt) = app.repo_setup.as_mut() {
                    prompt.error = Some("Repository path is required.".to_owned());
                }
                return Task::none();
            }

            match resolve_and_persist_repository(
                &app.exe_root,
                PathBuf::from(prompt.path_text.trim()),
                true,
            ) {
                Ok(repo_root) => match finish_repository_setup(app, repo_root) {
                    Ok(()) => {}
                    Err(error) => {
                        if let Some(prompt) = app.repo_setup.as_mut() {
                            prompt.error = Some(error.to_string());
                        }
                    }
                },
                Err(error) => {
                    if let Some(prompt) = app.repo_setup.as_mut() {
                        prompt.error = Some(error.to_string());
                    }
                }
            }
        }
        Message::FormBrowsePressed {
            command_id,
            field_index,
        } => {
            return Task::perform(
                async move { pick_config_file() },
                move |path| Message::FormBrowseResult {
                    command_id,
                    field_index,
                    path,
                },
            );
        }
        Message::FormBrowseResult {
            command_id,
            field_index,
            path,
        } => {
            if let Some(path) = path
                && let Some(form) = app.state.forms.get_mut(command_id)
                && let Some(FormValue::Text(current)) = form.values.get_mut(field_index)
            {
                *current = path.display().to_string();
            }
        }
        Message::CommandSelected(command_id) => {
            app.state.select_command(&app.registry, command_id);
        }
        Message::TreeToggleExpand(path_key) => {
            app.state.tree_view.toggle_expanded(&path_key);
        }
        Message::TabSelected(tab) => {
            app.state.main_tab = tab;
        }
        Message::FormTextChanged {
            command_id,
            field_index,
            value,
        } => {
            if let Some(form) = app.state.forms.get_mut(command_id)
                && let Some(FormValue::Text(current)) = form.values.get_mut(field_index)
            {
                *current = value;
            }
        }
        Message::FormBooleanChanged {
            command_id,
            field_index,
            value,
        } => {
            if let Some(form) = app.state.forms.get_mut(command_id)
                && let Some(FormValue::Boolean(current)) = form.values.get_mut(field_index)
            {
                *current = value;
            }
        }
        Message::FormSelectChanged {
            command_id,
            field_index,
            value,
        } => {
            let Some(command) = app.registry.commands().find(|c| c.id == command_id) else {
                return Task::none();
            };
            let Some(field) = command.ui.fields.get(field_index) else {
                return Task::none();
            };
            let FieldKind::Select(options) = field.kind else {
                return Task::none();
            };
            if let Some(form) = app.state.forms.get_mut(command_id)
                && let Some(FormValue::Select(selected)) = form.values.get_mut(field_index)
            {
                *selected = options
                    .iter()
                    .position(|option| *option == value)
                    .unwrap_or(0);
            }
        }
        Message::RunPressed => {
            if let Some(context) = app.context.as_ref() {
                app.state.run_selected(&app.registry, context);
            }
        }
        Message::CancelPressed => {
            app.state.cancel_active();
        }
        Message::ResetFormPressed => {
            if let Some(command) = app.state.selected_command(&app.registry).cloned() {
                app.state.reset_form(&command);
            }
        }
        Message::Tick => {
            app.state.poll_active_run();
            while let Ok(update) = app.progress_rx.lock().expect("progress rx lock").try_recv() {
                app.state.apply_progress_update(update);
            }
            if app.state.should_quit {
                return iced::exit();
            }
        }
        Message::ProgressUpdate(update) => {
            app.state.apply_progress_update(update);
        }
        Message::HistorySelected(index) => {
            app.state.selected_history_index = Some(index);
            app.state.main_tab = MainTab::History;
        }
        Message::ActivationConfirm => {
            if let Some(context) = app.context.as_ref() {
                if let Err(error) = app
                    .state
                    .apply_activation_confirm(&context.repo_root)
                {
                    app.state.status_message = error.to_string();
                }
            }
        }
        Message::ActivationDecline => {
            app.state.decline_activation();
        }
    }

    Task::none()
}

fn finish_repository_setup(app: &mut DharaApp, repo_root: PathBuf) -> Result<()> {
    let pending = run_activation(
        &repo_root,
        app.boot.yes,
        RunMode::Interactive,
    )?
    .unwrap_or_default();

    let context = ToolContext {
        repo_root: repo_root.clone(),
        tool_root: app.exe_root.clone(),
        run_mode: RunMode::Interactive,
        min: app.boot.min,
        trace: app.boot.trace,
        workers: app.boot.workers,
        package_dir: app.boot.package_dir.clone(),
        output_dir: app.boot.output_dir.clone(),
        logs_dir: app.boot.logs_dir.clone(),
    };
    let workspace = ensure_workspace_state(&context);
    let mut state = AppState::with_workspace(
        AppState::repository_label_from_path(&repo_root),
        workspace,
        &app.registry,
    );
    if !pending.is_empty() {
        state.activation_prompt = Some(ActivationPrompt::new(pending));
        state.status_message =
            "Configuration drift detected. Confirm activation to continue.".to_owned();
    }

    app.state = state;
    app.context = Some(context);
    app.repo_setup = None;
    Ok(())
}

fn view(app: &DharaApp) -> iced::Element<'_, Message> {
    if let Some(prompt) = app.repo_setup.as_ref() {
        return view_repo_setup_overlay(prompt);
    }

    let tab_stack = column![
        view_tab_bar(app.state.main_tab),
        container(view_tab_content(&app.state, &app.registry))
            .height(Length::Fill)
            .width(Length::Fill)
            .style(|theme: &Theme| tab_content_panel(theme)),
    ]
    .spacing(0)
    .height(Length::Fill);

    let right_column = column![tab_stack, view_action_bar(&app.state, &app.registry)]
        .spacing(8)
        .width(Length::FillPortion(5))
        .height(Length::Fill);

    let body = row![
        container(view_tree_nav(&app.state, &app.registry))
            .width(Length::FillPortion(2))
            .height(Length::Fill),
        right_column,
    ]
    .spacing(8)
    .padding(12)
    .height(Length::Fill);

    let main = container(body)
        .width(Length::Fill)
        .height(Length::Fill);

    if let Some(overlay) = view_activation_overlay(&app.state) {
        stack![main, overlay]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    } else {
        main.into()
    }
}

#[cfg(test)]
mod tests {
    use super::can_launch_gui;

    #[test]
    fn can_launch_gui_is_callable() {
        let _ = can_launch_gui();
    }
}
