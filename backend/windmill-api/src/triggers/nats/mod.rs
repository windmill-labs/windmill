use serde::{Deserialize, Serialize};
use sqlx::FromRow;

mod handler_ee;
pub mod handler_oss;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct NatsConfig {
    pub nats_resource_path: String,
    pub subjects: Vec<String>,
    pub stream_name: Option<String>,
    pub consumer_name: Option<String>,
    pub use_jetstream: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewNatsConfig {
    pub nats_resource_path: String,
    pub subjects: Vec<String>,
    pub stream_name: Option<String>,
    pub consumer_name: Option<String>,
    pub use_jetstream: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditNatsConfig {
    pub nats_resource_path: String,
    pub subjects: Vec<String>,
    pub stream_name: Option<String>,
    pub consumer_name: Option<String>,
    pub use_jetstream: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestNatsConfig {
    pub nats_resource_path: String,
}

// Utility functions
pub fn validate_nats_resource_path(path: &str) -> windmill_common::error::Result<()> {
    if path.trim().is_empty() {
        return Err(windmill_common::error::Error::BadRequest(
            "NATS resource path cannot be empty".to_string(),
        ));
    }
    Ok(())
}

pub fn validate_subjects(subjects: &[String]) -> windmill_common::error::Result<()> {
    if subjects.is_empty() {
        return Err(windmill_common::error::Error::BadRequest(
            "Subjects cannot be empty".to_string(),
        ));
    }

    for subject in subjects {
        if subject.trim().is_empty() {
            return Err(windmill_common::error::Error::BadRequest(
                "Subject names cannot be empty".to_string(),
            ));
        }
    }
    Ok(())
}
