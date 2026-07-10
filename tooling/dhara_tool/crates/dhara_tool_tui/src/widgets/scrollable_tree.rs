use crossterm::event::{KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui_interact::components::{TreeNode, TreeStyle, TreeView, TreeViewState};
use ratatui_interact::theme::Theme;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::theme as dhara_theme;

struct FlatNode<'a, T> {
    node: &'a TreeNode<T>,
    depth: usize,
    is_last: bool,
    parent_is_last: Vec<bool>,
}

fn flatten_visible<'a, T>(
    nodes: &'a [TreeNode<T>],
    state: &TreeViewState,
) -> Vec<FlatNode<'a, T>>
where
    T: std::fmt::Debug,
{
    let mut result = Vec::new();
    flatten_nodes(nodes, state, 0, &mut result, &[]);
    result
}

fn flatten_nodes<'a, T>(
    nodes: &'a [TreeNode<T>],
    state: &TreeViewState,
    depth: usize,
    result: &mut Vec<FlatNode<'a, T>>,
    parent_is_last: &[bool],
) where
    T: std::fmt::Debug,
{
    let count = nodes.len();
    for (idx, node) in nodes.iter().enumerate() {
        let is_last = idx == count - 1;
        result.push(FlatNode {
            node,
            depth,
            is_last,
            parent_is_last: parent_is_last.to_vec(),
        });
        if node.has_children() && !state.is_collapsed(&node.id) {
            let mut parents = parent_is_last.to_vec();
            parents.push(is_last);
            flatten_nodes(&node.children, state, depth + 1, result, &parents);
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

fn build_prefix<T>(
    style: &TreeStyle,
    flat: &FlatNode<'_, T>,
    widget: &TreeViewState,
) -> (String, Style) {
    let mut prefix = String::new();
    let row_style = style.normal_style;

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

fn clip_line(text: &str, max_width: usize, h_scroll: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    let chars: Vec<char> = text.chars().collect();
    let start = h_scroll.min(chars.len());
    let mut out = String::new();
    let mut width = 0usize;
    for ch in chars.into_iter().skip(start) {
        let w = ch.width().unwrap_or(0);
        if width + w > max_width {
            break;
        }
        out.push(ch);
        width += w;
    }
    out
}

pub fn visible_count<T: std::fmt::Debug>(nodes: &[TreeNode<T>], state: &TreeViewState) -> usize {
    TreeView::new(nodes, state).visible_count()
}

pub fn render_clipped_tree<T: std::fmt::Debug>(
    area: Rect,
    nodes: &[TreeNode<T>],
    state: &TreeViewState,
    h_scroll: u16,
    label: impl Fn(&TreeNode<T>) -> &str,
    theme: &Theme,
    buf: &mut ratatui::buffer::Buffer,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let style = tree_style(theme);
    let visible = flatten_visible(nodes, state);
    let scroll = state.scroll as usize;
    let h_scroll = h_scroll as usize;
    let viewport_height = area.height as usize;

    for (view_idx, flat_node) in visible
        .iter()
        .enumerate()
        .skip(scroll)
        .take(viewport_height)
    {
        let is_selected = view_idx == state.selected_index;
        let row_y = area.y + (view_idx - scroll) as u16;
        let row_area = Rect::new(area.x, row_y, area.width, 1);
        let row_style = if is_selected {
            style.selected_style
        } else {
            style.normal_style
        };

        if is_selected {
            for x in row_area.x..row_area.x + row_area.width {
                buf[(x, row_y)]
                    .set_bg(dhara_theme::SELECTED_BG)
                    .set_fg(dhara_theme::WARNING);
            }
        }

        let (prefix, prefix_style) = build_prefix(&style, flat_node, state);
        let prefix_width = prefix.width();
        buf.set_string(row_area.x, row_area.y, &prefix, prefix_style);

        let label_width = row_area.width.saturating_sub(prefix_width as u16) as usize;
        if label_width == 0 {
            continue;
        }

        let label = label(flat_node.node);
        let clipped = clip_line(label, label_width, h_scroll);
        buf.set_string(
            row_area.x + prefix_width as u16,
            row_area.y,
            &clipped,
            row_style,
        );
    }
}

pub fn clamp_horizontal_scroll(h_scroll: &mut u16, max: u16) {
    if *h_scroll > max {
        *h_scroll = max;
    }
}

pub fn auto_scroll_selection<T: std::fmt::Debug>(
    nodes: &[TreeNode<T>],
    state: &TreeViewState,
    h_scroll: &mut u16,
    viewport_width: u16,
    label: impl Fn(&TreeNode<T>) -> &str,
) {
    let max = max_horizontal_scroll(nodes, state, viewport_width, &label);
    clamp_horizontal_scroll(h_scroll, max);
    let style = TreeStyle::default();
    let visible = flatten_visible(nodes, state);
    let Some(flat) = visible.get(state.selected_index) else {
        return;
    };
    let (prefix, _) = build_prefix(&style, flat, state);
    let label_start = prefix.width();
    let label_end = label_start + label(flat.node).width();
    let view = viewport_width as usize;
    let start = *h_scroll as usize;
    let end = start + view;
    if label_end > end {
        *h_scroll = (label_end.saturating_sub(view)) as u16;
    } else if label_start < start {
        *h_scroll = label_start as u16;
    }
    clamp_horizontal_scroll(h_scroll, max);
}

pub fn handle_tree_wheel<T: std::fmt::Debug>(
    widget: &mut TreeViewState,
    nodes: &[TreeNode<T>],
    inner: Rect,
    mouse: &MouseEvent,
    h_scroll: &mut u16,
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

    match mouse.kind {
        MouseEventKind::ScrollUp if mouse.modifiers.contains(KeyModifiers::SHIFT) => {
            *h_scroll = h_scroll.saturating_sub(1);
            true
        }
        MouseEventKind::ScrollDown if mouse.modifiers.contains(KeyModifiers::SHIFT) => {
            *h_scroll = h_scroll.saturating_add(1);
            true
        }
        MouseEventKind::ScrollUp => {
            if widget.scroll > 0 {
                widget.scroll -= 1;
            } else if widget.selected_index > 0 {
                widget.selected_index -= 1;
            }
            widget.ensure_visible(count);
            true
        }
        MouseEventKind::ScrollDown => {
            let max_scroll = count.saturating_sub(inner.height as usize) as u16;
            if widget.scroll < max_scroll {
                widget.scroll += 1;
            } else if widget.selected_index + 1 < count {
                widget.selected_index += 1;
            }
            widget.ensure_visible(count);
            true
        }
        _ => false,
    }
}

pub fn max_horizontal_scroll<T: std::fmt::Debug>(
    nodes: &[TreeNode<T>],
    state: &TreeViewState,
    viewport_width: u16,
    label: impl Fn(&TreeNode<T>) -> &str,
) -> u16 {
    let style = TreeStyle::default();
    let visible = flatten_visible(nodes, state);
    let view = viewport_width as usize;
    let mut max_line = 0usize;
    for flat in &visible {
        let (prefix, _) = build_prefix(
            &style,
            flat,
            state,
        );
        max_line = max_line.max(prefix.width() + label(flat.node).width());
    }
    max_line.saturating_sub(view) as u16
}
