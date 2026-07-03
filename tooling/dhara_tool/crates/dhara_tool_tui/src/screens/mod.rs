pub mod modals;

use dhara_tool_cli::command::{CommandRegistry, CommandSpec};
use dhara_tool_cli::interactive::{AppState, DiagnosticSeverity, MainTab, VisibleTreeRow};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Tabs, Wrap};
use ratatui::Frame;

use crate::theme;

const TAB_LABELS: [&str; 4] = ["Info", "Options", "Troubleshooting", "System"];

pub fn render_tasks_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    _state: &AppState,
    rows: &[VisibleTreeRow],
    selected_row: usize,
    focused: bool,
) {
    let items: Vec<ListItem<'_>> = rows
        .iter()
        .enumerate()
        .map(|(index, row)| {
            let indent = "  ".repeat(row.depth);
            let marker = if row.has_children {
                if row.expanded { "▼ " } else { "▶ " }
            } else {
                "  "
            };
            let label = format!("{indent}{marker}{}", row.node.label);
            let style = if index == selected_row {
                theme::selected_style()
            } else {
                Style::default().fg(theme::TEXT)
            };
            ListItem::new(label).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(" Tasks ")
            .borders(Borders::ALL)
            .border_style(if focused {
                theme::border_style().fg(theme::ACCENT)
            } else {
                theme::border_style()
            }),
    );
    frame.render_widget(list, area);
}

pub fn render_tabs(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &AppState,
    focused: bool,
) {
    let selected = match state.main_tab {
        MainTab::Info => 0,
        MainTab::Options => 1,
        MainTab::Troubleshooting => 2,
        MainTab::SystemConfigs => 3,
    };
    let titles: Vec<Line<'_>> = TAB_LABELS
        .iter()
        .map(|title| Line::from(Span::raw(*title)))
        .collect();
    let tabs = Tabs::new(titles)
        .block(
            Block::default().borders(Borders::ALL).border_style(if focused {
                theme::border_style().fg(theme::ACCENT)
            } else {
                theme::border_style()
            }),
        )
        .select(selected)
        .style(Style::default().fg(theme::MUTED))
        .highlight_style(theme::selected_style());
    frame.render_widget(tabs, area);
}

pub fn render_tab_content(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &AppState,
    registry: &CommandRegistry,
    form_field: usize,
    scroll: usize,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme::border_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    match state.main_tab {
        MainTab::Info => render_info(frame, inner, state, registry),
        MainTab::Options => render_options(frame, inner, state, registry, form_field),
        MainTab::Troubleshooting => render_troubleshooting(frame, inner, state, scroll),
        MainTab::SystemConfigs => render_system_configs(frame, inner, state),
    }
}

fn render_info(frame: &mut Frame<'_>, area: Rect, state: &AppState, registry: &CommandRegistry) {
    let Some(command) = state.selected_command(registry) else {
        frame.render_widget(Paragraph::new("Select a task from the tree."), area);
        return;
    };

    let mut lines = vec![
        Line::from(vec![
            Span::styled("What: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(command.ui.description),
        ]),
        Line::from(vec![
            Span::styled("Summary: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(command.summary),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("How: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!(
                "{} {}",
                command.path_string(),
                command.args_summary
            )),
        ]),
    ];

    if !command.ui.fields.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Fields:",
            Style::default().add_modifier(Modifier::BOLD),
        )));
        for field in &command.ui.fields {
            lines.push(Line::from(format!("  {} — {}", field.label, field.help)));
        }
    }

    let text: Vec<Line<'_>> = lines;
    frame.render_widget(Paragraph::new(text).wrap(Wrap { trim: true }), area);
}

fn render_options(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &AppState,
    registry: &CommandRegistry,
    form_field: usize,
) {
    let Some(command) = state.selected_command(registry) else {
        frame.render_widget(Paragraph::new("Select a task to edit options."), area);
        return;
    };
    let Some(form) = state.forms.get(command.id) else {
        frame.render_widget(Paragraph::new("Loading form…"), area);
        return;
    };

    let mut lines = Vec::new();
    for (index, field) in command.ui.fields.iter().enumerate() {
        let value = form.display_value(command, index);
        let prefix = if index == form_field { "▸ " } else { "  " };
        let style = if index == form_field {
            theme::selected_style()
        } else {
            Style::default().fg(theme::TEXT)
        };
        lines.push(Line::styled(
            format!("{prefix}{}: {value}", field.label),
            style,
        ));
    }

    if lines.is_empty() {
        lines.push(Line::from("No options for this task. Press r to run."));
    } else {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Enter: edit | ←→: cycle select | Type to edit text",
            Style::default().fg(theme::MUTED),
        )));
    }

    frame.render_widget(Paragraph::new(lines), area);
}

fn render_troubleshooting(frame: &mut Frame<'_>, area: Rect, state: &AppState, scroll: usize) {
    if state.troubleshooting_lines.is_empty() {
        frame.render_widget(
            Paragraph::new("Warnings and errors appear here during a run."),
            area,
        );
        return;
    }

    let lines: Vec<Line<'_>> = state
        .troubleshooting_lines
        .iter()
        .skip(scroll)
        .map(|line| {
            let (prefix, color) = match line.severity {
                DiagnosticSeverity::Warn => ("WARN", theme::WARNING),
                DiagnosticSeverity::Error => ("ERR ", theme::ERROR),
            };
            Line::from(vec![
                Span::styled(format!("[{prefix}] "), Style::default().fg(color)),
                Span::raw(line.text.as_str()),
            ])
        })
        .collect();
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
}

fn render_system_configs(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    let text = state
        .system_configs_text
        .as_deref()
        .unwrap_or("Open this tab to load configuration.");
    frame.render_widget(
        Paragraph::new(text).wrap(Wrap { trim: false }),
        area,
    );
}

pub fn tab_from_index(index: usize) -> MainTab {
    match index {
        1 => MainTab::Options,
        2 => MainTab::Troubleshooting,
        3 => MainTab::SystemConfigs,
        _ => MainTab::Info,
    }
}

pub fn select_task_row(
    state: &mut AppState,
    registry: &CommandRegistry,
    rows: &[VisibleTreeRow],
    row_index: usize,
) {
    let Some(row) = rows.get(row_index) else {
        return;
    };
    if row.has_children {
        state
            .tree_view
            .toggle_expanded(&row.node.path_key);
    } else if let Some(command_id) = row.node.command_id {
        state.select_command(registry, command_id);
    }
}

pub fn cycle_form_field(command: &CommandSpec, form_field: &mut usize, delta: isize) {
    let count = command.ui.fields.len();
    if count == 0 {
        return;
    }
    let next = (*form_field as isize + delta).rem_euclid(count as isize) as usize;
    *form_field = next;
}
