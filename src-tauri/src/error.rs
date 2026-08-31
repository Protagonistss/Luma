use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, Serialize, Clone, PartialEq, Eq)]
#[serde(tag = "code", content = "message")]
pub enum AppError {
    #[error("network error: {0}")]
    Network(String),
    #[error("invalid playlist: {0}")]
    InvalidPlaylist(String),
    #[error("file error: {0}")]
    File(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("storage error: {0}")]
    Storage(String),
    #[error("playback error: {0}")]
    Playback(String),
}

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Network(_) => "NETWORK",
            Self::InvalidPlaylist(_) => "INVALID_PLAYLIST",
            Self::File(_) => "FILE",
            Self::NotFound(_) => "NOT_FOUND",
            Self::Storage(_) => "STORAGE",
            Self::Playback(_) => "PLAYBACK",
        }
    }
}

pub type AppResult<T> = Result<T, AppError>;

impl From<reqwest::Error> for AppError {
    fn from(value: reqwest::Error) -> Self {
        Self::Network(value.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        Self::File(value.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(value: serde_json::Error) -> Self {
        Self::Storage(value.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandError {
    pub code: String,
    pub message: String,
}

impl From<AppError> for CommandError {
    fn from(value: AppError) -> Self {
        Self {
            code: value.code().to_string(),
            message: value.to_string(),
        }
    }
}
