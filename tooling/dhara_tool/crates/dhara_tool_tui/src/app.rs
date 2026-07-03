use std::io::{self, IsTerminal, Stdout};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers, MouseEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use dhara_tool_cli::command::{CommandRegistry, RunMode, ToolContext};
use dhara_tool_cli::interactive::{
    ActivationPrompt, AppState, MainTab, VisibleTreeRow,
};
use dhara_tool_kernel::{
    ProgressSnapshot, activation::run_activation, ensure_workspace_state,
    logging::init_progress_settings, load_runtime_cache, register_interactive_progress_sender,
    repo_config::{ConfigDriftItem, show},
    resolve_and_persist_repository, unregister_interactive_progress_sender,
};

use crate::boot::TuiBootParams;
use crate::focus::{FocusRegion, FocusState};
use crate::screens::{
    modals::{RepoSetupPrompt, render_activation_modal, render_repo_setup_modal},
    render_tab_content, render_tabs, render_tasks_panel, select_task_row, tab_from_index,
};
use crate::widgets::{render_command_bar, render_title_bar};

pub fn can_launch_tui() -> bool {
    io::stdin().is_terminal() && io::stdout().is_terminal()
}

pub struct DharaTui {
    pub state: AppState,
    pub registry: CommandRegistry,
    pub exe_root: PathBuf,
    pub boot: TuiBootParams,
    pub context: Option<ToolContext>,
    pub repo_setup: Option<RepoSetupPrompt>,
    pub progress_rx: Arc<Mutex<Receiver<ProgressSnapshot>>>,
    pub focus: FocusState,
    pub task_row: usize,
    pub form_field: usize,
    pub trouble_scroll: usize,
    pub editing_form: bool,
}

pub fn run_tui(
    registry: &CommandRegistry,
    exe_root: PathBuf,
    boot: TuiBootParams,
    initial_context: Option<ToolContext>,
    pending_activation: Vec<ConfigDriftItem>,
    stale_repository_hint: Option<PathBuf>,
) -> Result<()> {
    let (progress_tx, progress_rx) = mpsc::channel();
    register_interactive_progress_sender(progress_tx);
    let progress_rx = Arc::new(Mutex::new(progress_rx));

    let mut terminal = setup_terminal()?;

    let mut app = if let Some(context) = initial_context {
        init_progress_settings(&context);
        let workspace = ensure_workspace_state(&context);
        let mut state = AppState::with_workspace(
            AppState::repository_label_from_path(&context.repo_root),
            workspace,
            registry,
        );
        if !pending_activation.is_empty() {
            state.activation_prompt = Some(ActivationPrompt::new(pending_activation));
            state.status_message =
                "Configuration drift detected. Confirm activation to continue.".to_owned();
        }
        DharaTui {
            state,
            registry: registry.clone(),
            exe_root,
            boot,
            context: Some(context),
            repo_setup: None,
            progress_rx,
            focus: FocusState::default(),
            task_row: 0,
            form_field: 0,
            trouble_scroll: 0,
            editing_form: false,
        }
    } else {
        let initial = stale_repository_hint
            .as_ref()
            .map(|path| path.display().to_string());
        DharaTui {
            state: AppState::with_repository_label("repository required"),
            registry: registry.clone(),
            exe_root,
            boot,
            context: None,
            repo_setup: Some(RepoSetupPrompt::new(initial)),
            progress_rx,
            focus: FocusState {
                region: FocusRegion::Modal,
                ..Default::default()
            },
            task_row: 0,
            form_field: 0,
            trouble_scroll: 0,
            editing_form: false,
        }
    };

    let result = run_loop(&mut terminal, &mut app);
    unregister_interactive_progress_sender();
    restore_terminal(&mut terminal)?;
    result
}

fn setup_terminal() -> Result<ratatui::Terminal<ratatui_crossterm::CrosstermBackend<Stdout>>> {
    enable_raw_mode().context("failed to enable raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, event::EnableMouseCapture)
        .context("failed to enter alternate screen")?;
    let backend = ratatui_crossterm::CrosstermBackend::new(stdout);
    ratatui::Terminal::new(backend).context("failed to create terminal")
}

fn restore_terminal(terminal: &mut ratatui::Terminal<ratatui_crossterm::CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode().context("failed to disable raw mode")?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        event::DisableMouseCapture
    )
    .context("failed to leave alternate screen")?;
    terminal.show_cursor().context("failed to show cursor")?;
    Ok(())
}

fn run_loop(
    terminal: &mut ratatui::Terminal<ratatui_crossterm::CrosstermBackend<Stdout>>,
    app: &mut DharaTui,
) -> Result<()> {
    loop {
        app.state.poll_active_run();
        while let Ok(snapshot) = app.progress_rx.lock().expect("progress rx").try_recv() {
            app.state.apply_progress_snapshot(snapshot);
        }

        let rows = app.state.tree_view.visible_rows(&app.state.nav_tree);
        if app.task_row >= rows.len() && !rows.is_empty() {
            app.task_row = rows.len() - 1;
        }

        terminal.draw(|frame| draw(frame, app, &rows))?;

        if app.state.should_quit {
            break;
        }

        if event::poll(Duration::from_millis(100)).context("failed to poll events")? {
            match event::read().context("failed to read event")? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    handle_key(app, &rows, key.code, key.modifiers)?;
                }
                Event::Mouse(mouse) => {
                    if let MouseEventKind::Down(_) = mouse.kind {
                        // Mouse selection deferred; keyboard-first for v1.
                    }
                }
                _ => {}
            }
        }
    }
    Ok(())
}

fn draw(frame: &mut ratatui::Frame<'_>, app: &DharaTui, rows: &[VisibleTreeRow]) {
    let area = frame.area();
    let layout = ratatui::layout::Layout::vertical([
        ratatui::layout::Constraint::Length(2),
        ratatui::layout::Constraint::Min(1),
        ratatui::layout::Constraint::Length(1),
    ])
    .split(area);

    let repo = app
        .context
        .as_ref()
        .map(|ctx| AppState::repository_label_from_path(&ctx.repo_root))
        .unwrap_or_else(|| "select repository".to_owned());
    render_title_bar(frame, layout[0], env!("CARGO_PKG_VERSION"), &repo);

    let body = ratatui::layout::Layout::horizontal([
        ratatui::layout::Constraint::Ratio(2, 10),
        ratatui::layout::Constraint::Ratio(5, 10),
        ratatui::layout::Constraint::Ratio(3, 10),
    ])
    .split(layout[1]);

    render_tasks_panel(
        frame,
        body[0],
        &app.state,
        rows,
        app.task_row,
        app.focus.region == FocusRegion::Tasks,
    );

    let center = ratatui::layout::Layout::vertical([
        ratatui::layout::Constraint::Length(3),
        ratatui::layout::Constraint::Min(1),
    ])
    .split(body[1]);

    render_tabs(
        frame,
        center[0],
        &app.state,
        app.focus.region == FocusRegion::Tabs,
    );
    render_tab_content(
        frame,
        center[1],
        &app.state,
        &app.registry,
        app.form_field,
        app.trouble_scroll,
    );

    crate::widgets::action_panel::render_action_panel(
        frame,
        body[2],
        &app.state,
        app.focus.region == FocusRegion::ActionButtons,
        app.focus.action_button,
    );

    render_command_bar(
        frame,
        layout[2],
        app.state.active_run.is_some(),
    );

    if app.state.activation_prompt.is_some() {
        render_activation_modal(frame, &app.state);
    }
    if let Some(prompt) = &app.repo_setup {
        render_repo_setup_modal(frame, prompt);
    }
}

fn handle_key(
    app: &mut DharaTui,
    rows: &[VisibleTreeRow],
    code: KeyCode,
    modifiers: KeyModifiers,
) -> Result<()> {
    if app.repo_setup.is_some() {
        return handle_repo_setup_key(app, code);
    }

    if app.state.activation_prompt.is_some() {
        return handle_activation_key(app, code);
    }

    if app.editing_form {
        return handle_form_edit_key(app, code);
    }

    match code {
        KeyCode::Char('q') => {
            if app.state.active_run.is_none() {
                app.state.should_quit = true;
            } else {
                app.state.status_message =
                    "A command is still running. Cancel it first or wait.".to_owned();
            }
        }
        KeyCode::Char('r') if app.state.active_run.is_none() => {
            if let Some(context) = app.context.clone() {
                app.state.run_selected(&app.registry, &context);
            }
        }
        KeyCode::Char('c') if app.state.active_run.is_some() => {
            app.state.cancel_active();
        }
        KeyCode::Tab => {
            if modifiers.contains(KeyModifiers::SHIFT) {
                app.focus.prev_region();
            } else {
                app.focus.next_region();
            }
        }
        KeyCode::Up => navigate_up(app, rows),
        KeyCode::Down => navigate_down(app, rows),
        KeyCode::Left => navigate_horizontal(app, -1),
        KeyCode::Right => navigate_horizontal(app, 1),
        KeyCode::Enter => activate(app, rows),
        _ => {}
    }
    Ok(())
}

fn handle_repo_setup_key(app: &mut DharaTui, code: KeyCode) -> Result<()> {
    let Some(prompt) = app.repo_setup.as_mut() else {
        return Ok(());
    };
    match code {
        KeyCode::Esc => app.state.should_quit = true,
        KeyCode::Enter => {
            if prompt.path_input.trim().is_empty() {
                app.state.status_message = "Repository path is required.".to_owned();
                return Ok(());
            }
            match resolve_and_persist_repository(
                &app.exe_root,
                PathBuf::from(prompt.path_input.trim()),
                true,
            ) {
                Ok(repo_root) => finish_repository_setup(app, repo_root)?,
                Err(error) => app.state.status_message = error.to_string(),
            }
        }
        KeyCode::Backspace => {
            prompt.path_input.pop();
        }
        KeyCode::Char(ch) => {
            prompt.path_input.push(ch);
        }
        _ => {}
    }
    Ok(())
}

fn handle_activation_key(app: &mut DharaTui, code: KeyCode) -> Result<()> {
    match code {
        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
            if let Some(context) = app.context.as_ref() {
                if let Err(error) = app.state.apply_activation_confirm(&context.repo_root) {
                    app.state.status_message = error.to_string();
                }
            }
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.state.decline_activation();
        }
        _ => {}
    }
    Ok(())
}

fn handle_form_edit_key(app: &mut DharaTui, code: KeyCode) -> Result<()> {
    let Some(command) = app.state.selected_command(&app.registry).cloned() else {
        app.editing_form = false;
        return Ok(());
    };
    let Some(form) = app.state.forms.get_mut(command.id) else {
        app.editing_form = false;
        return Ok(());
    };

    match code {
        KeyCode::Esc | KeyCode::Enter => {
            app.editing_form = false;
            form.selected_field = app.form_field;
        }
        KeyCode::Backspace => form.backspace(),
        KeyCode::Char(' ') => form.toggle_bool(),
        KeyCode::Char(ch) => form.insert_char(ch),
        _ => {}
    }
    Ok(())
}

fn finish_repository_setup(app: &mut DharaTui, repo_root: PathBuf) -> Result<()> {
    let context = build_context(&app.exe_root, &app.boot, repo_root.clone());
    init_progress_settings(&context);
    let pending = run_activation(&repo_root, app.boot.yes, RunMode::Interactive)?.unwrap_or_default();
    let workspace = ensure_workspace_state(&context);
    app.state = AppState::with_workspace(
        AppState::repository_label_from_path(&repo_root),
        workspace,
        &app.registry,
    );
    if !pending.is_empty() {
        app.state.activation_prompt = Some(ActivationPrompt::new(pending));
    }
    app.context = Some(context);
    app.repo_setup = None;
    app.focus.region = FocusRegion::Tasks;
    app.state.status_message = "Repository configured.".to_owned();
    Ok(())
}

fn build_context(exe_root: &PathBuf, boot: &TuiBootParams, repo_root: PathBuf) -> ToolContext {
    ToolContext {
        repo_root,
        tool_root: exe_root.clone(),
        run_mode: RunMode::Interactive,
        min: boot.min,
        trace: boot.trace,
        workers: boot.workers,
        package_dir: boot.package_dir.clone(),
        output_dir: boot.output_dir.clone(),
        logs_dir: boot.logs_dir.clone(),
    }
}

fn navigate_up(app: &mut DharaTui, _rows: &[VisibleTreeRow]) {
    match app.focus.region {
        FocusRegion::Tasks if app.task_row > 0 => app.task_row -= 1,
        FocusRegion::Tabs => {
            let index = tab_index(app.state.main_tab);
            app.state.main_tab = tab_from_index(index.saturating_sub(1));
        }
        FocusRegion::TabContent if app.state.main_tab == MainTab::Options => {
            if let Some(command) = app.state.selected_command(&app.registry) {
                crate::screens::cycle_form_field(command, &mut app.form_field, -1);
            }
        }
        FocusRegion::TabContent if app.state.main_tab == MainTab::Troubleshooting => {
            app.trouble_scroll = app.trouble_scroll.saturating_sub(1);
        }
        FocusRegion::ActionButtons if app.focus.action_button > 0 => app.focus.action_button -= 1,
        _ => {}
    }
}

fn navigate_down(app: &mut DharaTui, rows: &[VisibleTreeRow]) {
    match app.focus.region {
        FocusRegion::Tasks if app.task_row + 1 < rows.len() => app.task_row += 1,
        FocusRegion::Tabs => {
            let index = tab_index(app.state.main_tab);
            app.state.main_tab = tab_from_index((index + 1).min(3));
        }
        FocusRegion::TabContent if app.state.main_tab == MainTab::Options => {
            if let Some(command) = app.state.selected_command(&app.registry) {
                crate::screens::cycle_form_field(command, &mut app.form_field, 1);
            }
        }
        FocusRegion::TabContent if app.state.main_tab == MainTab::Troubleshooting => {
            app.trouble_scroll = app.trouble_scroll.saturating_add(1);
        }
        FocusRegion::ActionButtons if app.focus.action_button + 1 < 3 => {
            app.focus.action_button += 1;
        }
        _ => {}
    }
}

fn navigate_horizontal(app: &mut DharaTui, delta: isize) {
    if app.focus.region != FocusRegion::TabContent || app.state.main_tab != MainTab::Options {
        return;
    }
    let Some(command) = app.state.selected_command(&app.registry).cloned() else {
        return;
    };
    let Some(form) = app.state.forms.get_mut(command.id) else {
        return;
    };
    form.selected_field = app.form_field;
    if delta < 0 {
        form.cycle_previous_option(&command);
    } else {
        form.cycle_next_option(&command);
    }
}

fn activate(app: &mut DharaTui, rows: &[VisibleTreeRow]) {
    match app.focus.region {
        FocusRegion::Tasks => {
            select_task_row(&mut app.state, &app.registry, rows, app.task_row);
        }
        FocusRegion::Tabs => {}
        FocusRegion::TabContent => match app.state.main_tab {
            MainTab::SystemConfigs => load_system_configs(app),
            MainTab::Options => {
                app.editing_form = true;
                if let Some(command) = app.state.selected_command(&app.registry) {
                    if let Some(form) = app.state.forms.get_mut(command.id) {
                        form.selected_field = app.form_field;
                    }
                }
            }
            _ => {}
        },
        FocusRegion::ActionButtons => trigger_action_button(app),
        FocusRegion::Modal => {}
    }
}

fn load_system_configs(app: &mut DharaTui) {
    let Some(context) = app.context.as_ref() else {
        app.state.system_configs_text = Some("No repository configured.".to_owned());
        return;
    };
    let repo_text = show(&context.repo_root).unwrap_or_else(|error| error.to_string());
    let runtime = load_runtime_cache(&context.tool_root)
        .ok()
        .flatten()
        .map(|cache| format!("repository = {}", cache.repository.display()))
        .unwrap_or_else(|| "runtime.toml not found".to_owned());
    let effective = format!(
        "workers = {}\npackage_dir = {}\noutput_dir = {}\nlogs_dir = {}",
        context.workers,
        context
            .package_dir
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "(default)".to_owned()),
        context
            .output_dir
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "(default)".to_owned()),
        context
            .logs_dir
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "(default)".to_owned()),
    );
    app.state.system_configs_text = Some(format!(
        "=== Repository (dhara.config.toml) ===\n{repo_text}\n\n=== Runtime (runtime.toml) ===\n{runtime}\n\n=== Effective tool context ===\n{effective}"
    ));
    app.state.main_tab = MainTab::SystemConfigs;
}

fn trigger_action_button(app: &mut DharaTui) {
    let running = app.state.active_run.is_some();
    match app.focus.action_button {
        0 if !running => {
            if let Some(context) = app.context.clone() {
                app.state.run_selected(&app.registry, &context);
            }
        }
        1 if running => app.state.cancel_active(),
        2 if !running => {
            if let Some(command) = app.state.selected_command(&app.registry).cloned() {
                app.state.reset_form(&command);
            }
        }
        _ => {}
    }
}

fn tab_index(tab: MainTab) -> usize {
    match tab {
        MainTab::Info => 0,
        MainTab::Options => 1,
        MainTab::Troubleshooting => 2,
        MainTab::SystemConfigs => 3,
    }
}
