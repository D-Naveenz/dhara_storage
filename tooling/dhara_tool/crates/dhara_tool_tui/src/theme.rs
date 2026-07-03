use ratatui::style::{Color, Modifier, Style};

pub const PANEL_BG: Color = Color::Rgb(32, 32, 38);
pub const BORDER: Color = Color::Rgb(64, 64, 72);
pub const TEXT: Color = Color::Rgb(220, 220, 225);
pub const MUTED: Color = Color::Rgb(140, 140, 150);
pub const ACCENT: Color = Color::Rgb(90, 160, 220);
pub const SUCCESS: Color = Color::Rgb(80, 180, 120);
pub const WARNING: Color = Color::Rgb(220, 180, 80);
pub const ERROR: Color = Color::Rgb(220, 90, 90);
pub const SELECTED_BG: Color = Color::Rgb(50, 90, 130);

pub fn title_style() -> Style {
    Style::default().fg(TEXT).add_modifier(Modifier::BOLD)
}

pub fn panel_style() -> Style {
    Style::default().fg(TEXT).bg(PANEL_BG)
}

pub fn border_style() -> Style {
    Style::default().fg(BORDER)
}

pub fn selected_style() -> Style {
    Style::default().fg(Color::White).bg(SELECTED_BG)
}

pub fn status_style_for_tone(tone: dhara_tool_cli::StatusTone) -> Style {
    let color = match tone {
        dhara_tool_cli::StatusTone::Ready => TEXT,
        dhara_tool_cli::StatusTone::Running => ACCENT,
        dhara_tool_cli::StatusTone::Success => SUCCESS,
        dhara_tool_cli::StatusTone::Failed => ERROR,
        dhara_tool_cli::StatusTone::Warning => WARNING,
    };
    Style::default().fg(color)
}
