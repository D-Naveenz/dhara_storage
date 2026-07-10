use ratatui::layout::Rect;
use ratatui::Frame;
use ratatui_interact::components::{Input, InputState};
use ratatui_interact::theme::Theme;
use ratatui_interact::traits::ClickRegion;

pub fn render_path_input(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &InputState,
    placeholder: &str,
    theme: &Theme,
) -> ClickRegion<ratatui_interact::components::InputAction> {
    Input::new(state)
        .placeholder(placeholder)
        .theme(theme)
        .render_stateful(frame, area)
}

pub fn render_field_input(
    frame: &mut Frame<'_>,
    area: Rect,
    label: &str,
    state: &InputState,
    theme: &Theme,
) -> ClickRegion<ratatui_interact::components::InputAction> {
    Input::new(state)
        .label(label)
        .with_border(false)
        .theme(theme)
        .render_stateful(frame, area)
}
