//! Shared AI types used across all providers.
//!
//! This module contains common types for OpenAI-compatible message formats
//! that are used by all AI providers, not just Bedrock.

use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

use crate::s3_helpers::S3Object;

// ============================================================================
// Shared Types for OpenAI-compatible message format
// ============================================================================

/// URL citation annotation for web search results
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct UrlCitation {
    pub start_index: usize,
    pub end_index: usize,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    Text {
        text: String,
    },
    #[serde(rename = "image_url")]
    ImageUrl {
        image_url: ImageUrlData,
    },
    #[serde(rename = "s3_object")]
    S3Object {
        s3_object: S3Object,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ImageUrlData {
    pub url: String, // data:image/png;base64,... or https://...
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum OpenAIContent {
    Text(String),
    Parts(Vec<ContentPart>),
}

#[derive(Serialize, Clone, Debug)]
pub struct ToolDefFunction {
    pub name: String,
    pub description: Option<String>,
    pub parameters: Box<RawValue>,
}

#[derive(Serialize, Clone, Debug)]
pub struct ToolDef {
    pub r#type: String,
    pub function: ToolDefFunction,
}
