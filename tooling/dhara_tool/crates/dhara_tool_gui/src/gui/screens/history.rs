use iced::widget::{column, container, row, scrollable, text};
use iced::{Element, Length};

use super::super::app::Message;
use super::super::state::AppState;
use super::super::widgets::button::history_button;
use super::terminal::view_output_lines;

pub fn view_history<'a>(state: &'a AppState) -> Element<'a, Message> {
    let mut layout = row![].spacing(8).width(Length::Fill).height(Length::Fill);

    let mut history_list = column![text("Recent runs").size(14)]
        .spacing(4)
        .width(Length::FillPortion(1));
    if state.session_history.is_empty() {
        history_list = history_list.push(text("No history yet.").size(13));
    } else {
        for (index, entry) in state.session_history.iter().enumerate().rev() {
            let selected = state.selected_history_index == Some(index);
            let label = format!("[{}] {}", entry.status, entry.label);
            history_list = history_list.push(history_button(
                label,
                selected,
                Message::HistorySelected(index),
            ));
        }
    }

    let preview = view_output_lines(state.history_preview_lines());
    layout = layout
        .push(scrollable(history_list).height(Length::Fill))
        .push(
            container(preview)
                .width(Length::FillPortion(2))
                .height(Length::Fill),
        );

    container(layout).padding(super::super::style::PANEL_INSET).into()
}
