use iced::widget::{column, text};
use iced::{Element, Length};

pub fn labeled_field<'a, Message: 'a>(
    label: &str,
    help: Option<&'a str>,
    control: Element<'a, Message>,
) -> Element<'a, Message> {
    let mut field = column![text(format!("{label}:")).size(14)]
        .spacing(4)
        .width(Length::Fill);
    field = field.push(control);
    if let Some(help) = help.filter(|text| !text.is_empty()) {
        field = field.push(text(help).size(12));
    }
    field.into()
}
