use async_trait::async_trait;
use chrono::NaiveDate;
use std::fs;
use std::path::PathBuf;

use crate::backends::TaskBackend;
use crate::error::{Result, TasukiError};
use crate::model::{BackendSource, NewTask, Task, TaskFilter, TaskId, TaskStatus, TaskUpdate, Priority};

pub struct LocalFileConfig {
    pub path: PathBuf,
}

impl LocalFileConfig {
    pub fn default_dir() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("~"))
            .join(".tasuki")
    }

    pub fn from_table(table: &toml::Table) -> Result<Self> {
        let default_dir = Self::default_dir();

        let path = table
            .get("path")
            .and_then(|v| v.as_str())
            .map(|s| shellexpand::tilde(s).into_owned())
            .map(PathBuf::from)
            .unwrap_or_else(|| default_dir.join("todo.txt"));

        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    TasukiError::Config(format!("Failed to create {}: {}", parent.display(), e))
                })?;
            }
        }

        Ok(Self { path })
    }
}

pub struct LocalFileBackend {
    config: LocalFileConfig,
}

impl LocalFileBackend {
    pub fn new(config: LocalFileConfig) -> Self {
        Self { config }
    }

    fn parse_line(&self, line: &str, line_num: usize) -> Option<Task> {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return None;
        }

        let (status, rest) = if line.starts_with("x ") {
            (TaskStatus::Done, &line[2..])
        } else {
            (TaskStatus::Pending, line)
        };

        let mut rest = rest.trim_start();
        
        // Priority
        let priority = if rest.starts_with("(p1)") {
            rest = &rest[4..].trim_start();
            Priority::High
        } else if rest.starts_with("(p2)") {
            rest = &rest[4..].trim_start();
            Priority::Medium
        } else if rest.starts_with("(p3)") {
            rest = &rest[4..].trim_start();
            Priority::Low
        } else {
            Priority::None
        };

        let (completed_at, rest) = if status == TaskStatus::Done {
            if let Some((date_str, remaining)) = Self::parse_date_prefix(rest) {
                (date_str, remaining)
            } else {
                (None, rest)
            }
        } else {
            (None, rest)
        };

        let (created_at, rest) = if let Some((date_str, remaining)) = Self::parse_date_prefix(rest) {
            (date_str, remaining)
        } else {
            (None, rest)
        };

        let mut tags = Vec::new();
        let mut due = None;
        let mut title_parts = Vec::new();

        for word in rest.split_whitespace() {
            if word.starts_with('#') {
                tags.push(word[1..].to_string());
            } else if word.starts_with("due:") {
                if let Some(date_str) = word.strip_prefix("due:") {
                    due = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok();
                }
            } else {
                title_parts.push(word);
            }
        }

        let title = title_parts.join(" ");
        if title.is_empty() {
            return None;
        }

        Some(Task {
            id: format!("local:{}", line_num),
            title,
            status,
            priority,
            due,
            tags,
            source: BackendSource::LocalFile,
            source_line: Some(line_num),
            source_path: Some(self.config.path.to_string_lossy().into_owned()),
            created_at: created_at.map(|d| d.and_hms_opt(0, 0, 0).unwrap()),
            completed_at: completed_at.map(|d| d.and_hms_opt(0, 0, 0).unwrap()),
        })
    }

    fn parse_date_prefix(s: &str) -> Option<(Option<NaiveDate>, &str)> {
        let s = s.trim_start();
        if s.len() >= 10 {
            let date_part = &s[..10];
            if let Ok(date) = NaiveDate::parse_from_str(date_part, "%Y-%m-%d") {
                return Some((Some(date), &s[10..].trim_start()));
            }
        }
        None
    }

    fn read_tasks(&self) -> Result<Vec<Task>> {
        if !self.config.path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&self.config.path)?;
        let tasks: Vec<Task> = content
            .lines()
            .enumerate()
            .filter_map(|(i, line)| self.parse_line(line, i + 1))
            .collect();

        Ok(tasks)
    }
}

#[async_trait]
impl TaskBackend for LocalFileBackend {
    fn name(&self) -> &str {
        "local"
    }

    fn source(&self) -> BackendSource {
        BackendSource::LocalFile
    }

    async fn fetch_tasks(&self, filter: &TaskFilter) -> Result<Vec<Task>> {
        let mut tasks = self.read_tasks()?;

        if let Some(ref status) = filter.status {
            tasks.retain(|t| &t.status == status);
        }

        if let Some(ref due_before) = filter.due_before {
            tasks.retain(|t| t.due.map_or(false, |d| d <= *due_before));
        }

        if let Some(ref due_after) = filter.due_after {
            tasks.retain(|t| t.due.map_or(false, |d| d >= *due_after));
        }

        if let Some(ref search) = filter.search {
            let search_lower = search.to_lowercase();
            tasks.retain(|t| t.title.to_lowercase().contains(&search_lower));
        }

        Ok(tasks)
    }

    async fn create_task(&self, task: &NewTask) -> Result<Task> {
        let line_num = if self.config.path.exists() {
            fs::read_to_string(&self.config.path)?.lines().count() + 1
        } else {
            1
        };

        let mut parts = Vec::new();

        match task.priority {
            Priority::High => parts.push("(p1)".to_string()),
            Priority::Medium => parts.push("(p2)".to_string()),
            Priority::Low => parts.push("(p3)".to_string()),
            Priority::None => {}
        }

        let today = chrono::Local::now().date_naive();
        parts.push(today.to_string());
        parts.push(task.title.clone());

        for tag in &task.tags {
            parts.push(format!("#{}", tag));
        }

        if let Some(due) = task.due {
            parts.push(format!("due:{}", due));
        }

        let line = parts.join(" ") + "\n";

        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.config.path)?;
        file.write_all(line.as_bytes())?;

        Ok(Task {
            id: format!("local:{}", line_num),
            title: task.title.clone(),
            status: TaskStatus::Pending,
            priority: task.priority,
            due: task.due,
            tags: task.tags.clone(),
            source: BackendSource::LocalFile,
            source_line: Some(line_num),
            source_path: Some(self.config.path.to_string_lossy().into_owned()),
            created_at: Some(today.and_hms_opt(0, 0, 0).unwrap()),
            completed_at: None,
        })
    }

    async fn update_task(&self, id: &TaskId, update: &TaskUpdate) -> Result<Task> {
        let line_num: usize = id
            .strip_prefix("local:")
            .ok_or_else(|| TasukiError::Parse(format!("Invalid task ID: {}", id)))?
            .parse()
            .map_err(|_| TasukiError::Parse(format!("Invalid task ID: {}", id)))?;

        if !self.config.path.exists() {
            return Err(TasukiError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "todo.txt not found",
            )));
        }

        let content = fs::read_to_string(&self.config.path)?;
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

        if line_num == 0 || line_num > lines.len() {
            return Err(TasukiError::Parse(format!("Line {} not found", line_num)));
        }

        let current_line = &lines[line_num - 1];
        let mut task = self.parse_line(current_line, line_num)
            .ok_or_else(|| TasukiError::Parse(format!("Could not parse line {}", line_num)))?;

        if let Some(ref title) = update.title {
            task.title = title.clone();
        }
        if let Some(status) = update.status {
            task.status = status;
        }
        if let Some(ref priority) = update.priority {
            task.priority = *priority;
        }
        if let Some(ref due) = update.due {
            task.due = *due;
        }
        if let Some(ref tags) = update.tags {
            task.tags = tags.clone();
        }

        let mut parts = Vec::new();

        if task.status == TaskStatus::Done {
            parts.push("x".to_string());
            if let Some(completed) = task.completed_at {
                parts.push(completed.date().to_string());
            } else {
                parts.push(chrono::Local::now().date_naive().to_string());
            }
        }

        match task.priority {
            Priority::High => parts.push("(p1)".to_string()),
            Priority::Medium => parts.push("(p2)".to_string()),
            Priority::Low => parts.push("(p3)".to_string()),
            Priority::None => {}
        }

        if let Some(created) = task.created_at {
            parts.push(created.date().to_string());
        }

        parts.push(task.title.clone());

        for tag in &task.tags {
            parts.push(format!("#{}", tag));
        }

        if let Some(due) = task.due {
            parts.push(format!("due:{}", due));
        }

        lines[line_num - 1] = parts.join(" ");

        fs::write(&self.config.path, lines.join("\n") + "\n")?;

        Ok(task)
    }

    async fn complete_task(&self, id: &TaskId) -> Result<()> {
        let update = TaskUpdate {
            status: Some(TaskStatus::Done),
            ..Default::default()
        };
        self.update_task(id, &update).await?;
        Ok(())
    }

    async fn uncomplete_task(&self, id: &TaskId) -> Result<()> {
        let update = TaskUpdate {
            status: Some(TaskStatus::Pending),
            ..Default::default()
        };
        self.update_task(id, &update).await?;
        Ok(())
    }

    async fn delete_task(&self, id: &TaskId) -> Result<()> {
        let line_num: usize = id
            .strip_prefix("local:")
            .ok_or_else(|| TasukiError::Parse(format!("Invalid task ID: {}", id)))?
            .parse()
            .map_err(|_| TasukiError::Parse(format!("Invalid task ID: {}", id)))?;

        if !self.config.path.exists() {
            return Err(TasukiError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "todo.txt not found",
            )));
        }

        let content = fs::read_to_string(&self.config.path)?;
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

        if line_num == 0 || line_num > lines.len() {
            return Err(TasukiError::Parse(format!("Line {} not found", line_num)));
        }

        lines.remove(line_num - 1);
        fs::write(&self.config.path, lines.join("\n") + "\n")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_task() {
        let config = LocalFileConfig {
            path: PathBuf::from("/tmp/test.txt"),
        };
        let backend = LocalFileBackend::new(config);

        let task = backend.parse_line("Buy milk", 1).unwrap();
        assert_eq!(task.title, "Buy milk");
        assert_eq!(task.status, TaskStatus::Pending);
        assert_eq!(task.priority, Priority::None);
    }

    #[test]
    fn test_parse_priority_task() {
        let config = LocalFileConfig {
            path: PathBuf::from("/tmp/test.txt"),
        };
        let backend = LocalFileBackend::new(config);

        let task = backend.parse_line("(p1) Call dentist", 1).unwrap();
        assert_eq!(task.title, "Call dentist");
        assert_eq!(task.priority, Priority::High);
    }

    #[test]
    fn test_parse_done_task() {
        let config = LocalFileConfig {
            path: PathBuf::from("/tmp/test.txt"),
        };
        let backend = LocalFileBackend::new(config);

        let task = backend.parse_line("x 2025-02-20 Buy milk", 1).unwrap();
        assert_eq!(task.title, "Buy milk");
        assert_eq!(task.status, TaskStatus::Done);
    }

    #[test]
    fn test_parse_with_due_date() {
        let config = LocalFileConfig {
            path: PathBuf::from("/tmp/test.txt"),
        };
        let backend = LocalFileBackend::new(config);

        let task = backend.parse_line("Buy groceries due:2025-02-25", 1).unwrap();
        assert_eq!(task.title, "Buy groceries");
        assert_eq!(task.due, Some(NaiveDate::from_ymd_opt(2025, 2, 25).unwrap()));
    }
}
