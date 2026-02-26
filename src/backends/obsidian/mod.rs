use async_trait::async_trait;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

mod parser;
use crate::backends::TaskBackend;
use crate::error::{Result, TasukiError};
use crate::model::{
    BackendSource, NewTask, Priority, Task, TaskFilter, TaskId, TaskStatus, TaskUpdate,
};

pub struct ObsidianConfig {
    pub vault_path: PathBuf,
    pub folders: Option<Vec<String>>,
    pub ignore_folders: Vec<String>,
    pub inbox_file: String,
}

impl ObsidianConfig {
    pub fn from_table(table: &toml::Table) -> Result<Self> {
        let vault_path = table
            .get("vault_path")
            .and_then(|v| v.as_str())
            .map(|s| shellexpand::tilde(s).into_owned())
            .map(PathBuf::from)
            .ok_or_else(|| TasukiError::Config("obsidian.vault_path is required".into()))?;

        let folders = table.get("folders").and_then(|v| {
            v.as_array().map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
        });

        let ignore_folders = table
            .get("ignore_folders")
            .and_then(|v| {
                v.as_array().map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
            })
            .unwrap_or_else(|| {
                vec![
                    ".obsidian".to_string(),
                    ".trash".to_string(),
                    ".git".to_string(),
                ]
            });

        let inbox_file = table
            .get("inbox_file")
            .and_then(|v| v.as_str())
            .unwrap_or("Inbox.md")
            .to_string();

        Ok(Self {
            vault_path,
            folders,
            ignore_folders,
            inbox_file,
        })
    }

    pub fn is_obsidian_vault(&self) -> bool {
        self.vault_path.join(".obsidian").exists()
    }

    pub fn obsidian_app_installed() -> bool {
        if std::process::Command::new("which")
            .arg("obsidian")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return true;
        }

        let desktop_dirs = [
            dirs::home_dir().map(|h| h.join(".local/share/applications")),
            Some(PathBuf::from("/usr/share/applications")),
        ];

        for dir in desktop_dirs.iter().flatten() {
            if dir.join("obsidian.desktop").exists() {
                return true;
            }
        }

        false
    }

    pub fn vault_name(&self) -> String {
        self.vault_path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "vault".to_string())
    }
}

pub struct ObsidianBackend {
    config: ObsidianConfig,
}

impl ObsidianBackend {
    pub fn new(config: ObsidianConfig) -> Self {
        Self { config }
    }

    fn markdown_files(&self) -> Vec<PathBuf> {
        let mut files = Vec::new();

        let walker = WalkDir::new(&self.config.vault_path)
            .follow_links(true)
            .into_iter()
            .filter_entry(|entry| {
                let name = entry.file_name().to_string_lossy();

                if name.starts_with('.') && entry.depth() > 0 {
                    return false;
                }

                if entry.file_type().is_dir() {
                    return !self.config.ignore_folders.iter().any(|f| name == *f);
                }

                true
            });

        for entry in walker.filter_map(|e| e.ok()) {
            let path = entry.path();

            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }

            if let Some(ref folders) = self.config.folders {
                let rel_path = path
                    .strip_prefix(&self.config.vault_path)
                    .unwrap_or(path);

                let in_allowed_folder = folders.iter().any(|folder| {
                    rel_path.starts_with(folder)
                });

                if !in_allowed_folder {
                    continue;
                }
            }

            files.push(path.to_path_buf());
        }

        files
    }

    fn parse_file_tasks(&self, path: &Path) -> Result<Vec<Task>> {
        let content = fs::read_to_string(path).map_err(|e| TasukiError::Backend {
            backend: "obsidian".to_string(),
            message: format!("Failed to read {}: {}", path.display(), e),
        })?;

        let rel_path = path
            .strip_prefix(&self.config.vault_path)
            .unwrap_or(path)
            .to_string_lossy()
            .into_owned();

        let parsed = parser::parse_file(&content);

        let tasks = parsed
            .into_iter()
            .map(|(line_num, parsed)| Task {
                id: format!("obsidian:{}:{}", rel_path, line_num),
                title: parsed.title,
                status: parsed.status,
                priority: parsed.priority,
                due: parsed.due,
                tags: parsed.tags,
                source: BackendSource::Obsidian,
                source_line: Some(line_num),
                source_path: Some(path.to_string_lossy().into_owned()),
                created_at: parsed.created_at.map(|d| d.and_hms_opt(0, 0, 0).unwrap()),
                completed_at: parsed
                    .completed_at
                    .map(|d| d.and_hms_opt(0, 0, 0).unwrap()),
            })
            .collect();

        Ok(tasks)
    }
    
    // Use 1-indexing for lines
    fn modify_line<F>(&self, path: &str, line_num: usize, modify: F) -> Result<()>
    where
        F: FnOnce(&str) -> String,
    {
        let content = fs::read_to_string(path).map_err(|e| TasukiError::Backend {
            backend: "obsidian".to_string(),
            message: format!("Failed to read {}: {}", path, e),
        })?;

        let mut lines: Vec<String> = content.lines().map(String::from).collect();

        let idx = line_num.checked_sub(1).ok_or_else(|| TasukiError::Backend {
            backend: "obsidian".to_string(),
            message: format!("Invalid line number: {}", line_num),
        })?;

        if idx >= lines.len() {
            return Err(TasukiError::Backend {
                backend: "obsidian".to_string(),
                message: format!(
                    "Line {} out of range (file has {} lines)",
                    line_num,
                    lines.len()
                ),
            });
        }

        lines[idx] = modify(&lines[idx]);

        // Preserve trailing newline if original had one
        let mut output = lines.join("\n");
        if content.ends_with('\n') {
            output.push('\n');
        }

        fs::write(path, output).map_err(|e| TasukiError::Backend {
            backend: "obsidian".to_string(),
            message: format!("Failed to write {}: {}", path, e),
        })?;

        Ok(())
    }

    // ID format: obsidian:{relative_path}:{line_number}
    fn parse_task_id(id: &TaskId) -> Result<(String, usize)> {
        let rest = id.strip_prefix("obsidian:").ok_or_else(|| {
            TasukiError::Parse(format!("Invalid Obsidian task ID: {}", id))
        })?;

        let last_colon = rest.rfind(':').ok_or_else(|| {
            TasukiError::Parse(format!("Invalid Obsidian task ID format: {}", id))
        })?;

        let rel_path = &rest[..last_colon];
        let line_num: usize = rest[last_colon + 1..]
            .parse()
            .map_err(|_| TasukiError::Parse(format!("Invalid line number in task ID: {}", id)))?;

        Ok((rel_path.to_string(), line_num))
    }

    fn resolve_path(&self, rel_path: &str) -> PathBuf {
        self.config.vault_path.join(rel_path)
    }

    pub fn open_command(&self, task: &Task) -> Option<Vec<String>> {
        let source_path = task.source_path.as_ref()?;
        let line_num = task.source_line.unwrap_or(1);

        // Try Obsidian app first
        if self.config.is_obsidian_vault() && ObsidianConfig::obsidian_app_installed() {
            let rel_path = Path::new(source_path)
                .strip_prefix(&self.config.vault_path)
                .ok()?
                .with_extension("")
                .to_string_lossy()
                .into_owned();

            let vault_name = self.config.vault_name();
            let uri = format!(
                "obsidian://open?vault={}&file={}",
                urlencoding_simple(&vault_name),
                urlencoding_simple(&rel_path),
            );

            return Some(vec!["xdg-open".to_string(), uri]);
        }

        // Fall back to $EDITOR
        if let Ok(editor) = std::env::var("EDITOR") {
            return Some(vec![
                editor,
                format!("+{}", line_num),
                source_path.clone(),
            ]);
        }

        None
    }
}

fn urlencoding_simple(s: &str) -> String {
    s.replace(' ', "%20")
        .replace('/', "%2F")
        .replace('#', "%23")
}

#[async_trait]
impl TaskBackend for ObsidianBackend {
    fn name(&self) -> &str {
        "Obsidian"
    }

    fn source(&self) -> BackendSource {
        BackendSource::Obsidian
    }

    async fn fetch_tasks(&self, filter: &TaskFilter) -> Result<Vec<Task>> {
        let files = self.markdown_files();
        let mut all_tasks = Vec::new();

        for file in files {
            match self.parse_file_tasks(&file) {
                Ok(tasks) => all_tasks.extend(tasks),
                Err(e) => {
                    tracing::warn!("Failed to parse {}: {}", file.display(), e);
                }
            }
        }

        let filtered: Vec<Task> = all_tasks
            .into_iter()
            .filter(|task| {
                if let Some(ref status) = filter.status {
                    if task.status != *status {
                        return false;
                    }
                }
                if let Some(ref due_before) = filter.due_before {
                    match task.due {
                        Some(d) if d > *due_before => return false,
                        _ => {}
                    }
                }
                if let Some(ref due_after) = filter.due_after {
                    match task.due {
                        Some(d) if d < *due_after => return false,
                        None => return false,
                        _ => {}
                    }
                }
                if let Some(ref search) = filter.search {
                    let search_lower = search.to_lowercase();
                    if !task.title.to_lowercase().contains(&search_lower) {
                        return false;
                    }
                }
                true
            })
            .collect();

        Ok(filtered)
    }

    async fn create_task(&self, task: &NewTask) -> Result<Task> {
        let inbox_path = self.config.vault_path.join(&self.config.inbox_file);

        let mut line = format!("- [ ] {}", task.title);

        // Priority
        match task.priority {
            Priority::High => line.push_str(" ⏫"),
            Priority::Medium => line.push_str(" 🔼"),
            Priority::Low => line.push_str(" 🔽"),
            Priority::None => {}
        }

        // Due date
        if let Some(due) = task.due {
            line.push_str(&format!(" 📅 {}", due.format("%Y-%m-%d")));
        }

        // Tags
        for tag in &task.tags {
            line.push_str(&format!(" #{}", tag));
        }

        if !inbox_path.exists() {
            fs::write(&inbox_path, "").map_err(|e| TasukiError::Backend {
                backend: "obsidian".to_string(),
                message: format!("Failed to create inbox file: {}", e),
            })?;
        }

        let mut content = fs::read_to_string(&inbox_path).map_err(|e| TasukiError::Backend {
            backend: "obsidian".to_string(),
            message: format!("Failed to read inbox file: {}", e),
        })?;

        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(&line);
        content.push('\n');

        let line_count = content.lines().count();

        fs::write(&inbox_path, &content).map_err(|e| TasukiError::Backend {
            backend: "obsidian".to_string(),
            message: format!("Failed to write inbox file: {}", e),
        })?;

        let rel_path = self.config.inbox_file.clone();

        Ok(Task {
            id: format!("obsidian:{}:{}", rel_path, line_count),
            title: task.title.clone(),
            status: TaskStatus::Pending,
            priority: task.priority,
            due: task.due,
            tags: task.tags.clone(),
            source: BackendSource::Obsidian,
            source_line: Some(line_count),
            source_path: Some(inbox_path.to_string_lossy().into_owned()),
            created_at: None,
            completed_at: None,
        })
    }

    async fn update_task(&self, id: &TaskId, update: &TaskUpdate) -> Result<Task> {
        let (rel_path, line_num) = Self::parse_task_id(id)?;
        let abs_path = self.resolve_path(&rel_path);
        let abs_path_str = abs_path.to_string_lossy().into_owned();

        let content =
            fs::read_to_string(&abs_path).map_err(|e| TasukiError::Backend {
                backend: "obsidian".to_string(),
                message: format!("Failed to read {}: {}", abs_path.display(), e),
            })?;

        let lines: Vec<&str> = content.lines().collect();
        let idx = line_num.checked_sub(1).ok_or_else(|| TasukiError::Backend {
            backend: "obsidian".to_string(),
            message: format!("Invalid line number: {}", line_num),
        })?;

        if idx >= lines.len() {
            return Err(TasukiError::Backend {
                backend: "obsidian".to_string(),
                message: format!("Line {} out of range", line_num),
            });
        }

        let current = parser::parse_checkbox_line(lines[idx]).ok_or_else(|| {
            TasukiError::Backend {
                backend: "obsidian".to_string(),
                message: format!("Line {} is not a checkbox", line_num),
            }
        })?;

        let title = update.title.clone().unwrap_or(current.title);
        let status = update.status.clone().unwrap_or(current.status);
        let priority = update.priority.unwrap_or(current.priority);
        let due = match &update.due {
            Some(d) => *d,
            None => current.due,
        };
        let tags = update.tags.clone().unwrap_or(current.tags);

        let checkbox = match status {
            TaskStatus::Pending => "- [ ]",
            TaskStatus::Done => "- [x]",
        };

        let mut new_line = format!("{} {}", checkbox, title);

        match priority {
            Priority::High => new_line.push_str(" ⏫"),
            Priority::Medium => new_line.push_str(" 🔼"),
            Priority::Low => new_line.push_str(" 🔽"),
            Priority::None => {}
        }

        if let Some(due) = due {
            new_line.push_str(&format!(" 📅 {}", due.format("%Y-%m-%d")));
        }

        for tag in &tags {
            new_line.push_str(&format!(" #{}", tag));
        }

        self.modify_line(&abs_path_str, line_num, |_| new_line.clone())?;

        Ok(Task {
            id: id.clone(),
            title,
            status,
            priority,
            due,
            tags,
            source: BackendSource::Obsidian,
            source_line: Some(line_num),
            source_path: Some(abs_path_str),
            created_at: None,
            completed_at: None,
        })
    }

    async fn complete_task(&self, id: &TaskId) -> Result<()> {
        let (rel_path, line_num) = Self::parse_task_id(id)?;
        let abs_path = self.resolve_path(&rel_path);
        let abs_path_str = abs_path.to_string_lossy().into_owned();

        self.modify_line(&abs_path_str, line_num, |line| {
            line.replacen("- [ ]", "- [x]", 1)
        })?;

        Ok(())
    }

    async fn uncomplete_task(&self, id: &TaskId) -> Result<()> {
        let (rel_path, line_num) = Self::parse_task_id(id)?;
        let abs_path = self.resolve_path(&rel_path);
        let abs_path_str = abs_path.to_string_lossy().into_owned();

        self.modify_line(&abs_path_str, line_num, |line| {
            line.replacen("- [x]", "- [ ]", 1)
                .replacen("- [X]", "- [ ]", 1)
        })?;

        Ok(())
    }

    async fn delete_task(&self, id: &TaskId) -> Result<()> {
        let (rel_path, line_num) = Self::parse_task_id(id)?;
        let abs_path = self.resolve_path(&rel_path);

        let content =
            fs::read_to_string(&abs_path).map_err(|e| TasukiError::Backend {
                backend: "obsidian".to_string(),
                message: format!("Failed to read {}: {}", abs_path.display(), e),
            })?;

        let mut lines: Vec<&str> = content.lines().collect();

        let idx = line_num.checked_sub(1).ok_or_else(|| TasukiError::Backend {
            backend: "obsidian".to_string(),
            message: format!("Invalid line number: {}", line_num),
        })?;

        if idx >= lines.len() {
            return Err(TasukiError::Backend {
                backend: "obsidian".to_string(),
                message: format!("Line {} out of range", line_num),
            });
        }

        lines.remove(idx);

        let mut output = lines.join("\n");
        if content.ends_with('\n') && !output.is_empty() {
            output.push('\n');
        }

        fs::write(&abs_path, output).map_err(|e| TasukiError::Backend {
            backend: "obsidian".to_string(),
            message: format!("Failed to write {}: {}", abs_path.display(), e),
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_vault() -> (TempDir, ObsidianConfig) {
        let dir = TempDir::new().unwrap();
        let vault_path = dir.path().to_path_buf();

        fs::create_dir_all(vault_path.join(".obsidian")).unwrap();

        fs::create_dir_all(vault_path.join("Daily Notes")).unwrap();
        fs::write(
            vault_path.join("Daily Notes/2025-02-25.md"),
            "\
# February 25, 2025

- [ ] Call dentist
- [ ] Buy groceries due:2025-02-26
- [x] Morning workout
",
        )
        .unwrap();

        fs::write(vault_path.join("Inbox.md"), "").unwrap();

        fs::write(
            vault_path.join("code-examples.md"),
            "\
# Code Examples

- [ ] Real task above code block

```markdown
- [ ] Not a real task
- [x] Also not real
```

- [ ] Real task below code block
",
        )
        .unwrap();

        let config = ObsidianConfig {
            vault_path,
            folders: None,
            ignore_folders: vec![
                ".obsidian".to_string(),
                ".trash".to_string(),
                ".git".to_string(),
            ],
            inbox_file: "Inbox.md".to_string(),
        };

        (dir, config)
    }

    #[tokio::test]
    async fn test_fetch_all_tasks() {
        let (_dir, config) = create_test_vault();
        let backend = ObsidianBackend::new(config);

        let filter = TaskFilter::default();
        let tasks = backend.fetch_tasks(&filter).await.unwrap();
        assert_eq!(tasks.len(), 5);
    }

    #[tokio::test]
    async fn test_fetch_pending_only() {
        let (_dir, config) = create_test_vault();
        let backend = ObsidianBackend::new(config);

        let filter = TaskFilter {
            status: Some(TaskStatus::Pending),
            ..Default::default()
        };
        let tasks = backend.fetch_tasks(&filter).await.unwrap();
        assert!(tasks.iter().all(|t| t.status == TaskStatus::Pending));
        assert_eq!(tasks.len(), 4);
    }

    #[tokio::test]
    async fn test_task_ids() {
        let (_dir, config) = create_test_vault();
        let backend = ObsidianBackend::new(config);

        let filter = TaskFilter::default();
        let tasks = backend.fetch_tasks(&filter).await.unwrap();

        for task in &tasks {
            assert!(task.id.starts_with("obsidian:"));
            let parts: Vec<&str> = task.id.splitn(2, ':').collect();
            assert_eq!(parts[0], "obsidian");
            assert!(parts[1].contains(':'));
        }
    }

    #[tokio::test]
    async fn test_complete_task() {
        let (_dir, config) = create_test_vault();
        let vault_path = config.vault_path.clone();
        let backend = ObsidianBackend::new(config);

        let filter = TaskFilter {
            status: Some(TaskStatus::Pending),
            ..Default::default()
        };
        let tasks = backend.fetch_tasks(&filter).await.unwrap();
        let task = tasks
            .iter()
            .find(|t| t.title == "Call dentist")
            .expect("Should find 'Call dentist' task");

        backend.complete_task(&task.id).await.unwrap();

        let content =
            fs::read_to_string(vault_path.join("Daily Notes/2025-02-25.md")).unwrap();
        assert!(content.contains("- [x] Call dentist"));
    }

    #[tokio::test]
    async fn test_uncomplete_task() {
        let (_dir, config) = create_test_vault();
        let vault_path = config.vault_path.clone();
        let backend = ObsidianBackend::new(config);

        let filter = TaskFilter {
            status: Some(TaskStatus::Done),
            ..Default::default()
        };
        let tasks = backend.fetch_tasks(&filter).await.unwrap();
        let task = tasks
            .iter()
            .find(|t| t.title == "Morning workout")
            .expect("Should find 'Morning workout' task");

        backend.uncomplete_task(&task.id).await.unwrap();

        let content =
            fs::read_to_string(vault_path.join("Daily Notes/2025-02-25.md")).unwrap();
        assert!(content.contains("- [ ] Morning workout"));
    }

    #[tokio::test]
    async fn test_create_task() {
        let (_dir, config) = create_test_vault();
        let vault_path = config.vault_path.clone();
        let backend = ObsidianBackend::new(config);

        let new_task = NewTask {
            title: "New task from tasuki".to_string(),
            priority: Priority::High,
            due: Some(chrono::NaiveDate::from_ymd_opt(2025, 4, 1).unwrap()),
            tags: vec!["work".to_string()],
            backend: BackendSource::Obsidian,
        };

        let task = backend.create_task(&new_task).await.unwrap();
        assert_eq!(task.title, "New task from tasuki");
        assert_eq!(task.source, BackendSource::Obsidian);

        let content = fs::read_to_string(vault_path.join("Inbox.md")).unwrap();
        assert!(content.contains("- [ ] New task from tasuki ⏫ 📅 2025-04-01 #work"));
    }

    #[tokio::test]
    async fn test_delete_task() {
        let (_dir, config) = create_test_vault();
        let vault_path = config.vault_path.clone();
        let backend = ObsidianBackend::new(config);

        let filter = TaskFilter::default();
        let tasks = backend.fetch_tasks(&filter).await.unwrap();
        let task = tasks
            .iter()
            .find(|t| t.title == "Call dentist")
            .expect("Should find 'Call dentist' task");

        let task_id = task.id.clone();
        backend.delete_task(&task_id).await.unwrap();

        let content =
            fs::read_to_string(vault_path.join("Daily Notes/2025-02-25.md")).unwrap();
        assert!(!content.contains("Call dentist"));
    }

    #[tokio::test]
    async fn test_ignores_obsidian_folder() {
        let (_dir, config) = create_test_vault();
        let vault_path = config.vault_path.clone();

        fs::write(
            vault_path.join(".obsidian/workspace.md"),
            "- [ ] Should not appear",
        )
        .unwrap();

        let backend = ObsidianBackend::new(config);
        let filter = TaskFilter::default();
        let tasks = backend.fetch_tasks(&filter).await.unwrap();
        assert!(!tasks.iter().any(|t| t.title == "Should not appear"));
    }

    #[tokio::test]
    async fn test_code_blocks_skipped() {
        let (_dir, config) = create_test_vault();
        let backend = ObsidianBackend::new(config);

        let filter = TaskFilter::default();
        let tasks = backend.fetch_tasks(&filter).await.unwrap();

        assert!(!tasks.iter().any(|t| t.title == "Not a real task"));
        assert!(!tasks.iter().any(|t| t.title == "Also not real"));
        assert!(tasks
            .iter()
            .any(|t| t.title == "Real task above code block"));
        assert!(tasks
            .iter()
            .any(|t| t.title == "Real task below code block"));
    }

    #[test]
    fn test_parse_task_id() {
        let (path, line) =
            ObsidianBackend::parse_task_id(&"obsidian:Daily Notes/2025-02-25.md:3".to_string())
                .unwrap();
        assert_eq!(path, "Daily Notes/2025-02-25.md");
        assert_eq!(line, 3);
    }

    #[test]
    fn test_is_obsidian_vault() {
        let dir = TempDir::new().unwrap();
        let config = ObsidianConfig {
            vault_path: dir.path().to_path_buf(),
            folders: None,
            ignore_folders: vec![],
            inbox_file: "Inbox.md".to_string(),
        };
        assert!(!config.is_obsidian_vault());

        fs::create_dir_all(dir.path().join(".obsidian")).unwrap();
        assert!(config.is_obsidian_vault());
    }
}
