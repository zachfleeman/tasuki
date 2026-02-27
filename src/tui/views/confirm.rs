use ratatui::{
    layout::{Alignment, Rect},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::tui::app::App;
use crate::tui::theme::Theme;

pub fn draw_confirm(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let block = Block::default()
        .title(" Confirm ")
        .borders(Borders::ALL)
        .border_style(theme.style_accent());

    let text = Paragraph::new(app.confirm_message.clone())
        .block(block)
        .style(theme.style_default())
        .alignment(Alignment::Center);

    let popup = super::centered_rect(50, 15, area);
    f.render_widget(Clear, popup);
    f.render_widget(text, popup);

    let hint = Paragraph::new("y / Enter = confirm    n / Esc = cancel")
        .style(theme.style_muted())
        .alignment(Alignment::Center);

    let hint_area = Rect {
        x: popup.x,
        y: popup.y + popup.height + 1,
        width: popup.width,
        height: 1,
    };
    f.render_widget(hint, hint_area);
}
