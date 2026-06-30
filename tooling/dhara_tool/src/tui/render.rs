use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};

use crate::command::CommandRegistry;

use super::state::{ActivationPrompt, AppState, Focus, MainView};

pub fn render(frame: &mut Frame<'_>, state: &AppState, registry: &CommandRegistry) {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(frame.area());
    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(34), Constraint::Min(1)])
        .split(root[1]);
    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(8), Constraint::Min(1)])
        .split(body[0]);
    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
        .split(body[1]);
    let output_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(right[1]);

    render_header(frame, root[0], state);
    render_sections(frame, left[0], state, registry);
    render_commands(frame, left[1], state, registry);
    match state.main_view {
        MainView::Dashboard => render_dashboard(frame, right[0], state, registry),
        MainView::Form => render_form(frame, right[0], state, registry),
    }
    render_output(frame, output_layout[0], state);
    render_history(frame, output_layout[1], state);
    render_footer(frame, root[2], state);
    if let Some(prompt) = &state.activation_prompt {
        render_activation_prompt(frame, frame.area(), prompt);
    }
}

fn render_header(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    let style = Style::default().fg(Color::White).bg(Color::DarkGray);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(area);
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(rows[0]);

    let title = Paragraph::new(format!(" Dhara Tool  v{}", crate::version())).style(style);
    let repository = Paragraph::new(format!("{} ", state.repository_label))
        .style(style)
        .alignment(Alignment::Right);

    frame.render_widget(Paragraph::new("").style(style), rows[0]);
    frame.render_widget(title, columns[0]);
    frame.render_widget(repository, columns[1]);
}

fn render_sections(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &AppState,
    registry: &CommandRegistry,
) {
    let items = registry
        .sections()
        .map(|section| ListItem::new(format!("{:<10} {}", section.name, section.summary)))
        .collect::<Vec<_>>();
    let mut list_state = ListState::default().with_selected(Some(state.selected_section));
    let list = List::new(items)
        .block(focused_block("Sections", state.focus == Focus::Sections))
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan));
    frame.render_stateful_widget(list, area, &mut list_state);
}

fn render_commands(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &AppState,
    registry: &CommandRegistry,
) {
    let items = state
        .commands_for_current_section(registry)
        .into_iter()
        .map(|command| ListItem::new(format!("{:<18} {}", command.path_string(), command.summary)))
        .collect::<Vec<_>>();
    let mut list_state = ListState::default().with_selected(Some(state.selected_command));
    let list = List::new(items)
        .block(focused_block("Commands", state.focus == Focus::Commands))
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Green));
    frame.render_stateful_widget(list, area, &mut list_state);
}

fn render_dashboard(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &AppState,
    registry: &CommandRegistry,
) {
    let quick_actions = [
        "verify package",
        "config show",
        "version bump",
        "release run",
    ]
    .iter()
    .enumerate()
    .map(|(index, label)| {
        let prefix = if index == state.selected_quick_action {
            ">"
        } else {
            " "
        };
        format!("{prefix} {label}")
    })
    .collect::<Vec<_>>()
    .join("\n");
    let tool_version = crate::version();
    let packaging_version = crate::defs_package_version();
    let next_revision = state
        .workspace
        .next_package_revision(packaging_version)
        .map(|value| value.to_string())
        .unwrap_or_else(|_| "?".to_owned());
    let package_version = state.workspace.package_version.as_deref().unwrap_or("—");
    let revision = state
        .workspace
        .package_revision
        .map(|value| value.to_string())
        .unwrap_or_else(|| "—".to_owned());
    let release = state
        .workspace
        .definitions_release
        .as_deref()
        .unwrap_or("—");
    let definitions = state
        .workspace
        .definition_count
        .map(|value| value.to_string())
        .unwrap_or_else(|| "—".to_owned());
    let context = format!(
        "Dhara TUI command hub\n\n\
         Definitions package\n\
         Path: {}\n\
         Status: {}\n\
         Package version: {package_version} (DAL: {packaging_version}, tool: {tool_version}, {})\n\
         Revision: {revision} (next build: {next_revision})\n\
         Release: {release}\n\
         Definitions: {definitions}\n\n\
         Quick actions\n{quick_actions}\n\n\
         Current section: {}\n\
         Commands in section: {}\n\
         Repo root: {}\n\
         Output lines: {}\n\
         Recent history: {}",
        state.workspace.defs_path.display(),
        state.workspace.status_label(),
        state.workspace.packaging_lineage_label(packaging_version),
        state.current_section(registry).unwrap_or("none"),
        state.commands_for_current_section(registry).len(),
        std::env::current_dir()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|_| ".".to_owned()),
        state.output_lines().len(),
        state.session_history.len(),
    );
    let paragraph = Paragraph::new(context)
        .block(focused_block("Dashboard", state.focus == Focus::Main))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn render_form(frame: &mut Frame<'_>, area: Rect, state: &AppState, registry: &CommandRegistry) {
    let Some(command) = state.current_command(registry) else {
        frame.render_widget(
            Paragraph::new("No command selected.")
                .block(focused_block("Command", state.focus == Focus::Main)),
            area,
        );
        return;
    };
    let form = state.forms.get(command.id);
    let mut lines = vec![
        Line::raw(command.path_string()),
        Line::raw(command.ui.description),
    ];
    if !command.args_summary.is_empty() {
        lines.push(Line::raw(format!(
            "Usage: {} {}",
            command.path_string(),
            command.args_summary
        )));
    }
    lines.push(Line::raw(""));
    for (index, field) in command.ui.fields.iter().enumerate() {
        let value = form
            .map(|form| form.display_value(command, index))
            .unwrap_or_default();
        let marker = if form.is_some_and(|form| form.selected_field == index) {
            ">"
        } else {
            " "
        };
        let editing = if state.focus == Focus::Main
            && state.editing_text
            && form.is_some_and(|form| form.selected_field == index)
        {
            " (editing)"
        } else {
            ""
        };
        lines.push(Line::raw(format!(
            "{marker} {:<18} {}{}",
            field.label, value, editing
        )));
        lines.push(Line::raw(format!("  {}", field.help)));
    }
    if command.ui.fields.is_empty() {
        lines.push(Line::raw(
            "This command has no form fields and can be run directly.",
        ));
    }
    let paragraph = Paragraph::new(lines)
        .block(focused_block("Command Form", state.focus == Focus::Main))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn render_output(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    let output = if state.output_lines().is_empty() {
        vec![Line::raw("No command output yet.")]
    } else {
        state
            .output_lines()
            .iter()
            .map(|line| {
                let style = if line.is_error {
                    Style::default().fg(Color::LightRed)
                } else {
                    Style::default()
                };
                Line::styled(line.text.clone(), style)
            })
            .collect()
    };
    let paragraph = Paragraph::new(output)
        .block(focused_block("Run Output", state.focus == Focus::Output))
        .wrap(Wrap { trim: false })
        .scroll((state.output_scroll as u16, 0));
    frame.render_widget(Clear, area);
    frame.render_widget(paragraph, area);
}

fn render_history(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    let items = if state.session_history.is_empty() {
        vec![ListItem::new("No history yet.")]
    } else {
        state
            .session_history
            .iter()
            .rev()
            .take(8)
            .map(|entry| ListItem::new(format!("[{}] {}", entry.status, entry.label)))
            .collect()
    };
    let list = List::new(items).block(
        Block::default()
            .title("Recent History")
            .borders(Borders::ALL),
    );
    frame.render_widget(list, area);
}

fn render_footer(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    let help = if state.activation_prompt.is_some() {
        "y/Enter apply | n/Esc decline"
    } else if state.active_run.is_some() {
        "Tab focus | Enter select/edit | r run | c cancel | d dashboard | q quit"
    } else {
        "Tab focus | Enter select/edit | r run | d dashboard | q quit"
    };
    let footer = Paragraph::new(format!("{help} | {}", state.status_message))
        .style(Style::default().fg(Color::Black).bg(Color::Gray));
    frame.render_widget(footer, area);
}

fn render_activation_prompt(frame: &mut Frame<'_>, area: Rect, prompt: &ActivationPrompt) {
    let width = area.width.min(72).max(40);
    let height = (prompt.drifts.len() as u16 + 8).min(area.height.saturating_sub(2));
    let popup = centered_rect(width, height, area);
    frame.render_widget(Clear, popup);

    let yes_style = if prompt.confirm_yes {
        Style::default().fg(Color::Black).bg(Color::Green)
    } else {
        Style::default()
    };
    let no_style = if prompt.confirm_yes {
        Style::default()
    } else {
        Style::default().fg(Color::Black).bg(Color::Red)
    };

    let mut lines = vec![
        Line::from("Configuration drift detected"),
        Line::from("(dhara.config.toml is truth):"),
        Line::from(""),
    ];
    for item in &prompt.drifts {
        lines.push(Line::from(format!("  - {}", item.summary)));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(" Yes ", yes_style),
        Span::raw("  "),
        Span::styled(" No ", no_style),
    ]));

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .title("Apply configuration changes?")
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(paragraph, popup);
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let horizontal = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((area.height.saturating_sub(height)) / 2),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length((area.width.saturating_sub(width)) / 2),
            Constraint::Length(width),
            Constraint::Min(0),
        ])
        .split(horizontal[1])[1]
}

fn focused_block<'a>(title: &'a str, focused: bool) -> Block<'a> {
    let style = if focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(style)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use anyhow::Result;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use crate::command::{
        CommandRegistry, CommandResult, CommandSpec, CommandUi, SectionSpec, ToolContext,
    };

    use super::render;
    use crate::tui::state::{AppState, MainView};

    fn noop(_: &ToolContext, _: &[String]) -> Result<CommandResult> {
        Ok(CommandResult::success())
    }

    fn registry() -> CommandRegistry {
        let mut registry = CommandRegistry::new();
        registry.add_section(SectionSpec {
            name: "verify",
            prompt: "verify> ",
            summary: "Verification commands",
        });
        registry.add_command(CommandSpec {
            id: "verify.package",
            path: &["verify", "package"],
            summary: "Verify package",
            args_summary: "",
            section: "verify",
            ui: CommandUi::empty("Verify package"),
            handler: Arc::new(noop),
        });
        registry
    }

    fn buffer_text(terminal: &Terminal<TestBackend>) -> String {
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<Vec<_>>()
            .join("")
    }

    #[test]
    fn renders_dashboard_view() {
        let registry = registry();
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = AppState::new();

        terminal
            .draw(|frame| render(frame, &state, &registry))
            .unwrap();
        let text = buffer_text(&terminal);
        assert!(text.contains("Dhara Tool"));
        assert!(text.contains("Dashboard"));
        assert!(text.contains("Run Output"));
        assert!(text.contains("Recent History"));
    }

    #[test]
    fn renders_form_view() {
        let registry = registry();
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = AppState::new();
        state.main_view = MainView::Form;
        state.focus = crate::tui::state::Focus::Main;

        terminal
            .draw(|frame| render(frame, &state, &registry))
            .unwrap();
        let text = buffer_text(&terminal);
        assert!(text.contains("Dhara Tool"));
        assert!(text.contains("Command Form"));
        assert!(text.contains("verify package"));
    }
}
