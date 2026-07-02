use iced::widget::pick_list;
use iced::{Element, Length};

pub fn dropdown<'a, Message: Clone + 'a>(
    options: Vec<String>,
    current: Option<String>,
    placeholder: &str,
    on_select: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    pick_list(options, current, on_select)
        .placeholder(placeholder)
        .width(Length::Fill)
        .into()
}
