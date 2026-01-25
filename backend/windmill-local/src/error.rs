//! Error types for local mode

use thiserror::Error;

#[derive(Error, Debug)]
pub enum LocalError {
    #[error("Database error: {0}")]
    Database(#[from] libsql::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Job not found: {0}")]
    JobNotFound(uuid::Uuid),

    #[error("Invalid job state: {0}")]
    InvalidJobState(String),

    #[error("Queue is empty")]
    QueueEmpty,

    #[error("Execution error: {0}")]
    Execution(String),

    #[error("Timeout")]
    Timeout,
}

pub type Result<T> = std::result::Result<T, LocalError>;
