#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FocusRegion {
    #[default]
    Tasks,
    Tabs,
    TabContent,
    ActionButtons,
    Modal,
}

#[derive(Debug, Clone, Default)]
pub struct FocusState {
    pub region: FocusRegion,
    pub task_row: usize,
    pub tab_index: usize,
    pub form_field: usize,
    pub action_button: usize,
}

impl FocusState {
    pub fn next_region(&mut self) {
        self.region = match self.region {
            FocusRegion::Tasks => FocusRegion::Tabs,
            FocusRegion::Tabs => FocusRegion::TabContent,
            FocusRegion::TabContent => FocusRegion::ActionButtons,
            FocusRegion::ActionButtons => FocusRegion::Tasks,
            FocusRegion::Modal => FocusRegion::Modal,
        };
    }

    pub fn prev_region(&mut self) {
        self.region = match self.region {
            FocusRegion::Tasks => FocusRegion::ActionButtons,
            FocusRegion::Tabs => FocusRegion::Tasks,
            FocusRegion::TabContent => FocusRegion::Tabs,
            FocusRegion::ActionButtons => FocusRegion::TabContent,
            FocusRegion::Modal => FocusRegion::Modal,
        };
    }
}
