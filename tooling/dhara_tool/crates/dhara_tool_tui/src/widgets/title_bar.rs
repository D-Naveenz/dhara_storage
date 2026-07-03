use ratatui::layout::Rect;
use ratatui::style::Modifier;
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

pub fn render_command_bar(frame: &mut Frame<'_>, area: Rect, running: bool) {
    let hints = if running {
        "Tab focus | ↑↓ navigate | Enter select | r Run | c Cancel | q Quit"
    } else {
        "Tab focus | ↑↓ navigate | Enter select | r Run | q Quit"
    };
    let bar = Paragraph::new(hints)
        .style(theme::panel_style().add_modifier(Modifier::DIM))
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(theme::border_style()),
        );
    frame.render_widget(bar, area);
}
