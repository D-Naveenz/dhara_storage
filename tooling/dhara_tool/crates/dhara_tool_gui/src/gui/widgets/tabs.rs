use iced::widget::row;
use iced::{Element, Length};

use super::button::tab_button;

pub fn tab_strip<'a, Message: Clone + 'a, T: Copy + PartialEq + 'static>(
    tabs: &[(T, &'static str)],
    active: T,
    on_select: impl Fn(T) -> Message + Copy + 'a,
) -> Element<'a, Message> {
    let mut strip = row![].spacing(0).height(Length::Shrink);
    for (tab, label) in tabs {
        strip = strip.push(tab_button(
            label,
            *tab == active,
            on_select(*tab),
        ));
    }
    strip.into()
}
