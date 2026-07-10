use ratatui::layout::{Constraint, Rect};
use ratatui_interact::components::ButtonState;
use ratatui_interact::theme::Theme;
use ratatui_interact::traits::{ClickRegionRegistry, ContainerAction};

use super::padded_button::{self, BUTTON_GAP, BUTTON_MIN_WIDTH, BUTTON_SLOT_HEIGHT};

pub struct ModalFooterButton {
    pub label: &'static str,
    pub action: ContainerAction,
    pub state: ButtonState,
}

pub fn render_modal_footer(
    area: Rect,
    buttons: &mut [ModalFooterButton],
    focused_index: usize,
    theme: &Theme,
    clicks: &mut ClickRegionRegistry<ContainerAction>,
    buf: &mut ratatui::buffer::Buffer,
) {
    clicks.clear();
    if buttons.is_empty() || area.height == 0 {
        return;
    }

    for (index, button) in buttons.iter_mut().enumerate() {
        button.state.set_focused(index == focused_index);
    }

    let count = buttons.len() as u16;
    let gap_total = BUTTON_GAP.saturating_mul(count.saturating_sub(1));
    let min_total = BUTTON_MIN_WIDTH
        .saturating_mul(count)
        .saturating_add(gap_total);
    let slot_width = if area.width >= min_total {
        (area.width.saturating_sub(gap_total)) / count
    } else {
        area.width / count.max(1)
    }
    .max(BUTTON_MIN_WIDTH.min(area.width));

    let mut x = area.x + (area.width.saturating_sub(slot_width * count + gap_total)) / 2;
    let button_area = Rect::new(x, area.y, slot_width, BUTTON_SLOT_HEIGHT.min(area.height));

    for button in buttons.iter_mut() {
        let slot = Rect::new(x, area.y, slot_width, BUTTON_SLOT_HEIGHT.min(area.height));
        padded_button::render_padded_button(slot, button.label, &button.state, theme, buf);
        clicks.register(slot, button.action.clone());
        x = x.saturating_add(slot_width + BUTTON_GAP);
        let _ = button_area;
    }
}

pub fn footer_layout_constraints() -> [Constraint; 4] {
    [
        Constraint::Min(2),
        Constraint::Length(3),
        Constraint::Length(1),
        Constraint::Length(padded_button::BUTTON_SLOT_HEIGHT),
    ]
}
