use ratatui::layout::Rect;
use ratatui::widgets::Widget;
use ratatui_interact::components::{ScrollableContent, ScrollableContentState};
use ratatui_interact::theme::Theme;

use crate::theme as dhara_theme;

pub fn render_scroll_body(
    area: Rect,
    scroll: &mut ScrollableContentState,
    theme: &Theme,
    buf: &mut ratatui::buffer::Buffer,
) {
    ScrollableContent::new(scroll)
        .style(dhara_theme::scroll_body_style())
        .theme(theme)
        .render(area, buf);
}
