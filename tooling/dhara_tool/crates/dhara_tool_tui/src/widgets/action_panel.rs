use dhara_tool_kernel::RunPhase;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::widgets::{Block, Borders, Widget};
use ratatui::Frame;
use ratatui_interact::components::{
    ButtonState, Progress, ProgressStyle, Spinner, SpinnerState,
};
use ratatui_interact::theme::Theme;
use ratatui_interact::traits::ClickRegionRegistry;

use dhara_tool_cli::interactive::AppState;

use crate::focus::TuiFocus;
use crate::theme as dhara_theme;
use crate::widgets::{panel, padded_button, status_line};
use crate::theme::ValidationTone;

fn dhara_progress_style(theme: &Theme) -> ProgressStyle {
    let mut style = ProgressStyle::from(theme);
    style.bordered = false;
    style.unfilled_color = dhara_theme::PANEL_BG;
    style.label_style = style.label_style.remove_modifier(ratatui::style::Modifier::BOLD);
    style
}

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
    let focused = matches!(
        focused_region,
        Some(TuiFocus::ActionRun | TuiFocus::ActionCancel | TuiFocus::ActionReset)
    );
    let inner = panel::render_panel(frame, area, "Actions", focused, true);

    let layout = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(1),
        Constraint::Length(padded_button::BUTTON_SLOT_HEIGHT),
    ])
    .split(inner);

    render_progress(frame, layout[0], state, theme);
    render_status(frame, layout[1], state, spinner, theme);

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

    let button_row = Layout::horizontal([
        Constraint::Ratio(1, 3),
        Constraint::Length(padded_button::BUTTON_GAP),
        Constraint::Ratio(1, 3),
        Constraint::Length(padded_button::BUTTON_GAP),
        Constraint::Ratio(1, 3),
    ])
    .split(layout[2]);

    let buf = frame.buffer_mut();
    let slots = [
        (button_row[0], "Run", run_btn, TuiFocus::ActionRun),
        (button_row[2], "Cancel", cancel_btn, TuiFocus::ActionCancel),
        (button_row[4], "Reset", reset_btn, TuiFocus::ActionReset),
    ];
    for (slot, label, btn, focus) in slots {
        padded_button::render_padded_button(slot, label, btn, theme, buf);
        shell_clicks.register(slot, focus);
    }
}

fn render_progress(frame: &mut Frame<'_>, area: Rect, state: &AppState, theme: &Theme) {
    let ratio = match state.progress.as_ref() {
        Some(snapshot) if snapshot.phase == RunPhase::Analyzing => 0.0,
        Some(snapshot) => f64::from(snapshot.overall),
        None => 0.0,
    };

    let mut style = dhara_progress_style(theme);
    if state
        .progress
        .as_ref()
        .is_some_and(|snapshot| snapshot.phase == RunPhase::Complete)
    {
        style.filled_color = dhara_theme::SUCCESS;
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(dhara_theme::border_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    Progress::new(ratio)
        .style(style)
        .render(inner, frame.buffer_mut());
}

fn format_status_line(state: &AppState) -> String {
    if let Some(snapshot) = state.progress.as_ref() {
        if snapshot.phase == RunPhase::Analyzing && !snapshot.analyzing_message.is_empty() {
            return snapshot.analyzing_message.clone();
        }
        if !snapshot.step_label.is_empty() {
            if let Some(secs) = snapshot.elapsed_secs {
                return format!("{}… ({secs}s)", snapshot.step_label.trim_end_matches('…'));
            }
            return snapshot.step_label.clone();
        }
        if !snapshot.activity_label.is_empty() {
            if let Some(secs) = snapshot.elapsed_secs {
                return format!("{}… ({secs}s)", snapshot.activity_label);
            }
            return format!("{}…", snapshot.activity_label);
        }
    }
    state.status_message.clone()
}

fn status_tone(state: &AppState) -> ValidationTone {
    match state.status_tone {
        dhara_tool_cli::StatusTone::Ready => ValidationTone::Muted,
        dhara_tool_cli::StatusTone::Running => ValidationTone::Muted,
        dhara_tool_cli::StatusTone::Success => ValidationTone::Success,
        dhara_tool_cli::StatusTone::Failed => ValidationTone::Error,
        dhara_tool_cli::StatusTone::Warning => ValidationTone::Muted,
    }
}

fn render_status(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &AppState,
    spinner: &mut SpinnerState,
    theme: &Theme,
) {
    let status = format_status_line(state);
    if state.active_run.is_some() && area.width > 4 {
        let spin_area = Rect::new(area.x, area.y, 2, 1);
        Spinner::new(spinner).theme(theme).render(spin_area, frame.buffer_mut());
        status_line::render_status_line(
            Rect::new(area.x + 2, area.y, area.width.saturating_sub(2), 1),
            &status,
            status_tone(state),
            frame.buffer_mut(),
        );
    } else {
        status_line::render_status_line(area, &status, status_tone(state), frame.buffer_mut());
    }
}
