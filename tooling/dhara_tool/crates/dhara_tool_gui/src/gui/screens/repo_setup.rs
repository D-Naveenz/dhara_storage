use iced::widget::{button, column, row, text};
use iced::Element;

use crate::gui::widgets::modal_overlay::modal_overlay;
use crate::gui::widgets::path_field::browsable_path_field;

use super::super::app::Message;

#[derive(Debug, Clone)]
pub struct RepoSetupPrompt {
    pub path_text: String,
    pub error: Option<String>,
}

impl RepoSetupPrompt {
    pub fn new(initial_path: Option<String>) -> Self {
        Self {
            path_text: initial_path.unwrap_or_default(),
            error: None,
        }
    }
}

pub fn view_repo_setup_overlay<'a>(prompt: &'a RepoSetupPrompt) -> Element<'a, Message> {
    let mut lines = column![
        text("Select repository").size(16),
        text("Choose the folder containing dhara.config.toml or browse to the file.").size(13),
        browsable_path_field(
            &prompt.path_text,
            "Repository path",
            Message::RepoPathChanged,
            Message::RepoBrowsePressed,
        ),
    ]
    .spacing(8);

    if let Some(error) = &prompt.error {
        lines = lines.push(text(error).size(12));
    }

    lines = lines.push(
        row![
            button("Continue").on_press(Message::RepoConfirm),
            button("Cancel").on_press(Message::RepoCancel),
        ]
        .spacing(8),
    );

    modal_overlay(lines.into())
}
