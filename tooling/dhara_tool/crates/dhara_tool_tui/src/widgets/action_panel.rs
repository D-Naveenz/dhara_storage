use dhara_tool_kernel::RunPhase;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, Paragraph};
use ratatui::Frame;

use dhara_tool_cli::interactive::AppState;

use crate::theme;

const ACTION_LABELS: &[&str] = &["Run", "Cancel", "Reset"];

pub fn render_action_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &AppState,
    focused: bool,
    action_button: usize,
) {
    let block = Block::default()
        .title(" Actions ")
        .borders(Borders::ALL)
        .border_style(if focused {
            theme::border_style().fg(theme::ACCENT)
        } else {
            theme::border_style()
        })
        .style(theme::panel_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let layout = ratatui::layout::Layout::vertical([
        ratatui::layout::Constraint::Length(3),
        ratatui::layout::Constraint::Length(2),
        ratatui::layout::Constraint::Length(1),
        ratatui::layout::Constraint::Min(1),
    ])
    .split(inner);

    render_progress(frame, layout[0], state);
    render_step_label(frame, layout[1], state);
    render_status(frame, layout[2], state);

    let running = state.active_run.is_some();
    let cancelable = state
        .active_run
        .as_ref()
        .is_some_and(|run| run.cancelable);
    let can_reset = state.selected_command_id().is_some();

    let mut labels = Vec::new();
    for (index, label) in ACTION_LABELS.iter().enumerate() {
        let enabled = match *label {
            "Run" => !running,
            "Cancel" => running && cancelable,
            "Reset" => can_reset && !running,
            _ => false,
        };
        let style = if focused && action_button == index && enabled {
            theme::selected_style()
        } else if enabled {
            Style::default().fg(theme::TEXT)
        } else {
            Style::default().fg(theme::MUTED).add_modifier(Modifier::DIM)
        };
        labels.push(Span::styled(format!(" {label} "), style));
        labels.push(Span::raw(" "));
    }

    frame.render_widget(Paragraph::new(Line::from(labels)), layout[3]);
}

fn render_progress(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    let (ratio, label, percent_text) = match state.progress.as_ref() {
        Some(snapshot) => match snapshot.phase {
            RunPhase::Analyzing => (
                0.0,
                snapshot.analyzing_message.clone(),
                "...".to_owned(),
            ),
            RunPhase::Running | RunPhase::Complete => (
                snapshot.overall,
                snapshot.step_label.clone(),
                format!("{}%", snapshot.percent),
            ),
        },
        None => (0.0, String::new(), "0%".to_owned()),
    };

    let gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Progress ")
                .border_style(theme::border_style()),
        )
        .gauge_style(Style::default().fg(theme::ACCENT).bg(theme::PANEL_BG))
        .ratio(ratio as f64)
        .label(label);
    frame.render_widget(gauge, area);

    let percent_area = Rect {
        x: area.x + area.width / 2 - percent_text.len() as u16 / 2,
        y: area.y + 1,
        width: percent_text.len() as u16,
        height: 1,
    };
    if percent_area.width > 0 && percent_area.x + percent_area.width <= area.x + area.width {
        frame.render_widget(
            Paragraph::new(percent_text).style(Style::default().fg(theme::TEXT).add_modifier(Modifier::BOLD)),
            percent_area,
        );
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
    frame.render_widget(
        Paragraph::new(text).style(Style::default().fg(theme::MUTED)),
        area,
    );
}

fn render_status(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    frame.render_widget(
        Paragraph::new(state.status_message.as_str())
            .style(theme::status_style_for_tone(state.status_tone)),
        area,
    );
}

trait AppStateExt {
    fn selected_command_id(&self) -> Option<&'static str>;
}

impl AppStateExt for AppState {
    fn selected_command_id(&self) -> Option<&'static str> {
        self.tree_view.selected_command_id
    }
}
