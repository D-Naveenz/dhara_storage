use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui_interact::components::{ScrollableContentStyle, TabViewStyle};
use ratatui_interact::theme::{ColorPalette, Theme};

pub const WORKSPACE_DISPLAY_NAME: &str = "Dhara Storage";

pub const PANEL_BG: Color = Color::Rgb(32, 32, 38);
pub const MODAL_BG: Color = Color::Rgb(44, 44, 52);
pub const BORDER: Color = Color::Rgb(64, 64, 72);
pub const TEXT: Color = Color::Rgb(220, 220, 225);
pub const MUTED: Color = Color::Rgb(140, 140, 150);
pub const ACCENT: Color = Color::Rgb(90, 160, 220);
pub const SUCCESS: Color = Color::Rgb(80, 180, 120);
pub const WARNING: Color = Color::Rgb(220, 180, 80);
pub const ERROR: Color = Color::Rgb(220, 90, 90);
pub const SELECTED_BG: Color = Color::Rgb(50, 90, 130);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationTone {
    Muted,
    Success,
    Error,
}

pub fn title_style() -> Style {
    Style::default().fg(TEXT).add_modifier(Modifier::BOLD)
}

pub fn panel_style() -> Style {
    Style::default().fg(TEXT).bg(PANEL_BG)
}

pub fn modal_panel_style() -> Style {
    Style::default().fg(TEXT).bg(MODAL_BG)
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

pub fn validation_status_style(tone: ValidationTone) -> Style {
    let color = match tone {
        ValidationTone::Muted => MUTED,
        ValidationTone::Success => SUCCESS,
        ValidationTone::Error => ERROR,
    };
    Style::default().fg(color)
}

pub fn initial_config_description_lines() -> Vec<Line<'static>> {
    vec![
        Line::from(vec![
            Span::raw("Enter the path to your "),
            Span::styled(
                WORKSPACE_DISPLAY_NAME,
                Style::default().fg(ACCENT).add_modifier(Modifier::ITALIC),
            ),
            Span::raw(" workspace."),
        ]),
        Line::from(Span::styled(
            "Must contain dhara.config.toml",
            Style::default().fg(MUTED),
        )),
    ]
}

pub fn scroll_body_style() -> ScrollableContentStyle {
    let mut style = ScrollableContentStyle::borderless();
    style.text_style = Style::default().fg(TEXT);
    style
}

pub fn tab_table_style(theme: &Theme) -> TabViewStyle {
    let mut style = TabViewStyle::from(theme);
    style.bordered_content = false;
    style.show_indicator = false;
    style.selected_style = tree_selected_style();
    style.focused_style = tree_selected_style();
    style.normal_style = Style::default().fg(TEXT);
    style
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
            surface_raised: MODAL_BG,
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
