use thiserror::Error;
use std::path::PathBuf;

/// Result type for pressr-core
pub type Result<T> = std::result::Result<T, Error>;

/// Error type for pressr-core
#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("HTTP client error: {0}")]
    HttpClient(#[from] reqwest::Error),

    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Failed to load data file '{path}': {source}")]
    DataLoad {
        path: PathBuf,
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("Failed to execute request: {source}")]
    RequestExecution {
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("Missing required data: {0}")]
    MissingData(String),

    #[error("{0}")]
    Other(String),
} 