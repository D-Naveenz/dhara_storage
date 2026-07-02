use iced::widget::{column, row};
use iced::{Element, Length};

use dhara_tool_cli::command::CommandRegistry;

use super::super::app::Message;
use super::super::state::AppState;
use super::button::action_button;
use super::panel::{inset_panel, PanelVariant};
use super::progress::labeled_progress;

pub fn action_bar<'a>(
    state: &'a AppState,
    registry: &'a CommandRegistry,
) -> Element<'a, Message> {
    let progress_value = state
        .progress
        .as_ref()
        .map(|progress| progress.value)
        .unwrap_or(0.0);
    let progress_label = state
        .progress
        .as_ref()
        .map(|progress| progress.label.as_str())
        .unwrap_or(&state.status_message);

    let running = state.active_run.is_some();
    let cancelable = state
        .active_run
        .as_ref()
        .is_some_and(|run| run.cancelable);

    let can_reset = state.selected_command(registry).is_some();

    let mut actions = row![
        action_button("Run", (!running).then_some(Message::RunPressed)),
        action_button(
            "Cancel",
            (running && cancelable).then_some(Message::CancelPressed),
        ),
        action_button(
            "Reset",
            (can_reset && !running).then_some(Message::ResetFormPressed),
        ),
    ]
    .spacing(8);

    if state.activation_prompt.is_some() {
        actions = actions.push(action_button(
            "Apply drift",
            Some(Message::ActivationConfirm),
        ));
        actions = actions.push(action_button("Decline", Some(Message::ActivationDecline)));
    }

    inset_panel(
        column![
            labeled_progress(progress_value, progress_label),
            actions,
        ]
        .spacing(8)
        .width(Length::Fill),
        PanelVariant::Default,
    )
}
