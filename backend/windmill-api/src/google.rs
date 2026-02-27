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
        gemini_event_to_openai_sse_chunks, gemini_response_to_openai, openai_messages_to_gemini,
        parse_gemini_response, parse_gemini_sse_event, sanitize_schema_for_google,
        GeminiFunctionDeclaration, GeminiGenerationConfig, GeminiTextRequest, GeminiTool,
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
    #[serde(default)]
    tools: Option<Vec<ChatRequestTool>>,
}

#[derive(Deserialize, Debug)]
struct ChatRequestTool {
    function: ChatRequestToolFunction,
}

#[derive(Deserialize, Debug)]
struct ChatRequestToolFunction {
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    parameters: Option<serde_json::Value>,
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

    let gemini_tools = request.tools.as_ref().map(|tools| {
        let declarations: Vec<GeminiFunctionDeclaration> = tools
            .iter()
            .map(|t| {
                let mut params = t.function.parameters.clone().unwrap_or(json!({}));
                sanitize_schema_for_google(&mut params);
                GeminiFunctionDeclaration {
                    name: t.function.name.clone(),
                    description: t.function.description.clone(),
                    parameters: params,
                }
            })
            .collect();
        vec![GeminiTool {
            function_declarations: Some(declarations),
            google_search: None,
        }]
    });

    let gemini_request = GeminiTextRequest {
        contents,
        tools: gemini_tools,
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
        let mut tool_call_index: usize = 0;
        while let Some(event) = gemini_sse_stream.next().await {
            match event {
                Ok(event) => match parse_gemini_sse_event(&event.data) {
                    Ok(Some(parsed)) => {
                        for chunk in gemini_event_to_openai_sse_chunks(
                            &parsed, &id, &model_str, &mut tool_call_index,
                        ) {
                            yield Ok::<Bytes, reqwest::Error>(Bytes::from(chunk));
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
// Model listing
// ============================================================================

/// List available Gemini models and convert to OpenAI format.
///
/// Gemini returns `{ models: [{ name: "models/gemini-2.5-flash", displayName, ... }] }`.
/// The frontend expects OpenAI format `{ data: [{ id: "models/gemini-2.5-flash", ... }] }`.
pub async fn handle_google_ai_models(
    api_key: &str,
    base_url: &str,
) -> Result<(http::StatusCode, http::HeaderMap, Body)> {
    #[derive(Deserialize)]
    struct GeminiModel {
        name: String,
        #[serde(rename = "displayName", default)]
        display_name: String,
    }

    #[derive(Deserialize)]
    struct GeminiModelsResponse {
        #[serde(default)]
        models: Vec<GeminiModel>,
    }

    let endpoint = format!("{}/models", base_url.trim_end_matches('/'));
    let response = HTTP_CLIENT
        .get(&endpoint)
        .header("x-goog-api-key", api_key)
        .send()
        .await
        .map_err(|e| Error::internal_err(format!("Failed to fetch Gemini models: {}", e)))?;

    if response.error_for_status_ref().is_err() {
        let err_msg = response.text().await.unwrap_or_default();
        return Err(Error::AIError(err_msg));
    }

    let gemini_resp: GeminiModelsResponse = response.json().await.map_err(|e| {
        Error::internal_err(format!("Failed to parse Gemini models response: {}", e))
    })?;

    let data: Vec<serde_json::Value> = gemini_resp
        .models
        .into_iter()
        .map(|m| {
            json!({
                "id": m.name,
                "object": "model",
                "display_name": m.display_name,
            })
        })
        .collect();

    let body_bytes = serde_json::to_vec(&json!({ "data": data }))
        .map_err(|e| Error::internal_err(format!("Failed to serialize models: {}", e)))?;

    let mut headers = http::HeaderMap::new();
    headers.insert("content-type", "application/json".parse().unwrap());

    Ok((http::StatusCode::OK, headers, Body::from(body_bytes)))
}

// ============================================================================
// Non-streaming path
// ============================================================================

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

    let body = response.bytes().await.map_err(|e| {
        Error::internal_err(format!("Failed to read Gemini response body: {}", e))
    })?;

    let parsed = parse_gemini_response(&body)?;
    let openai_response = gemini_response_to_openai(&parsed, model);

    let body_bytes = serde_json::to_vec(&openai_response)
        .map_err(|e| Error::internal_err(format!("Failed to serialize response: {}", e)))?;

    let mut headers = http::HeaderMap::new();
    headers.insert("content-type", "application/json".parse().unwrap());

    Ok((http::StatusCode::OK, headers, Body::from(body_bytes)))
}
