use ratatui::{
    layout::{Alignment, Rect},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::tui::app::{App, InputMode};
use crate::tui::theme::Theme;

pub fn draw_input(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let title = match &app.input_mode {
        Some(InputMode::QuickAdd) => " Quick Add ",
        Some(InputMode::Search) => " Search ",
        Some(InputMode::EditTask(_)) => " Edit Task ",
        None => " Input ",
    };

    let input_block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(theme.style_accent());

    let input = Paragraph::new(app.input_buffer.clone())
        .block(input_block)
        .style(theme.style_default());

    let area = super::centered_rect(80, 20, area);
    f.render_widget(Clear, area);
    f.render_widget(input, area);

    let cursor_x = area.x + app.input_buffer.len() as u16 + 1;
    let cursor_y = area.y + 1;
    f.set_cursor_position((cursor_x, cursor_y));

    let hint_text = match &app.input_mode {
        Some(InputMode::QuickAdd) => {
            "Supports: #tags @backends (p1/p2/p3) today/tomorrow/YYYY-MM-DD"
        }
        Some(InputMode::Search) => "Type to filter tasks, Enter to confirm, Esc to cancel",
        Some(InputMode::EditTask(_)) => "Edit task and press Enter to save, Esc to cancel",
        None => "",
    };

    let hint = Paragraph::new(hint_text)
        .style(theme.style_muted())
        .alignment(Alignment::Center);

    let hint_area = Rect {
        x: area.x,
        y: area.y + area.height + 1,
        width: area.width,
        height: 1,
    };
    f.render_widget(hint, hint_area);
}
