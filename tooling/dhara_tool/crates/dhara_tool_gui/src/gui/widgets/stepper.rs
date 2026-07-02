use iced::widget::{button, row, text_input};
use iced::{Element, Length};

/// Clamps `value` to `[min, max]` when bounds are provided.
pub fn clamp_stepper_value(value: i64, min: Option<i64>, max: Option<i64>) -> i64 {
    let mut next = value;
    if let Some(min) = min {
        next = next.max(min);
    }
    if let Some(max) = max {
        next = next.min(max);
    }
    next
}

pub fn stepper<'a, Message: Clone + 'a>(
    value: i64,
    min: Option<i64>,
    max: Option<i64>,
    on_change: impl Fn(i64) -> Message + Copy + 'a,
) -> Element<'a, Message> {
    let display = value.to_string();
    row![
        button("−").on_press(on_change(clamp_stepper_value(value - 1, min, max))),
        text_input("", &display)
            .on_input(move |input| {
                let parsed = input.parse::<i64>().unwrap_or(value);
                on_change(clamp_stepper_value(parsed, min, max))
            })
            .padding(6)
            .width(Length::Fixed(64.0)),
        button("+").on_press(on_change(clamp_stepper_value(value + 1, min, max))),
    ]
    .spacing(6)
    .align_y(iced::Alignment::Center)
    .into()
}

#[cfg(test)]
mod tests {
    use super::clamp_stepper_value;

    #[test]
    fn clamp_respects_bounds() {
        assert_eq!(clamp_stepper_value(5, Some(0), Some(10)), 5);
        assert_eq!(clamp_stepper_value(-1, Some(0), None), 0);
        assert_eq!(clamp_stepper_value(99, None, Some(10)), 10);
    }
}
