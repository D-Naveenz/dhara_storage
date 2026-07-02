use iced::widget::container;
use iced::{Element, Length, Theme};

use super::super::style::{panel_container, tab_content_panel, PANEL_INSET};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelVariant {
    Default,
    TabContent,
}

pub fn panel<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    variant: PanelVariant,
) -> Element<'a, Message> {
    let style = match variant {
        PanelVariant::Default => panel_container,
        PanelVariant::TabContent => tab_content_panel,
    };
    container(content)
        .width(Length::Fill)
        .style(move |theme: &Theme| style(theme))
        .into()
}

pub fn inset_panel<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    variant: PanelVariant,
) -> Element<'a, Message> {
    panel(
        container(content.into())
            .padding(PANEL_INSET)
            .width(Length::Fill),
        variant,
    )
}
