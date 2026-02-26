use chrono::NaiveDate;

use crate::model::{Priority, TaskStatus};

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedTask {
    pub title: String,
    pub status: TaskStatus,
    pub priority: Priority,
    pub due: Option<NaiveDate>,
    pub completed_at: Option<NaiveDate>,
    pub created_at: Option<NaiveDate>,
    pub tags: Vec<String>,
}

// Parse a checkbox line into a ParsedTask
pub fn parse_checkbox_line(line: &str) -> Option<ParsedTask> {
    let trimmed = line.trim_start();

    if !trimmed.starts_with("- [") {
        return None;
    }

    let after_bracket = &trimmed[3..];
    let status_char = after_bracket.chars().next()?;
    if !after_bracket.starts_with(']') && after_bracket.chars().nth(1) != Some(']') {
        return None;
    }

    let status = match status_char {
        ' ' => TaskStatus::Pending,
        'x' | 'X' => TaskStatus::Done,
        _ => return None,
    };

    let rest = &trimmed[5..].trim_start();
    if rest.is_empty() {
        return None;
    }

    let mut title_parts: Vec<String> = Vec::new();
    let mut priority = Priority::None;
    let mut due: Option<NaiveDate> = None;
    let mut completed_at: Option<NaiveDate> = None;
    let mut created_at: Option<NaiveDate> = None;
    let mut tags: Vec<String> = Vec::new();

    const SKIP_WITH_VALUE: &[&str] = &["⏳", "🛫", "🆔", "⛔", "🏁"];

    let tokens: Vec<&str> = rest.split_whitespace().collect();
    let mut i = 0;

    while i < tokens.len() {
        let token = tokens[i];

        // Priorities
        if token == "⏫" || token == "🔺" {
            priority = Priority::High;
            i += 1;
            continue;
        }
        if token == "🔼" {
            priority = Priority::Medium;
            i += 1;
            continue;
        }
        if token == "🔽" || token == "⏬" {
            priority = Priority::Low;
            i += 1;
            continue;
        }

        // Dates
        if token == "📅" || token == "🗓️" || token == "🗓" {
            if let Some(date) = try_parse_next_date(&tokens, i + 1) {
                due = Some(date);
                i += 2;
                continue;
            }
        }
        if token == "✅" {
            if let Some(date) = try_parse_next_date(&tokens, i + 1) {
                completed_at = Some(date);
                i += 2;
                continue;
            }
        }
        if token == "➕" {
            if let Some(date) = try_parse_next_date(&tokens, i + 1) {
                created_at = Some(date);
                i += 2;
                continue;
            }
        }

        if SKIP_WITH_VALUE.contains(&token) {
            i += 2;
            continue;
        }

        // Recurrence
        if token == "🔁" {
            i += 1;
            while i < tokens.len() && !is_metadata_token(tokens[i]) {
                i += 1;
            }
            continue;
        }

        if token == "(p1)" {
            priority = Priority::High;
            i += 1;
            continue;
        }
        if token == "(p2)" {
            priority = Priority::Medium;
            i += 1;
            continue;
        }
        if token == "(p3)" {
            priority = Priority::Low;
            i += 1;
            continue;
        }

        // Tags
        if let Some(tag) = token.strip_prefix('#') {
            if !tag.is_empty() {
                tags.push(tag.to_string());
            }
            i += 1;
            continue;
        }

        // Due date
        if let Some(date_str) = token.strip_prefix("due:") {
            if let Ok(date) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                due = Some(date);
                i += 1;
                continue;
            }
        }

        title_parts.push(token.to_string());
        i += 1;
    }

    let title = title_parts.join(" ");
    if title.is_empty() {
        return None;
    }

    Some(ParsedTask {
        title,
        status,
        priority,
        due,
        completed_at,
        created_at,
        tags,
    })
}

// Parse checkbox tasks from a markdown file, skipping code blocks
pub fn parse_file(content: &str) -> Vec<(usize, ParsedTask)> {
    let mut results = Vec::new();
    let mut in_code_block = false;

    for (idx, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }

        if in_code_block {
            continue;
        }

        if let Some(task) = parse_checkbox_line(line) {
            results.push((idx + 1, task));
        }
    }

    results
}

fn try_parse_next_date(tokens: &[&str], idx: usize) -> Option<NaiveDate> {
    if idx >= tokens.len() {
        return None;
    }
    NaiveDate::parse_from_str(tokens[idx], "%Y-%m-%d").ok()
}

fn is_metadata_token(token: &str) -> bool {
    matches!(
        token,"📅"|"🗓️"| "🗓"| "✅"| "➕"
        | "⏳"| "🛫"| "⏫"| "🔺"| "🔼"
        | "🔽"| "⏬"| "🔁"| "🆔"|"⛔"
        | "🏁") || token.starts_with('#')
        || token.starts_with("due:")
        || matches!(token, "(p1)" | "(p2)" | "(p3)")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_plain_checkbox_pending() {
        let result = parse_checkbox_line("- [ ] Buy groceries").unwrap();
        assert_eq!(result.title, "Buy groceries");
        assert_eq!(result.status, TaskStatus::Pending);
        assert_eq!(result.priority, Priority::None);
        assert_eq!(result.due, None);
    }

    #[test]
    fn test_plain_checkbox_done() {
        let result = parse_checkbox_line("- [x] Submit report").unwrap();
        assert_eq!(result.title, "Submit report");
        assert_eq!(result.status, TaskStatus::Done);
    }

    #[test]
    fn test_uppercase_x() {
        let result = parse_checkbox_line("- [X] Submit report").unwrap();
        assert_eq!(result.status, TaskStatus::Done);
    }

    #[test]
    fn test_indented_checkbox() {
        let result = parse_checkbox_line("    - [ ] Nested task").unwrap();
        assert_eq!(result.title, "Nested task");
        assert_eq!(result.status, TaskStatus::Pending);
    }

    #[test]
    fn test_tasks_plugin_due_date() {
        let result = parse_checkbox_line("- [ ] Fix bug 📅 2025-03-15").unwrap();
        assert_eq!(result.title, "Fix bug");
        assert_eq!(
            result.due,
            Some(NaiveDate::from_ymd_opt(2025, 3, 15).unwrap())
        );
    }

    #[test]
    fn test_tasks_plugin_completion_date() {
        let result = parse_checkbox_line("- [x] Done thing 📅 2025-01-15 ✅ 2025-01-14").unwrap();
        assert_eq!(result.title, "Done thing");
        assert_eq!(result.status, TaskStatus::Done);
        assert_eq!(
            result.due,
            Some(NaiveDate::from_ymd_opt(2025, 1, 15).unwrap())
        );
        assert_eq!(
            result.completed_at,
            Some(NaiveDate::from_ymd_opt(2025, 1, 14).unwrap())
        );
    }

    #[test]
    fn test_tasks_plugin_priority_high() {
        let result = parse_checkbox_line("- [ ] Important task ⏫").unwrap();
        assert_eq!(result.title, "Important task");
        assert_eq!(result.priority, Priority::High);
    }

    #[test]
    fn test_tasks_plugin_priority_medium() {
        let result = parse_checkbox_line("- [ ] Normal task 🔼").unwrap();
        assert_eq!(result.title, "Normal task");
        assert_eq!(result.priority, Priority::Medium);
    }

    #[test]
    fn test_tasks_plugin_priority_low() {
        let result = parse_checkbox_line("- [ ] Low task 🔽").unwrap();
        assert_eq!(result.title, "Low task");
        assert_eq!(result.priority, Priority::Low);
    }

    #[test]
    fn test_inline_priority() {
        let result = parse_checkbox_line("- [ ] Fix bug (p1)").unwrap();
        assert_eq!(result.title, "Fix bug");
        assert_eq!(result.priority, Priority::High);
    }

    #[test]
    fn test_tags() {
        let result = parse_checkbox_line("- [ ] Review PR #work #urgent").unwrap();
        assert_eq!(result.title, "Review PR");
        assert_eq!(result.tags, vec!["work", "urgent"]);
    }

    #[test]
    fn test_due_date_todotxt_style() {
        let result = parse_checkbox_line("- [ ] Call dentist due:2025-03-20").unwrap();
        assert_eq!(result.title, "Call dentist");
        assert_eq!(
            result.due,
            Some(NaiveDate::from_ymd_opt(2025, 3, 20).unwrap())
        );
    }

    #[test]
    fn test_full_tasks_plugin_line() {
        let result =
            parse_checkbox_line("- [ ] Review PR #work ⏫ 📅 2025-03-15 ➕ 2025-03-01").unwrap();
        assert_eq!(result.title, "Review PR");
        assert_eq!(result.priority, Priority::High);
        assert_eq!(
            result.due,
            Some(NaiveDate::from_ymd_opt(2025, 3, 15).unwrap())
        );
        assert_eq!(
            result.created_at,
            Some(NaiveDate::from_ymd_opt(2025, 3, 1).unwrap())
        );
        assert_eq!(result.tags, vec!["work"]);
    }

    #[test]
    fn test_recurrence_skipped() {
        let result =
            parse_checkbox_line("- [ ] Weekly review 🔁 every Monday 📅 2025-03-17").unwrap();
        assert_eq!(result.title, "Weekly review");
        assert_eq!(
            result.due,
            Some(NaiveDate::from_ymd_opt(2025, 3, 17).unwrap())
        );
    }

    #[test]
    fn test_not_a_checkbox() {
        assert!(parse_checkbox_line("Just some text").is_none());
        assert!(parse_checkbox_line("- Regular list item").is_none());
        assert!(parse_checkbox_line("* [ ] Asterisk checkbox").is_none());
        assert!(parse_checkbox_line("").is_none());
        assert!(parse_checkbox_line("# Heading").is_none());
    }

    #[test]
    fn test_empty_checkbox() {
        assert!(parse_checkbox_line("- [ ] ").is_none());
    }

    #[test]
    fn test_parse_file_basic() {
        let content = "\
# Project Alpha

## Tasks
- [ ] First task 📅 2025-03-15
- [x] Done task ✅ 2025-03-10
- Regular list item

## Notes
Some notes here
- [ ] Another task #work
";
        let tasks = parse_file(content);
        assert_eq!(tasks.len(), 3);
        assert_eq!(tasks[0].0, 4);
        assert_eq!(tasks[0].1.title, "First task");
        assert_eq!(tasks[1].0, 5);
        assert_eq!(tasks[1].1.status, TaskStatus::Done);
        assert_eq!(tasks[2].0, 10);
        assert_eq!(tasks[2].1.title, "Another task");
    }

    #[test]
    fn test_parse_file_skips_code_blocks() {
        let content = "\
- [ ] Real task

```
- [ ] Not a task (in code block)
```

- [ ] Another real task

```markdown
- [ ] Also not a task
```
";
        let tasks = parse_file(content);
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].1.title, "Real task");
        assert_eq!(tasks[1].1.title, "Another real task");
    }

    #[test]
    fn test_parse_file_empty() {
        let tasks = parse_file("");
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_parse_file_no_tasks() {
        let content = "# Just a heading\n\nSome paragraph text.\n";
        let tasks = parse_file(content);
        assert!(tasks.is_empty());
    }
}
