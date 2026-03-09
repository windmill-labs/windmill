//! Shared Google AI (Gemini API) types and conversion utilities.
//!
//! This module provides:
//! - Gemini request/response types
//! - OpenAI → Gemini message conversion
//! - Gemini SSE event parsing
//!
//! Used by both windmill-api (chat proxy) and windmill-worker (AI agent).

use serde::{Deserialize, Serialize};

use crate::ai_types::{ContentPart, ExtraContent, GoogleExtraContent, OpenAIContent, OpenAIMessage, ToolDef, UrlCitation};
use crate::error::Error;

// ============================================================================
// Request / Content Types
// ============================================================================

/// Inline data for binary content (images).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GeminiInlineData {
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    pub data: String,
}

/// A part of content — text, inline data, function call, or function response.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum GeminiPart {
    Text {
        text: String,
    },
    InlineData {
        #[serde(rename = "inlineData")]
        inline_data: GeminiInlineData,
    },
    FunctionCall {
        #[serde(rename = "functionCall")]
        function_call: GeminiFunctionCall,
        /// Thought signature for Gemini 3+ models — required when replaying function calls.
        #[serde(rename = "thoughtSignature", skip_serializing_if = "Option::is_none")]
        thought_signature: Option<String>,
    },
    FunctionResponse {
        #[serde(rename = "functionResponse")]
        function_response: GeminiFunctionResponse,
    },
}

/// A function call from the model.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GeminiFunctionCall {
    pub name: String,
    pub args: serde_json::Value,
}

/// A function response sent back to the model.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GeminiFunctionResponse {
    pub name: String,
    pub response: serde_json::Value,
}

/// Content message with an optional role and a list of parts.
#[derive(Serialize, Clone, Debug)]
pub struct GeminiContentMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    pub parts: Vec<GeminiPart>,
}

/// Main request body for `generateContent` / `streamGenerateContent`.
#[derive(Serialize)]
pub struct GeminiTextRequest {
    pub contents: Vec<GeminiContentMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<GeminiTool>>,
    #[serde(rename = "toolConfig", skip_serializing_if = "Option::is_none")]
    pub tool_config: Option<GeminiToolConfig>,
    #[serde(rename = "systemInstruction", skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<GeminiContentMessage>,
    #[serde(rename = "generationConfig", skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GeminiGenerationConfig>,
}

/// Tool definition — function declarations and/or Google Search grounding.
#[derive(Serialize)]
pub struct GeminiTool {
    #[serde(rename = "functionDeclarations", skip_serializing_if = "Option::is_none")]
    pub function_declarations: Option<Vec<GeminiFunctionDeclaration>>,
    #[serde(rename = "googleSearch", skip_serializing_if = "Option::is_none")]
    pub google_search: Option<serde_json::Value>,
}

/// A single function declaration.
///
/// `parameters` holds a pre-serialized (and, for the worker, pre-sanitized) JSON Schema.
#[derive(Serialize)]
pub struct GeminiFunctionDeclaration {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub parameters: serde_json::Value,
}

/// Tool configuration controlling when and how functions are called.
#[derive(Serialize)]
pub struct GeminiToolConfig {
    #[serde(rename = "functionCallingConfig")]
    pub function_calling_config: GeminiFunctionCallingConfig,
}

/// Function calling mode and optional allow-list.
#[derive(Serialize)]
pub struct GeminiFunctionCallingConfig {
    pub mode: String,
    #[serde(rename = "allowedFunctionNames", skip_serializing_if = "Option::is_none")]
    pub allowed_function_names: Option<Vec<String>>,
}

/// Generation parameters (temperature, token limits, structured output).
#[derive(Serialize)]
pub struct GeminiGenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(rename = "maxOutputTokens", skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,
    #[serde(rename = "responseMimeType", skip_serializing_if = "Option::is_none")]
    pub response_mime_type: Option<String>,
    #[serde(rename = "responseSchema", skip_serializing_if = "Option::is_none")]
    pub response_schema: Option<serde_json::Value>,
}

// ============================================================================
// Image Generation Types
// ============================================================================

/// Request body for Imagen / Gemini image generation.
#[derive(Serialize)]
pub struct GeminiImageRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contents: Option<Vec<GeminiImageContent>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instances: Option<Vec<GeminiPredictContent>>,
}

/// Content wrapper used in `generateContent` image requests.
#[derive(Serialize)]
pub struct GeminiImageContent {
    pub parts: Vec<GeminiPart>,
}

/// Prompt wrapper for Imagen `predict` endpoint.
#[derive(Serialize)]
pub struct GeminiPredictContent {
    pub prompt: String,
}

/// Top-level response from Gemini/Imagen image generation.
#[derive(Deserialize)]
pub struct GeminiImageResponse {
    pub candidates: Option<Vec<GeminiImageCandidate>>,
    pub predictions: Option<Vec<GeminiPredictCandidate>>,
}

#[derive(Deserialize)]
pub struct GeminiImageCandidate {
    pub content: GeminiImageCandidateContent,
}

#[derive(Deserialize)]
pub struct GeminiImageCandidateContent {
    pub parts: Vec<GeminiImageCandidatePart>,
}

#[derive(Deserialize)]
pub struct GeminiImageCandidatePart {
    #[serde(rename = "inlineData")]
    pub inline_data: Option<GeminiInlineData>,
}

#[derive(Deserialize)]
pub struct GeminiPredictCandidate {
    #[serde(rename = "bytesBase64Encoded")]
    pub bytes_base64_encoded: String,
}

// ============================================================================
// SSE Response Types
// ============================================================================

/// One part inside a streaming candidate — text, function call, or thought signature.
#[derive(Deserialize, Debug)]
pub struct GeminiSSEPart {
    #[serde(default)]
    pub text: Option<String>,
    #[serde(rename = "functionCall")]
    pub function_call: Option<GeminiSSEFunctionCall>,
    /// Thought signature for Gemini 3+ models.
    #[serde(rename = "thoughtSignature")]
    pub thought_signature: Option<String>,
}

/// Function call contained in a streaming part.
#[derive(Deserialize, Debug)]
pub struct GeminiSSEFunctionCall {
    pub name: String,
    pub args: serde_json::Value,
}

/// Content block inside a streaming candidate.
#[derive(Deserialize, Debug)]
pub struct GeminiSSEContent {
    pub parts: Option<Vec<GeminiSSEPart>>,
}

/// Web source from a Gemini grounding chunk.
#[derive(Deserialize, Debug)]
pub struct GeminiGroundingChunkWeb {
    pub uri: String,
    #[serde(default)]
    pub title: Option<String>,
}

/// One grounding chunk (search result) from Gemini web search.
#[derive(Deserialize, Debug)]
pub struct GeminiGroundingChunk {
    pub web: Option<GeminiGroundingChunkWeb>,
}

/// Grounding metadata attached to a streaming candidate.
#[derive(Deserialize, Debug)]
pub struct GeminiGroundingMetadata {
    #[serde(rename = "groundingChunks", default)]
    pub grounding_chunks: Vec<GeminiGroundingChunk>,
    #[serde(rename = "webSearchQueries", default)]
    pub web_search_queries: Vec<String>,
}

/// One candidate inside a streaming Gemini response.
#[derive(Deserialize, Debug)]
pub struct GeminiSSECandidate {
    pub content: Option<GeminiSSEContent>,
    #[serde(rename = "finishReason")]
    pub finish_reason: Option<String>,
    #[serde(rename = "groundingMetadata")]
    pub grounding_metadata: Option<GeminiGroundingMetadata>,
}

/// Token usage from the `usageMetadata` field of a Gemini SSE event.
#[derive(Deserialize, Debug, Clone)]
pub struct GeminiUsageMetadata {
    #[serde(rename = "promptTokenCount", default)]
    pub prompt_token_count: Option<i32>,
    #[serde(rename = "candidatesTokenCount", default)]
    pub candidates_token_count: Option<i32>,
    #[serde(rename = "totalTokenCount", default)]
    pub total_token_count: Option<i32>,
}

/// Top-level structure of one Gemini SSE event.
#[derive(Deserialize, Debug)]
pub struct GeminiSSEEvent {
    pub candidates: Option<Vec<GeminiSSECandidate>>,
    #[serde(rename = "usageMetadata")]
    pub usage_metadata: Option<GeminiUsageMetadata>,
}

// ============================================================================
// Parsed Event Result
// ============================================================================

/// A single function call extracted from a Gemini SSE event.
#[derive(Debug)]
pub struct GeminiToolCallEvent {
    pub name: String,
    pub args: serde_json::Value,
    pub thought_signature: Option<String>,
}

impl GeminiToolCallEvent {
    /// Convert the thought signature (if present) into an [`ExtraContent`].
    pub fn to_extra_content(&self) -> Option<ExtraContent> {
        self.thought_signature.as_ref().map(|sig| ExtraContent {
            google: Some(GoogleExtraContent { thought_signature: Some(sig.clone()) }),
        })
    }
}

/// Structured result of parsing a Gemini response (streaming SSE event or non-streaming body).
#[derive(Debug, Default)]
pub struct GeminiParsedEvent {
    pub text: Option<String>,
    pub tool_calls: Vec<GeminiToolCallEvent>,
    pub annotations: Vec<UrlCitation>,
    pub used_websearch: bool,
    pub usage: Option<GeminiUsageMetadata>,
    pub finish_reason: Option<String>,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Parse a data URL into `(mime_type, base64_data)`.
///
/// Expected format: `data:<mime_type>;base64,<data>`.
pub fn parse_data_url(url: &str) -> Option<(String, String)> {
    let rest = url.strip_prefix("data:")?;
    let (header, data) = rest.split_once(',')?;
    let media_type = header.strip_suffix(";base64")?;
    Some((media_type.to_string(), data.to_string()))
}

/// Find the function name associated with a `tool_call_id` by scanning prior messages.
pub fn find_gemini_function_name(messages: &[OpenAIMessage], tool_call_id: &str) -> String {
    messages
        .iter()
        .filter_map(|msg| msg.tool_calls.as_ref())
        .flatten()
        .find(|tc| tc.id == tool_call_id)
        .map(|tc| tc.function.name.clone())
        .unwrap_or_else(|| "unknown_function".to_string())
}

/// Convert an [`OpenAIContent`] value to a list of [`GeminiPart`]s.
///
/// Handles text and `image_url` (data URLs). `S3Object` variants are skipped here;
/// the worker handles them by downloading and injecting inline data beforehand.
pub fn convert_content_to_gemini_parts(content: &OpenAIContent) -> Vec<GeminiPart> {
    match content {
        OpenAIContent::Text(text) if !text.is_empty() => {
            vec![GeminiPart::Text { text: text.clone() }]
        }
        OpenAIContent::Text(_) => vec![],
        OpenAIContent::Parts(parts) => parts
            .iter()
            .filter_map(|part| match part {
                ContentPart::Text { text } if !text.is_empty() => {
                    Some(GeminiPart::Text { text: text.clone() })
                }
                ContentPart::ImageUrl { image_url } => {
                    parse_data_url(&image_url.url).map(|(mime_type, data)| {
                        GeminiPart::InlineData {
                            inline_data: GeminiInlineData { mime_type, data },
                        }
                    })
                }
                // S3Objects are handled by the worker
                _ => None,
            })
            .collect(),
    }
}

/// Convert OpenAI-format messages to Gemini `contents` and an optional `systemInstruction`.
///
/// Returns `(contents, system_instruction)`.
///
/// `S3Object` images in content parts are skipped (the worker pre-converts them).
/// Tool call history is preserved correctly for multi-turn agent conversations.
pub fn openai_messages_to_gemini(
    messages: &[OpenAIMessage],
) -> (Vec<GeminiContentMessage>, Option<GeminiContentMessage>) {
    let mut contents: Vec<GeminiContentMessage> = Vec::new();
    let mut system_instruction: Option<GeminiContentMessage> = None;

    for msg in messages {
        match msg.role.as_str() {
            "system" => {
                if let Some(content) = &msg.content {
                    let parts = convert_content_to_gemini_parts(content);
                    if !parts.is_empty() {
                        system_instruction =
                            Some(GeminiContentMessage { role: None, parts });
                    }
                }
            }
            "tool" => {
                if let (Some(tool_call_id), Some(content)) =
                    (&msg.tool_call_id, &msg.content)
                {
                    let func_name = find_gemini_function_name(messages, tool_call_id);
                    let response_text = match content {
                        OpenAIContent::Text(text) => text.clone(),
                        OpenAIContent::Parts(parts) => parts
                            .iter()
                            .filter_map(|p| {
                                if let ContentPart::Text { text } = p {
                                    Some(text.as_str())
                                } else {
                                    None
                                }
                            })
                            .collect::<Vec<_>>()
                            .join(" "),
                    };
                    contents.push(GeminiContentMessage {
                        role: Some("user".to_string()),
                        parts: vec![GeminiPart::FunctionResponse {
                            function_response: GeminiFunctionResponse {
                                name: func_name,
                                response: serde_json::json!({ "result": response_text }),
                            },
                        }],
                    });
                }
            }
            role => {
                let gemini_role = if role == "assistant" { "model" } else { "user" };
                let mut parts: Vec<GeminiPart> = Vec::new();

                if let Some(content) = &msg.content {
                    parts.extend(convert_content_to_gemini_parts(content));
                }

                if let Some(tool_calls) = &msg.tool_calls {
                    for tc in tool_calls {
                        let args: serde_json::Value =
                            serde_json::from_str(&tc.function.arguments).unwrap_or_default();
                        let thought_signature = tc
                            .extra_content
                            .as_ref()
                            .and_then(|ec| ec.google.as_ref())
                            .and_then(|g| g.thought_signature.clone());
                        parts.push(GeminiPart::FunctionCall {
                            function_call: GeminiFunctionCall {
                                name: tc.function.name.clone(),
                                args,
                            },
                            thought_signature,
                        });
                    }
                }

                if !parts.is_empty() {
                    contents.push(GeminiContentMessage {
                        role: Some(gemini_role.to_string()),
                        parts,
                    });
                }
            }
        }
    }

    (contents, system_instruction)
}

/// Convert OpenAI tool definitions to Gemini format.
///
/// `tool_params` must be pre-serialized (and, for the worker, pre-sanitized for Google)
/// JSON schema values, one per entry in `tools` in the same order.
pub fn openai_tools_to_gemini(
    tools: &[ToolDef],
    tool_params: &[serde_json::Value],
    has_websearch: bool,
) -> Option<Vec<GeminiTool>> {
    let mut gemini_tools: Vec<GeminiTool> = Vec::new();

    let declarations: Vec<GeminiFunctionDeclaration> = tools
        .iter()
        .zip(tool_params.iter())
        .map(|(t, params)| GeminiFunctionDeclaration {
            name: t.function.name.clone(),
            description: t.function.description.clone(),
            parameters: params.clone(),
        })
        .collect();

    if !declarations.is_empty() {
        gemini_tools.push(GeminiTool {
            function_declarations: Some(declarations),
            google_search: None,
        });
    }

    if has_websearch {
        gemini_tools.push(GeminiTool {
            function_declarations: None,
            google_search: Some(serde_json::json!({})),
        });
    }

    if gemini_tools.is_empty() {
        None
    } else {
        Some(gemini_tools)
    }
}

/// Parse one Gemini SSE data line into a [`GeminiParsedEvent`].
///
/// Returns `Ok(None)` for empty data or unrecognised payloads (e.g. `"[DONE]"`).
/// Logs a warning and returns `Ok(None)` on JSON parse errors rather than propagating.
pub fn parse_gemini_sse_event(data: &str) -> Result<Option<GeminiParsedEvent>, Error> {
    if data.is_empty() || data == "[DONE]" {
        return Ok(None);
    }

    let event: GeminiSSEEvent = match serde_json::from_str(data) {
        Ok(e) => e,
        Err(e) => {
            tracing::error!("Failed to parse Gemini SSE event {}: {}", data, e);
            return Ok(None);
        }
    };

    let mut parsed = GeminiParsedEvent { usage: event.usage_metadata, ..Default::default() };

    let Some(candidates) = event.candidates else {
        return Ok(Some(parsed));
    };

    extract_candidates_into(&candidates, &mut parsed);

    Ok(Some(parsed))
}

/// Parse a non-streaming Gemini `generateContent` response body.
pub fn parse_gemini_response(data: &[u8]) -> Result<GeminiParsedEvent, Error> {
    let event: GeminiSSEEvent = serde_json::from_slice(data)
        .map_err(|e| Error::internal_err(format!("Failed to parse Gemini response: {}", e)))?;

    let mut parsed = GeminiParsedEvent { usage: event.usage_metadata, ..Default::default() };

    if let Some(candidates) = event.candidates {
        extract_candidates_into(&candidates, &mut parsed);
    }

    Ok(parsed)
}

// ============================================================================
// Gemini → OpenAI Format Conversion
// ============================================================================

/// Convert a `GeminiParsedEvent` from a non-streaming response to an OpenAI chat completion JSON.
pub fn gemini_response_to_openai(parsed: &GeminiParsedEvent, model: &str) -> serde_json::Value {
    let content = parsed.text.as_deref().unwrap_or_default();

    let tool_calls: Vec<serde_json::Value> = parsed
        .tool_calls
        .iter()
        .enumerate()
        .map(|(i, tc)| {
            serde_json::json!({
                "index": i,
                "id": format!("call_{}", uuid::Uuid::new_v4().simple()),
                "type": "function",
                "function": {
                    "name": tc.name,
                    "arguments": serde_json::to_string(&tc.args).unwrap_or_default()
                }
            })
        })
        .collect();

    let finish_reason = parsed
        .finish_reason
        .as_deref()
        .map(|r| r.to_lowercase())
        .unwrap_or_else(|| "stop".to_string());

    let usage = parsed.usage.as_ref().map(|u| {
        serde_json::json!({
            "prompt_tokens": u.prompt_token_count.unwrap_or(0),
            "completion_tokens": u.candidates_token_count.unwrap_or(0),
            "total_tokens": u.total_token_count.unwrap_or(0),
        })
    });

    let mut message = serde_json::json!({
        "role": "assistant",
        "content": content,
    });
    if !tool_calls.is_empty() {
        message["tool_calls"] = serde_json::json!(tool_calls);
    }

    serde_json::json!({
        "id": format!("chatcmpl-{}", uuid::Uuid::new_v4().simple()),
        "object": "chat.completion",
        "model": model,
        "choices": [{
            "index": 0,
            "message": message,
            "finish_reason": finish_reason,
        }],
        "usage": usage,
    })
}

/// Convert a `GeminiParsedEvent` from a streaming SSE event into OpenAI-format SSE lines.
///
/// Returns the serialized `"data: {...}\n\n"` lines ready to be written to the response stream.
/// `tool_call_index` is mutated to track the running index across multiple SSE events.
pub fn gemini_event_to_openai_sse_chunks(
    parsed: &GeminiParsedEvent,
    id: &str,
    model: &str,
    tool_call_index: &mut usize,
) -> Vec<String> {
    let mut chunks = Vec::new();

    if let Some(text) = &parsed.text {
        let chunk = serde_json::json!({
            "id": id,
            "object": "chat.completion.chunk",
            "model": model,
            "choices": [{
                "index": 0,
                "delta": { "content": text },
                "finish_reason": null,
            }]
        });
        chunks.push(format!("data: {}\n\n", chunk));
    }

    for tc in &parsed.tool_calls {
        let args_str = serde_json::to_string(&tc.args).unwrap_or_default();
        let call_id = format!("call_{}", uuid::Uuid::new_v4().simple());
        let chunk = serde_json::json!({
            "id": id,
            "object": "chat.completion.chunk",
            "model": model,
            "choices": [{
                "index": 0,
                "delta": {
                    "tool_calls": [{
                        "index": *tool_call_index,
                        "id": call_id,
                        "type": "function",
                        "function": {
                            "name": tc.name,
                            "arguments": args_str,
                        }
                    }]
                },
                "finish_reason": null,
            }]
        });
        chunks.push(format!("data: {}\n\n", chunk));
        *tool_call_index += 1;
    }

    chunks
}

/// Recursively remove JSON Schema fields unsupported by the Gemini API.
pub fn sanitize_schema_for_google(value: &mut serde_json::Value) {
    const UNSUPPORTED: &[&str] = &[
        "additionalProperties",
        "strict",
        "$schema",
        "default",
        "exclusiveMinimum",
        "exclusiveMaximum",
        "const",
        "multipleOf",
    ];

    if let Some(obj) = value.as_object_mut() {
        for field in UNSUPPORTED {
            obj.remove(*field);
        }
        for v in obj.values_mut() {
            sanitize_schema_for_google(v);
        }
    } else if let Some(arr) = value.as_array_mut() {
        for v in arr.iter_mut() {
            sanitize_schema_for_google(v);
        }
    }
}

// ============================================================================
// Internal Helpers
// ============================================================================

fn extract_candidates_into(candidates: &[GeminiSSECandidate], parsed: &mut GeminiParsedEvent) {
    for candidate in candidates {
        if let Some(content) = &candidate.content {
            if let Some(parts) = &content.parts {
                for part in parts {
                    if let Some(text) = &part.text {
                        if !text.is_empty() {
                            match parsed.text.as_mut() {
                                Some(existing) => existing.push_str(text),
                                None => parsed.text = Some(text.clone()),
                            }
                        }
                    }

                    if let Some(function_call) = &part.function_call {
                        parsed.tool_calls.push(GeminiToolCallEvent {
                            name: function_call.name.clone(),
                            args: function_call.args.clone(),
                            thought_signature: part.thought_signature.clone(),
                        });
                    }
                }
            }
        }

        if candidate.finish_reason.is_some() {
            parsed.finish_reason = candidate.finish_reason.clone();
        }

        if let Some(grounding) = &candidate.grounding_metadata {
            if !grounding.web_search_queries.is_empty() || !grounding.grounding_chunks.is_empty() {
                parsed.used_websearch = true;
            }
            for chunk in &grounding.grounding_chunks {
                if let Some(web) = &chunk.web {
                    parsed.annotations.push(UrlCitation {
                        start_index: 0,
                        end_index: 0,
                        url: web.uri.clone(),
                        title: web.title.clone(),
                    });
                }
            }
        }
    }
}
