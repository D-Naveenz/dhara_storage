use crossterm::event::{KeyCode, MouseEvent};
use dhara_tool_cli::command::CommandRegistry;
use dhara_tool_cli::interactive::{AppState, NavTree, TreeNode as NavNode, TreeViewState as NavTreeState};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders, Widget};
use ratatui::Frame;
use ratatui_interact::components::{
    MarqueeMode, MarqueeState, MarqueeStyle, MarqueeText, TreeNode, TreeStyle, TreeView,
    TreeViewState as WidgetTreeState, get_selected_id,
};
use ratatui_interact::theme::Theme;
use unicode_width::UnicodeWidthStr;

use crate::theme as dhara_theme;

#[derive(Debug, Clone)]
pub struct TaskTreeData {
    pub label: String,
    pub path_key: String,
    pub command_id: Option<&'static str>,
    pub has_children: bool,
}

struct FlatNode<'a> {
    node: &'a TreeNode<TaskTreeData>,
    depth: usize,
    is_last: bool,
    parent_is_last: Vec<bool>,
}

pub fn build_tree_nodes(nav: &NavTree) -> Vec<TreeNode<TaskTreeData>> {
    nav.roots.iter().map(convert_nav_node).collect()
}

fn convert_nav_node(node: &NavNode) -> TreeNode<TaskTreeData> {
    let has_children = !node.children.is_empty();
    let children = node.children.iter().map(convert_nav_node).collect();
    TreeNode::new(
        node.path_key.clone(),
        TaskTreeData {
            label: node.label.clone(),
            path_key: node.path_key.clone(),
            command_id: node.command_id,
            has_children,
        },
    )
    .with_children(children)
}

pub fn sync_widget_from_nav(
    widget: &mut WidgetTreeState,
    nav: &NavTreeState,
    nodes: &[TreeNode<TaskTreeData>],
    task_row: usize,
) {
    widget.collapsed.clear();
    collect_collapsed(nodes, nav, widget);

    let tree = TreeView::new(nodes, widget);
    let visible = tree.visible_count();
    if visible == 0 {
        widget.selected_index = 0;
        return;
    }
    widget.selected_index = task_row.min(visible - 1);
    widget.ensure_visible(visible.max(1));
}

fn collect_collapsed(
    nodes: &[TreeNode<TaskTreeData>],
    nav: &NavTreeState,
    widget: &mut WidgetTreeState,
) {
    for node in nodes {
        if node.has_children() && !nav.expanded.contains(&node.data.path_key) {
            widget.collapse(&node.id);
        } else if node.has_children() {
            widget.expand(&node.id);
        }
        collect_collapsed(&node.children, nav, widget);
    }
}

pub fn sync_nav_from_widget(
    widget: &WidgetTreeState,
    nav: &mut NavTreeState,
    nodes: &[TreeNode<TaskTreeData>],
) {
    sync_expanded(nodes, nav, widget);
}

fn sync_expanded(
    nodes: &[TreeNode<TaskTreeData>],
    nav: &mut NavTreeState,
    widget: &WidgetTreeState,
) {
    for node in nodes {
        if node.has_children() {
            if widget.is_collapsed(&node.id) {
                nav.expanded.remove(&node.data.path_key);
            } else {
                nav.expanded.insert(node.data.path_key.clone());
            }
            sync_expanded(&node.children, nav, widget);
        }
    }
}

pub fn visible_count(nodes: &[TreeNode<TaskTreeData>], widget: &WidgetTreeState) -> usize {
    TreeView::new(nodes, widget).visible_count()
}

fn flatten_visible<'a>(
    nodes: &'a [TreeNode<TaskTreeData>],
    widget: &WidgetTreeState,
) -> Vec<FlatNode<'a>> {
    let mut result = Vec::new();
    flatten_nodes(nodes, widget, 0, &mut result, &[]);
    result
}

fn flatten_nodes<'a>(
    nodes: &'a [TreeNode<TaskTreeData>],
    widget: &WidgetTreeState,
    depth: usize,
    result: &mut Vec<FlatNode<'a>>,
    parent_is_last: &[bool],
) {
    let count = nodes.len();
    for (idx, node) in nodes.iter().enumerate() {
        let is_last = idx == count - 1;
        result.push(FlatNode {
            node,
            depth,
            is_last,
            parent_is_last: parent_is_last.to_vec(),
        });
        if node.has_children() && !widget.is_collapsed(&node.id) {
            let mut parents = parent_is_last.to_vec();
            parents.push(is_last);
            flatten_nodes(&node.children, widget, depth + 1, result, &parents);
        }
    }
}

fn tree_style(theme: &Theme) -> TreeStyle {
    let mut style = TreeStyle::from(theme);
    style.selected_style = dhara_theme::tree_selected_style();
    style.normal_style = Style::default().fg(dhara_theme::TEXT);
    style.cursor_normal = "";
    style.cursor_selected = "";
    style
}

fn build_prefix(
    style: &TreeStyle,
    flat: &FlatNode<'_>,
    is_selected: bool,
    widget: &WidgetTreeState,
) -> (String, Style) {
    let mut prefix = String::new();
    let row_style = if is_selected {
        style.selected_style
    } else {
        style.normal_style
    };

    for &parent_is_last in &flat.parent_is_last {
        let connector = if parent_is_last {
            style.connector_space
        } else {
            style.connector_vertical
        };
        prefix.push_str(connector);
    }

    if flat.depth > 0 {
        let connector = if flat.is_last {
            style.connector_last
        } else {
            style.connector_branch
        };
        prefix.push_str(connector);
    }

    if flat.node.has_children() {
        let icon = if widget.is_collapsed(&flat.node.id) {
            style.collapsed_icon
        } else {
            style.expanded_icon
        };
        prefix.push_str(icon);
    }

    (prefix, row_style)
}

pub fn render_task_tree(
    frame: &mut Frame<'_>,
    area: Rect,
    nodes: &[TreeNode<TaskTreeData>],
    widget: &WidgetTreeState,
    theme: &Theme,
    focused: bool,
    tree_marquee: &mut MarqueeState,
) -> Rect {
    let block = Block::default()
        .title(" Tasks ")
        .borders(Borders::ALL)
        .border_style(if focused {
            dhara_theme::border_style().fg(dhara_theme::ACCENT)
        } else {
            dhara_theme::border_style()
        })
        .style(dhara_theme::border_only_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let tree_area = Rect {
        x: inner.x.saturating_add(1),
        y: inner.y,
        width: inner.width.saturating_sub(1),
        height: inner.height,
    };

    if tree_area.width == 0 || tree_area.height == 0 {
        return tree_area;
    }

    let style = tree_style(theme);
    let visible = flatten_visible(nodes, widget);
    let scroll = widget.scroll as usize;
    let viewport_height = tree_area.height as usize;
    let marquee_style = MarqueeStyle::from(theme);
    let buf = frame.buffer_mut();

    for (view_idx, flat_node) in visible
        .iter()
        .enumerate()
        .skip(scroll)
        .take(viewport_height)
    {
        let is_selected = view_idx == widget.selected_index;
        let row_y = tree_area.y + (view_idx - scroll) as u16;
        let row_area = Rect::new(tree_area.x, row_y, tree_area.width, 1);

        let (prefix, row_style) = build_prefix(&style, flat_node, is_selected, widget);
        if is_selected {
            for x in row_area.x..row_area.x + row_area.width {
                buf[(x, row_y)]
                    .set_bg(dhara_theme::SELECTED_BG)
                    .set_fg(dhara_theme::WARNING);
            }
        }
        let prefix_width = prefix.width();
        buf.set_string(row_area.x, row_area.y, &prefix, row_style);

        let label_width = row_area.width.saturating_sub(prefix_width as u16);
        if label_width == 0 {
            continue;
        }

        let label_area = Rect::new(
            row_area.x + prefix_width as u16,
            row_area.y,
            label_width,
            1,
        );
        let label = &flat_node.node.data.label;

        if is_selected {
            tree_marquee.tick(label.width(), label_width as usize, &marquee_style);
            MarqueeText::new(label, tree_marquee)
                .mode(MarqueeMode::Continuous)
                .text_style(row_style)
                .render(label_area, buf);
        } else {
            MarqueeText::new(label, &mut MarqueeState::new())
                .mode(MarqueeMode::Static)
                .text_style(row_style)
                .render(label_area, buf);
        }
    }

    tree_area
}

pub fn handle_tree_mouse(
    widget: &mut WidgetTreeState,
    nodes: &[TreeNode<TaskTreeData>],
    inner: Rect,
    mouse: &MouseEvent,
) -> bool {
    if inner.width == 0
        || inner.height == 0
        || mouse.column < inner.x
        || mouse.column >= inner.x + inner.width
        || mouse.row < inner.y
        || mouse.row >= inner.y + inner.height
    {
        return false;
    }

    let count = visible_count(nodes, widget);
    if count == 0 {
        return false;
    }

    let rel_row = (mouse.row - inner.y) as usize;
    let visible_idx = widget.scroll as usize + rel_row;
    widget.selected_index = visible_idx.min(count - 1);
    widget.ensure_visible(count);
    true
}

pub fn apply_tree_selection(
    state: &mut AppState,
    registry: &CommandRegistry,
    nodes: &[TreeNode<TaskTreeData>],
    widget: &WidgetTreeState,
) {
    let Some(id) = get_selected_id(nodes, widget) else {
        return;
    };
    let Some(node) = find_node(nodes, &id) else {
        return;
    };
    if node.data.has_children {
        state.tree_view.toggle_expanded(&node.data.path_key);
    } else if let Some(command_id) = node.data.command_id {
        state.select_command(registry, command_id);
    }
}

fn find_node<'a>(
    nodes: &'a [TreeNode<TaskTreeData>],
    id: &str,
) -> Option<&'a TreeNode<TaskTreeData>> {
    for node in nodes {
        if node.id == id {
            return Some(node);
        }
        if let Some(found) = find_node(&node.children, id) {
            return Some(found);
        }
    }
    None
}

pub fn handle_tree_key(
    widget: &mut WidgetTreeState,
    nodes: &[TreeNode<TaskTreeData>],
    code: KeyCode,
) -> TreeKeyAction {
    let count = visible_count(nodes, widget);
    match code {
        KeyCode::Up => {
            widget.select_prev();
            widget.ensure_visible(count.max(1));
            TreeKeyAction::SelectionChanged
        }
        KeyCode::Down => {
            widget.select_next(count);
            widget.ensure_visible(count.max(1));
            TreeKeyAction::SelectionChanged
        }
        KeyCode::Home => {
            widget.selected_index = 0;
            widget.ensure_visible(count.max(1));
            TreeKeyAction::SelectionChanged
        }
        KeyCode::End => {
            if count > 0 {
                widget.selected_index = count - 1;
            }
            widget.ensure_visible(count.max(1));
            TreeKeyAction::SelectionChanged
        }
        KeyCode::Left => {
            if let Some(id) = get_selected_id(nodes, widget) {
                widget.collapse(&id);
            }
            TreeKeyAction::Toggled
        }
        KeyCode::Right => {
            if let Some(id) = get_selected_id(nodes, widget) {
                widget.expand(&id);
            }
            TreeKeyAction::Toggled
        }
        KeyCode::Enter | KeyCode::Char(' ') => TreeKeyAction::Activate,
        _ => TreeKeyAction::None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeKeyAction {
    None,
    SelectionChanged,
    Toggled,
    Activate,
}

pub fn task_row_from_widget(widget: &WidgetTreeState) -> usize {
    widget.selected_index
}
