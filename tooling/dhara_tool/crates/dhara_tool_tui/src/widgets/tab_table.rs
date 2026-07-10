use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Block, Widget};
use ratatui::Frame;
use ratatui_interact::components::{Tab, TabView, TabViewState};
use ratatui_interact::theme::Theme;
use ratatui_interact::traits::ClickRegionRegistry;

use crate::theme as dhara_theme;

pub struct TabTableLayout {
    pub header: Rect,
    pub body: Rect,
}

pub fn split_tab_table(area: Rect) -> TabTableLayout {
    let chunks = Layout::vertical([Constraint::Length(1), Constraint::Length(1), Constraint::Min(0)])
        .split(area);
    TabTableLayout {
        header: chunks[0],
        body: chunks[2],
    }
}

pub fn render_tab_header(
    area: Rect,
    tabs: &[Tab<'_>],
    tab_state: &mut TabViewState,
    theme: &Theme,
    click_registry: &mut ClickRegionRegistry<ratatui_interact::components::TabViewAction>,
    buf: &mut ratatui::buffer::Buffer,
) {
    Block::default()
        .style(dhara_theme::bar_style())
        .render(area, buf);

    let tab_style = dhara_theme::tab_table_style(theme);
    let tab_view = TabView::new(tabs, tab_state)
        .style(tab_style)
        .content(|_, _, _| {});
    tab_view.render_with_registry(area, buf, click_registry);
}

pub fn render_tab_separator(frame: &mut Frame<'_>, area: Rect) {
    Block::default()
        .borders(ratatui::widgets::Borders::BOTTOM)
        .border_style(Style::default().fg(dhara_theme::BORDER))
        .render(area, frame.buffer_mut());
}
