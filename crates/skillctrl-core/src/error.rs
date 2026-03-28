//! Core error types.

use std::path::PathBuf;
use thiserror::Error;

/// Core error type for skillctrl.
#[derive(Error, Debug)]
pub enum Error {
    /// IO error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error.
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Deserialization error.
    #[error("Deserialization error: {0}")]
    Deserialization(String),

    /// Git error.
    #[error("Git error: {0}")]
    Git(String),

    /// Database error.
    #[error("Database error: {0}")]
    Database(String),

    /// Validation error.
    #[error("Validation error: {0}")]
    Validation(String),

    /// Not found error.
    #[error("Not found: {0}")]
    NotFound(String),

    /// Already exists error.
    #[error("Already exists: {0}")]
    AlreadyExists(String),

    /// Invalid input error.
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Unsupported operation error.
    #[error("Unsupported operation: {0}")]
    Unsupported(String),

    /// Conflict error.
    #[error("Conflict: {0}")]
    Conflict(String),

    /// Adapter error.
    #[error("Adapter error: {endpoint} - {message}")]
    Adapter { endpoint: String, message: String },

    /// Importer error.
    #[error("Importer error: {endpoint} - {message}")]
    Importer { endpoint: String, message: String },

    /// Dependency resolution error.
    #[error("Dependency error: {0}")]
    Dependency(String),

    /// Manifest parse error.
    #[error("Manifest parse error at {path}: {message}")]
    ManifestParse { path: PathBuf, message: String },

    /// Config error.
    #[error("Config error: {0}")]
    Config(String),

    /// Network error.
    #[error("Network error: {0}")]
    Network(String),

    /// Other error.
    #[error("{0}")]
    Other(String),
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::Serialization(e.to_string())
    }
}

impl From<serde_yaml::Error> for Error {
    fn from(e: serde_yaml::Error) -> Self {
        Self::Deserialization(e.to_string())
    }
}

/// Result type alias.
pub type Result<T, E = Error> = std::result::Result<T, E>;
