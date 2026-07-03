use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use dhara_tool_cli::interactive::AppState;

use crate::theme;

pub struct RepoSetupPrompt {
    pub path_input: String,
}

impl RepoSetupPrompt {
    pub fn new(initial: Option<String>) -> Self {
        Self {
            path_input: initial.unwrap_or_default(),
        }
    }
}

pub fn render_activation_modal(frame: &mut Frame<'_>, state: &AppState) {
    let Some(prompt) = &state.activation_prompt else {
        return;
    };
    let area = centered_rect(60, 40, frame.area());
    frame.render_widget(Clear, area);
    let mut lines = vec![
        Line::from(Span::styled(
            "Configuration drift detected",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];
    for drift in &prompt.drifts {
        lines.push(Line::from(format!("  • {:?}", drift.kind)));
    }
    lines.push(Line::from(""));
    lines.push(Line::from("Apply drift from dhara.config.toml? (y/n)"));

    let block = Block::default()
        .title(" Activation ")
        .borders(Borders::ALL)
        .border_style(theme::border_style().fg(theme::WARNING))
        .style(theme::panel_style());
    frame.render_widget(Paragraph::new(lines).block(block).wrap(Wrap { trim: true }), area);
}

pub fn render_repo_setup_modal(frame: &mut Frame<'_>, prompt: &RepoSetupPrompt) {
    let area = centered_rect(70, 30, frame.area());
    frame.render_widget(Clear, area);
    let lines = vec![
        Line::from("Repository path (folder or dhara.config.toml):"),
        Line::from(""),
        Line::from(Span::styled(
            if prompt.path_input.is_empty() {
                "_"
            } else {
                prompt.path_input.as_str()
            },
            theme::selected_style(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Enter: confirm | Esc: quit",
            Style::default().fg(theme::MUTED),
        )),
    ];
    let block = Block::default()
        .title(" Repository setup ")
        .borders(Borders::ALL)
        .border_style(theme::border_style().fg(theme::ACCENT))
        .style(theme::panel_style());
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = ratatui::layout::Layout::vertical([
        ratatui::layout::Constraint::Percentage((100 - percent_y) / 2),
        ratatui::layout::Constraint::Percentage(percent_y),
        ratatui::layout::Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(area);

    ratatui::layout::Layout::horizontal([
        ratatui::layout::Constraint::Percentage((100 - percent_x) / 2),
        ratatui::layout::Constraint::Percentage(percent_x),
        ratatui::layout::Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup_layout[1])[1]
}
