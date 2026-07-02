use iced::widget::{button, text};
use iced::{Element, Length};

use super::super::style::tab_button_style;

pub fn tab_button<'a, Message: Clone + 'a>(
    label: &'a str,
    active: bool,
    on_press: Message,
) -> Element<'a, Message> {
    button(text(label).size(13))
        .padding([8, 16])
        .style(tab_button_style(active))
        .on_press(on_press)
        .into()
}

pub fn action_button<'a, Message: Clone + 'a>(
    label: &'a str,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    let mut control = button(text(label)).padding([8, 16]);
    if let Some(message) = on_press {
        control = control.on_press(message);
    }
    control.into()
}

pub fn history_button<'a, Message: Clone + 'a>(
    label: String,
    selected: bool,
    on_press: Message,
) -> Element<'a, Message> {
    let mut control = button(text(label).size(12))
        .width(Length::Fill)
        .padding([4, 8]);
    if selected {
        control = control.style(button::primary);
    }
    control.on_press(on_press).into()
}
