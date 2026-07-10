use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};
use ratatui_interact::components::ButtonState;
use ratatui_interact::theme::Theme;

use crate::theme as dhara_theme;

pub const BUTTON_SLOT_HEIGHT: u16 = 2;
pub const BUTTON_MIN_WIDTH: u16 = 12;
pub const BUTTON_GAP: u16 = 1;

fn button_style(state: &ButtonState, theme: &Theme) -> Style {
    let palette = &theme.palette;
    if !state.enabled {
        return Style::default()
            .fg(palette.text_disabled)
            .bg(palette.surface);
    }
    if state.focused {
        return Style::default()
            .fg(palette.highlight_fg)
            .bg(palette.highlight_bg)
            .add_modifier(Modifier::BOLD);
    }
    Style::default()
        .fg(palette.text)
        .bg(palette.surface_raised)
}

pub fn render_padded_button(
    area: Rect,
    label: &str,
    state: &ButtonState,
    theme: &Theme,
    buf: &mut ratatui::buffer::Buffer,
) -> Rect {
    let style = button_style(state, theme);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(if state.focused && state.enabled {
            Style::default().fg(dhara_theme::ACCENT)
        } else {
            dhara_theme::border_style()
        })
        .style(style);
    let inner = block.inner(area);
    block.render(area, buf);

    let label_area = if inner.height >= 2 {
        Rect::new(inner.x, inner.y + inner.height / 2, inner.width, 1)
    } else {
        inner
    };

    Paragraph::new(format!(" {label} "))
        .alignment(Alignment::Center)
        .style(style)
        .render(label_area, buf);

    area
}
