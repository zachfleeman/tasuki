use crossterm::{
    event::{self, Event, KeyEvent},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use notify::{EventKind, Event as NotifyEvent, RecommendedWatcher, RecursiveMode, Watcher};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::io;
use std::sync::mpsc::{channel, Receiver};
use std::time::{Duration, Instant};

use std::path::Path;

use crate::backends::BackendManager;
use crate::tui::app::{App, AppMode};
use crate::tui::keybindings::{Action, KeyBindings};
use crate::tui::theme::{DynamicTheme, Theme};

pub mod app;
pub mod keybindings;
pub mod theme;
pub mod ui;
pub mod views;

fn setup_theme_watcher(theme: &Theme) -> crate::error::Result<(RecommendedWatcher, Receiver<NotifyEvent>)> {
    let (tx, rx) = channel::<NotifyEvent>();
    
    let mut watcher = RecommendedWatcher::new(
        move |res: Result<NotifyEvent, notify::Error>| {
            if let Ok(event) = res {
                let is_theme_event = event.paths.iter().any(|p| {
                    p.to_string_lossy().contains("/theme/") || 
                    p.file_name().map(|n| n == "theme").unwrap_or(false)
                });
                
                if is_theme_event {
                    // Only Create/Modify — Omarchy removes folder first, then recreates
                    match event.kind {
                        EventKind::Modify(_) | EventKind::Create(_) => {
                            let _ = tx.send(event);
                        }
                        _ => {}
                    }
                }
            }
        },
        notify::Config::default(),
    )?;
    
    // Watch parent dir — Omarchy replaces the theme subfolder on switch
    if let Some(path) = theme.watch_path() {
        watcher.watch(&path, RecursiveMode::NonRecursive)?;
    }
    
    Ok((watcher, rx))
}

fn setup_vault_watcher(config: &crate::config::Config) -> Option<(RecommendedWatcher, Receiver<NotifyEvent>)> {
    let vault_path = config
        .backends
        .obsidian
        .as_ref()
        .filter(|t| t.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false))
        .and_then(|t| t.get("vault_path").and_then(|v| v.as_str()))
        .map(|s| shellexpand::tilde(s).into_owned())?;

    let vault_path = Path::new(&vault_path).to_path_buf();
    if !vault_path.exists() {
        return None;
    }

    let (tx, rx) = channel::<NotifyEvent>();

    let mut watcher = RecommendedWatcher::new(
        move |res: Result<NotifyEvent, notify::Error>| {
            if let Ok(event) = res {
                let is_md_event = event.paths.iter().any(|p| {
                    p.extension().and_then(|e| e.to_str()) == Some("md")
                });

                if is_md_event {
                    match event.kind {
                        EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_) => {
                            let _ = tx.send(event);
                        }
                        _ => {}
                    }
                }
            }
        },
        notify::Config::default(),
    )
    .ok()?;

    watcher
        .watch(&vault_path, RecursiveMode::Recursive)
        .ok()?;

    Some((watcher, rx))
}

fn setup_config_watcher() -> Option<(RecommendedWatcher, Receiver<NotifyEvent>)> {
    let config_path = crate::config::Config::default_config_path().ok()?;
    let parent = config_path.parent()?.to_path_buf();
    if !parent.exists() {
        return None;
    }

    let (tx, rx) = channel::<NotifyEvent>();

    let mut watcher = RecommendedWatcher::new(
        move |res: Result<NotifyEvent, notify::Error>| {
            if let Ok(event) = res {
                let is_config_event = event.paths.iter().any(|p| {
                    p.file_name().and_then(|n| n.to_str()) == Some("config.toml")
                });

                if is_config_event {
                    match event.kind {
                        EventKind::Modify(_) | EventKind::Create(_) => {
                            let _ = tx.send(event);
                        }
                        _ => {}
                    }
                }
            }
        },
        notify::Config::default(),
    )
    .ok()?;

    watcher.watch(&parent, RecursiveMode::NonRecursive).ok()?;

    Some((watcher, rx))
}

pub async fn run(backend_manager: BackendManager, config: crate::config::Config) -> crate::error::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let initial_theme = Theme::load(&config.general.theme);
    let theme = DynamicTheme::new(initial_theme.clone());
    
    // _watcher must stay alive for the duration of the event loop
    let (_watcher, theme_rx) = match setup_theme_watcher(&initial_theme) {
        Ok((watcher, rx)) => (Some(watcher), Some(rx)),
        Err(_) => (None, None),
    };

    let (_vault_watcher, vault_rx) = match setup_vault_watcher(&config) {
        Some((watcher, rx)) => (Some(watcher), Some(rx)),
        None => (None, None),
    };

    let (_config_watcher, config_rx) = match setup_config_watcher() {
        Some((watcher, rx)) => (Some(watcher), Some(rx)),
        None => (None, None),
    };

    let mut app = App::new(backend_manager, config);
    app.refresh_tasks().await;

    let tick_rate = Duration::from_millis(250);
    let mut last_tick = Instant::now();
    let mut last_theme_change = Instant::now();
    let mut last_vault_change = Instant::now();
    let mut last_config_change = Instant::now();

    loop {
        let current_theme = theme.get();
        terminal.draw(|f| ui::render(f, &app, &current_theme))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        let mut should_quit = false;
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if let Some(action) = handle_key(key, &app) {
                    // Actions that need to suspend the TUI for an external process
                    let external_cmd = match action {
                        Action::OpenInSource => {
                            match get_open_command(&app) {
                                Some(cmd) => Some(cmd),
                                None => {
                                    app.set_status(
                                        "Set $EDITOR or install source app to open this task",
                                        crate::tui::app::StatusLevel::Error,
                                    );
                                    None
                                }
                            }
                        }
                        Action::OpenConfig => get_config_command(),
                        _ => None,
                    };

                    if let Some(cmd) = external_cmd {
                        disable_raw_mode()?;
                        terminal.backend_mut().execute(LeaveAlternateScreen)?;
                        terminal.show_cursor()?;

                        let status = std::process::Command::new(&cmd[0])
                            .args(&cmd[1..])
                            .status();

                        enable_raw_mode()?;
                        terminal.backend_mut().execute(EnterAlternateScreen)?;
                        terminal.hide_cursor()?;
                        terminal.clear()?;

                        match status {
                            Ok(s) if s.success() => {
                                if action == Action::OpenConfig {
                                    app.reload_config().await;
                                } else {
                                    app.refresh_tasks().await;
                                }
                            }
                            Ok(s) => {
                                app.set_status(
                                    format!("Editor exited with code {}", s.code().unwrap_or(-1)),
                                    crate::tui::app::StatusLevel::Warning,
                                );
                            }
                            Err(e) => {
                                app.set_status(
                                    format!("Failed to open: {}", e),
                                    crate::tui::app::StatusLevel::Error,
                                );
                            }
                        }
                    } else if action != Action::OpenInSource && action != Action::OpenConfig {
                        if process_action(action, &mut app).await {
                            should_quit = true;
                        }
                    }
                }
            }
        }

        if let Some(ref rx) = theme_rx {
            while let Ok(_event) = rx.try_recv() {
                if last_theme_change.elapsed() >= Duration::from_secs(1) {
                    let new_theme = Theme::load(&app.config.general.theme);
                    theme.update(new_theme);
                    last_theme_change = Instant::now();
                }
            }
        }

        if let Some(ref rx) = vault_rx {
            while let Ok(_event) = rx.try_recv() {
                if last_vault_change.elapsed() >= Duration::from_secs(1) {
                    app.refresh_tasks().await;
                    last_vault_change = Instant::now();
                }
            }
        }

        if let Some(ref rx) = config_rx {
            while let Ok(_event) = rx.try_recv() {
                if last_config_change.elapsed() >= Duration::from_secs(1) {
                    app.reload_config().await;
                    let new_theme = Theme::load(&app.config.general.theme);
                    theme.update(new_theme);
                    last_config_change = Instant::now();
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }

        if app.should_quit || should_quit {
            break;
        }
    }

    disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

fn get_open_command(app: &App) -> Option<Vec<String>> {
    let task = app.get_selected_visible_task()?;

    if task.source == crate::model::BackendSource::Obsidian {
        if let Some(ref table) = app.config.backends.obsidian {
            if let Ok(obs_config) = crate::backends::obsidian::ObsidianConfig::from_table(table) {
                let backend = crate::backends::obsidian::ObsidianBackend::new(obs_config);
                if let Some(cmd) = backend.open_command(&task) {
                    return Some(cmd);
                }
            }
        }
    }

    let source_path = task.source_path.as_ref()?;
    let line_num = task.source_line.unwrap_or(1);

    if let Ok(editor) = std::env::var("EDITOR") {
        return Some(vec![editor, format!("+{}", line_num), source_path.clone()]);
    }

    None
}

fn get_config_command() -> Option<Vec<String>> {
    let editor = std::env::var("EDITOR").ok()?;
    let config_path = crate::config::Config::default_config_path().ok()?;
    Some(vec![editor, config_path.to_string_lossy().into_owned()])
}

fn handle_key(key: KeyEvent, app: &App) -> Option<Action> {
    match app.mode {
        AppMode::Normal => KeyBindings::handle_normal(key),
        AppMode::Input => KeyBindings::handle_input(key),
        AppMode::Help => KeyBindings::handle_help(key),
        AppMode::Confirm => KeyBindings::handle_confirm(key),
    }
}

async fn process_action(action: Action, app: &mut App) -> bool {
    match action {
        Action::Quit => {
            app.should_quit = true;
            return true;
        }
        Action::MoveUp => {
            app.move_selection_up();
        }
        Action::MoveDown => {
            app.move_selection_down();
        }
        Action::MoveToNextGroup => {
            app.move_to_next_group();
        }
        Action::MoveToPreviousGroup => {
            app.move_to_previous_group();
        }
        Action::ToggleGroup => {
            app.toggle_selected_group();
        }
        Action::ToggleAllGroups => {
            app.toggle_all_groups();
        }
        Action::ToggleTask => {
            app.toggle_selected_task().await;
        }
        Action::EditTask => {
            app.edit_selected_task();
        }
        Action::OpenInSource | Action::OpenConfig => {}
        Action::DeleteTask => {
            app.start_delete_confirmation();
        }
        Action::QuickAdd => {
            app.start_quick_add();
        }
        Action::Search => {
            app.start_search();
        }
        Action::Refresh => {
            app.refresh_tasks().await;
            app.set_status("Tasks refreshed", crate::tui::app::StatusLevel::Info);
        }
        Action::Help => {
            app.toggle_help();
        }
        Action::Cancel => {
            match app.mode {
                AppMode::Help => app.mode = AppMode::Normal,
                AppMode::Confirm => app.cancel_confirm(),
                _ => app.cancel_input(),
            }
        }
        Action::Submit => {
            if app.mode == AppMode::Confirm {
                app.execute_confirm().await;
            } else {
                app.submit_input().await;
            }
        }
        Action::Backspace => {
            app.input_buffer.pop();
        }
        Action::Char(c) => {
            app.input_buffer.push(c);
        }
    }
    false
}
