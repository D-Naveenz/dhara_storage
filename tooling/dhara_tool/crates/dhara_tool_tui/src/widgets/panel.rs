use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders};
use ratatui::Frame;

use crate::theme as dhara_theme;

pub fn render_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    title: &str,
    focused: bool,
    fill_panel_bg: bool,
) -> Rect {
    let block = Block::default()
        .title(format!(" {title} "))
        .borders(Borders::ALL)
        .border_style(if focused {
            dhara_theme::border_style().fg(dhara_theme::ACCENT)
        } else {
            dhara_theme::border_style()
        })
        .style(if fill_panel_bg {
            dhara_theme::panel_style()
        } else {
            dhara_theme::border_only_style()
        });
    let inner = block.inner(area);
    frame.render_widget(block, area);
    inner
}
