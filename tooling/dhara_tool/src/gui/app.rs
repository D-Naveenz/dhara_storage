use std::sync::{Arc, Mutex};
use std::sync::mpsc::{self, Receiver};
use iced::time;

use anyhow::Result;
use iced::widget::{column, container, row, stack};
use iced::{Length, Subscription, Task, Theme};

use crate::command::{CommandRegistry, ToolContext};
use crate::ensure_workspace_state;
use crate::filedefs::TridBuildProgress;
use crate::logging::{register_gui_progress_sender, unregister_gui_progress_sender};
use crate::repo_config::ConfigDriftItem;
use crate::ui::FormValue;

use super::panels::{
    view_action_bar, view_activation_overlay, view_header, view_main_tabs, view_tab_content,
    view_tree_nav,
};
use super::state::{ActivationPrompt, AppState, MainTab};

pub struct DharaApp {
    pub state: AppState,
    pub registry: CommandRegistry,
    pub context: ToolContext,
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
    RunPressed,
    CancelPressed,
    ResetFormPressed,
    Tick,
    ProgressUpdate(TridBuildProgress),
    HistorySelected(usize),
    ActivationConfirm,
    ActivationDecline,
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
    context: &ToolContext,
    pending_activation: Vec<ConfigDriftItem>,
) -> Result<()> {
    let (progress_tx, progress_rx) = mpsc::channel();
    register_gui_progress_sender(progress_tx);
    let progress_rx = Arc::new(Mutex::new(progress_rx));
    let registry_boot = Arc::new(registry.clone());
    let context_boot = Arc::new(context.clone());
    let pending_boot = Arc::new(pending_activation);

    let boot = {
        let registry_boot = Arc::clone(&registry_boot);
        let context_boot = Arc::clone(&context_boot);
        let pending_boot = Arc::clone(&pending_boot);
        let progress_rx = Arc::clone(&progress_rx);
        move || {
            let workspace = ensure_workspace_state(&context_boot);
            let mut state = AppState::with_workspace(
                AppState::repository_label_from_path(&context_boot.repo_root),
                workspace,
                &registry_boot,
            );
            if !pending_boot.is_empty() {
                state.activation_prompt = Some(ActivationPrompt::new(pending_boot.as_ref().clone()));
                state.status_message =
                    "Configuration drift detected. Confirm activation to continue.".to_owned();
            }

            (
                DharaApp {
                    state,
                    registry: registry_boot.as_ref().clone(),
                    context: context_boot.as_ref().clone(),
                    progress_rx: Arc::clone(&progress_rx),
                },
                Task::none(),
            )
        }
    };

    let run_result = iced::application(boot, update, view)
        .title("Dhara Tool")
        .subscription(subscription)
        .theme(Theme::Dark)
        .window(iced::window::Settings {
            size: iced::Size::new(1200.0, 800.0),
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

fn update(app: &mut DharaApp, message: Message) -> Task<Message> {
    match message {
        Message::CommandSelected(command_id) => {
            app.state
                .select_command(&app.registry, command_id);
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
            let crate::command::FieldKind::Select(options) = field.kind else {
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
            app.state
                .run_selected(&app.registry, &app.context);
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
            if let Err(error) = app
                .state
                .apply_activation_confirm(&app.context.repo_root)
            {
                app.state.status_message = error.to_string();
            }
        }
        Message::ActivationDecline => {
            app.state.decline_activation();
        }
    }

    Task::none()
}

fn view(app: &DharaApp) -> iced::Element<'_, Message> {
    let body = row![
        container(view_tree_nav(&app.state, &app.registry))
            .width(Length::FillPortion(2))
            .height(Length::Fill)
            .padding(4),
        container(
            column![
                view_main_tabs(app.state.main_tab),
                container(view_tab_content(&app.state, &app.registry))
                    .height(Length::FillPortion(3))
                    .width(Length::Fill),
                container(view_action_bar(&app.state, &app.registry))
                    .height(Length::FillPortion(1))
                    .width(Length::Fill),
            ]
            .spacing(4),
        )
        .width(Length::FillPortion(5))
        .height(Length::Fill),
    ]
    .spacing(8)
    .padding(8)
    .height(Length::Fill);

    let layout = column![view_header(&app.state), body]
        .spacing(4)
        .height(Length::Fill);

    let main = container(layout)
        .width(Length::Fill)
        .height(Length::Fill);

    if let Some(overlay) = view_activation_overlay(&app.state) {
        stack![main, overlay].width(Length::Fill).height(Length::Fill).into()
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
