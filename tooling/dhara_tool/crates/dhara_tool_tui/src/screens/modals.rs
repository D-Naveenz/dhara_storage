use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use dhara_tool_cli::interactive::AppState;
use dhara_tool_kernel::paths::normalize_repository_input;
use ratatui::Frame;
use ratatui::layout::{Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph, Wrap, Widget};
use ratatui_interact::components::{DialogConfig, DialogFocusTarget, DialogState, PopupDialog};
use ratatui_interact::events::{get_char, is_backspace, is_delete};
use ratatui_interact::theme::Theme;
use ratatui_interact::traits::{ClickRegionRegistry, ContainerAction, EventResult};
use ratatui_interact::components::InputState;

use crate::theme::{self, ValidationTone};
use crate::widgets::{
    dhara_input, modal_footer, modal_shell, status_line,
    modal_footer::{ModalFooterButton, footer_layout_constraints},
};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepoPathStatus {
    Empty,
    Valid(PathBuf),
    Invalid(String),
}

impl RepoPathStatus {
    fn recompute(text: &str) -> Self {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Self::Empty;
        }
        match normalize_repository_input(PathBuf::from(trimmed)) {
            Ok(root) => Self::Valid(root),
            Err(error) => Self::Invalid(error.to_string()),
        }
    }

    fn status_text(&self) -> String {
        match self {
            Self::Empty => String::new(),
            Self::Valid(root) => format!("Valid: {}", root.display()),
            Self::Invalid(message) => message.lines().next().unwrap_or("Invalid path").to_owned(),
        }
    }

    fn tone(&self) -> ValidationTone {
        match self {
            Self::Empty => ValidationTone::Muted,
            Self::Valid(_) => ValidationTone::Success,
            Self::Invalid(_) => ValidationTone::Error,
        }
    }
}

pub struct RepoDialogContent {
    pub path_input: InputState,
    pub path_status: RepoPathStatus,
    pub content_clicks: ClickRegionRegistry<usize>,
    pub footer_clicks: ClickRegionRegistry<ContainerAction>,
    pub footer_focus: usize,
    pub cancel_btn: ratatui_interact::components::ButtonState,
    pub ok_btn: ratatui_interact::components::ButtonState,
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
        let theme = theme::interact_theme();

        let mut activation_state = DialogState::new(());
        activation_state.register_button(0);
        activation_state.register_button(1);

        let mut repo_state = DialogState::new(RepoDialogContent {
            path_input: InputState::new(String::new()),
            path_status: RepoPathStatus::Empty,
            content_clicks: ClickRegionRegistry::new(),
            footer_clicks: ClickRegionRegistry::new(),
            footer_focus: 1,
            cancel_btn: ratatui_interact::components::ButtonState::enabled(),
            ok_btn: ratatui_interact::components::ButtonState::enabled(),
        });
        repo_state.register_child(0);

        Self {
            active: None,
            activation_config: DialogConfig::new("Activation")
                .yes_no()
                .width_percent(60)
                .height_percent(40)
                .close_on_outside_click(false)
                .theme(&theme),
            activation_state,
            repo_config: DialogConfig::new("Initial Configuration")
                .no_buttons()
                .width_percent(70)
                .height_percent(45)
                .min_size(52, 14)
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
        let text = initial.unwrap_or_default();
        self.repo_state.children.path_input = InputState::new(text.clone());
        self.repo_state.children.path_status = RepoPathStatus::recompute(&text);
        self.repo_state.children.content_clicks.clear();
        self.repo_state.children.footer_clicks.clear();
        self.repo_state.children.footer_focus = 1;
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
                ("Tab", "footer buttons"),
                ("Enter", "confirm"),
                ("Esc", "quit"),
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
            ContainerAction::Submit => match &self.repo_state.children.path_status {
                RepoPathStatus::Valid(root) => {
                    Some(ModalOutcome::RepositoryResolved(root.clone()))
                }
                RepoPathStatus::Empty => Some(ModalOutcome::RepositoryPathRequired),
                RepoPathStatus::Invalid(_) => None,
            },
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
                modal_shell::fill_modal_background(frame, area);
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
            modal_shell::fill_modal_background(frame, area);
            content.content_clicks.clear();

            let chunks = Layout::vertical(footer_layout_constraints()).split(area);

            Paragraph::new(theme::initial_config_description_lines())
                .wrap(Wrap { trim: true })
                .render(chunks[0], frame.buffer_mut());

            let mut input = content.path_input.clone();
            input.focused = true;
            let region = dhara_input::render_path_input(
                frame,
                chunks[1],
                &input,
                "path/to/workspace",
                theme,
            );
            content.content_clicks.register(region.area, 0);

            status_line::render_status_line(
                chunks[2],
                &content.path_status.status_text(),
                content.path_status.tone(),
                frame.buffer_mut(),
            );

            let mut buttons = [
                ModalFooterButton {
                    label: "Cancel",
                    action: ContainerAction::Close,
                    state: content.cancel_btn.clone(),
                },
                ModalFooterButton {
                    label: "OK",
                    action: ContainerAction::Submit,
                    state: content.ok_btn.clone(),
                },
            ];
            content.ok_btn.set_enabled(matches!(
                content.path_status,
                RepoPathStatus::Valid(_)
            ));
            buttons[0].state = content.cancel_btn.clone();
            buttons[1].state = content.ok_btn.clone();

            modal_footer::render_modal_footer(
                chunks[3],
                &mut buttons,
                content.footer_focus,
                theme,
                &mut content.footer_clicks,
                frame.buffer_mut(),
            );
            content.cancel_btn = buttons[0].state.clone();
            content.ok_btn = buttons[1].state.clone();
        });
        dialog.render(frame);
    }

    fn refresh_repo_path_status(&mut self) {
        let text = self.repo_state.children.path_input.text();
        self.repo_state.children.path_status = RepoPathStatus::recompute(text);
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
            self.refresh_repo_path_status();
            return None;
        }

        match key.code {
            KeyCode::Backspace if is_backspace(&key) => {
                self.repo_state.children.path_input.delete_char_backward();
                self.refresh_repo_path_status();
                return None;
            }
            KeyCode::Delete if is_delete(&key) => {
                self.repo_state.children.path_input.delete_char_forward();
                self.refresh_repo_path_status();
                return None;
            }
            KeyCode::Esc => {
                self.repo_state.hide();
                return Some(ContainerAction::Close);
            }
            KeyCode::Tab => {
                self.repo_state.children.footer_focus =
                    (self.repo_state.children.footer_focus + 1) % 2;
                return None;
            }
            KeyCode::BackTab => {
                self.repo_state.children.footer_focus =
                    (self.repo_state.children.footer_focus + 1) % 2;
                return None;
            }
            KeyCode::Enter => {
                if self.repo_state.children.footer_focus == 1 {
                    return Some(ContainerAction::Submit);
                }
                return Some(ContainerAction::Close);
            }
            _ => {}
        }
        None
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

        if let Some(action) = self
            .repo_state
            .children
            .footer_clicks
            .handle_click(mouse.column, mouse.row)
            .cloned()
        {
            if action == ContainerAction::Close {
                self.repo_state.children.footer_focus = 0;
                self.repo_state.hide();
            } else {
                self.repo_state.children.footer_focus = 1;
            }
            return Some(action);
        }

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
            return None;
        }

        let mut dialog =
            PopupDialog::new(&self.repo_config, &mut self.repo_state, |_, _, _| {});
        match dialog.handle_mouse_with_screen(mouse, screen) {
            EventResult::Action(action) => Some(action),
            _ => None,
        }
    }
}

fn render_scrim(frame: &mut Frame<'_>, area: Rect) {
    let block = Block::default().style(Style::default().bg(Color::Black));
    frame.render_widget(Clear, area);
    frame.render_widget(block, area);
}
