use std::collections::HashMap;

use crate::backends::BackendManager;
use crate::config::Config;
use crate::model::{Task, TaskFilter, TaskStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Normal,
    Input,
    Help,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputMode {
    QuickAdd,
    Search,
    EditTask(String), // Stores the task ID being edited
}

#[derive(Debug, Clone)]
pub struct TaskGroup {
    pub label: String,
    pub date: Option<chrono::NaiveDate>,
    pub tasks: Vec<Task>,
    pub collapsed: bool,
}

pub struct App {
    pub mode: AppMode,
    pub tasks: Vec<Task>,
    pub task_groups: Vec<TaskGroup>,
    pub selected_task: usize,
    pub selected_group: usize,
    pub task_filter: TaskFilter,
    pub input_buffer: String,
    pub input_mode: Option<InputMode>,
    pub status_message: Option<(String, StatusLevel)>,
    pub backend_manager: BackendManager,
    pub config: Config,
    pub should_quit: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusLevel {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub enum VisibleItem {
    Group(usize),
    Task(usize, Task),
    None,
}

impl App {
    pub fn new(backend_manager: BackendManager, config: Config) -> Self {
        Self {
            mode: AppMode::Normal,
            tasks: Vec::new(),
            task_groups: Vec::new(),
            selected_task: 0,
            selected_group: 0,
            task_filter: TaskFilter::default(),
            input_buffer: String::new(),
            input_mode: None,
            status_message: None,
            backend_manager,
            config,
            should_quit: false,
        }
    }

    pub fn group_tasks(&mut self) {
        use chrono::Local;

        let today = Local::now().date_naive();
        let mut groups: Vec<TaskGroup> = Vec::new();
        let mut group_map: HashMap<Option<chrono::NaiveDate>, Vec<Task>> = HashMap::new();

        for task in &self.tasks {
            group_map.entry(task.due).or_default().push(task.clone());
        }

        let mut dates: Vec<_> = group_map.keys().copied().collect();
        dates.sort_by(|a, b| match (a, b) {
            (Some(da), Some(db)) => da.cmp(db),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        });

        for date in dates {
            let tasks = group_map.remove(&date).unwrap();
            let label = match date {
                Some(d) if d < today => format!("Overdue - {}", d),
                Some(d) if d == today => "Today".to_string(),
                Some(d) if d == today + chrono::Duration::days(1) => "Tomorrow".to_string(),
                Some(d) => format!("{}", d.format("%A %Y-%m-%d")),
                None => "No due date".to_string(),
            };

            let collapsed = self
                .task_groups
                .iter()
                .find(|g| g.date == date)
                .map(|g| g.collapsed)
                .unwrap_or(false);

            groups.push(TaskGroup {
                label,
                date,
                tasks,
                collapsed,
            });
        }

        self.task_groups = groups;

        if !self.task_groups.is_empty() && self.selected_group >= self.task_groups.len() {
            self.selected_group = self.task_groups.len() - 1;
        }
    }

    pub fn visible_count(&self) -> usize {
        let mut count = self.task_groups.len();
        for group in &self.task_groups {
            if !group.collapsed {
                count += group.tasks.len();
            }
        }
        count
    }

    pub fn get_visible_item(&self, index: usize) -> VisibleItem {
        let mut current = 0;

        for (group_idx, group) in self.task_groups.iter().enumerate() {
            if current == index {
                return VisibleItem::Group(group_idx);
            }
            current += 1;

            if !group.collapsed {
                for task in group.tasks.iter() {
                    if current == index {
                        return VisibleItem::Task(group_idx, task.clone());
                    }
                    current += 1;
                }
            }
        }

        VisibleItem::None
    }

    pub fn toggle_selected_group(&mut self) {
        if let Some(group) = self.task_groups.get_mut(self.selected_group) {
            group.collapsed = !group.collapsed;
        }
    }

    pub fn toggle_all_groups(&mut self) {
        let all_collapsed = self.task_groups.iter().all(|g| g.collapsed);
        for group in &mut self.task_groups {
            group.collapsed = !all_collapsed;
        }
    }

    pub fn get_selected_visible_task(&self) -> Option<Task> {
        match self.get_visible_item(self.selected_task) {
            VisibleItem::Task(_, task) => Some(task),
            _ => None,
        }
    }

    pub fn move_selection_down(&mut self) {
        let visible = self.visible_count();
        if visible > 0 && self.selected_task < visible - 1 {
            self.selected_task += 1;
            self.update_selected_group();
        }
    }

    pub fn move_selection_up(&mut self) {
        if self.selected_task > 0 {
            self.selected_task -= 1;
            self.update_selected_group();
        }
    }

    fn update_selected_group(&mut self) {
        match self.get_visible_item(self.selected_task) {
            VisibleItem::Group(idx) => {
                self.selected_group = idx;
            }
            VisibleItem::Task(group_idx, _) => {
                self.selected_group = group_idx;
            }
            _ => {}
        }
    }

    pub fn move_to_next_group(&mut self) {
        if self.selected_group < self.task_groups.len().saturating_sub(1) {
            self.selected_group += 1;
            self.selected_task = self.find_group_start(self.selected_group);
        }
    }

    pub fn move_to_previous_group(&mut self) {
        if self.selected_group > 0 {
            self.selected_group -= 1;
            self.selected_task = self.find_group_start(self.selected_group);
        }
    }

    fn find_group_start(&self, group_idx: usize) -> usize {
        let mut current = 0;
        for (idx, group) in self.task_groups.iter().enumerate() {
            if idx == group_idx {
                return current;
            }
            current += 1;
            if !group.collapsed {
                current += group.tasks.len();
            }
        }
        current
    }

    pub fn set_status(&mut self, message: impl Into<String>, level: StatusLevel) {
        self.status_message = Some((message.into(), level));
    }

    pub async fn reload_config(&mut self) {
        match Config::load(None) {
            Ok(new_config) => {
                match crate::backends::BackendManager::from_config(&new_config) {
                    Ok(new_manager) => {
                        self.config = new_config;
                        self.backend_manager = new_manager;
                        self.refresh_tasks().await;
                        self.set_status("Config reloaded", StatusLevel::Success);
                    }
                    Err(e) => {
                        self.set_status(format!("Backend error: {}", e), StatusLevel::Error);
                    }
                }
            }
            Err(e) => {
                self.set_status(format!("Config error: {}", e), StatusLevel::Error);
            }
        }
    }

    pub async fn refresh_tasks(&mut self) {
        match self.backend_manager.all_tasks(&self.task_filter).await {
            Ok(tasks) => {
                self.tasks = tasks;
                self.group_tasks();
                let visible = self.visible_count();
                if self.selected_task >= visible && visible > 0 {
                    self.selected_task = visible - 1;
                }
            }
            Err(e) => {
                self.set_status(format!("Error loading tasks: {}", e), StatusLevel::Error);
            }
        }
    }

    pub async fn toggle_selected_task(&mut self) {
        if let Some(task) = self.get_selected_visible_task() {
            let task_id = task.id.clone();
            match task.status {
                TaskStatus::Pending => {
                    if let Err(e) = self.backend_manager.complete_task(&task_id).await {
                        self.set_status(format!("Failed to complete task: {}", e), StatusLevel::Error);
                    } else {
                        self.set_status("Task completed", StatusLevel::Success);
                    }
                }
                TaskStatus::Done => {
                    if let Err(e) = self.backend_manager.uncomplete_task(&task_id).await {
                        self.set_status(format!("Failed to uncomplete task: {}", e), StatusLevel::Error);
                    } else {
                        self.set_status("Task marked as pending", StatusLevel::Success);
                    }
                }
            }
            self.refresh_tasks().await;
        }
    }

    pub async fn delete_selected_task(&mut self) {
        if let Some(task) = self.get_selected_visible_task() {
            let task_id = task.id.clone();
            if let Err(e) = self.backend_manager.delete_task(&task_id).await {
                self.set_status(format!("Failed to delete task: {}", e), StatusLevel::Error);
            } else {
                self.set_status("Task deleted", StatusLevel::Success);
            }
            self.refresh_tasks().await;
        }
    }

        pub fn edit_selected_task(&mut self) {
        use crate::model::Priority;
        
        if let Some(task) = self.get_selected_visible_task() {
            let mut parts = vec![task.title.clone()];
            
            match task.priority {
                Priority::High => parts.push("(p1)".to_string()),
                Priority::Medium => parts.push("(p2)".to_string()),
                Priority::Low => parts.push("(p3)".to_string()),
                Priority::None => {}
            }
            
            if let Some(due) = task.due {
                parts.push(due.to_string());
            }
            
            for tag in &task.tags {
                parts.push(format!("#{}", tag));
            }

            let edit_text = parts.join(" ");
            
            self.mode = AppMode::Input;
            self.input_mode = Some(InputMode::EditTask(task.id.clone()));
            self.input_buffer = edit_text;
        }
    }

    pub fn start_quick_add(&mut self) {
        self.mode = AppMode::Input;
        self.input_mode = Some(InputMode::QuickAdd);
        self.input_buffer.clear();
    }

    pub fn start_search(&mut self) {
        self.mode = AppMode::Input;
        self.input_mode = Some(InputMode::Search);
        self.input_buffer.clear();
    }

    pub fn cancel_input(&mut self) {
        self.mode = AppMode::Normal;
        self.input_mode = None;
        self.input_buffer.clear();
    }

    pub async fn submit_input(&mut self) {
        if let Some(ref input_mode) = self.input_mode {
            match input_mode {
                InputMode::QuickAdd => {
                    if !self.input_buffer.is_empty() {
                        use crate::nlp::parse_quick_add;
                        use crate::model::NewTask;
                        
                        match parse_quick_add(&self.input_buffer, &self.backend_manager) {
                            Ok((title, priority, due, tags, backend)) => {
                                let new_task = NewTask {
                                    title,
                                    priority,
                                    due,
                                    tags,
                                    backend,
                                };
                                
                                match self.backend_manager.create_task(&new_task).await {
                                    Ok(task) => {
                                        self.set_status(format!("Created: {}", task.title), StatusLevel::Success);
                                    }
                                    Err(e) => {
                                        self.set_status(format!("Failed to create task: {}", e), StatusLevel::Error);
                                    }
                                }
                            }
                            Err(e) => {
                                self.set_status(format!("Parse error: {}", e), StatusLevel::Error);
                            }
                        }
                        self.refresh_tasks().await;
                    }
                }
                InputMode::Search => {
                    self.task_filter.search = if self.input_buffer.is_empty() {
                        None
                    } else {
                        Some(self.input_buffer.clone())
                    };
                    self.refresh_tasks().await;
                }
                InputMode::EditTask(task_id) => {
                    let task_id = task_id.clone();
                    if !self.input_buffer.is_empty() {
                        use crate::nlp::parse_quick_add;
                        use crate::model::TaskUpdate;
                        
                        match parse_quick_add(&self.input_buffer, &self.backend_manager) {
                            Ok((title, priority, due, tags, _)) => {
                                let update = TaskUpdate {
                                    title: Some(title),
                                    status: None,
                                    priority: Some(priority),
                                    due: Some(due),
                                    tags: Some(tags),
                                };
                                
                                match self.backend_manager.update_task(&task_id, &update).await {
                                    Ok(task) => {
                                        self.set_status(format!("Updated: {}", task.title), StatusLevel::Success);
                                    }
                                    Err(e) => {
                                        self.set_status(format!("Failed to update task: {}", e), StatusLevel::Error);
                                    }
                                }
                            }
                            Err(e) => {
                                self.set_status(format!("Parse error: {}", e), StatusLevel::Error);
                            }
                        }
                        self.refresh_tasks().await;
                    }
                }
            }
        }
        self.mode = AppMode::Normal;
        self.input_mode = None;
        self.input_buffer.clear();
    }

    pub fn toggle_help(&mut self) {
        if self.mode == AppMode::Help {
            self.mode = AppMode::Normal;
        } else {
            self.mode = AppMode::Help;
        }
    }
}
