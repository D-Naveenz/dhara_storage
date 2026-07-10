use ratatui::layout::Rect;
use ratatui::widgets::{Paragraph, Widget};

use crate::theme::{self, ValidationTone};

pub fn render_status_line(
    area: Rect,
    text: &str,
    tone: ValidationTone,
    buf: &mut ratatui::buffer::Buffer,
) {
    if text.is_empty() {
        return;
    }
    Paragraph::new(text)
        .style(theme::validation_status_style(tone))
        .render(area, buf);
}
