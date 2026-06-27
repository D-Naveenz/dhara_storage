use std::io::{self, IsTerminal};
use std::time::Duration;

use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::command::CommandRegistry;
use crate::command::ToolContext;
use crate::ops::ensure_workspace_state;

use super::render::render;
use super::state::{AppState, Focus, MainView};

pub fn can_launch() -> bool {
    io::stdin().is_terminal() && io::stdout().is_terminal()
}

pub fn run_tui(registry: &CommandRegistry, context: &ToolContext) -> Result<()> {
    let mut stdout = io::stdout();
    enable_raw_mode().context("failed to enable raw mode")?;
    execute!(stdout, EnterAlternateScreen).context("failed to enter alternate screen")?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).context("failed to create terminal")?;

    let workspace = ensure_workspace_state(context);
    let mut state = AppState::with_workspace(
        AppState::repository_label_from_path(&context.repo_root),
        workspace,
    );
    loop {
        state.poll_active_run();
        terminal.draw(|frame| render(frame, &state, registry))?;

        if state.should_quit {
            break;
        }

        if event::poll(Duration::from_millis(100)).context("failed to poll terminal events")? {
            let Event::Key(key) = event::read().context("failed to read terminal event")? else {
                continue;
            };
            if key.kind != KeyEventKind::Press {
                continue;
            }

            match key.code {
                KeyCode::Char('q') if !state.editing_text => {
                    if state.active_run.is_some() {
                        state.status_message =
                            "A command is still running. Cancel it first or wait for completion."
                                .to_owned();
                    } else {
                        state.should_quit = true;
                    }
                }
                KeyCode::Tab => state.cycle_focus(false),
                KeyCode::BackTab => state.cycle_focus(true),
                KeyCode::Char('d') if !state.editing_text => {
                    state.main_view = MainView::Dashboard;
                    state.focus = Focus::Main;
                    state.editing_text = false;
                }
                KeyCode::Char('r') if !state.editing_text => state.run_selected(registry, context),
                KeyCode::Char('c') if !state.editing_text => state.cancel_active(),
                KeyCode::Up if !state.editing_text => match state.focus {
                    Focus::Sections => state.move_sections(registry, -1),
                    Focus::Commands => state.move_commands(registry, -1),
                    Focus::Main if matches!(state.main_view, MainView::Dashboard) => {
                        state.move_quick_actions(-1)
                    }
                    Focus::Main => {
                        if let Some(command) = state.current_command(registry).cloned() {
                            state.ensure_form(&command);
                            if let Some(form) = state.selected_form(&command) {
                                form.move_previous(command.ui.fields.len());
                            }
                        }
                    }
                    Focus::Output => {
                        state.output_scroll = state.output_scroll.saturating_sub(1);
                    }
                },
                KeyCode::Down if !state.editing_text => match state.focus {
                    Focus::Sections => state.move_sections(registry, 1),
                    Focus::Commands => state.move_commands(registry, 1),
                    Focus::Main if matches!(state.main_view, MainView::Dashboard) => {
                        state.move_quick_actions(1)
                    }
                    Focus::Main => {
                        if let Some(command) = state.current_command(registry).cloned() {
                            state.ensure_form(&command);
                            if let Some(form) = state.selected_form(&command) {
                                form.move_next(command.ui.fields.len());
                            }
                        }
                    }
                    Focus::Output => {
                        state.output_scroll = state.output_scroll.saturating_add(1);
                    }
                },
                KeyCode::Left if !state.editing_text => {
                    if let Some(command) = state.current_command(registry).cloned() {
                        state.ensure_form(&command);
                        if let Some(form) = state.selected_form(&command) {
                            form.cycle_previous_option(&command);
                        }
                    }
                }
                KeyCode::Right if !state.editing_text => {
                    if let Some(command) = state.current_command(registry).cloned() {
                        state.ensure_form(&command);
                        if let Some(form) = state.selected_form(&command) {
                            form.cycle_next_option(&command);
                        }
                    }
                }
                KeyCode::Enter => match state.focus {
                    Focus::Commands => state.activate_command(registry),
                    Focus::Main if matches!(state.main_view, MainView::Dashboard) => {
                        state.activate_quick_action(registry)
                    }
                    Focus::Main => {
                        if let Some(command) = state.current_command(registry).cloned() {
                            state.ensure_form(&command);
                            if let Some(form) = state.selected_form(&command)
                                && let Some(field) = command.ui.fields.get(form.selected_field)
                            {
                                match field.kind {
                                    crate::command::FieldKind::Boolean => form.toggle_bool(),
                                    crate::command::FieldKind::Select(_) => {
                                        form.cycle_next_option(&command)
                                    }
                                    crate::command::FieldKind::Text
                                    | crate::command::FieldKind::Path => {
                                        state.editing_text = !state.editing_text;
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                },
                KeyCode::Esc => state.editing_text = false,
                KeyCode::Backspace if state.editing_text => {
                    if let Some(command) = state.current_command(registry).cloned() {
                        state.ensure_form(&command);
                        if let Some(form) = state.selected_form(&command) {
                            form.backspace();
                        }
                    }
                }
                KeyCode::Char(ch)
                    if state.editing_text && !key.modifiers.contains(KeyModifiers::CONTROL) =>
                {
                    if let Some(command) = state.current_command(registry).cloned() {
                        state.ensure_form(&command);
                        if let Some(form) = state.selected_form(&command) {
                            form.insert_char(ch);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    disable_raw_mode().context("failed to disable raw mode")?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)
        .context("failed to leave alternate screen")?;
    terminal.show_cursor().context("failed to restore cursor")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::can_launch;

    #[test]
    fn can_launch_is_callable() {
        let _ = can_launch();
    }
}
