use dhara_tool_cli::interactive::AppState;
use ratatui::Frame;
use ratatui_interact::components::{
    DialogConfig, DialogState, Input, InputState, PopupDialog,
};
use ratatui_interact::events::{get_char, is_backspace, is_delete};
use ratatui_interact::theme::Theme;
use ratatui_interact::traits::{ContainerAction, EventResult};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap, Widget};
use crossterm::event::{KeyCode, KeyEvent, MouseEvent};

use crate::theme as dhara_theme;

pub struct RepoSetupPrompt;

impl RepoSetupPrompt {
    pub fn new(_initial: Option<String>) -> Self {
        Self
    }
}

pub struct ModalLayer {
    pub activation_config: DialogConfig,
    pub activation_state: DialogState<()>,
    pub repo_config: DialogConfig,
    pub repo_state: DialogState<RepoDialogContent>,
}

pub struct RepoDialogContent {
    pub path_input: InputState,
}

impl Default for ModalLayer {
    fn default() -> Self {
        let mut activation_state = DialogState::new(());
        activation_state.register_button(0);
        activation_state.register_button(1);

        let mut repo_state = DialogState::new(RepoDialogContent {
            path_input: InputState::new(String::new()),
        });
        repo_state.register_child(0);
        repo_state.register_button(0);
        repo_state.register_button(1);

        Self {
            activation_config: DialogConfig::new("Activation")
                .yes_no()
                .width_percent(60)
                .height_percent(40)
                .theme(&dhara_theme::interact_theme()),
            activation_state,
            repo_config: DialogConfig::new("Repository setup")
                .ok_cancel()
                .width_percent(70)
                .height_percent(30)
                .theme(&dhara_theme::interact_theme()),
            repo_state,
        }
    }
}

impl ModalLayer {
    pub fn show_activation(&mut self) {
        self.activation_state.show();
    }

    pub fn hide_activation(&mut self) {
        self.activation_state.hide();
    }

    pub fn show_repo(&mut self, initial: Option<String>) {
        self.repo_state.children.path_input = InputState::new(initial.unwrap_or_default());
        self.repo_state.show();
    }

    pub fn hide_repo(&mut self) {
        self.repo_state.hide();
    }

    pub fn render_activation(&mut self, frame: &mut Frame<'_>, state: &AppState, theme: &Theme) {
        if state.activation_prompt.is_none() || !self.activation_state.visible {
            return;
        }
        let prompt = state.activation_prompt.as_ref().expect("activation prompt");
        let mut config = self.activation_config.clone();
        config = config.theme(theme);
        let mut dialog = PopupDialog::new(&config, &mut self.activation_state, |frame, area, _| {
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

    pub fn render_repo(&mut self, frame: &mut Frame<'_>, theme: &Theme) {
        if !self.repo_state.visible {
            return;
        }
        let mut config = self.repo_config.clone();
        config = config.theme(theme);
        let mut dialog = PopupDialog::new(&config, &mut self.repo_state, |frame, area, content| {
            let lines = vec![
                Line::from("Repository path (folder or dhara.config.toml):"),
                Line::from(""),
            ];
            Paragraph::new(lines).render(area, frame.buffer_mut());
            let input_area = Rect::new(area.x, area.y + 2, area.width, 1);
            let mut input = content.path_input.clone();
            input.focused = true;
            Input::new(&input)
                .theme(theme)
                .render_stateful(frame, input_area);
        });
        dialog.render(frame);
    }

    pub fn handle_activation_key(&mut self, key: KeyEvent, _screen: Rect) -> Option<ContainerAction> {
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

    pub fn handle_repo_key(&mut self, key: KeyEvent, _screen: Rect) -> Option<ContainerAction> {
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

    pub fn handle_activation_mouse(
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

    pub fn handle_repo_mouse(&mut self, mouse: MouseEvent, screen: Rect) -> Option<ContainerAction> {
        if !self.repo_state.visible {
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
