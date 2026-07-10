use ratatui::layout::Rect;
use ratatui::widgets::{Block, Widget};
use ratatui::Frame;

use crate::theme as dhara_theme;

/// Fill a dialog inner area with the distinct modal background.
pub fn fill_modal_background(frame: &mut Frame<'_>, area: Rect) {
    Block::default()
        .style(dhara_theme::modal_panel_style())
        .render(area, frame.buffer_mut());
}
