use crossterm::event::{KeyCode, KeyEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Quit,
    MoveUp,
    MoveDown,
    MoveToNextGroup,
    MoveToPreviousGroup,
    ToggleGroup,
    ToggleAllGroups,
    ToggleTask,
    EditTask,
    OpenInSource,
    OpenConfig,
    DeleteTask,
    QuickAdd,
    Search,
    Refresh,
    Help,
    Cancel,
    Submit,
    Backspace,
    Char(char),
}

pub struct KeyBindings;

impl KeyBindings {
    pub fn handle_normal(key: KeyEvent) -> Option<Action> {
        match key.code {
            // Quit
            KeyCode::Char('q') | KeyCode::Esc => Some(Action::Quit),

            // Navigation
            KeyCode::Char('j') | KeyCode::Down => Some(Action::MoveDown),
            KeyCode::Char('k') | KeyCode::Up => Some(Action::MoveUp),
            KeyCode::Tab => Some(Action::MoveToNextGroup),
            KeyCode::BackTab => Some(Action::MoveToPreviousGroup),

            // Group actions
            KeyCode::Char(' ') => Some(Action::ToggleGroup),
            KeyCode::Char('C') => Some(Action::ToggleAllGroups),

            // Actions
            KeyCode::Char('x') | KeyCode::Enter => Some(Action::ToggleTask),
            KeyCode::Char('e') => Some(Action::EditTask),
            KeyCode::Char('o') => Some(Action::OpenInSource),
            KeyCode::Char('c') => Some(Action::OpenConfig),
            KeyCode::Char('d') => {
                // Check for 'dd' (vim-style delete)
                // For now, just single 'd' opens delete confirmation
                Some(Action::DeleteTask)
            }
            KeyCode::Char('a') => Some(Action::QuickAdd),
            KeyCode::Char('/') => Some(Action::Search),
            KeyCode::Char('r') => Some(Action::Refresh),
            KeyCode::Char('?') => Some(Action::Help),

            _ => None,
        }
    }

    pub fn handle_input(key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Esc => Some(Action::Cancel),
            KeyCode::Enter => Some(Action::Submit),
            KeyCode::Backspace => Some(Action::Backspace),
            KeyCode::Char(c) => Some(Action::Char(c)),
            _ => None,
        }
    }

    pub fn handle_help(key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('?') => Some(Action::Cancel),
            _ => Some(Action::Cancel), // Any key closes help
        }
    }
}
