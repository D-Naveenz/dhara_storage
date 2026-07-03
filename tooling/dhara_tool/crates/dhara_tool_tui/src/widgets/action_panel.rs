use dhara_tool_kernel::RunPhase;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders, Paragraph, Widget};
use ratatui::Frame;
use ratatui_interact::components::{
    Button, ButtonState, ButtonVariant, Progress, ProgressStyle, Spinner, SpinnerState,
};
use ratatui_interact::theme::Theme;
use ratatui_interact::traits::ClickRegionRegistry;

use dhara_tool_cli::interactive::AppState;

use crate::focus::TuiFocus;
use crate::theme as dhara_theme;

pub fn render_action_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &AppState,
    theme: &Theme,
    shell_focus: &crate::focus::ShellFocus,
    run_btn: &mut ButtonState,
    cancel_btn: &mut ButtonState,
    reset_btn: &mut ButtonState,
    spinner: &mut SpinnerState,
    shell_clicks: &mut ClickRegionRegistry<TuiFocus>,
) {
    let focused_region = shell_focus.current().copied();
    let block = Block::default()
        .title(" Actions ")
        .borders(Borders::ALL)
        .border_style(if matches!(
            focused_region,
            Some(TuiFocus::ActionRun | TuiFocus::ActionCancel | TuiFocus::ActionReset)
        ) {
            dhara_theme::border_style().fg(dhara_theme::ACCENT)
        } else {
            dhara_theme::border_style()
        })
        .style(dhara_theme::panel_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let layout = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Min(3),
    ])
    .split(inner);

    render_progress(frame, layout[0], state, theme, spinner);
    render_step_label(frame, layout[1], state);
    render_status(frame, layout[2], state);

    let running = state.active_run.is_some();
    let cancelable = state
        .active_run
        .as_ref()
        .is_some_and(|run| run.cancelable);
    let can_reset = state
        .tree_view
        .selected_command_id
        .is_some();

    run_btn.set_enabled(!running);
    cancel_btn.set_enabled(running && cancelable);
    reset_btn.set_enabled(can_reset && !running);

    run_btn.set_focused(shell_focus.is_focused(&TuiFocus::ActionRun));
    cancel_btn.set_focused(shell_focus.is_focused(&TuiFocus::ActionCancel));
    reset_btn.set_focused(shell_focus.is_focused(&TuiFocus::ActionReset));

    let mut registry = ClickRegionRegistry::new();
    let button_row = Layout::horizontal([
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
    ])
    .split(layout[3]);

    let run = Button::new("Run", run_btn)
        .variant(ButtonVariant::Block)
        .theme(theme);
    let cancel = Button::new("Cancel", cancel_btn)
        .variant(ButtonVariant::Block)
        .theme(theme);
    let reset = Button::new("Reset", reset_btn)
        .variant(ButtonVariant::Block)
        .theme(theme);

    if run_btn.enabled {
        let region = run.render_stateful(button_row[0], frame.buffer_mut());
        if region.area.width > 0 {
            registry.register(region.area, TuiFocus::ActionRun);
        }
    } else {
        run.render(button_row[0], frame.buffer_mut());
    }
    if cancel_btn.enabled {
        let region = cancel.render_stateful(button_row[1], frame.buffer_mut());
        if region.area.width > 0 {
            registry.register(region.area, TuiFocus::ActionCancel);
        }
    } else {
        cancel.render(button_row[1], frame.buffer_mut());
    }
    if reset_btn.enabled {
        let region = reset.render_stateful(button_row[2], frame.buffer_mut());
        if region.area.width > 0 {
            registry.register(region.area, TuiFocus::ActionReset);
        }
    } else {
        reset.render(button_row[2], frame.buffer_mut());
    }

    for region in registry.regions() {
        shell_clicks.register(region.area, region.data.clone());
    }
}

fn render_progress(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &AppState,
    theme: &Theme,
    spinner: &mut SpinnerState,
) {
    match state.progress.as_ref() {
        Some(snapshot) if snapshot.phase == RunPhase::Analyzing => {
            spinner.tick();
            let label = if snapshot.analyzing_message.is_empty() {
                "Analyzing…"
            } else {
                snapshot.analyzing_message.as_str()
            };
            let spin = Spinner::new(spinner)
                .label(label)
                .theme(theme);
            spin.render(area, frame.buffer_mut());
        }
        Some(snapshot) => {
            let mut style = ProgressStyle::from(theme);
            if snapshot.phase == RunPhase::Complete {
                style.filled_color = dhara_theme::SUCCESS;
            }
            let progress = Progress::new(f64::from(snapshot.overall))
                .label(&snapshot.step_label)
                .style(style);
            progress.render(area, frame.buffer_mut());
            let percent = format!("{}%", snapshot.percent);
            let percent_area = Rect {
                x: area.x + area.width.saturating_sub(percent.len() as u16 + 2),
                y: area.y,
                width: percent.len() as u16,
                height: 1,
            };
            if percent_area.width > 0 && percent_area.x + percent_area.width <= area.x + area.width
            {
                Paragraph::new(percent)
                    .style(Style::default().fg(dhara_theme::TEXT))
                    .render(percent_area, frame.buffer_mut());
            }
        }
        None => {
            Progress::new(0.0)
                .label("Ready")
                .theme(theme)
                .render(area, frame.buffer_mut());
        }
    }
}

fn render_step_label(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    let text = state
        .progress
        .as_ref()
        .map(|snapshot| {
            if snapshot.phase == RunPhase::Analyzing {
                snapshot.analyzing_message.clone()
            } else {
                snapshot.step_label.clone()
            }
        })
        .unwrap_or_default();
    Paragraph::new(text)
        .style(Style::default().fg(dhara_theme::MUTED))
        .render(area, frame.buffer_mut());
}

fn render_status(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    Paragraph::new(state.status_message.as_str())
        .style(dhara_theme::status_style_for_tone(state.status_tone))
        .render(area, frame.buffer_mut());
}
