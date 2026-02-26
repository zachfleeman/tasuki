use thiserror::Error;

#[derive(Debug, Error)]
pub enum TasukiError {
    #[error("Config error: {0}")]
    Config(String),

    #[error("Backend '{backend}' error: {message}")]
    Backend { backend: String, message: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("JSON error: {0}")]
    Json(String),

    #[error("Watch error: {0}")]
    Watch(String),
}

impl From<notify::Error> for TasukiError {
    fn from(e: notify::Error) -> Self {
        TasukiError::Watch(e.to_string())
    }
}

impl From<serde_json::Error> for TasukiError {
    fn from(e: serde_json::Error) -> Self {
        TasukiError::Json(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, TasukiError>;
