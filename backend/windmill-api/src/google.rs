//! Google AI (Gemini API) handler for the AI chat proxy.
//!
//! Handles POST `chat/completions` requests using the native Gemini API,
//! converting from/to OpenAI format so the existing frontend parsers continue to work.
//!
//! Used by `windmill-api/src/ai.rs` when the provider is `GoogleAI`.
//! Shared conversion logic lives in `windmill_common::ai_google`.

use axum::body::Body;
use bytes::Bytes;
use eventsource_stream::Eventsource;
use futures::StreamExt;
use serde::Deserialize;
use serde_json::json;
use windmill_common::{
    ai_google::{
        openai_messages_to_gemini, parse_gemini_sse_event, GeminiGenerationConfig,
        GeminiTextRequest,
    },
    ai_types::OpenAIMessage,
    error::{Error, Result},
};

use crate::ai::{inject_keepalives, HTTP_CLIENT, KEEPALIVE_INTERVAL_SECS};

// ============================================================================
// Request type (OpenAI format received from the frontend)
// ============================================================================

#[derive(Deserialize, Debug)]
struct ChatRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    #[serde(default)]
    stream: bool,
    #[serde(default)]
    temperature: Option<f32>,
    #[serde(default)]
    max_tokens: Option<u32>,
}

// ============================================================================
// Public handler
// ============================================================================

/// Handle a `chat/completions` POST request using the native Gemini API.
///
/// Converts the incoming OpenAI-format body to a `GeminiTextRequest`, sends it
/// to the appropriate Gemini endpoint, and converts the response back to the
/// OpenAI SSE or JSON format that the frontend expects.
pub async fn handle_google_ai_chat(
    body: &Bytes,
    api_key: &str,
    base_url: &str,
) -> Result<(http::StatusCode, http::HeaderMap, Body)> {
    let request: ChatRequest = serde_json::from_slice(body)
        .map_err(|e| Error::BadRequest(format!("Failed to parse request body: {}", e)))?;

    let (contents, system_instruction) = openai_messages_to_gemini(&request.messages);

    let generation_config =
        if request.temperature.is_some() || request.max_tokens.is_some() {
            Some(GeminiGenerationConfig {
                temperature: request.temperature,
                max_output_tokens: request.max_tokens,
                response_mime_type: None,
                response_schema: None,
            })
        } else {
            None
        };

    let gemini_request = GeminiTextRequest {
        contents,
        tools: None,
        tool_config: None,
        system_instruction,
        generation_config,
    };

    let request_body = serde_json::to_string(&gemini_request)
        .map_err(|e| Error::internal_err(format!("Failed to serialize Gemini request: {}", e)))?;

    let base_url = base_url.trim_end_matches('/');

    if request.stream {
        handle_streaming(&request.model, request_body, api_key, base_url).await
    } else {
        handle_non_streaming(&request.model, request_body, api_key, base_url).await
    }
}

// ============================================================================
// Streaming path
// ============================================================================

async fn handle_streaming(
    model: &str,
    request_body: String,
    api_key: &str,
    base_url: &str,
) -> Result<(http::StatusCode, http::HeaderMap, Body)> {
    let endpoint = format!("{}/models/{}:streamGenerateContent?alt=sse", base_url, model);

    let response = HTTP_CLIENT
        .post(&endpoint)
        .header("content-type", "application/json")
        .header("x-goog-api-key", api_key)
        .body(request_body)
        .send()
        .await
        .map_err(|e| {
            Error::internal_err(format!("Failed to send request to Gemini API: {}", e))
        })?;

    if response.error_for_status_ref().is_err() {
        let err_msg = response.text().await.unwrap_or_default();
        return Err(Error::AIError(err_msg));
    }

    let id = format!("chatcmpl-{}", uuid::Uuid::new_v4().simple());
    let model_str = model.to_string();

    let gemini_sse_stream = response.bytes_stream().eventsource();
    let openai_sse_stream = async_stream::stream! {
        tokio::pin!(gemini_sse_stream);
        while let Some(event) = gemini_sse_stream.next().await {
            match event {
                Ok(event) => match parse_gemini_sse_event(&event.data) {
                    Ok(Some(parsed)) => {
                        if let Some(text) = parsed.text {
                            let chunk = json!({
                                "id": id,
                                "object": "chat.completion.chunk",
                                "model": model_str,
                                "choices": [{
                                    "index": 0,
                                    "delta": { "content": text },
                                    "finish_reason": null
                                }]
                            });
                            yield Ok::<Bytes, reqwest::Error>(
                                Bytes::from(format!("data: {}\n\n", chunk))
                            );
                        }
                    }
                    Ok(None) => {}
                    Err(e) => tracing::error!("Error parsing Gemini SSE event: {}", e),
                },
                Err(e) => tracing::error!("Error reading Gemini SSE stream: {}", e),
            }
        }
        yield Ok::<Bytes, reqwest::Error>(Bytes::from("data: [DONE]\n\n"));
    };

    let mut headers = http::HeaderMap::new();
    headers.insert("content-type", "text/event-stream".parse().unwrap());
    headers.insert("cache-control", "no-cache".parse().unwrap());
    headers.insert("connection", "keep-alive".parse().unwrap());

    Ok((
        http::StatusCode::OK,
        headers,
        Body::from_stream(inject_keepalives(
            Box::pin(openai_sse_stream),
            std::time::Duration::from_secs(KEEPALIVE_INTERVAL_SECS),
        )),
    ))
}

// ============================================================================
// Non-streaming path
// ============================================================================

/// Response types for Gemini's `generateContent` endpoint (non-streaming).
#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<GeminiCandidate>>,
    #[serde(rename = "usageMetadata")]
    usage_metadata: Option<GeminiResponseUsage>,
}

#[derive(Deserialize)]
struct GeminiCandidate {
    content: Option<GeminiCandidateContent>,
    #[serde(rename = "finishReason")]
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct GeminiCandidateContent {
    parts: Option<Vec<GeminiResponsePart>>,
}

#[derive(Deserialize)]
struct GeminiResponsePart {
    text: Option<String>,
}

#[derive(Deserialize)]
struct GeminiResponseUsage {
    #[serde(rename = "promptTokenCount", default)]
    prompt_token_count: Option<i32>,
    #[serde(rename = "candidatesTokenCount", default)]
    candidates_token_count: Option<i32>,
    #[serde(rename = "totalTokenCount", default)]
    total_token_count: Option<i32>,
}

async fn handle_non_streaming(
    model: &str,
    request_body: String,
    api_key: &str,
    base_url: &str,
) -> Result<(http::StatusCode, http::HeaderMap, Body)> {
    let endpoint = format!("{}/models/{}:generateContent", base_url, model);

    let response = HTTP_CLIENT
        .post(&endpoint)
        .header("content-type", "application/json")
        .header("x-goog-api-key", api_key)
        .body(request_body)
        .send()
        .await
        .map_err(|e| {
            Error::internal_err(format!("Failed to send request to Gemini API: {}", e))
        })?;

    if response.error_for_status_ref().is_err() {
        let err_msg = response.text().await.unwrap_or_default();
        return Err(Error::AIError(err_msg));
    }

    let gemini_resp: GeminiResponse = response.json().await.map_err(|e| {
        Error::internal_err(format!("Failed to parse Gemini response: {}", e))
    })?;

    // Extract text content from first candidate
    let content = gemini_resp
        .candidates
        .as_ref()
        .and_then(|c| c.first())
        .and_then(|c| c.content.as_ref())
        .and_then(|c| c.parts.as_ref())
        .and_then(|p| p.first())
        .and_then(|p| p.text.as_deref())
        .unwrap_or("")
        .to_string();

    let finish_reason = gemini_resp
        .candidates
        .as_ref()
        .and_then(|c| c.first())
        .and_then(|c| c.finish_reason.as_deref())
        .map(|r| r.to_lowercase())
        .unwrap_or_else(|| "stop".to_string());

    let usage = gemini_resp.usage_metadata.as_ref().map(|u| {
        json!({
            "prompt_tokens": u.prompt_token_count.unwrap_or(0),
            "completion_tokens": u.candidates_token_count.unwrap_or(0),
            "total_tokens": u.total_token_count.unwrap_or(0),
        })
    });

    let openai_response = json!({
        "id": format!("chatcmpl-{}", uuid::Uuid::new_v4().simple()),
        "object": "chat.completion",
        "model": model,
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": content
            },
            "finish_reason": finish_reason
        }],
        "usage": usage
    });

    let body_bytes = serde_json::to_vec(&openai_response)
        .map_err(|e| Error::internal_err(format!("Failed to serialize response: {}", e)))?;

    let mut headers = http::HeaderMap::new();
    headers.insert("content-type", "application/json".parse().unwrap());

    Ok((http::StatusCode::OK, headers, Body::from(body_bytes)))
}
