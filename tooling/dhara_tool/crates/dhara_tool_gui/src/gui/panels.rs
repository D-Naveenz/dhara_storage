use iced::widget::{
    button, column, container, mouse_area, progress_bar, row, scrollable, space, text,
};
use iced::{Alignment, Element, Length, Padding, Theme};

use dhara_tool_cli::command::CommandRegistry;

use super::app::Message;
use super::form::view_options_form;
use super::state::{AppState, MainTab, OutputLine};
use super::style::{
    chevron_icon_rotated, panel_container, tab_button_style, tree_row_accent, tree_row_container,
    tree_row_text_color, PANEL_INSET, TREE_ROW_INSET,
};
use super::tree::VisibleTreeRow;

pub fn view_tree_nav<'a>(
    state: &'a AppState,
    _registry: &'a CommandRegistry,
) -> Element<'a, Message> {
    let rows = state.tree_view.visible_rows(&state.nav_tree);
    let mut list = column![text("Tasks").size(14)].spacing(4);

    for row in rows {
        let is_selected = row
            .node
            .command_id
            .is_some_and(|id| state.tree_view.selected_command_id == Some(id));
        list = list.push(tree_row(&row, is_selected));
    }

    container(scrollable(list).height(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(PANEL_INSET)
        .style(|theme: &Theme| panel_container(theme))
        .into()
}

fn tree_row<'a>(row: &VisibleTreeRow, selected: bool) -> Element<'a, Message> {
    let indent = container(space::horizontal()).width(f32::from(row.depth as u16 * 16));

    let chevron_cell: Element<'a, Message> = if row.has_children {
        container(chevron_icon_rotated(row.expanded))
            .width(16)
            .height(12)
            .align_y(Alignment::Center)
            .into()
    } else {
        container(space::horizontal()).width(16).into()
    };

    let accent: Element<'a, Message> = if selected {
        container(space::horizontal())
            .width(3)
            .style(|theme: &Theme| tree_row_accent(theme))
            .into()
    } else {
        container(space::horizontal()).width(0).into()
    };

    let label = text(row.node.label.clone()).size(13).style(move |theme: &Theme| {
        text::Style {
            color: Some(tree_row_text_color(theme, selected)),
        }
    });

    let row_content = row![indent, chevron_cell, accent, label]
        .spacing(4)
        .align_y(Alignment::Center)
        .width(Length::Fill)
        .height(Length::Shrink);

    let interactive = container(row_content)
        .width(Length::Fill)
        .height(Length::Shrink)
        .padding(Padding::from(TREE_ROW_INSET))
        .style(tree_row_container(selected));

    let message = if row.has_children {
        Message::TreeToggleExpand(row.node.path_key.clone())
    } else if let Some(command_id) = row.node.command_id {
        Message::CommandSelected(command_id)
    } else {
        Message::TreeToggleExpand(row.node.path_key.clone())
    };

    mouse_area(interactive)
        .on_press(message)
        .interaction(iced::mouse::Interaction::Pointer)
        .into()
}

pub fn view_tab_bar<'a>(active: MainTab) -> Element<'a, Message> {
    row![
        tab_button("Options", MainTab::Options, active),
        tab_button("Terminal", MainTab::Terminal, active),
        tab_button("History", MainTab::History, active),
    ]
    .spacing(0)
    .height(Length::Shrink)
    .into()
}

fn tab_button<'a>(label: &'a str, tab: MainTab, active: MainTab) -> Element<'a, Message> {
    let is_active = tab == active;
    button(text(label).size(13))
        .padding([8, 16])
        .style(tab_button_style(is_active))
        .on_press(Message::TabSelected(tab))
        .into()
}

pub fn view_tab_content<'a>(
    state: &'a AppState,
    registry: &'a CommandRegistry,
) -> Element<'a, Message> {
    match state.main_tab {
        MainTab::Options => view_options_tab(state, registry),
        MainTab::Terminal => view_terminal_tab(state),
        MainTab::History => view_history_tab(state),
    }
}

fn view_options_tab<'a>(
    state: &'a AppState,
    registry: &'a CommandRegistry,
) -> Element<'a, Message> {
    let content: Element<'a, Message> = if let Some(command) = state.selected_command(registry) {
        if let Some(form) = state.forms.get(command.id) {
            view_options_form(command, form)
        } else {
            text("Loading form...").into()
        }
    } else {
        text("Select a command from the tree.").size(14).into()
    };

    scrollable(container(content).padding(PANEL_INSET).width(Length::Fill))
        .height(Length::Fill)
        .into()
}

fn view_terminal_tab<'a>(state: &'a AppState) -> Element<'a, Message> {
    let lines = state.terminal_lines();
    view_output_lines(lines)
}

fn view_history_tab<'a>(state: &'a AppState) -> Element<'a, Message> {
    let mut layout = row![].spacing(8).width(Length::Fill).height(Length::Fill);

    let mut history_list = column![text("Recent runs").size(14)].spacing(4).width(Length::FillPortion(1));
    if state.session_history.is_empty() {
        history_list = history_list.push(text("No history yet.").size(13));
    } else {
        for (index, entry) in state.session_history.iter().enumerate().rev() {
            let selected = state.selected_history_index == Some(index);
            let label = format!("[{}] {}", entry.status, entry.label);
            let mut item = button(text(label).size(12)).width(Length::Fill).padding([4, 8]);
            if selected {
                item = item.style(button::primary);
            }
            history_list =
                history_list.push(item.on_press(Message::HistorySelected(index)));
        }
    }

    let preview = view_output_lines(state.history_preview_lines());
    layout = layout
        .push(scrollable(history_list).height(Length::Fill))
        .push(container(preview).width(Length::FillPortion(2)).height(Length::Fill));

    container(layout).padding(PANEL_INSET).into()
}

fn view_output_lines(lines: &[OutputLine]) -> Element<'static, Message> {
    let mut output = column![].spacing(2).width(Length::Fill);
    if lines.is_empty() {
        output = output.push(text("No output yet.").size(13));
    } else {
        for line in lines {
            let content = line.text.clone();
            if line.is_error {
                output = output.push(
                    text(content)
                        .size(12)
                        .style(|theme: &Theme| text::Style {
                            color: Some(theme.extended_palette().danger.strong.color),
                        }),
                );
            } else {
                output = output.push(text(content).size(12));
            }
        }
    }

    scrollable(output)
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
}

pub fn view_action_bar<'a>(
    state: &'a AppState,
    registry: &'a CommandRegistry,
) -> Element<'a, Message> {
    let progress_value = state
        .progress
        .as_ref()
        .map(|progress| progress.value)
        .unwrap_or(0.0);
    let progress_label = state
        .progress
        .as_ref()
        .map(|progress| progress.label.as_str())
        .unwrap_or(&state.status_message);

    let running = state.active_run.is_some();
    let cancelable = state
        .active_run
        .as_ref()
        .is_some_and(|run| run.cancelable);

    let can_reset = state.selected_command(registry).is_some();

    let mut actions = row![
        button("Run")
            .on_press_maybe((!running).then_some(Message::RunPressed))
            .padding([8, 16]),
        button("Cancel")
            .on_press_maybe((running && cancelable).then_some(Message::CancelPressed))
            .padding([8, 16]),
        button("Reset")
            .on_press_maybe((can_reset && !running).then_some(Message::ResetFormPressed))
            .padding([8, 16]),
    ]
    .spacing(8);

    if state.activation_prompt.is_some() {
        actions = actions.push(button("Apply drift").on_press(Message::ActivationConfirm));
        actions = actions.push(button("Decline").on_press(Message::ActivationDecline));
    }

    container(
        column![
            progress_bar(0.0..=1.0, progress_value),
            text(progress_label).size(12),
            actions,
        ]
        .spacing(8)
        .width(Length::Fill),
    )
    .padding(PANEL_INSET)
    .width(Length::Fill)
    .style(|theme: &Theme| panel_container(theme))
    .into()
}

pub fn view_activation_overlay<'a>(state: &'a AppState) -> Option<Element<'a, Message>> {
    let prompt = state.activation_prompt.as_ref()?;
    let mut lines = column![
        text("Configuration drift detected").size(16),
        text("Apply updates from dhara.config.toml?").size(13),
    ]
    .spacing(8);

    for drift in &prompt.drifts {
        lines = lines.push(text(format!("• {}", drift.summary)).size(12));
    }

    lines = lines.push(
        row![
            button("Apply").on_press(Message::ActivationConfirm),
            button("Decline").on_press(Message::ActivationDecline),
        ]
        .spacing(8),
    );

    Some(
        container(
            container(lines)
                .padding(16)
                .style(|theme: &Theme| container::Style {
                    background: Some(iced::Background::Color(theme.palette().background)),
                    border: iced::Border {
                        color: theme.palette().primary,
                        width: 1.0,
                        radius: 6.0.into(),
                    },
                    ..Default::default()
                }),
        )
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|theme: &Theme| container::Style {
            background: Some(iced::Background::Color(iced::Color {
                a: 0.65,
                ..theme.palette().background
            })),
            ..Default::default()
        })
        .into(),
    )
}
