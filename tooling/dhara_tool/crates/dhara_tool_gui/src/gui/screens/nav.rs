use iced::widget::{column, text};
use iced::Element;

use dhara_tool_cli::command::CommandRegistry;

use super::super::app::Message;
use super::super::state::AppState;
use super::super::widgets::{
    panel::{inset_panel, PanelVariant},
    scroll_area::scroll_area,
    tree_row::tree_row,
};

pub fn view_nav<'a>(
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

    inset_panel(
        scroll_area(list.into()),
        PanelVariant::Default,
    )
}
