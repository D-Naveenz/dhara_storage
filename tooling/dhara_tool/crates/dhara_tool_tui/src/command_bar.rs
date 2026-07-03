use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, Paragraph, Widget};
use ratatui::Frame;
use ratatui_interact::components::key_hints_footer;

use dhara_tool_cli::interactive::{AppState, MainTab};

use crate::focus::{ShellFocus, TuiFocus};
use crate::theme;

pub struct FooterContext<'a> {
    pub state: &'a AppState,
    pub shell_focus: &'a ShellFocus,
    pub repo_setup: bool,
    pub editing_form: bool,
}

pub fn footer_hints(ctx: &FooterContext<'_>) -> Vec<(&'static str, &'static str)> {
    if ctx.repo_setup {
        return vec![
            ("Type", "path"),
            ("Enter", "confirm"),
            ("Esc", "quit"),
        ];
    }
    if ctx.state.activation_prompt.is_some() {
        return vec![
            ("Tab", "buttons"),
            ("Enter/Y", "apply"),
            ("Esc/N", "decline"),
            ("Click", "button"),
        ];
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
            }
            MainTab::Troubleshooting | MainTab::SystemConfigs => {
                hints.push(("↑↓", "scroll"));
                hints.push(("PgUp/Dn", "page"));
                hints.push(("Wheel", "scroll"));
            }
            MainTab::Info => {
                hints.push(("↑↓", "scroll"));
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
        if !running {
            hints.push(("q", "quit"));
        } else if cancelable {
            hints.push(("c", "cancel"));
        }
    }

    hints
}

pub fn render_command_bar(frame: &mut Frame<'_>, area: Rect, ctx: &FooterContext<'_>) {
    let hints = footer_hints(ctx);
    let lines = key_hints_footer(&hints);
    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(theme::border_style())
        .style(theme::panel_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);
    Paragraph::new(lines)
        .style(theme::panel_style())
        .render(inner, frame.buffer_mut());
}
