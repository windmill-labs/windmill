//! AWS Bedrock SDK-based streaming for the AI chat proxy.
//!
//! This module provides SDK-based request handling for Bedrock:
//!
//! ## SDK-Based (primary approach):
//! - `handle_bedrock_sdk_streaming`: Uses BedrockClient for streaming requests
//! - `handle_bedrock_sdk_non_streaming`: Uses BedrockClient for non-streaming requests
//! - `sdk_stream_to_sse`: Converts SDK ConverseStream events to SSE format
//!
//! ## HTTP-Based (for GET requests only):
//! - `sign_bedrock_request`: Manual SigV4 signing for IAM authentication
//!
//! Shared AWS SDK code is available in `windmill_common::ai_bedrock`, including:
//! - `BedrockClient`: SDK wrapper with bearer token and IAM auth
//! - Stream event parsing functions
//! - Helper utilities

use axum::body::Bytes;
use aws_sigv4::http_request::{sign, SignableBody, SignableRequest, SigningSettings};
use aws_sigv4::sign::v4;
use std::time::SystemTime;
use serde::Deserialize;
use windmill_common::ai_bedrock::{
    bedrock_stream_event_is_block_stop, bedrock_stream_event_to_text,
    bedrock_stream_event_to_tool_delta, bedrock_stream_event_to_tool_start, format_bedrock_error,
    BedrockClient, OpenAIMessage, OpenAIToolCall,
};
use windmill_common::error::{Error, Result};

// ============================================================================
// Shared Request Types for SDK-Based Handlers
// ============================================================================

/// OpenAI-format request body for Bedrock SDK handlers
#[derive(Deserialize, Debug)]
struct OpenAIRequest {
    messages: Vec<OpenAIMessage>,
    #[serde(default)]
    tools: Option<Vec<OpenAIToolDef>>,
    #[serde(default)]
    tool_choice: Option<serde_json::Value>,
    #[serde(default)]
    max_tokens: Option<i32>,
    #[serde(default)]
    temperature: Option<f32>,
}

#[derive(Deserialize, Debug)]
struct OpenAIToolDef {
    #[serde(default)]
    #[allow(dead_code)]
    r#type: Option<String>,
    function: OpenAIToolFunction,
}

#[derive(Deserialize, Debug)]
struct OpenAIToolFunction {
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    parameters: Option<serde_json::Value>,
}

// ============================================================================
// Shared Helper Functions for SDK-Based Handlers
// ============================================================================

/// Create a BedrockClient with auth priority: bearer token → IAM credentials → environment
async fn create_bedrock_client(
    api_key: Option<&str>,
    aws_access_key_id: Option<&str>,
    aws_secret_access_key: Option<&str>,
    region: &str,
    context: &str, // "streaming" or "non-streaming" for logging
) -> Result<BedrockClient> {
    if let Some(key) = api_key.filter(|k| !k.is_empty()) {
        // 1. Bearer token if API key provided
        tracing::info!(
            "Bedrock SDK {}: auth=bearer token, region={}",
            context,
            region
        );
        BedrockClient::from_bearer_token(key.to_string(), region).await
    } else if let (Some(access_key_id), Some(secret_access_key)) = (
        aws_access_key_id.filter(|s| !s.is_empty()),
        aws_secret_access_key.filter(|s| !s.is_empty()),
    ) {
        // 2. IAM credentials if provided and not empty
        tracing::info!(
            "Bedrock SDK {}: auth=IAM credentials, region={}, access_key_id={}...",
            context,
            region,
            access_key_id.get(..8).unwrap_or("N/A")
        );
        BedrockClient::from_credentials(
            access_key_id.to_string(),
            secret_access_key.to_string(),
            region,
        )
        .await
    } else {
        // 3. Environment credentials as fallback
        tracing::info!(
            "Bedrock SDK {}: auth=environment credentials, region={}",
            context,
            region
        );
        BedrockClient::from_env(region).await
    }
}

/// Convert OpenAIToolDef array to tool configuration for Bedrock SDK
fn build_tool_config_from_request(
    tools: Option<&[OpenAIToolDef]>,
    tool_choice: Option<&serde_json::Value>,
) -> Result<Option<aws_sdk_bedrockruntime::types::ToolConfiguration>> {
    if let Some(tools) = tools {
        let tool_defs: Vec<windmill_common::ai_bedrock::ToolDef> = tools
            .iter()
            .map(|t| windmill_common::ai_bedrock::ToolDef {
                r#type: "function".to_string(),
                function: windmill_common::ai_bedrock::ToolDefFunction {
                    name: t.function.name.clone(),
                    description: t.function.description.clone(),
                    parameters: Box::from(
                        serde_json::value::RawValue::from_string(
                            serde_json::to_string(
                                &t.function.parameters.clone().unwrap_or(serde_json::json!({})),
                            )
                            .unwrap_or_default(),
                        )
                        .unwrap_or_else(|_| {
                            serde_json::value::RawValue::from_string("{}".to_string()).unwrap()
                        }),
                    ),
                },
            })
            .collect();

        // Determine if we should force tool use based on tool_choice
        let force_tool_use = tool_choice
            .map(|tc| tc == "required" || tc.as_str() == Some("required"))
            .unwrap_or(false);

        windmill_common::ai_bedrock::build_tool_config(Some(&tool_defs), force_tool_use)
    } else {
        Ok(None)
    }
}

/// Sign a request for AWS Bedrock using SigV4
///
/// Returns a vector of (header_name, header_value) tuples to add to the request
pub fn sign_bedrock_request(
    method: &str,
    uri: &str,
    body: &[u8],
    access_key_id: &str,
    secret_access_key: &str,
    region: &str,
) -> Result<Vec<(String, String)>> {
    tracing::info!(
        "Bedrock HTTP path (SigV4 signing): method={}, region={}, access_key_id={}...",
        method,
        region,
        access_key_id.get(..8).unwrap_or("N/A")
    );
    tracing::debug!("Bedrock HTTP path: uri={}", uri);

    let identity = aws_credential_types::Credentials::new(
        access_key_id,
        secret_access_key,
        None, // session token
        None, // expiration
        "windmill",
    )
    .into();

    let signing_settings = SigningSettings::default();
    let signing_params = v4::SigningParams::builder()
        .identity(&identity)
        .region(region)
        .name("bedrock")
        .time(SystemTime::now())
        .settings(signing_settings)
        .build()
        .map_err(|e| Error::internal_err(format!("Failed to build signing params: {}", e)))?;

    // Parse the URI to extract path and query
    let parsed_uri: http::Uri = uri
        .parse()
        .map_err(|e| Error::internal_err(format!("Failed to parse URI: {}", e)))?;

    let path_and_query = parsed_uri
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("/");

    let signable_request = SignableRequest::new(
        method,
        path_and_query,
        std::iter::once(("host", parsed_uri.host().unwrap_or(""))),
        SignableBody::Bytes(body),
    )
    .map_err(|e| Error::internal_err(format!("Failed to create signable request: {}", e)))?;

    let (signing_instructions, _signature) = sign(signable_request, &signing_params.into())
        .map_err(|e| Error::internal_err(format!("Failed to sign request: {}", e)))?
        .into_parts();

    // Collect the headers to add
    let mut headers = Vec::new();
    for (name, value) in signing_instructions.headers() {
        headers.push((name.to_string(), value.to_string()));
    }

    Ok(headers)
}

// ============================================================================
// SDK-Based Streaming (for IAM credentials auth)
// ============================================================================

// Note: HTTP-based transform functions (transform_openai_to_bedrock, transform_bedrock_to_openai,
// transform_bedrock_stream_to_openai) have been removed as Bedrock POST requests now use
// the SDK-based approach (handle_bedrock_sdk_streaming, handle_bedrock_sdk_non_streaming).

/// Handle Bedrock streaming request using the AWS SDK.
///
/// This function uses the shared BedrockClient to make streaming requests
/// and converts the SDK stream events to SSE format for the proxy response.
///
/// Auth priority: bearer token → IAM credentials → environment credentials
pub async fn handle_bedrock_sdk_streaming(
    model: &str,
    body: &Bytes,
    api_key: Option<&str>,
    aws_access_key_id: Option<&str>,
    aws_secret_access_key: Option<&str>,
    region: &str,
) -> Result<(
    http::StatusCode,
    http::HeaderMap,
    axum::body::Body,
)> {
    let openai_req: OpenAIRequest = serde_json::from_slice(body)
        .map_err(|e| Error::internal_err(format!("Failed to parse OpenAI request: {}", e)))?;

    // Create Bedrock client using shared helper
    let bedrock_client = create_bedrock_client(
        api_key,
        aws_access_key_id,
        aws_secret_access_key,
        region,
        "streaming",
    )
    .await?;

    // Convert messages using shared conversion
    let (bedrock_messages, system_prompts) =
        windmill_common::ai_bedrock::openai_messages_to_bedrock(&openai_req.messages)?;

    // Build inference configuration
    let inference_config = windmill_common::ai_bedrock::create_inference_config(
        openai_req.temperature,
        openai_req.max_tokens,
    );

    // Convert tools using shared helper
    let tool_config = build_tool_config_from_request(
        openai_req.tools.as_deref(),
        openai_req.tool_choice.as_ref(),
    )?;

    // Build the SDK request
    let mut request_builder = bedrock_client
        .client()
        .converse_stream()
        .model_id(model)
        .set_messages(Some(bedrock_messages));

    if !system_prompts.is_empty() {
        request_builder = request_builder.set_system(Some(system_prompts));
    }

    if let Some(config) = inference_config {
        request_builder = request_builder.inference_config(config);
    }

    if let Some(config) = tool_config {
        request_builder = request_builder.set_tool_config(Some(config));
    }

    // Send the request and get the stream
    tracing::debug!("Bedrock SDK streaming: sending converse_stream request");
    let stream_output = request_builder.send().await.map_err(|e| {
        let error_msg = format!("Bedrock SDK streaming error: {}", format_bedrock_error(&e));
        tracing::error!("Bedrock SDK streaming failed: {}", error_msg);
        Error::internal_err(error_msg)
    })?;
    tracing::debug!("Bedrock SDK streaming: stream established successfully");

    // Convert SDK stream to SSE (pass the inner stream, not the full output)
    let sse_stream = sdk_stream_to_sse(stream_output.stream, model.to_string());

    // Build response headers
    let mut response_headers = http::HeaderMap::new();
    response_headers.insert("content-type", "text/event-stream".parse().unwrap());
    response_headers.insert("cache-control", "no-cache".parse().unwrap());
    response_headers.insert("connection", "keep-alive".parse().unwrap());

    Ok((
        http::StatusCode::OK,
        response_headers,
        axum::body::Body::from_stream(sse_stream),
    ))
}

/// Convert AWS SDK ConverseStream events to SSE format.
///
/// Uses shared stream parsing functions from windmill_common::ai_bedrock
/// to extract text deltas and tool calls from the SDK stream events.
pub fn sdk_stream_to_sse(
    stream: aws_sdk_bedrockruntime::primitives::event_stream::EventReceiver<
        aws_sdk_bedrockruntime::types::ConverseStreamOutput,
        aws_sdk_bedrockruntime::types::error::ConverseStreamOutputError,
    >,
    model: String,
) -> impl futures::Stream<Item = std::result::Result<bytes::Bytes, std::io::Error>> + Send {
    use std::collections::HashMap;

    let id = format!("chatcmpl-{}", uuid::Uuid::new_v4().simple());
    let created = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // State to track partial tool calls
    struct StreamState {
        id: String,
        model: String,
        created: u64,
        tool_calls: HashMap<usize, (String, String, String)>, // index -> (id, name, args)
        current_tool_index: usize,
    }

    let state = std::sync::Arc::new(tokio::sync::Mutex::new(StreamState {
        id: id.clone(),
        model: model.clone(),
        created,
        tool_calls: HashMap::new(),
        current_tool_index: 0,
    }));

    async_stream::stream! {
        let mut stream = stream;
        let state = state.clone();

        loop {
            match stream.recv().await {
                Ok(Some(event)) => {
                    let mut state = state.lock().await;

                    // Handle tool use start
                    if let Some(tool_call) = bedrock_stream_event_to_tool_start(&event) {
                        let index = state.current_tool_index;
                        state.tool_calls.insert(
                            index,
                            (tool_call.id.clone(), tool_call.name.clone(), String::new()),
                        );

                        // Send initial tool call chunk
                        let chunk = serde_json::json!({
                            "id": state.id,
                            "object": "chat.completion.chunk",
                            "created": state.created,
                            "model": state.model,
                            "choices": [{
                                "index": 0,
                                "delta": {
                                    "tool_calls": [{
                                        "index": index,
                                        "id": tool_call.id,
                                        "type": "function",
                                        "function": {
                                            "name": tool_call.name,
                                            "arguments": ""
                                        }
                                    }]
                                },
                                "finish_reason": serde_json::Value::Null
                            }]
                        });

                        yield Ok(bytes::Bytes::from(format!("data: {}\n\n", chunk)));
                    }

                    // Handle text delta
                    if let Some(text) = bedrock_stream_event_to_text(&event) {
                        let chunk = serde_json::json!({
                            "id": state.id,
                            "object": "chat.completion.chunk",
                            "created": state.created,
                            "model": state.model,
                            "choices": [{
                                "index": 0,
                                "delta": {
                                    "content": text
                                },
                                "finish_reason": serde_json::Value::Null
                            }]
                        });

                        yield Ok(bytes::Bytes::from(format!("data: {}\n\n", chunk)));
                    }

                    // Handle tool use input delta
                    if let Some(input_delta) = bedrock_stream_event_to_tool_delta(&event) {
                        let index = state.current_tool_index;
                        if let Some((_id, _name, ref mut args)) = state.tool_calls.get_mut(&index) {
                            args.push_str(&input_delta);

                            let chunk = serde_json::json!({
                                "id": state.id,
                                "object": "chat.completion.chunk",
                                "created": state.created,
                                "model": state.model,
                                "choices": [{
                                    "index": 0,
                                    "delta": {
                                        "tool_calls": [{
                                            "index": index,
                                            "function": {
                                                "arguments": input_delta
                                            }
                                        }]
                                    },
                                    "finish_reason": serde_json::Value::Null
                                }]
                            });

                            yield Ok(bytes::Bytes::from(format!("data: {}\n\n", chunk)));
                        }
                    }

                    // Handle content block stop
                    if bedrock_stream_event_is_block_stop(&event) {
                        state.current_tool_index += 1;
                    }

                    // Handle message stop
                    if let aws_sdk_bedrockruntime::types::ConverseStreamOutput::MessageStop(stop) = &event {
                        let stop_reason = stop.stop_reason().as_str();
                        let finish_reason = match stop_reason {
                            "end_turn" => "stop",
                            "max_tokens" => "length",
                            "tool_use" => "tool_calls",
                            "stop_sequence" => "stop",
                            "guardrail_intervened" | "content_filtered" => "content_filter",
                            _ => "stop",
                        };

                        let chunk = serde_json::json!({
                            "id": state.id,
                            "object": "chat.completion.chunk",
                            "created": state.created,
                            "model": state.model,
                            "choices": [{
                                "index": 0,
                                "delta": {},
                                "finish_reason": finish_reason
                            }]
                        });

                        yield Ok(bytes::Bytes::from(format!("data: {}\n\n", chunk)));
                    }
                }
                Ok(None) => break,
                Err(e) => {
                    yield Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e.to_string(),
                    ));
                    break;
                }
            }
        }

        // Send [DONE] at the end
        yield Ok(bytes::Bytes::from("data: [DONE]\n\n"));
    }
}

/// Handle non-streaming Bedrock request using the AWS SDK.
///
/// Auth priority: bearer token → IAM credentials → environment credentials
pub async fn handle_bedrock_sdk_non_streaming(
    model: &str,
    body: &Bytes,
    api_key: Option<&str>,
    aws_access_key_id: Option<&str>,
    aws_secret_access_key: Option<&str>,
    region: &str,
) -> Result<(http::StatusCode, http::HeaderMap, axum::body::Body)> {
    let openai_req: OpenAIRequest = serde_json::from_slice(body)
        .map_err(|e| Error::internal_err(format!("Failed to parse OpenAI request: {}", e)))?;

    // Create Bedrock client using shared helper
    let bedrock_client = create_bedrock_client(
        api_key,
        aws_access_key_id,
        aws_secret_access_key,
        region,
        "non-streaming",
    )
    .await?;

    // Convert messages using shared conversion
    let (bedrock_messages, system_prompts) =
        windmill_common::ai_bedrock::openai_messages_to_bedrock(&openai_req.messages)?;

    // Build inference configuration
    let inference_config = windmill_common::ai_bedrock::create_inference_config(
        openai_req.temperature,
        openai_req.max_tokens,
    );

    // Convert tools using shared helper
    let tool_config = build_tool_config_from_request(
        openai_req.tools.as_deref(),
        openai_req.tool_choice.as_ref(),
    )?;

    // Build the SDK request (non-streaming)
    let mut request_builder = bedrock_client
        .client()
        .converse()
        .model_id(model)
        .set_messages(Some(bedrock_messages));

    if !system_prompts.is_empty() {
        request_builder = request_builder.set_system(Some(system_prompts));
    }

    if let Some(config) = inference_config {
        request_builder = request_builder.inference_config(config);
    }

    if let Some(config) = tool_config {
        request_builder = request_builder.set_tool_config(Some(config));
    }

    // Send the request
    tracing::debug!("Bedrock SDK non-streaming: sending converse request");
    let response = request_builder.send().await.map_err(|e| {
        let error_msg = format!(
            "Bedrock SDK non-streaming error: {}",
            format_bedrock_error(&e)
        );
        tracing::error!("Bedrock SDK non-streaming failed: {}", error_msg);
        Error::internal_err(error_msg)
    })?;
    tracing::debug!(
        "Bedrock SDK non-streaming: response received, stop_reason={}",
        response.stop_reason().as_str()
    );

    // Convert response to OpenAI format
    let id = format!("chatcmpl-{}", uuid::Uuid::new_v4().simple());
    let created = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Extract stop reason
    let stop_reason = response.stop_reason().as_str();
    let finish_reason = match stop_reason {
        "end_turn" => "stop",
        "max_tokens" => "length",
        "tool_use" => "tool_calls",
        "stop_sequence" => "stop",
        "guardrail_intervened" | "content_filtered" => "content_filter",
        _ => "stop",
    };

    // Extract message content
    let mut text_content = String::new();
    let mut tool_calls: Vec<OpenAIToolCall> = Vec::new();

    if let Some(output) = response.output() {
        if let aws_sdk_bedrockruntime::types::ConverseOutput::Message(message) = output {
            for block in message.content() {
                match block {
                    aws_sdk_bedrockruntime::types::ContentBlock::Text(text) => {
                        text_content.push_str(text);
                    }
                    aws_sdk_bedrockruntime::types::ContentBlock::ToolUse(tool_use) => {
                        // Convert Document back to JSON string
                        let input_json = document_to_json(tool_use.input());
                        tool_calls.push(OpenAIToolCall {
                            id: tool_use.tool_use_id().to_string(),
                            function: windmill_common::ai_bedrock::OpenAIFunction {
                                name: tool_use.name().to_string(),
                                arguments: serde_json::to_string(&input_json).unwrap_or_default(),
                            },
                            r#type: "function".to_string(),
                            extra_content: None,
                        });
                    }
                    _ => {}
                }
            }
        }
    }

    // Build the message
    let message = if !tool_calls.is_empty() {
        serde_json::json!({
            "role": "assistant",
            "content": if text_content.is_empty() { serde_json::Value::Null } else { serde_json::Value::String(text_content) },
            "tool_calls": tool_calls
        })
    } else {
        serde_json::json!({
            "role": "assistant",
            "content": text_content
        })
    };

    // Extract usage information
    let usage = if let Some(usage_data) = response.usage() {
        serde_json::json!({
            "prompt_tokens": usage_data.input_tokens(),
            "completion_tokens": usage_data.output_tokens(),
            "total_tokens": usage_data.total_tokens()
        })
    } else {
        serde_json::json!({
            "prompt_tokens": 0,
            "completion_tokens": 0,
            "total_tokens": 0
        })
    };

    // Build OpenAI-format response
    let openai_resp = serde_json::json!({
        "id": id,
        "object": "chat.completion",
        "created": created,
        "model": model,
        "choices": [{
            "index": 0,
            "message": message,
            "finish_reason": finish_reason
        }],
        "usage": usage
    });

    let response_body = serde_json::to_vec(&openai_resp)
        .map_err(|e| Error::internal_err(format!("Failed to serialize OpenAI response: {}", e)))?;

    let mut response_headers = http::HeaderMap::new();
    response_headers.insert("content-type", "application/json".parse().unwrap());

    Ok((
        http::StatusCode::OK,
        response_headers,
        axum::body::Body::from(response_body),
    ))
}

/// Convert AWS Smithy Document to serde_json::Value
fn document_to_json(doc: &aws_smithy_types::Document) -> serde_json::Value {
    match doc {
        aws_smithy_types::Document::Object(map) => {
            let mut json_map = serde_json::Map::new();
            for (k, v) in map {
                json_map.insert(k.clone(), document_to_json(v));
            }
            serde_json::Value::Object(json_map)
        }
        aws_smithy_types::Document::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(document_to_json).collect())
        }
        aws_smithy_types::Document::Number(num) => match num {
            aws_smithy_types::Number::PosInt(n) => serde_json::Value::Number((*n).into()),
            aws_smithy_types::Number::NegInt(n) => serde_json::Value::Number((*n).into()),
            aws_smithy_types::Number::Float(f) => serde_json::json!(*f),
        },
        aws_smithy_types::Document::String(s) => serde_json::Value::String(s.clone()),
        aws_smithy_types::Document::Bool(b) => serde_json::Value::Bool(*b),
        aws_smithy_types::Document::Null => serde_json::Value::Null,
    }
}
