use ratatui::layout::Rect;

use ratatui_interact::state::FocusManager;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TuiFocus {
    TaskTree,
    MainTabs,
    TabContent,
    ActionRun,
    ActionCancel,
    ActionReset,
}

pub struct ShellFocus {
    pub manager: FocusManager<TuiFocus>,
}

impl Default for ShellFocus {
    fn default() -> Self {
        let mut manager = FocusManager::new();
        manager.register(TuiFocus::TaskTree);
        manager.register(TuiFocus::MainTabs);
        manager.register(TuiFocus::TabContent);
        manager.register(TuiFocus::ActionRun);
        manager.register(TuiFocus::ActionCancel);
        manager.register(TuiFocus::ActionReset);
        Self { manager }
    }
}

impl ShellFocus {
    pub fn current(&self) -> Option<&TuiFocus> {
        self.manager.current()
    }

    pub fn is_focused(&self, target: &TuiFocus) -> bool {
        self.manager.current() == Some(target)
    }

    pub fn focus(&mut self, target: TuiFocus) {
        self.manager.set(target);
    }

    pub fn next(&mut self) {
        self.manager.next();
    }

    pub fn prev(&mut self) {
        self.manager.prev();
    }
}

pub fn point_in_rect(rect: Rect, col: u16, row: u16) -> bool {
    rect.width > 0
        && rect.height > 0
        && col >= rect.x
        && col < rect.x + rect.width
        && row >= rect.y
        && row < rect.y + rect.height
}

/// Focus the shell panel under the pointer (hover-to-focus glue).
pub fn focus_panel_at_pointer(
    shell_focus: &mut ShellFocus,
    task_tree: Rect,
    center: Rect,
    action: Rect,
    col: u16,
    row: u16,
) {
    if point_in_rect(action, col, row) {
        shell_focus.focus(TuiFocus::ActionRun);
    } else if point_in_rect(center, col, row) {
        shell_focus.focus(TuiFocus::TabContent);
    } else if point_in_rect(task_tree, col, row) {
        shell_focus.focus(TuiFocus::TaskTree);
    }
}
