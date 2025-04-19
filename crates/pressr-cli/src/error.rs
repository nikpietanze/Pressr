use thiserror::Error;
use std::io;

/// Application error types
#[derive(Debug, Error)]
pub enum AppError {
    /// I/O errors (file operations, etc.)
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    
    /// Request errors
    #[error("Request error: {0}")]
    Request(#[from] reqwest::Error),
    
    /// JSON serialization/deserialization errors
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    
    /// Data loading errors
    #[error("Data error: {0}")]
    Data(#[from] DataError),
    
    /// Runner errors
    #[error("Runner error: {0}")]
    Runner(#[from] RunnerError),
    
    /// Generic error with message
    #[error("{0}")]
    Generic(String),
}

/// Errors related to data loading and parsing
#[derive(Debug, Error)]
pub enum DataError {
    /// File read error
    #[error("Failed to read file: {0}")]
    FileRead(#[from] io::Error),
    
    /// JSON parsing error
    #[error("Failed to parse JSON: {0}")]
    JsonParse(#[from] serde_json::Error),

    /// File not found
    #[error("File not found at: {0}")]
    FileNotFound(String),
}

/// Errors related to request execution
#[derive(Debug, Error)]
pub enum RunnerError {
    /// Request execution error
    #[error("Failed to execute request: {0}")]
    RequestFailed(#[from] reqwest::Error),
    
    /// Response body read error
    #[error("Failed to read response body: {0}")]
    ResponseBodyFailed(String),
    
    /// Concurrency error
    #[error("Concurrency error: {0}")]
    ConcurrencyError(String),
}

/// Result type for the application
pub type Result<T> = std::result::Result<T, AppError>;

/// Convert a string error to an AppError
pub fn err_msg<S: Into<String>>(msg: S) -> AppError {
    AppError::Generic(msg.into())
} 