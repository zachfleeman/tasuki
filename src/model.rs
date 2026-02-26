use chrono::{NaiveDate, NaiveDateTime};
use serde::Serialize;

pub type TaskId = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum TaskStatus {
    Pending,
    Done,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum Priority {
    None = 0,
    Low = 1,
    Medium = 2,
    High = 3,
}

#[derive(Debug, Clone, Serialize)]
pub struct Task {
    pub id: TaskId,
    pub title: String,
    pub status: TaskStatus,
    pub priority: Priority,
    pub due: Option<NaiveDate>,
    pub tags: Vec<String>,
    pub source: BackendSource,
    pub source_line: Option<usize>,
    pub source_path: Option<String>,
    pub created_at: Option<NaiveDateTime>,
    pub completed_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum BackendSource {
    Obsidian,
    LocalFile,
}

impl BackendSource {
    pub fn name(&self) -> &str {
        match self {
            Self::Obsidian => "obsidian",
            Self::LocalFile => "local",
        }
    }

    pub fn icon(&self) -> &str {
        match self {
            Self::Obsidian => "◆",
            Self::LocalFile => "■",
        }
    }
}

pub struct NewTask {
    pub title: String,
    pub priority: Priority,
    pub due: Option<NaiveDate>,
    pub tags: Vec<String>,
    pub backend: BackendSource,
}

#[derive(Debug, Clone, Default)]
pub struct TaskUpdate {
    pub title: Option<String>,
    pub status: Option<TaskStatus>,
    pub priority: Option<Priority>,
    pub due: Option<Option<NaiveDate>>,
    pub tags: Option<Vec<String>>,
}

#[derive(Default)]
pub struct TaskFilter {
    pub status: Option<TaskStatus>,
    pub due_before: Option<NaiveDate>,
    pub due_after: Option<NaiveDate>,
    pub search: Option<String>,
}
