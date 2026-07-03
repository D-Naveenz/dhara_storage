use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;

use crate::theme;

pub fn render_title_bar(frame: &mut Frame<'_>, area: Rect, version: &str, repository: &str) {
    frame.render_widget(Block::default().style(theme::bar_style()), area);

    let cols = ratatui::layout::Layout::horizontal([
        ratatui::layout::Constraint::Percentage(55),
        ratatui::layout::Constraint::Percentage(45),
    ])
    .split(area);

    let title = Paragraph::new(Line::from(vec![
        Span::styled("Dhara Tool", theme::title_style()),
        Span::raw(format!("  v{version}")),
    ]))
    .style(theme::bar_style());
    let repo = Paragraph::new(repository)
        .style(theme::bar_style())
        .alignment(ratatui::layout::Alignment::Right);
    frame.render_widget(title, cols[0]);
    frame.render_widget(repo, cols[1]);
}
