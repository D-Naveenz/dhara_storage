use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, Widget};
use ratatui::Frame;

use dhara_tool_cli::interactive::{AppState, MainTab};

use crate::focus::{ShellFocus, TuiFocus};
use crate::theme;

pub struct FooterContext<'a> {
    pub state: &'a AppState,
    pub shell_focus: &'a ShellFocus,
    pub modal_hints: Option<Vec<(&'static str, &'static str)>>,
    pub editing_form: bool,
}

pub fn footer_hints(ctx: &FooterContext<'_>) -> Vec<(&'static str, &'static str)> {
    if let Some(hints) = &ctx.modal_hints {
        return hints.clone();
    }

    let running = ctx.state.active_run.is_some();
    let cancelable = ctx
        .state
        .active_run
        .as_ref()
        .is_some_and(|run| run.cancelable);

    let mut hints = Vec::new();
    match ctx.shell_focus.current() {
        Some(TuiFocus::TaskTree) => {
            hints.push(("↑↓", "navigate"));
            hints.push(("Enter", "select/expand"));
            hints.push(("Click", "select"));
        }
        Some(TuiFocus::MainTabs) => {
            hints.push(("←→", "switch tab"));
            hints.push(("Click", "tab"));
        }
        Some(TuiFocus::TabContent) => match ctx.state.main_tab {
            MainTab::Options if ctx.editing_form => {
                hints.push(("Type", "edit"));
                hints.push(("←→", "cycle select"));
                hints.push(("Enter", "done"));
                hints.push(("Esc", "cancel edit"));
            }
            MainTab::Options => {
                hints.push(("↑↓", "field"));
                hints.push(("Enter", "edit"));
                hints.push(("Click", "field"));
            }
            MainTab::Troubleshooting | MainTab::SystemConfigs => {
                hints.push(("↑↓", "scroll"));
                hints.push(("PgUp/Dn", "page"));
                hints.push(("Wheel", "scroll"));
            }
            MainTab::Info => {
                hints.push(("↑↓", "scroll"));
                hints.push(("Wheel", "scroll"));
            }
        },
        Some(TuiFocus::ActionRun) | Some(TuiFocus::ActionCancel) | Some(TuiFocus::ActionReset) => {
            hints.push(("Enter", "activate"));
            hints.push(("Click", "button"));
            if !running {
                hints.push(("r", "run"));
            }
            if running && cancelable {
                hints.push(("c", "cancel"));
            }
        }
        None => {}
    }

    if hints.is_empty() {
        hints.push(("Tab", "focus"));
        hints.push(("q", "quit"));
    } else {
        hints.push(("Tab", "next panel"));
        hints.push(("Hover", "focus panel"));
        if !running {
            hints.push(("q", "quit"));
        } else if cancelable {
            hints.push(("c", "cancel"));
        }
    }

    hints
}

fn hints_line(hints: &[(&str, &str)]) -> Line<'static> {
    let mut spans = Vec::new();
    for (idx, (key, desc)) in hints.iter().enumerate() {
        if idx > 0 {
            spans.push(Span::raw(" | "));
        }
        spans.push(Span::styled(
            (*key).to_owned(),
            Style::default().fg(Color::Green),
        ));
        spans.push(Span::raw(format!(": {desc}")));
    }
    Line::from(spans)
}

pub fn render_command_bar(frame: &mut Frame<'_>, area: Rect, ctx: &FooterContext<'_>) {
    let hints = footer_hints(ctx);
    let line = hints_line(&hints);

    frame.render_widget(Block::default().style(theme::bar_style()), area);
    Paragraph::new(line)
        .style(theme::bar_style())
        .render(
            Rect {
                x: area.x + 1,
                y: area.y,
                width: area.width.saturating_sub(2),
                height: 1,
            },
            frame.buffer_mut(),
        );
}
