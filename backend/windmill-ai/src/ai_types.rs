//! Shared AI types used across all providers.
//!
//! This module contains common types for OpenAI-compatible message formats
//! that are used by all AI providers, not just Bedrock.

use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

use windmill_common::flow_status::AgentAction;
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
    /// File content block for OpenAI Chat Completions format (PDFs, etc.)
    #[serde(rename = "file")]
    File {
        file: FileData,
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
pub struct FileData {
    pub filename: String,
    pub file_data: String, // data:application/pdf;base64,...
}

/// Check if a MIME type represents a document (as opposed to an image).
pub fn is_document_mime(mime_type: &str) -> bool {
    matches!(
        mime_type,
        "application/pdf"
            | "text/csv"
            | "text/html"
            | "text/plain"
            | "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
            | "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
    )
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

/// Bedrock-specific extra content carrying the Claude reasoning block emitted
/// in the same assistant turn as a tool call. Anthropic requires reasoning
/// blocks (text + signature, unmodified) to be replayed before `toolUse` when
/// thinking is enabled, so the proxy round-trips them through the
/// OpenAI-shaped tool call.
#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct BedrockExtraContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    /// Base64 of a redacted (encrypted) reasoning block, when the provider
    /// returned one instead of readable text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redacted_content: Option<String>,
}

/// Native-Anthropic reasoning block emitted in the same assistant turn as a
/// tool call. Like Bedrock, Anthropic requires the thinking block (text +
/// unmodified signature, or redacted bytes) to precede `tool_use` on replay when
/// thinking is enabled, so it is round-tripped through the OpenAI-shaped tool call.
#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct AnthropicExtraContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    /// Base64 `data` of a redacted (encrypted) thinking block, when the provider
    /// returned one instead of readable text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redacted_thinking: Option<String>,
}

/// Extra content for provider-specific metadata (e.g., Google thought signatures)
#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct ExtraContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub google: Option<GoogleExtraContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bedrock: Option<BedrockExtraContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anthropic: Option<AnthropicExtraContent>,
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

// ============================================================================
// Model pricing
// ============================================================================

/// Far above any real per-million-token rate, so a value beyond it is a unit
/// mistake rather than a price. The floor matters more: a negative rate would make
/// spend subtract, and NaN/infinity would poison every total derived from it.
pub const MAX_MODEL_RATE: f64 = 1000.0;

/// Bound the `model_pricing` map of an AI config that is only available untyped —
/// the instance config is stored through the generic global-settings endpoint,
/// which never deserializes it into `AIConfig`, so the typed check on the
/// workspace path does not cover it.
pub fn validate_model_pricing_json(ai_config: &serde_json::Value) -> Result<(), String> {
    // The container itself has to be checked too: a non-object `ai_config` persists
    // here and then fails to deserialize as `AIConfig`, which drops the whole
    // instance config back to its default for every workspace inheriting it.
    if !ai_config.is_null() && !ai_config.is_object() {
        return Err("ai_config must be an object".to_string());
    }
    let pricing = match ai_config.get("model_pricing") {
        None | Some(serde_json::Value::Null) => return Ok(()),
        // A present-but-wrong shape must be rejected, not skipped: it would persist
        // and then fail to deserialize as `AIConfig`, which silently drops the whole
        // instance config back to its default for every workspace inheriting it.
        Some(v) => v
            .as_object()
            .ok_or_else(|| "model_pricing must be an object".to_string())?,
    };
    for (key, price) in pricing {
        let Some(price) = price.as_object() else {
            return Err(format!("Price override for {} is not an object", key));
        };
        for field in ["input", "output", "cache_read", "cache_write"] {
            let Some(rate) = price.get(field) else { continue };
            let rate = rate
                .as_f64()
                .filter(|r| r.is_finite() && *r >= 0.0 && *r <= MAX_MODEL_RATE);
            if rate.is_none() {
                return Err(format!(
                    "Price override for {}: {} must be between 0 and {}",
                    key, field, MAX_MODEL_RATE
                ));
            }
        }
        for required in ["input", "output"] {
            if !price.contains_key(required) {
                return Err(format!("Price override for {} is missing {}", key, required));
            }
        }
    }
    Ok(())
}
