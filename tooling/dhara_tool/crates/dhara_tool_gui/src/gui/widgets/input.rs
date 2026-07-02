use iced::widget::text_input;
use iced::{Element, Length};

pub fn text_field<'a, Message: Clone + 'a>(
    value: &str,
    placeholder: &str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    text_input(placeholder, value)
        .on_input(on_input)
        .padding(6)
        .width(Length::Fill)
        .into()
}
