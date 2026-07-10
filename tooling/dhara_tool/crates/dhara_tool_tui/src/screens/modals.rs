use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use dhara_tool_cli::interactive::AppState;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph, Wrap, Widget};
use ratatui_interact::components::{
    DialogConfig, DialogFocusTarget, DialogState, Input, InputState, PopupDialog,
};
use ratatui_interact::events::{get_char, is_backspace, is_delete};
use ratatui_interact::theme::Theme;
use ratatui_interact::traits::{ClickRegionRegistry, ContainerAction, EventResult};

use crate::theme as dhara_theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveModal {
    RepositorySetup,
    Activation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModalOutcome {
    RepositoryResolved(PathBuf),
    RepositoryPathRequired,
    RepositoryCancelled,
    ActivationConfirmed,
    ActivationDeclined,
}

pub struct RepoDialogContent {
    pub path_input: InputState,
    pub content_clicks: ClickRegionRegistry<usize>,
}

/// Modal dialogs with input capture over the shell (thin wrapper over `PopupDialog`).
pub struct ModalHost {
    pub active: Option<ActiveModal>,
    pub activation_config: DialogConfig,
    pub activation_state: DialogState<()>,
    pub repo_config: DialogConfig,
    pub repo_state: DialogState<RepoDialogContent>,
}

impl Default for ModalHost {
    fn default() -> Self {
        let theme = dhara_theme::interact_theme();

        let mut activation_state = DialogState::new(());
        activation_state.register_button(0);
        activation_state.register_button(1);

        let mut repo_state = DialogState::new(RepoDialogContent {
            path_input: InputState::new(String::new()),
            content_clicks: ClickRegionRegistry::new(),
        });
        repo_state.register_child(0);
        repo_state.register_button(0);
        repo_state.register_button(1);

        Self {
            active: None,
            activation_config: DialogConfig::new("Activation")
                .yes_no()
                .width_percent(60)
                .height_percent(40)
                .close_on_outside_click(false)
                .theme(&theme),
            activation_state,
            repo_config: DialogConfig::new("Repository setup")
                .ok_cancel()
                .width_percent(70)
                .height_percent(40)
                .min_size(50, 12)
                .close_on_outside_click(false)
                .theme(&theme),
            repo_state,
        }
    }
}

impl ModalHost {
    pub fn is_blocking(&self) -> bool {
        self.active.is_some()
    }

    pub fn show_activation(&mut self) {
        self.activation_state.show();
        self.active = Some(ActiveModal::Activation);
    }

    pub fn hide_activation(&mut self) {
        self.activation_state.hide();
        if self.active == Some(ActiveModal::Activation) {
            self.active = None;
        }
    }

    pub fn show_repository(&mut self, initial: Option<String>) {
        self.repo_state.children.path_input = InputState::new(initial.unwrap_or_default());
        self.repo_state.children.content_clicks.clear();
        self.repo_state.show();
        self.active = Some(ActiveModal::RepositorySetup);
    }

    pub fn hide_repository(&mut self) {
        self.repo_state.hide();
        if self.active == Some(ActiveModal::RepositorySetup) {
            self.active = None;
        }
    }

    pub fn footer_hints(&self) -> Option<Vec<(&'static str, &'static str)>> {
        match self.active? {
            ActiveModal::RepositorySetup => Some(vec![
                ("Type", "workspace path"),
                ("Enter/OK", "confirm"),
                ("Esc/Cancel", "quit"),
            ]),
            ActiveModal::Activation => Some(vec![
                ("Enter/Y", "apply"),
                ("Esc/N", "decline"),
                ("Click", "button"),
            ]),
        }
    }

    pub fn render(&mut self, frame: &mut Frame<'_>, state: &AppState, theme: &Theme) {
        if !self.is_blocking() {
            return;
        }
        render_scrim(frame, frame.area());
        match self.active {
            Some(ActiveModal::Activation) => self.render_activation(frame, state, theme),
            Some(ActiveModal::RepositorySetup) => self.render_repository(frame, theme),
            None => {}
        }
    }

    pub fn handle_key(
        &mut self,
        key: KeyEvent,
        screen: Rect,
        _state: &AppState,
    ) -> Option<ModalOutcome> {
        match self.active? {
            ActiveModal::RepositorySetup => self
                .handle_repository_key(key, screen)
                .and_then(|action| self.map_repo_action(action)),
            ActiveModal::Activation => self
                .handle_activation_key(key, screen)
                .and_then(|action| self.map_activation_action(action)),
        }
    }

    pub fn handle_mouse(&mut self, mouse: MouseEvent, screen: Rect) -> Option<ModalOutcome> {
        match self.active? {
            ActiveModal::RepositorySetup => self
                .handle_repository_mouse(mouse, screen)
                .and_then(|action| self.map_repo_action(action)),
            ActiveModal::Activation => self
                .handle_activation_mouse(mouse, screen)
                .and_then(|action| self.map_activation_action(action)),
        }
    }

    fn map_repo_action(&mut self, action: ContainerAction) -> Option<ModalOutcome> {
        match action {
            ContainerAction::Submit => {
                let path = self.repo_state.children.path_input.text().trim().to_owned();
                if path.is_empty() {
                    return Some(ModalOutcome::RepositoryPathRequired);
                }
                Some(ModalOutcome::RepositoryResolved(PathBuf::from(path)))
            }
            ContainerAction::Close => {
                self.hide_repository();
                Some(ModalOutcome::RepositoryCancelled)
            }
            _ => None,
        }
    }

    fn map_activation_action(&mut self, action: ContainerAction) -> Option<ModalOutcome> {
        match action {
            ContainerAction::Submit => {
                self.hide_activation();
                Some(ModalOutcome::ActivationConfirmed)
            }
            ContainerAction::Close => {
                self.hide_activation();
                Some(ModalOutcome::ActivationDeclined)
            }
            _ => None,
        }
    }

    fn render_activation(&mut self, frame: &mut Frame<'_>, state: &AppState, theme: &Theme) {
        if state.activation_prompt.is_none() || !self.activation_state.visible {
            return;
        }
        let prompt = state.activation_prompt.as_ref().expect("activation prompt");
        let mut config = self.activation_config.clone();
        config = config.theme(theme);
        let mut dialog =
            PopupDialog::new(&config, &mut self.activation_state, |frame, area, _| {
                let mut lines = vec![
                    Line::from(Span::styled(
                        "Configuration drift detected",
                        Style::default().add_modifier(Modifier::BOLD),
                    )),
                    Line::from(""),
                ];
                for drift in &prompt.drifts {
                    lines.push(Line::from(format!("  • {}", drift.summary)));
                }
                lines.push(Line::from(""));
                lines.push(Line::from("Apply drift from dhara.config.toml?"));
                Paragraph::new(lines)
                    .wrap(Wrap { trim: true })
                    .render(area, frame.buffer_mut());
            });
        dialog.render(frame);
    }

    fn render_repository(&mut self, frame: &mut Frame<'_>, theme: &Theme) {
        if !self.repo_state.visible {
            return;
        }
        let mut config = self.repo_config.clone();
        config = config.theme(theme);
        let mut dialog = PopupDialog::new(&config, &mut self.repo_state, |frame, area, content| {
            content.content_clicks.clear();
            let chunks = Layout::vertical([
                Constraint::Length(4),
                Constraint::Length(3),
            ])
            .split(area);

            let lines = vec![
                Line::from("Enter the path to your Dhara Storage workspace."),
                Line::from("The folder must contain dhara.config.toml."),
                Line::from("You can type a directory path or the full path to dhara.config.toml."),
            ];
            Paragraph::new(lines)
                .wrap(Wrap { trim: true })
                .render(chunks[0], frame.buffer_mut());

            let mut input = content.path_input.clone();
            input.focused = true;
            let region = Input::new(&input)
                .label("Repository path")
                .placeholder("path/to/workspace")
                .theme(theme)
                .render_stateful(frame, chunks[1]);
            content.content_clicks.register(region.area, 0);
        });
        dialog.render(frame);
    }

    fn handle_activation_key(&mut self, key: KeyEvent, _screen: Rect) -> Option<ContainerAction> {
        if !self.activation_state.visible {
            return None;
        }
        let mut dialog = PopupDialog::new(
            &self.activation_config,
            &mut self.activation_state,
            |_, _, _| {},
        );
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.activation_state.hide();
                return Some(ContainerAction::Submit);
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                self.activation_state.hide();
                return Some(ContainerAction::Close);
            }
            _ => {}
        }
        match dialog.handle_key(key) {
            EventResult::Action(action) => Some(action),
            _ => None,
        }
    }

    fn handle_repository_key(&mut self, key: KeyEvent, _screen: Rect) -> Option<ContainerAction> {
        if !self.repo_state.visible {
            return None;
        }
        if let Some(ch) = get_char(&key) {
            self.repo_state.children.path_input.insert_char(ch);
            return None;
        }
        match key.code {
            KeyCode::Backspace if is_backspace(&key) => {
                self.repo_state.children.path_input.delete_char_backward();
                return None;
            }
            KeyCode::Delete if is_delete(&key) => {
                self.repo_state.children.path_input.delete_char_forward();
                return None;
            }
            KeyCode::Esc => {
                self.repo_state.hide();
                return Some(ContainerAction::Close);
            }
            _ => {}
        }
        let mut dialog = PopupDialog::new(&self.repo_config, &mut self.repo_state, |_, _, _| {});
        match dialog.handle_key(key) {
            EventResult::Action(action) => Some(action),
            _ => None,
        }
    }

    fn handle_activation_mouse(
        &mut self,
        mouse: MouseEvent,
        screen: Rect,
    ) -> Option<ContainerAction> {
        if !self.activation_state.visible {
            return None;
        }
        let mut dialog = PopupDialog::new(
            &self.activation_config,
            &mut self.activation_state,
            |_, _, _| {},
        );
        match dialog.handle_mouse_with_screen(mouse, screen) {
            EventResult::Action(action) => Some(action),
            _ => None,
        }
    }

    fn handle_repository_mouse(
        &mut self,
        mouse: MouseEvent,
        screen: Rect,
    ) -> Option<ContainerAction> {
        if !self.repo_state.visible {
            return None;
        }
        let mut dialog =
            PopupDialog::new(&self.repo_config, &mut self.repo_state, |_, _, _| {});
        match dialog.handle_mouse_with_screen(mouse, screen) {
            EventResult::Action(action) => Some(action),
            EventResult::NotHandled => {
                if self
                    .repo_state
                    .children
                    .content_clicks
                    .handle_click(mouse.column, mouse.row)
                    .is_some()
                {
                    self.repo_state
                        .focus
                        .set(DialogFocusTarget::Child(0));
                }
                None
            }
            _ => None,
        }
    }
}

fn render_scrim(frame: &mut Frame<'_>, area: Rect) {
    let block = Block::default().style(Style::default().bg(Color::Black));
    frame.render_widget(Clear, area);
    frame.render_widget(block, area);
}
