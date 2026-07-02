use iced::widget::{column, container, row, stack, text};
use iced::{Element, Length};

use dhara_tool_cli::command::CommandRegistry;

use super::super::app::{DharaApp, Message};
use super::super::state::{AppState, MainTab};
use super::super::widgets::{
    action_bar::action_bar,
    tab_view::{tab_content_scroll, tab_view},
};
use super::activation::view_activation_overlay;
use super::nav::view_nav;
use super::history::view_history;
use super::options::view_options;
use super::terminal::view_terminal;

const MAIN_TABS: &[(MainTab, &str)] = &[
    (MainTab::Options, "Options"),
    (MainTab::Terminal, "Terminal"),
    (MainTab::History, "History"),
];

pub fn view_main_shell<'a>(app: &'a DharaApp) -> Element<'a, Message> {
    let tab_content = view_tab_content(&app.state, &app.registry);

    let tab_stack = tab_view(
        MAIN_TABS,
        app.state.main_tab,
        Message::TabSelected,
        tab_content,
    );

    let right_column = column![tab_stack, action_bar(&app.state, &app.registry)]
        .spacing(8)
        .width(Length::FillPortion(5))
        .height(Length::Fill);

    let body = row![
        container(view_nav(&app.state, &app.registry))
            .width(Length::FillPortion(2))
            .height(Length::Fill),
        right_column,
    ]
    .spacing(8)
    .padding(12)
    .height(Length::Fill);

    let main = container(body)
        .width(Length::Fill)
        .height(Length::Fill);

    if let Some(overlay) = view_activation_overlay(&app.state) {
        stack![main, overlay]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    } else {
        main.into()
    }
}

fn view_tab_content<'a>(
    state: &'a AppState,
    registry: &'a CommandRegistry,
) -> Element<'a, Message> {
    let content = match state.main_tab {
        MainTab::Options => view_options_tab(state, registry),
        MainTab::Terminal => view_terminal(state),
        MainTab::History => view_history(state),
    };
    tab_content_scroll(content)
}

fn view_options_tab<'a>(
    state: &'a AppState,
    registry: &'a CommandRegistry,
) -> Element<'a, Message> {
    if let Some(command) = state.selected_command(registry) {
        if let Some(form) = state.forms.get(command.id) {
            view_options(command, form)
        } else {
            text("Loading form...").into()
        }
    } else {
        text("Select a command from the tree.").size(14).into()
    }
}
