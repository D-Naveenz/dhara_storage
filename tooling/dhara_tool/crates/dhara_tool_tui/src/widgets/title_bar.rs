use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::theme;

pub fn render_title_bar(frame: &mut Frame<'_>, area: Rect, version: &str, repository: &str) {
    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(theme::border_style())
        .style(theme::panel_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let cols = ratatui::layout::Layout::horizontal([
        ratatui::layout::Constraint::Percentage(55),
        ratatui::layout::Constraint::Percentage(45),
    ])
    .split(inner);

    let title = Paragraph::new(Line::from(vec![
        Span::styled("Dhara Tool", theme::title_style()),
        Span::raw(format!("  v{version}")),
    ]));
    let repo = Paragraph::new(repository).alignment(ratatui::layout::Alignment::Right);
    frame.render_widget(title, cols[0]);
    frame.render_widget(repo, cols[1]);
}
