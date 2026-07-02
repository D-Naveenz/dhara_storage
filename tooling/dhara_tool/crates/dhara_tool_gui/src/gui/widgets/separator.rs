use iced::widget::{container, space};
use iced::widget::container as container_widget;
use iced::{Element, Length, Theme};

use super::super::style::panel_border;

pub fn horizontal_separator<'a, Message: 'a>() -> Element<'a, Message> {
    container(space::vertical().height(1))
        .width(Length::Fill)
        .style(|theme: &Theme| container_widget::Style {
            background: Some(iced::Background::Color(panel_border(theme))),
            ..Default::default()
        })
        .into()
}
