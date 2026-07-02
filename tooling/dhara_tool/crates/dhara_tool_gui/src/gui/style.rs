use iced::border::{self, Radius};
use iced::widget::svg::Handle;
use iced::widget::button::Status;
use iced::widget::{button, container, svg};
use iced::{Background, Border, Color, ContentFit, Radians, Rotation, Theme};

const PANEL_ALPHA: f32 = 0.4;
const TAB_INACTIVE_LIFT: f32 = 0.04;
const ROW_LIFT: f32 = 0.12;
const ROW_SELECTED_LIFT: f32 = 0.16;

/// Vertical and horizontal inset for panel surfaces.
pub const PANEL_INSET: [f32; 2] = [16.0, 16.0];

/// Vertical and horizontal inset for tree row hit targets.
pub const TREE_ROW_INSET: [f32; 2] = [6.0, 10.0];

pub fn chevron_handle() -> Handle {
    Handle::from_memory(include_bytes!("../../../../assets/gui/chevron-right.svg"))
}

pub fn chevron_icon<'a, Message>() -> iced::Element<'a, Message> {
    chevron_icon_rotated::<Message>(false)
}

pub fn chevron_icon_rotated<'a, Message>(expanded: bool) -> iced::Element<'a, Message> {
    let handle = chevron_handle();
    let mut icon = svg(handle)
        .width(12)
        .height(12)
        .content_fit(ContentFit::Contain)
        .style(|theme: &Theme, _status| svg::Style {
            color: Some(theme.palette().text),
        });
    if expanded {
        icon = icon.rotation(Rotation::Solid(Radians(
            std::f32::consts::FRAC_PI_2,
        )));
    }
    icon.into()
}

pub fn panel_background(theme: &Theme) -> Color {
    let bg = theme.palette().background;
    Color {
        r: (bg.r + 0.1).min(1.0),
        g: (bg.g + 0.1).min(1.0),
        b: (bg.b + 0.1).min(1.0),
        a: PANEL_ALPHA,
    }
}

pub fn panel_border(theme: &Theme) -> Color {
    let bg = theme.palette().background;
    Color {
        r: (bg.r + 0.18).min(1.0),
        g: (bg.g + 0.18).min(1.0),
        b: (bg.b + 0.18).min(1.0),
        a: 0.55,
    }
}

pub fn tab_inactive_background(theme: &Theme) -> Color {
    let bg = theme.palette().background;
    Color {
        r: (bg.r + TAB_INACTIVE_LIFT).min(1.0),
        g: (bg.g + TAB_INACTIVE_LIFT).min(1.0),
        b: (bg.b + TAB_INACTIVE_LIFT).min(1.0),
        a: 0.55,
    }
}

fn row_background(theme: &Theme, selected: bool) -> Color {
    let lift = if selected {
        ROW_SELECTED_LIFT
    } else {
        ROW_LIFT
    };
    let bg = theme.palette().background;
    Color {
        r: (bg.r + lift).min(1.0),
        g: (bg.g + lift).min(1.0),
        b: (bg.b + lift).min(1.0),
        a: if selected { 0.68 } else { 0.50 },
    }
}

pub fn tree_row_text_color(theme: &Theme, selected: bool) -> Color {
    if selected {
        return theme.palette().text;
    }

    let text = theme.palette().text;
    Color {
        r: (text.r * 0.92 + 0.08).min(1.0),
        g: (text.g * 0.92 + 0.08).min(1.0),
        b: (text.b * 0.92 + 0.08).min(1.0),
        a: 1.0,
    }
}

pub fn panel_container(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(panel_background(theme))),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: border::radius(8),
        },
        ..Default::default()
    }
}

pub fn tab_content_panel(theme: &Theme) -> container::Style {
    let bg = panel_background(theme);
    container::Style {
        background: Some(Background::Color(bg)),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: Radius {
                top_left: 0.0,
                top_right: 8.0,
                bottom_right: 8.0,
                bottom_left: 8.0,
            },
        },
        ..Default::default()
    }
}

pub fn tree_row_container(selected: bool) -> impl Fn(&Theme) -> container::Style + Copy {
    move |theme: &Theme| container::Style {
        background: Some(Background::Color(row_background(theme, selected))),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: border::radius(6),
        },
        ..Default::default()
    }
}

pub fn tree_row_accent(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(theme.palette().primary)),
        ..Default::default()
    }
}

pub fn tab_button_style(active: bool) -> impl Fn(&Theme, Status) -> button::Style + Copy {
    move |theme: &Theme, status: Status| {
        let background = if active {
            panel_background(theme)
        } else {
            match status {
                Status::Hovered | Status::Pressed => tab_inactive_background(theme),
                _ => tab_inactive_background(theme),
            }
        };

        let (border_color, border_width) = if active {
            (Color::TRANSPARENT, 0.0)
        } else {
            (panel_border(theme), 1.0)
        };

        button::Style {
            background: Some(Background::Color(background)),
            text_color: theme.palette().text,
            border: Border {
                color: border_color,
                width: border_width,
                radius: border::top(6),
            },
            shadow: Default::default(),
            snap: false,
        }
    }
}
