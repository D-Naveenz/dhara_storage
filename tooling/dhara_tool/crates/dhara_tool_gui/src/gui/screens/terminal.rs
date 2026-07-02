use iced::widget::{column, text};
use iced::{Element, Length, Theme};

use super::super::app::Message;
use super::super::state::{AppState, OutputLine};
use super::super::widgets::scroll_area::scroll_area;

pub fn view_terminal<'a>(state: &'a AppState) -> Element<'a, Message> {
    view_output_lines(state.terminal_lines())
}

pub fn view_output_lines(lines: &[OutputLine]) -> Element<'static, Message> {
    let mut output = column![].spacing(2).width(Length::Fill);
    if lines.is_empty() {
        output = output.push(text("No output yet.").size(13));
    } else {
        for line in lines {
            let content = line.text.clone();
            if line.is_error {
                output = output.push(
                    text(content)
                        .size(12)
                        .style(|theme: &Theme| text::Style {
                            color: Some(theme.extended_palette().danger.strong.color),
                        }),
                );
            } else {
                output = output.push(text(content).size(12));
            }
        }
    }

    scroll_area(output.into())
}
