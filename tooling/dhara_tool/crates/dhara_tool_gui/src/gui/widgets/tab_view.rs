use iced::widget::{column, container};
use iced::{Element, Length, Theme};

use super::super::style::tab_content_panel;
use super::tabs::tab_strip;

pub fn tab_view<'a, Message: Clone + 'a, T: Copy + PartialEq + 'static>(
    tabs: &[(T, &'static str)],
    active: T,
    on_select: impl Fn(T) -> Message + Copy + 'a,
    content: Element<'a, Message>,
) -> Element<'a, Message> {
    column![
        tab_strip(tabs, active, on_select),
        container(content)
            .height(Length::Fill)
            .width(Length::Fill)
            .style(|theme: &Theme| tab_content_panel(theme)),
    ]
    .spacing(0)
    .height(Length::Fill)
    .into()
}

pub fn tab_content_scroll<'a, Message: 'a>(
    content: Element<'a, Message>,
) -> Element<'a, Message> {
    use iced::widget::container;
    use super::super::style::PANEL_INSET;

    super::scroll_area::scroll_area(
        container(content)
            .padding(PANEL_INSET)
            .width(Length::Fill)
            .into(),
    )
}
