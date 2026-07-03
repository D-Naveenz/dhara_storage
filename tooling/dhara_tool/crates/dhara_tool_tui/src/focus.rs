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
