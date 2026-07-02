use iced::widget::{container, mouse_area, row, space, text};
use iced::{Alignment, Element, Length, Padding, Theme};

use super::super::app::Message;
use super::super::style::{
    chevron_icon_rotated, tree_row_accent, tree_row_container, tree_row_text_color, TREE_ROW_INSET,
};
use super::super::tree::VisibleTreeRow;

pub fn tree_row<'a>(row: &VisibleTreeRow, selected: bool) -> Element<'a, Message> {
    let indent = container(space::horizontal()).width(f32::from(row.depth as u16 * 16));

    let chevron_cell: Element<'a, Message> = if row.has_children {
        container(chevron_icon_rotated(row.expanded))
            .width(16)
            .height(12)
            .align_y(Alignment::Center)
            .into()
    } else {
        container(space::horizontal()).width(16).into()
    };

    let accent: Element<'a, Message> = if selected {
        container(space::horizontal())
            .width(3)
            .style(|theme: &Theme| tree_row_accent(theme))
            .into()
    } else {
        container(space::horizontal()).width(0).into()
    };

    let label = text(row.node.label.clone()).size(13).style(move |theme: &Theme| {
        text::Style {
            color: Some(tree_row_text_color(theme, selected)),
        }
    });

    let row_content = row![indent, chevron_cell, accent, label]
        .spacing(4)
        .align_y(Alignment::Center)
        .width(Length::Fill)
        .height(Length::Shrink);

    let interactive = container(row_content)
        .width(Length::Fill)
        .height(Length::Shrink)
        .padding(Padding::from(TREE_ROW_INSET))
        .style(tree_row_container(selected));

    let message = if row.has_children {
        Message::TreeToggleExpand(row.node.path_key.clone())
    } else if let Some(command_id) = row.node.command_id {
        Message::CommandSelected(command_id)
    } else {
        Message::TreeToggleExpand(row.node.path_key.clone())
    };

    mouse_area(interactive)
        .on_press(message)
        .interaction(iced::mouse::Interaction::Pointer)
        .into()
}
