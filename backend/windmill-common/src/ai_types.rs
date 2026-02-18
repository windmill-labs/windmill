//! Shared AI types used across all providers.
//!
//! This module contains common types for OpenAI-compatible message formats
//! that are used by all AI providers, not just Bedrock.

use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

use crate::flow_status::AgentAction;
use windmill_types::s3::S3Object;

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

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct OpenAIFunction {
    pub name: String,
    pub arguments: String,
}

/// Google-specific extra content for thought signatures (Gemini 3 Pro / 2.5)
#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct GoogleExtraContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thought_signature: Option<String>,
}

/// Extra content for provider-specific metadata (e.g., Google thought signatures)
#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct ExtraContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub google: Option<GoogleExtraContent>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct OpenAIToolCall {
    pub id: String,
    pub function: OpenAIFunction,
    pub r#type: String,
    /// Extra content for provider-specific metadata (e.g., Google Gemini thought signatures)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_content: Option<ExtraContent>,
}

/// OpenAI-compatible message format used across all AI providers.
///
/// The `agent_action` field is used by the worker for flow-specific tracking
/// and is never serialized to JSON (skip_serializing, default).
#[derive(Deserialize, Serialize, Clone, Default, Debug)]
pub struct OpenAIMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<OpenAIContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<OpenAIToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// Worker-specific field for tracking agent actions in flows.
    /// Never serialized; defaults to None when deserializing.
    #[serde(skip_serializing, default)]
    pub agent_action: Option<AgentAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Vec<UrlCitation>>,
}
