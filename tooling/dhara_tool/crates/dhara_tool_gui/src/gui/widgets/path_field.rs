use iced::widget::{button, row, text_input};
use iced::{Element, Length};

pub fn browsable_path_field<'a, Message: Clone + 'static>(
    value: &str,
    placeholder: &str,
    on_input: impl Fn(String) -> Message + 'a,
    on_browse: Message,
) -> Element<'a, Message> {
    row![
        text_input(placeholder, value)
            .on_input(on_input)
            .padding(6)
            .width(Length::Fill),
        button("Browse").on_press(on_browse),
    ]
    .spacing(8)
    .width(Length::Fill)
    .into()
}
