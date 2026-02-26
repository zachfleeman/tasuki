use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use crate::model::{Priority, Task, TaskStatus};
use crate::tui::app::App;
use crate::tui::theme::Theme;

pub fn draw_task_list(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(3)])
        .split(area);

    let task_area = chunks[0];
    let status_area = chunks[1];

    let mut items: Vec<ListItem> = Vec::new();
    let mut visible_idx = 0;

    for (_group_idx, group) in app.task_groups.iter().enumerate() {
        let is_selected = visible_idx == app.selected_task;
        let group_style = if is_selected {
            theme.style_selected().add_modifier(Modifier::BOLD)
        } else {
            theme.style_accent()
        };

        let collapse_icon = if group.collapsed { "▶" } else { "▼" };
        let header_text = format!("{} {} ({})", collapse_icon, group.label, group.tasks.len());

        items.push(ListItem::new(Line::from(vec![Span::styled(
            header_text,
            group_style,
        )])));
        visible_idx += 1;

        if !group.collapsed {
            for task in &group.tasks {
                let is_selected = visible_idx == app.selected_task;
                let style = if is_selected {
                    theme.style_selected().add_modifier(Modifier::BOLD)
                } else {
                    theme.style_default()
                };

                let content = format_task_line(task, theme, task_area.width);
                items.push(ListItem::new(content).style(style));
                visible_idx += 1;
            }
        }
    }

    if items.is_empty() {
        items.push(
            ListItem::new("No tasks found. Press 'a' to add a task.").style(theme.style_muted()),
        );
    }

    let tasks_block = Block::default()
        .title(format!(" Tasks ({}) ", app.tasks.len()))
        .borders(Borders::ALL)
        .border_style(theme.style_muted());

    let list = List::new(items).block(tasks_block);
    f.render_widget(list, task_area);

    let status_text = if let Some((msg, level)) = &app.status_message {
        let style = match level {
            crate::tui::app::StatusLevel::Info => theme.style_default(),
            crate::tui::app::StatusLevel::Success => theme.style_success(),
            crate::tui::app::StatusLevel::Warning => theme.style_warning(),
            crate::tui::app::StatusLevel::Error => theme.style_error(),
        };
        Line::from(vec![Span::styled(msg.clone(), style)])
    } else {
        Line::from(vec![
            Span::styled("↑/↓", theme.style_accent()),
            Span::styled(" navigate  ", theme.style_muted()),
            Span::styled("a", theme.style_accent()),
            Span::styled(" add  ", theme.style_muted()),
            Span::styled("e", theme.style_accent()),
            Span::styled(" edit  ", theme.style_muted()),
            Span::styled("x", theme.style_accent()),
            Span::styled(" toggle  ", theme.style_muted()),
            Span::styled("/", theme.style_accent()),
            Span::styled(" search  ", theme.style_muted()),
            Span::styled("?", theme.style_accent()),
            Span::styled(" help", theme.style_muted()),
        ])
    };

    let status_bar = Paragraph::new(Text::from(vec![status_text])).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme.style_muted()),
    );
    f.render_widget(status_bar, status_area);
}

fn format_task_line<'a>(task: &'a Task, theme: &'a Theme, width: u16) -> Line<'a> {
    let icon = match task.status {
        TaskStatus::Pending => "☐",
        TaskStatus::Done => "✓",
    };

    let icon_style = match task.status {
        TaskStatus::Pending => theme.style_default(),
        TaskStatus::Done => theme.style_success(),
    };

    let priority_marker = match task.priority {
        Priority::High => "[!] ",
        Priority::Medium => "",
        Priority::Low => "",
        Priority::None => "",
    };

    let priority_style = match task.priority {
        Priority::High => theme.style_error(),
        Priority::Medium => theme.style_warning(),
        Priority::Low => theme.style_muted(),
        Priority::None => theme.style_default(),
    };

    let source_label = format!("[{}]", task.source.name());

    let mut tag_str = String::new();
    for tag in &task.tags {
        tag_str.push_str(&format!("#{} ", tag));
    }

    let left_len =
        2 + icon.len() + 1 + priority_marker.len() + task.title.len() + 1 + tag_str.len();
    let right_len = source_label.len();
    let available = width.saturating_sub(2) as usize;

    let padding = if left_len + right_len < available {
        available - left_len - right_len
    } else {
        1
    };

    let mut spans = vec![
        Span::raw("  "), // Indent
        Span::styled(format!("{} ", icon), icon_style),
        Span::styled(priority_marker.to_string(), priority_style),
        Span::styled(task.title.clone(), theme.style_default()),
        Span::raw(" "),
    ];

    for tag in &task.tags {
        spans.push(Span::styled(format!("#{} ", tag), theme.style_highlight()));
    }

    spans.push(Span::raw(" ".repeat(padding)));
    spans.push(Span::styled(source_label, theme.style_muted()));

    Line::from(spans)
}

pub fn draw_help(f: &mut Frame, theme: &Theme, area: Rect) {
    let help_text = vec![
        Line::from(vec![Span::styled(
            "Keybindings",
            theme.style_accent().add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("j, ↓", theme.style_accent()),
            Span::styled("     Move selection down", theme.style_default()),
        ]),
        Line::from(vec![
            Span::styled("k, ↑", theme.style_accent()),
            Span::styled("     Move selection up", theme.style_default()),
        ]),
        Line::from(vec![
            Span::styled("Tab", theme.style_accent()),
            Span::styled("       Go to next group", theme.style_default()),
        ]),
        Line::from(vec![
            Span::styled("S-Tab", theme.style_accent()),
            Span::styled("     Go to previous group", theme.style_default()),
        ]),
        Line::from(vec![
            Span::styled("space", theme.style_accent()),
            Span::styled("      Toggle group collapsed", theme.style_default()),
        ]),
        Line::from(vec![
            Span::styled("C", theme.style_accent()),
            Span::styled("         Toggle all groups", theme.style_default()),
        ]),
        Line::from(vec![
            Span::styled("x, Enter", theme.style_accent()),
            Span::styled(" Toggle task complete/pending", theme.style_default()),
        ]),
        Line::from(vec![
            Span::styled("e", theme.style_accent()),
            Span::styled("         Quick edit task", theme.style_default()),
        ]),
        Line::from(vec![
            Span::styled("o", theme.style_accent()),
            Span::styled("         Open in source app/editor", theme.style_default()),
        ]),
        Line::from(vec![
            Span::styled("a", theme.style_accent()),
            Span::styled("         Quick-add task", theme.style_default()),
        ]),
        Line::from(vec![
            Span::styled("/", theme.style_accent()),
            Span::styled("         Search tasks", theme.style_default()),
        ]),
        Line::from(vec![
            Span::styled("r", theme.style_accent()),
            Span::styled("         Refresh from backends", theme.style_default()),
        ]),
        Line::from(vec![
            Span::styled("d", theme.style_accent()),
            Span::styled("         Delete selected task", theme.style_default()),
        ]),
        Line::from(vec![
            Span::styled("c", theme.style_accent()),
            Span::styled("         Open config in $EDITOR", theme.style_default()),
        ]),
        Line::from(vec![
            Span::styled("?", theme.style_accent()),
            Span::styled("         Toggle this help", theme.style_default()),
        ]),
        Line::from(vec![
            Span::styled("q, Esc", theme.style_accent()),
            Span::styled("   Quit TUI", theme.style_default()),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Quick-add supports: #tags @backends (p1/p2/p3) today/tomorrow/YYYY-MM-DD",
            theme.style_muted(),
        )]),
    ];

    let help_paragraph = Paragraph::new(Text::from(help_text)).block(
        Block::default()
            .title(" Help (? to close) ")
            .borders(Borders::ALL)
            .border_style(theme.style_accent()),
    );

    let area = super::centered_rect(60, 70, area);
    f.render_widget(Clear, area);
    f.render_widget(help_paragraph, area);
}
