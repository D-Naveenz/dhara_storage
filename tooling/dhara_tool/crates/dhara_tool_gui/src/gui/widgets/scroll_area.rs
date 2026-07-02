use iced::widget::scrollable;
use iced::{Element, Length};

pub fn scroll_area<'a, Message: 'a>(content: Element<'a, Message>) -> Element<'a, Message> {
    scrollable(content)
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
}
