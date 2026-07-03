pub mod modals;

use dhara_tool_cli::command::{CommandRegistry, CommandSpec, FieldKind};
use dhara_tool_cli::forms::FormValue;
use dhara_tool_cli::interactive::{AppState, DiagnosticSeverity, MainTab};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Widget};
use ratatui::Frame;
use ratatui_interact::components::{
    CheckBox, CheckBoxState, InputState, ScrollableContent, ScrollableContentState, Tab,
    TabView, TabViewAction, TabViewState,
};
use ratatui_interact::theme::Theme;
use ratatui_interact::traits::ClickRegionRegistry;

use crate::focus::TuiFocus;
use crate::theme as dhara_theme;

const TAB_LABELS: [&str; 4] = ["Info", "Options", "Troubleshooting", "System"];

pub struct CenterPanelClicks {
    pub registry: ClickRegionRegistry<TabViewAction>,
}

pub fn sync_tab_view_from_state(tab_state: &mut TabViewState, main_tab: MainTab) {
    tab_state.select(tab_index(main_tab));
}

pub fn sync_state_from_tab_view(tab_state: &TabViewState) -> MainTab {
    tab_from_index(tab_state.selected_index)
}

pub fn render_center_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &AppState,
    registry: &CommandRegistry,
    theme: &Theme,
    tab_state: &mut TabViewState,
    shell_focus: &crate::focus::ShellFocus,
    info_scroll: &mut ScrollableContentState,
    trouble_scroll: &mut ScrollableContentState,
    system_scroll: &mut ScrollableContentState,
    form_field: usize,
    editing_form: bool,
    option_input: &InputState,
    option_checkbox: &CheckBoxState,
) -> CenterPanelClicks {
    sync_tab_view_from_state(tab_state, state.main_tab);
    tab_state.focused = shell_focus.is_focused(&TuiFocus::MainTabs)
        || shell_focus.is_focused(&TuiFocus::TabContent);

    let tabs_focused = shell_focus.is_focused(&TuiFocus::MainTabs)
        || shell_focus.is_focused(&TuiFocus::TabContent);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(if tabs_focused {
            dhara_theme::border_style().fg(dhara_theme::ACCENT)
        } else {
            dhara_theme::border_style()
        })
        .style(dhara_theme::border_only_style());
    let panel_inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::vertical([Constraint::Length(3), Constraint::Min(1)]).split(panel_inner);
    let tabs: Vec<Tab<'_>> = TAB_LABELS.iter().map(|label| Tab::new(label)).collect();

    let mut click_registry = ClickRegionRegistry::new();
    let tab_view = TabView::new(&tabs, tab_state)
        .theme(theme)
        .content(|_, _, _| {});
    tab_view.render_with_registry(chunks[0], frame.buffer_mut(), &mut click_registry);

    let selected_tab = tab_state.selected_index;
    let content_focused = shell_focus.is_focused(&TuiFocus::TabContent);
    info_scroll.set_focused(content_focused && selected_tab == 0);
    trouble_scroll.set_focused(content_focused && selected_tab == 2);
    system_scroll.set_focused(content_focused && selected_tab == 3);

    match selected_tab {
        0 => render_info_tab(chunks[1], frame.buffer_mut(), state, registry, info_scroll, theme),
        1 => render_options_tab(
            chunks[1],
            frame.buffer_mut(),
            state,
            registry,
            form_field,
            editing_form,
            option_input,
            option_checkbox,
            theme,
            content_focused,
        ),
        2 => render_trouble_tab(chunks[1], frame.buffer_mut(), state, trouble_scroll, theme),
        3 => render_system_tab(chunks[1], frame.buffer_mut(), state, system_scroll, theme),
        _ => {}
    }

    CenterPanelClicks {
        registry: click_registry,
    }
}

fn render_info_tab(
    area: Rect,
    buf: &mut ratatui::buffer::Buffer,
    state: &AppState,
    registry: &CommandRegistry,
    scroll: &mut ScrollableContentState,
    theme: &Theme,
) {
    let Some(command) = state.selected_command(registry) else {
        Paragraph::new("Select a task from the tree.").render(area, buf);
        return;
    };

    let mut lines = vec![
        format!("What: {}", command.ui.description),
        format!("Summary: {}", command.summary),
        String::new(),
        format!("How: {} {}", command.path_string(), command.args_summary),
    ];
    if !command.ui.fields.is_empty() {
        lines.push(String::new());
        lines.push("Fields:".to_owned());
        for field in &command.ui.fields {
            lines.push(format!("  {} — {}", field.label, field.help));
        }
    }

    scroll.set_lines(lines);
    ScrollableContent::new(scroll)
        .title("Info")
        .theme(theme)
        .render(area, buf);
}

fn render_options_tab(
    area: Rect,
    buf: &mut ratatui::buffer::Buffer,
    state: &AppState,
    registry: &CommandRegistry,
    form_field: usize,
    editing_form: bool,
    option_input: &InputState,
    _option_checkbox: &CheckBoxState,
    theme: &Theme,
    content_focused: bool,
) {
    let Some(command) = state.selected_command(registry) else {
        Paragraph::new("Select a task to edit options.").render(area, buf);
        return;
    };
    let Some(form) = state.forms.get(command.id) else {
        Paragraph::new("Loading form…").render(area, buf);
        return;
    };

    if command.ui.fields.is_empty() {
        Paragraph::new("No options for this task. Press Run or r.").render(area, buf);
        return;
    }

    let mut y = area.y;
    for (index, field) in command.ui.fields.iter().enumerate() {
        if y >= area.y + area.height {
            break;
        }
        let row = Rect::new(area.x, y, area.width, 1);
        let selected = index == form_field;
        let style = if selected && content_focused {
            dhara_theme::selected_style()
        } else {
            Style::default().fg(dhara_theme::TEXT)
        };

        match (&form.values[index], &field.kind) {
            (FormValue::Boolean(value), FieldKind::Boolean) if selected && editing_form => {
                let mut cb = CheckBoxState::new(*value);
                cb.set_focused(true);
                CheckBox::new(&field.label, &cb).theme(theme).render(row, buf);
            }
            (FormValue::Boolean(value), FieldKind::Boolean) => {
                let label = format!(
                    "{} {}",
                    if selected { "▸" } else { " " },
                    if *value { "[x]" } else { "[ ]" }
                );
                Paragraph::new(Line::styled(label, style)).render(row, buf);
            }
            (
                FormValue::Text(_),
                FieldKind::Text | FieldKind::Path | FieldKind::BrowsablePath { .. },
            ) if selected && editing_form => {
                let text = option_input.text();
                let prefix = "▸ ";
                Paragraph::new(Line::styled(
                    format!("{prefix}{}: {text}", field.label),
                    dhara_theme::selected_style(),
                ))
                .render(row, buf);
            }
            (
                FormValue::Text(text),
                FieldKind::Text | FieldKind::Path | FieldKind::BrowsablePath { .. },
            ) => {
                let prefix = if selected { "▸ " } else { "  " };
                Paragraph::new(Line::styled(
                    format!("{prefix}{}: {text}", field.label),
                    style,
                ))
                .render(row, buf);
            }
            (FormValue::Select(sel), FieldKind::Select(options)) => {
                let value = options.get(*sel).copied().unwrap_or("");
                let prefix = if selected { "▸ " } else { "  " };
                Paragraph::new(Line::styled(
                    format!("{prefix}{}: {value}", field.label),
                    style,
                ))
                .render(row, buf);
            }
            _ => {
                Paragraph::new(Line::styled(
                    format!("  {}: (unsupported)", field.label),
                    style,
                ))
                .render(row, buf);
            }
        }
        y += 1;
    }
}

fn render_trouble_tab(
    area: Rect,
    buf: &mut ratatui::buffer::Buffer,
    state: &AppState,
    scroll: &mut ScrollableContentState,
    theme: &Theme,
) {
    if state.troubleshooting_lines.is_empty() {
        Paragraph::new("Warnings and errors appear here during a run.").render(area, buf);
        return;
    }
    let lines: Vec<String> = state
        .troubleshooting_lines
        .iter()
        .map(|line| {
            let prefix = match line.severity {
                DiagnosticSeverity::Warn => "WARN",
                DiagnosticSeverity::Error => "ERR ",
            };
            format!("[{prefix}] {}", line.text)
        })
        .collect();
    scroll.set_lines(lines);
    ScrollableContent::new(scroll)
        .title("Troubleshooting")
        .theme(theme)
        .render(area, buf);
}

fn render_system_tab(
    area: Rect,
    buf: &mut ratatui::buffer::Buffer,
    state: &AppState,
    scroll: &mut ScrollableContentState,
    theme: &Theme,
) {
    let text = state
        .system_configs_text
        .as_deref()
        .unwrap_or("Open this tab to load configuration.");
    scroll.set_lines(text.lines().map(str::to_owned).collect());
    ScrollableContent::new(scroll)
        .title("System configs")
        .theme(theme)
        .render(area, buf);
}

pub fn tab_from_index(index: usize) -> MainTab {
    match index {
        1 => MainTab::Options,
        2 => MainTab::Troubleshooting,
        3 => MainTab::SystemConfigs,
        _ => MainTab::Info,
    }
}

pub fn tab_index(tab: MainTab) -> usize {
    match tab {
        MainTab::Info => 0,
        MainTab::Options => 1,
        MainTab::Troubleshooting => 2,
        MainTab::SystemConfigs => 3,
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

pub fn sync_option_widgets_from_form(
    state: &AppState,
    registry: &CommandRegistry,
    form_field: usize,
    option_input: &mut InputState,
    option_checkbox: &mut CheckBoxState,
) {
    let Some(command) = state.selected_command(registry) else {
        return;
    };
    let Some(form) = state.forms.get(command.id) else {
        return;
    };
    let Some(field) = command.ui.fields.get(form_field) else {
        return;
    };
    match (&form.values[form_field], &field.kind) {
        (
            FormValue::Text(text),
            FieldKind::Text | FieldKind::Path | FieldKind::BrowsablePath { .. },
        ) => {
            *option_input = InputState::new(text);
        }
        (FormValue::Boolean(value), FieldKind::Boolean) => {
            *option_checkbox = CheckBoxState::new(*value);
        }
        _ => {}
    }
}

pub fn apply_option_widgets_to_form(
    state: &mut AppState,
    registry: &CommandRegistry,
    form_field: usize,
    option_input: &InputState,
    option_checkbox: &CheckBoxState,
) {
    let _ = option_checkbox;
    let Some(command) = state.selected_command(registry).cloned() else {
        return;
    };
    let Some(form) = state.forms.get_mut(command.id) else {
        return;
    };
    let Some(field) = command.ui.fields.get(form_field) else {
        return;
    };
    match (&mut form.values[form_field], &field.kind) {
        (
            FormValue::Text(value),
            FieldKind::Text | FieldKind::Path | FieldKind::BrowsablePath { .. },
        ) => {
            *value = option_input.text().to_owned();
        }
        (FormValue::Boolean(value), FieldKind::Boolean) => {
            *value = option_checkbox.checked;
        }
        _ => {}
    }
}
