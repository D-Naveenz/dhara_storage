use ratatui::style::{Color, Modifier, Style};
use ratatui_interact::theme::{ColorPalette, Theme};

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

pub fn bar_style() -> Style {
    Style::default()
        .fg(TEXT)
        .bg(Color::Rgb(40, 40, 46))
}

pub fn border_only_style() -> Style {
    Style::default().fg(TEXT)
}

pub fn border_style() -> Style {
    Style::default().fg(BORDER)
}

pub fn selected_style() -> Style {
    Style::default().fg(Color::White).bg(SELECTED_BG)
}

pub fn tree_selected_style() -> Style {
    Style::default()
        .fg(WARNING)
        .bg(SELECTED_BG)
        .add_modifier(Modifier::BOLD)
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

pub fn interact_theme() -> Theme {
    Theme {
        name: "Dhara".to_owned(),
        palette: ColorPalette {
            primary: ACCENT,
            secondary: ACCENT,
            text: TEXT,
            text_dim: MUTED,
            text_disabled: MUTED,
            text_placeholder: MUTED,
            text_muted: MUTED,
            bg: PANEL_BG,
            surface: PANEL_BG,
            surface_raised: Color::Rgb(40, 40, 46),
            border_focused: ACCENT,
            border: BORDER,
            border_disabled: BORDER,
            border_accent: ACCENT,
            separator: BORDER,
            highlight_fg: Color::White,
            highlight_bg: SELECTED_BG,
            menu_highlight_fg: Color::White,
            menu_highlight_bg: SELECTED_BG,
            pressed_fg: Color::White,
            pressed_bg: Color::Rgb(70, 110, 150),
            success: SUCCESS,
            warning: WARNING,
            error: ERROR,
            info: ACCENT,
            diff_add_fg: SUCCESS,
            diff_add_bg: Color::Rgb(30, 50, 35),
            diff_del_fg: ERROR,
            diff_del_bg: Color::Rgb(50, 30, 30),
        },
    }
}
