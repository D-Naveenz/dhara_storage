use iced::widget::{button, column, row, text};
use iced::Element;

use super::super::app::Message;
use super::super::state::AppState;
use super::super::widgets::modal_overlay::modal_overlay;

pub fn view_activation_overlay<'a>(state: &'a AppState) -> Option<Element<'a, Message>> {
    let prompt = state.activation_prompt.as_ref()?;
    let mut lines = column![
        text("Configuration drift detected").size(16),
        text("Apply updates from dhara.config.toml?").size(13),
    ]
    .spacing(8);

    for drift in &prompt.drifts {
        lines = lines.push(text(format!("• {}", drift.summary)).size(12));
    }

    lines = lines.push(
        row![
            button("Apply").on_press(Message::ActivationConfirm),
            button("Decline").on_press(Message::ActivationDecline),
        ]
        .spacing(8),
    );

    Some(modal_overlay(lines.into()))
}
