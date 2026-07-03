use std::io::{self, IsTerminal, Stdout};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use dhara_tool_cli::command::{CommandRegistry, RunMode, ToolContext};
use dhara_tool_cli::interactive::{ActivationPrompt, AppState, MainTab};
use dhara_tool_kernel::{
    ProgressSnapshot, activation::run_activation, ensure_workspace_state,
    logging::init_progress_settings, load_runtime_cache, register_interactive_progress_sender,
    repo_config::{ConfigDriftItem, show},
    resolve_and_persist_repository, unregister_interactive_progress_sender,
};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::widgets::Block;
use ratatui_interact::components::{
    ButtonState, CheckBoxState, InputState, MousePointer, MousePointerState, ScrollableContentState,
    SpinnerState, TabViewAction, TabViewState, TreeViewState as WidgetTreeState,
    handle_scrollable_content_key, handle_scrollable_content_mouse, handle_tab_view_key,
    handle_tab_view_mouse,
};
use ratatui_interact::events::{get_char, is_left_click};
use ratatui_interact::theme::Theme;
use ratatui_interact::traits::{ClickRegionRegistry, ContainerAction};

use crate::adapters::task_tree::{
    TreeKeyAction, apply_tree_selection, build_tree_nodes, handle_tree_key, render_task_tree,
    sync_nav_from_widget, sync_widget_from_nav, task_row_from_widget,
};
use crate::boot::TuiBootParams;
use crate::command_bar::{FooterContext, render_command_bar};
use crate::focus::{ShellFocus, TuiFocus};
use crate::screens::modals::{ModalLayer, RepoSetupPrompt};
use crate::screens::{
    apply_option_widgets_to_form, cycle_form_field, render_center_panel, sync_option_widgets_from_form,
    sync_state_from_tab_view, tab_index,
};
use crate::theme::{self, interact_theme};
use crate::widgets::{action_panel, title_bar};

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
    pub shell_focus: ShellFocus,
    pub task_row: usize,
    pub form_field: usize,
    pub editing_form: bool,
    pub theme: Theme,
    pub task_tree_widget: WidgetTreeState,
    pub task_tree_nodes: Vec<ratatui_interact::components::TreeNode<crate::adapters::task_tree::TaskTreeData>>,
    pub tab_view_state: TabViewState,
    pub info_scroll: ScrollableContentState,
    pub trouble_scroll: ScrollableContentState,
    pub system_scroll: ScrollableContentState,
    pub run_btn: ButtonState,
    pub cancel_btn: ButtonState,
    pub reset_btn: ButtonState,
    pub spinner: SpinnerState,
    pub option_input: InputState,
    pub option_checkbox: CheckBoxState,
    pub modals: ModalLayer,
    pub shell_clicks: ClickRegionRegistry<TuiFocus>,
    pub tab_clicks: ClickRegionRegistry<TabViewAction>,
    pub mouse_pointer: MousePointerState,
    pub mouse_col: u16,
    pub mouse_row: u16,
    pub center_content_area: Rect,
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
    let theme = interact_theme();

    let mut app = if let Some(context) = initial_context {
        init_progress_settings(&context);
        let workspace = ensure_workspace_state(&context);
        let mut state = AppState::with_workspace(
            AppState::repository_label_from_path(&context.repo_root),
            workspace,
            registry,
        );
        let mut modals = ModalLayer::default();
        if !pending_activation.is_empty() {
            state.activation_prompt = Some(ActivationPrompt::new(pending_activation));
            state.status_message =
                "Configuration drift detected. Confirm activation to continue.".to_owned();
            modals.show_activation();
        }
        build_app(
            state,
            registry.clone(),
            exe_root,
            boot,
            Some(context),
            None,
            progress_rx,
            theme,
            modals,
        )
    } else {
        let initial = stale_repository_hint
            .as_ref()
            .map(|path| path.display().to_string());
        let mut modals = ModalLayer::default();
        modals.show_repo(initial.clone());
        build_app(
            AppState::with_repository_label("repository required"),
            registry.clone(),
            exe_root,
            boot,
            None,
            Some(RepoSetupPrompt::new(initial)),
            progress_rx,
            theme,
            modals,
        )
    };

    let result = run_loop(&mut terminal, &mut app);
    unregister_interactive_progress_sender();
    restore_terminal(&mut terminal)?;
    result
}

fn build_app(
    state: AppState,
    registry: CommandRegistry,
    exe_root: PathBuf,
    boot: TuiBootParams,
    context: Option<ToolContext>,
    repo_setup: Option<RepoSetupPrompt>,
    progress_rx: Arc<Mutex<Receiver<ProgressSnapshot>>>,
    theme: Theme,
    modals: ModalLayer,
) -> DharaTui {
    let task_tree_nodes = build_tree_nodes(&state.nav_tree);
    DharaTui {
        state,
        registry,
        exe_root,
        boot,
        context,
        repo_setup,
        progress_rx,
        shell_focus: ShellFocus::default(),
        task_row: 0,
        form_field: 0,
        editing_form: false,
        theme,
        task_tree_widget: WidgetTreeState::new(),
        task_tree_nodes,
        tab_view_state: TabViewState::new(4),
        info_scroll: ScrollableContentState::new(Vec::new()),
        trouble_scroll: ScrollableContentState::new(Vec::new()),
        system_scroll: ScrollableContentState::new(Vec::new()),
        run_btn: ButtonState::enabled(),
        cancel_btn: ButtonState::disabled(),
        reset_btn: ButtonState::disabled(),
        spinner: SpinnerState::new(),
        option_input: InputState::new(String::new()),
        option_checkbox: CheckBoxState::new(false),
        modals,
        shell_clicks: ClickRegionRegistry::new(),
        tab_clicks: ClickRegionRegistry::new(),
        mouse_pointer: MousePointerState::with_enabled(true),
        mouse_col: 0,
        mouse_row: 0,
        center_content_area: Rect::default(),
    }
}

fn setup_terminal() -> Result<ratatui::Terminal<ratatui_crossterm::CrosstermBackend<Stdout>>> {
    enable_raw_mode().context("failed to enable raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, event::EnableMouseCapture)
        .context("failed to enter alternate screen")?;
    let backend = ratatui_crossterm::CrosstermBackend::new(stdout);
    ratatui::Terminal::new(backend).context("failed to create terminal")
}

fn restore_terminal(
    terminal: &mut ratatui::Terminal<ratatui_crossterm::CrosstermBackend<Stdout>>,
) -> Result<()> {
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

        app.task_tree_nodes = build_tree_nodes(&app.state.nav_tree);
        sync_widget_from_nav(
            &mut app.task_tree_widget,
            &app.state.tree_view,
            &app.task_tree_nodes,
            app.task_row,
        );

        terminal.draw(|frame| draw(frame, app))?;

        if app.state.should_quit {
            break;
        }

        if event::poll(Duration::from_millis(100)).context("failed to poll events")? {
            match event::read().context("failed to read event")? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    handle_key(app, key)?;
                }
                Event::Mouse(mouse) => handle_mouse(app, mouse),
                _ => {}
            }
        }
    }
    Ok(())
}

fn draw(frame: &mut ratatui::Frame<'_>, app: &mut DharaTui) {
    let area = frame.area();
    frame.render_widget(
        Block::default().style(theme::panel_style()),
        area,
    );

    let layout = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(1),
        Constraint::Length(2),
    ])
    .split(area);

    let repo = app
        .context
        .as_ref()
        .map(|ctx| AppState::repository_label_from_path(&ctx.repo_root))
        .unwrap_or_else(|| "select repository".to_owned());
    title_bar::render_title_bar(frame, layout[0], env!("CARGO_PKG_VERSION"), &repo);

    let body = Layout::horizontal([
        Constraint::Percentage(20),
        Constraint::Percentage(50),
        Constraint::Fill(1),
    ])
    .split(layout[1]);

    app.shell_clicks.clear();
    app.tab_clicks.clear();

    let tree_focused = app.shell_focus.is_focused(&TuiFocus::TaskTree);
    render_task_tree(
        frame,
        body[0],
        &app.task_tree_nodes,
        &app.task_tree_widget,
        &app.theme,
        tree_focused,
    );

    let center_clicks = render_center_panel(
        frame,
        body[1],
        &app.state,
        &app.registry,
        &app.theme,
        &mut app.tab_view_state,
        &app.shell_focus,
        &mut app.info_scroll,
        &mut app.trouble_scroll,
        &mut app.system_scroll,
        app.form_field,
        app.editing_form,
        &app.option_input,
        &app.option_checkbox,
    );
    app.tab_clicks = center_clicks.registry;
    let center_chunks = Layout::vertical([Constraint::Length(3), Constraint::Min(1)]).split(body[1]);
    app.center_content_area = center_chunks[1];

    action_panel::render_action_panel(
        frame,
        body[2],
        &app.state,
        &app.theme,
        &app.shell_focus,
        &mut app.run_btn,
        &mut app.cancel_btn,
        &mut app.reset_btn,
        &mut app.spinner,
        &mut app.shell_clicks,
    );

    let footer = FooterContext {
        state: &app.state,
        shell_focus: &app.shell_focus,
        repo_setup: app.repo_setup.is_some(),
        editing_form: app.editing_form,
    };
    render_command_bar(frame, layout[2], &footer);

    if app.state.activation_prompt.is_some() {
        app.modals.render_activation(frame, &app.state, &app.theme);
    }
    if app.repo_setup.is_some() {
        app.modals.render_repo(frame, &app.theme);
    }

    app.mouse_pointer.update_position(app.mouse_col, app.mouse_row);
    MousePointer::new(&app.mouse_pointer)
        .theme(&app.theme)
        .render(frame.buffer_mut());
}

fn handle_key(app: &mut DharaTui, key: KeyEvent) -> Result<()> {
    let screen = Rect::new(0, 0, 200, 60);

    if app.repo_setup.is_some() {
        return handle_repo_modal_key(app, key, screen);
    }
    if app.state.activation_prompt.is_some() {
        return handle_activation_modal_key(app, key, screen);
    }

    if app.editing_form {
        return handle_form_edit_key(app, key);
    }

    if key.code == KeyCode::Tab {
        if key.modifiers.contains(KeyModifiers::SHIFT) {
            app.shell_focus.prev();
        } else {
            app.shell_focus.next();
        }
        return Ok(());
    }

    if app.shell_focus.is_focused(&TuiFocus::MainTabs) || app.shell_focus.is_focused(&TuiFocus::TabContent) {
        if handle_tab_view_key(&mut app.tab_view_state, &key, ratatui_interact::components::TabPosition::Top) {
            app.state.main_tab = sync_state_from_tab_view(&app.tab_view_state);
            return Ok(());
        }
    }

    if app.shell_focus.is_focused(&TuiFocus::TabContent) {
        if handle_tab_content_key(app, &key) {
            return Ok(());
        }
    }

    match app.shell_focus.current() {
        Some(TuiFocus::TaskTree) => handle_tree_keys(app, key.code)?,
        Some(TuiFocus::ActionRun) | Some(TuiFocus::ActionCancel) | Some(TuiFocus::ActionReset) => {
            if key.code == KeyCode::Enter {
                trigger_action_button(app);
            }
        }
        _ => handle_global_keys(app, key.code)?,
    }
    Ok(())
}

fn handle_tree_keys(app: &mut DharaTui, code: KeyCode) -> Result<()> {
    match handle_tree_key(&mut app.task_tree_widget, &app.task_tree_nodes, code) {
        TreeKeyAction::SelectionChanged => {
            app.task_row = task_row_from_widget(&app.task_tree_widget);
            sync_nav_from_widget(&app.task_tree_widget, &mut app.state.tree_view, &app.task_tree_nodes);
        }
        TreeKeyAction::Toggled => {
            sync_nav_from_widget(&app.task_tree_widget, &mut app.state.tree_view, &app.task_tree_nodes);
        }
        TreeKeyAction::Activate => {
            apply_tree_selection(
                &mut app.state,
                &app.registry,
                &app.task_tree_nodes,
                &app.task_tree_widget,
            );
            app.task_tree_nodes = build_tree_nodes(&app.state.nav_tree);
            sync_widget_from_nav(
                &mut app.task_tree_widget,
                &app.state.tree_view,
                &app.task_tree_nodes,
                app.task_row,
            );
        }
        TreeKeyAction::None => {}
    }
    Ok(())
}

fn handle_global_keys(app: &mut DharaTui, code: KeyCode) -> Result<()> {
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
        KeyCode::Enter => activate_focused(app),
        _ => {}
    }
    Ok(())
}

fn handle_tab_content_key(app: &mut DharaTui, key: &KeyEvent) -> bool {
    let tab = app.tab_view_state.selected_index;
    let height = app.center_content_area.height as usize;
    match tab {
        0 if app.shell_focus.is_focused(&TuiFocus::TabContent) => {
            handle_scrollable_content_key(&mut app.info_scroll, key, height).is_some()
        }
        2 if app.shell_focus.is_focused(&TuiFocus::TabContent) => {
            handle_scrollable_content_key(&mut app.trouble_scroll, key, height).is_some()
        }
        3 if app.shell_focus.is_focused(&TuiFocus::TabContent) => {
            if key.code == KeyCode::Enter {
                load_system_configs(app);
                return true;
            }
            handle_scrollable_content_key(&mut app.system_scroll, key, height).is_some()
        }
        1 if app.shell_focus.is_focused(&TuiFocus::TabContent) => match key.code {
            KeyCode::Up => {
                if let Some(command) = app.state.selected_command(&app.registry) {
                    cycle_form_field(command, &mut app.form_field, -1);
                }
                true
            }
            KeyCode::Down => {
                if let Some(command) = app.state.selected_command(&app.registry) {
                    cycle_form_field(command, &mut app.form_field, 1);
                }
                true
            }
            KeyCode::Left => {
                cycle_option_select(app, -1);
                true
            }
            KeyCode::Right => {
                cycle_option_select(app, 1);
                true
            }
            KeyCode::Enter => {
                app.editing_form = true;
                sync_option_widgets_from_form(
                    &app.state,
                    &app.registry,
                    app.form_field,
                    &mut app.option_input,
                    &mut app.option_checkbox,
                );
                if let Some(command) = app.state.selected_command(&app.registry) {
                    if let Some(form) = app.state.forms.get_mut(command.id) {
                        form.selected_field = app.form_field;
                    }
                }
                true
            }
            _ => false,
        },
        _ => false,
    }
}

fn handle_mouse(app: &mut DharaTui, mouse: MouseEvent) {
    app.mouse_col = mouse.column;
    app.mouse_row = mouse.row;
    let screen = Rect::new(0, 0, 200, 60);

    if app.repo_setup.is_some() {
        if let Some(action) = app.modals.handle_repo_mouse(mouse, screen) {
            apply_repo_modal_action(app, action);
        }
        return;
    }
    if app.state.activation_prompt.is_some() {
        if let Some(action) = app.modals.handle_activation_mouse(mouse, screen) {
            apply_activation_action(app, action);
        }
        return;
    }

    if is_left_click(&mouse) {
        if let Some(focus) = app.shell_clicks.handle_click(mouse.column, mouse.row) {
            app.shell_focus.focus(focus.clone());
            if matches!(focus, TuiFocus::ActionRun | TuiFocus::ActionCancel | TuiFocus::ActionReset) {
                trigger_action_button(app);
            }
        } else if let Some(action) = app.tab_clicks.handle_click(mouse.column, mouse.row) {
            handle_tab_view_mouse(&mut app.tab_view_state, &app.tab_clicks, &mouse);
            let _ = action;
            app.state.main_tab = sync_state_from_tab_view(&app.tab_view_state);
        }

        if app.shell_focus.is_focused(&TuiFocus::TabContent) {
            let tab = app.tab_view_state.selected_index;
            let h = app.center_content_area.height as usize;
            match tab {
                0 => {
                    let _ = handle_scrollable_content_mouse(
                        &mut app.info_scroll,
                        &mouse,
                        app.center_content_area,
                        h,
                    );
                }
                2 => {
                    let _ = handle_scrollable_content_mouse(
                        &mut app.trouble_scroll,
                        &mouse,
                        app.center_content_area,
                        h,
                    );
                }
                3 => {
                    let _ = handle_scrollable_content_mouse(
                        &mut app.system_scroll,
                        &mouse,
                        app.center_content_area,
                        h,
                    );
                }
                _ => {}
            }
        }
    }
}

fn handle_repo_modal_key(app: &mut DharaTui, key: KeyEvent, screen: Rect) -> Result<()> {
    if let Some(action) = app.modals.handle_repo_key(key, screen) {
        apply_repo_modal_action(app, action);
    }
    Ok(())
}

fn handle_activation_modal_key(app: &mut DharaTui, key: KeyEvent, screen: Rect) -> Result<()> {
    if let Some(action) = app.modals.handle_activation_key(key, screen) {
        apply_activation_action(app, action);
    }
    Ok(())
}

fn apply_repo_modal_action(app: &mut DharaTui, action: ContainerAction) {
    match action {
        ContainerAction::Submit => {
            let path = app.modals.repo_state.children.path_input.text().trim().to_owned();
            if path.is_empty() {
                app.state.status_message = "Repository path is required.".to_owned();
                return;
            }
            match resolve_and_persist_repository(&app.exe_root, PathBuf::from(path), true) {
                Ok(repo_root) => {
                    if finish_repository_setup(app, repo_root).is_err() {
                        // error already in status
                    }
                }
                Err(error) => app.state.status_message = error.to_string(),
            }
        }
        ContainerAction::Close => {
            app.state.should_quit = true;
        }
        _ => {}
    }
}

fn apply_activation_action(app: &mut DharaTui, action: ContainerAction) {
    match action {
        ContainerAction::Submit => {
            if let Some(context) = app.context.as_ref() {
                if let Err(error) = app.state.apply_activation_confirm(&context.repo_root) {
                    app.state.status_message = error.to_string();
                }
            }
            app.modals.hide_activation();
        }
        ContainerAction::Close => {
            app.state.decline_activation();
            app.modals.hide_activation();
        }
        _ => {}
    }
}

fn handle_form_edit_key(app: &mut DharaTui, key: KeyEvent) -> Result<()> {
    let Some(command) = app.state.selected_command(&app.registry).cloned() else {
        app.editing_form = false;
        return Ok(());
    };
    let field = command.ui.fields.get(app.form_field);

    match key.code {
        KeyCode::Esc | KeyCode::Enter => {
            apply_option_widgets_to_form(
                &mut app.state,
                &app.registry,
                app.form_field,
                &app.option_input,
                &app.option_checkbox,
            );
            app.editing_form = false;
            if let Some(form) = app.state.forms.get_mut(command.id) {
                form.selected_field = app.form_field;
            }
        }
        KeyCode::Char(' ') if field.is_some_and(|f| matches!(f.kind, dhara_tool_cli::command::FieldKind::Boolean)) => {
            app.option_checkbox.toggle();
        }
        KeyCode::Backspace => {
            app.option_input.delete_char_backward();
        }
        KeyCode::Char(_) if get_char(&key).is_some() => {
            if field.is_some_and(|f| {
                matches!(
                    f.kind,
                    dhara_tool_cli::command::FieldKind::Text
                        | dhara_tool_cli::command::FieldKind::Path
                        | dhara_tool_cli::command::FieldKind::BrowsablePath { .. }
                )
            }) {
                if let Some(ch) = get_char(&key) {
                    app.option_input.insert_char(ch);
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn cycle_option_select(app: &mut DharaTui, delta: isize) {
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

fn activate_focused(app: &mut DharaTui) {
    match app.shell_focus.current() {
        Some(TuiFocus::TaskTree) => {
            apply_tree_selection(
                &mut app.state,
                &app.registry,
                &app.task_tree_nodes,
                &app.task_tree_widget,
            );
            app.task_tree_nodes = build_tree_nodes(&app.state.nav_tree);
            sync_widget_from_nav(
                &mut app.task_tree_widget,
                &app.state.tree_view,
                &app.task_tree_nodes,
                app.task_row,
            );
        }
        Some(TuiFocus::TabContent) if app.state.main_tab == MainTab::SystemConfigs => {
            load_system_configs(app);
        }
        Some(TuiFocus::TabContent) if app.state.main_tab == MainTab::Options => {
            app.editing_form = true;
            sync_option_widgets_from_form(
                &app.state,
                &app.registry,
                app.form_field,
                &mut app.option_input,
                &mut app.option_checkbox,
            );
            if let Some(command) = app.state.selected_command(&app.registry) {
                if let Some(form) = app.state.forms.get_mut(command.id) {
                    form.selected_field = app.form_field;
                }
            }
        }
        Some(TuiFocus::ActionRun) | Some(TuiFocus::ActionCancel) | Some(TuiFocus::ActionReset) => {
            trigger_action_button(app);
        }
        _ => {}
    }
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
        app.modals.show_activation();
    } else {
        app.modals.hide_activation();
    }
    app.context = Some(context);
    app.repo_setup = None;
    app.modals.hide_repo();
    app.shell_focus.focus(TuiFocus::TaskTree);
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
    app.tab_view_state.select(tab_index(MainTab::SystemConfigs));
}

fn trigger_action_button(app: &mut DharaTui) {
    let running = app.state.active_run.is_some();
    match app.shell_focus.current() {
        Some(TuiFocus::ActionRun) if !running => {
            if let Some(context) = app.context.clone() {
                app.state.run_selected(&app.registry, &context);
            }
        }
        Some(TuiFocus::ActionCancel) if running => app.state.cancel_active(),
        Some(TuiFocus::ActionReset) if !running => {
            if let Some(command) = app.state.selected_command(&app.registry).cloned() {
                app.state.reset_form(&command);
            }
        }
        _ => {}
    }
}
