use ratatui::Frame;

use crate::tui::app::{App, AppMode};
use crate::tui::theme::Theme;
use crate::tui::views::{quick_add, task_list};

pub fn render(f: &mut Frame, app: &App, theme: &Theme) {
    let area = f.area();

    f.render_widget(
        ratatui::widgets::Block::default().style(theme.style_default()),
        area,
    );

    match app.mode {
        AppMode::Normal | AppMode::Input => {
            task_list::draw_task_list(f, app, theme, area);

            if app.mode == AppMode::Input {
                quick_add::draw_input(f, app, theme, area);
            }
        }
        AppMode::Help => {
            task_list::draw_task_list(f, app, theme, area);
            task_list::draw_help(f, theme, area);
        }
    }
}
