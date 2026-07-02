use iced::widget::{column, progress_bar, text};
use iced::{Element, Length};

pub fn labeled_progress<'a, Message: 'a>(
    value: f32,
    label: &'a str,
) -> Element<'a, Message> {
    column![
        progress_bar(0.0..=1.0, value),
        text(label).size(12),
    ]
    .spacing(4)
    .width(Length::Fill)
    .into()
}
