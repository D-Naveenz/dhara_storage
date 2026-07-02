use iced::widget::container;
use iced::{Element, Length, Theme};

pub fn modal_overlay<'a, Message: 'a>(
    card: Element<'a, Message>,
) -> Element<'a, Message> {
    container(
        container(card)
            .padding(16)
            .style(|theme: &Theme| container::Style {
                background: Some(iced::Background::Color(theme.palette().background)),
                border: iced::Border {
                    color: theme.palette().primary,
                    width: 1.0,
                    radius: 6.0.into(),
                },
                ..Default::default()
            }),
    )
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .width(Length::Fill)
    .height(Length::Fill)
    .style(|theme: &Theme| container::Style {
        background: Some(iced::Background::Color(iced::Color {
            a: 0.65,
            ..theme.palette().background
        })),
        ..Default::default()
    })
    .into()
}
