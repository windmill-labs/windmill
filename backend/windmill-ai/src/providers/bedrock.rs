//! AWS Bedrock provider for AI requests.
//!
//! Uses shared SDK code from windmill_ai::ai_bedrock for:
//! - BedrockClient (SDK wrapper with auth)
//! - Message conversion (OpenAI format -> Bedrock format)
//! - Stream event parsing
//! - Helper utilities

use crate::{
    ai_bedrock::{
        bedrock_model_supports_prompt_caching, bedrock_stream_event_is_block_stop,
        bedrock_stream_event_to_reasoning_delta, bedrock_stream_event_to_text,
        bedrock_stream_event_to_tool_delta, bedrock_stream_event_to_tool_delta_with_block_index,
        bedrock_stream_event_to_tool_start, bedrock_stream_event_to_tool_start_with_block_index,
        build_tool_config, create_inference_config, format_bedrock_error, json_to_document,
        openai_messages_to_bedrock, streaming_tool_calls_to_openai, BearerTokenProvider,
        BedrockClient, StreamingToolCall,
    },
    ai_providers::USE_ENV_REGION,
    ai_types::{
        BedrockExtraContent, ExtraContent, OpenAIFunction, OpenAIToolCall, ToolDefFunction,
    },
    image_handler::prepare_messages_for_api,
    proxy::ProxyBuildArgs,
    query_builder::{ParsedResponse, StreamEventSink},
    types::{OpenAIMessage, StreamingEvent, TokenUsage, ToolDef},
};
use bytes::Bytes;
use futures::{stream::BoxStream, StreamExt};
use http::{HeaderMap, Method, StatusCode};
use serde::Deserialize;
use std::collections::HashMap;
use windmill_common::{client::AuthedClient, error::Error};

// ============================================================================
// Native Proxy Execution
// ============================================================================

/// OpenAI-format request body for Bedrock SDK proxy handlers.
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
    /// Anthropic effort token from the chat reasoning setting (e.g. `low`,
    /// `high`, `max`). Enables adaptive thinking via
    /// `additionalModelRequestFields` — see [`bedrock_thinking_fields`].
    #[serde(default)]
    reasoning_effort: Option<String>,
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

#[derive(Deserialize, Debug)]
struct BedrockProxyChatRequest {
    model: String,
    #[serde(default)]
    stream: bool,
}

enum BedrockAuthConfig {
    BearerToken(String),
    IamCredentials {
        access_key_id: String,
        secret_access_key: String,
        session_token: Option<String>,
    },
    Environment,
}

pub enum BedrockProxyResponseBody {
    Fixed(Bytes),
    Stream(BoxStream<'static, std::result::Result<Bytes, std::io::Error>>),
}

pub struct BedrockProxyResponse {
    pub status_code: StatusCode,
    pub headers: HeaderMap,
    pub body: BedrockProxyResponseBody,
}

/// Handle a workspace Bedrock proxy request through the AWS SDK.
///
/// The API still owns credential resolution, route authorization, auditing, and
/// cache behavior. This helper owns Bedrock-specific control-plane and
/// OpenAI-compatible Converse transformations.
pub async fn handle_bedrock_proxy(
    args: &ProxyBuildArgs<'_>,
) -> Result<BedrockProxyResponse, Error> {
    let region = args.credentials.region.as_deref().unwrap_or(USE_ENV_REGION);

    if *args.method == Method::GET {
        return match args.path {
            "foundation-models" => list_foundation_models(args, region).await,
            "inference-profiles" => list_inference_profiles(args, region).await,
            _ => Err(Error::BadRequest(format!(
                "Unsupported AWS Bedrock proxy path: {}",
                args.path
            ))),
        };
    }

    if *args.method != Method::POST {
        return Err(Error::BadRequest(format!(
            "Unsupported AWS Bedrock proxy method: {}",
            args.method
        )));
    }

    let request: BedrockProxyChatRequest = serde_json::from_slice(args.body)
        .map_err(|e| Error::internal_err(format!("Failed to parse request body: {}", e)))?;

    if request.stream {
        handle_bedrock_sdk_streaming(&request.model, args.body, args, region).await
    } else {
        handle_bedrock_sdk_non_streaming(&request.model, args.body, args, region).await
    }
}

fn determine_auth_config(
    api_key: Option<&str>,
    aws_access_key_id: Option<&str>,
    aws_secret_access_key: Option<&str>,
    aws_session_token: Option<&str>,
) -> BedrockAuthConfig {
    if let Some(key) = api_key.filter(|k| !k.is_empty()) {
        BedrockAuthConfig::BearerToken(key.to_string())
    } else if let (Some(access_key_id), Some(secret_access_key)) = (
        aws_access_key_id.filter(|s| !s.is_empty()),
        aws_secret_access_key.filter(|s| !s.is_empty()),
    ) {
        BedrockAuthConfig::IamCredentials {
            access_key_id: access_key_id.to_string(),
            secret_access_key: secret_access_key.to_string(),
            session_token: aws_session_token
                .filter(|token| !token.is_empty())
                .map(str::to_string),
        }
    } else {
        BedrockAuthConfig::Environment
    }
}

async fn create_bedrock_client(
    args: &ProxyBuildArgs<'_>,
    region: &str,
) -> Result<BedrockClient, Error> {
    match determine_auth_config(
        args.credentials.api_key.as_deref(),
        args.credentials.aws_access_key_id.as_deref(),
        args.credentials.aws_secret_access_key.as_deref(),
        args.credentials.aws_session_token.as_deref(),
    ) {
        BedrockAuthConfig::BearerToken(key) => BedrockClient::from_bearer_token(key, region).await,
        BedrockAuthConfig::IamCredentials { access_key_id, secret_access_key, session_token } => {
            BedrockClient::from_credentials(access_key_id, secret_access_key, session_token, region)
                .await
        }
        BedrockAuthConfig::Environment => BedrockClient::from_env(region).await,
    }
}

fn build_tool_config_from_request(
    tools: Option<&[OpenAIToolDef]>,
    tool_choice: Option<&serde_json::Value>,
    enable_prompt_caching: bool,
) -> Result<Option<aws_sdk_bedrockruntime::types::ToolConfiguration>, Error> {
    if let Some(tools) = tools {
        let tool_defs: Vec<ToolDef> = tools
            .iter()
            .map(|t| ToolDef {
                r#type: "function".to_string(),
                function: ToolDefFunction {
                    name: t.function.name.clone(),
                    description: t.function.description.clone(),
                    parameters: Box::from(
                        serde_json::value::RawValue::from_string(
                            serde_json::to_string(
                                &t.function
                                    .parameters
                                    .clone()
                                    .unwrap_or(serde_json::json!({})),
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

        let force_tool_use = tool_choice
            .map(|tc| tc == "required" || tc.as_str() == Some("required"))
            .unwrap_or(false);

        build_tool_config(Some(&tool_defs), force_tool_use, enable_prompt_caching)
    } else {
        Ok(None)
    }
}

async fn create_bedrock_control_client(
    args: &ProxyBuildArgs<'_>,
    region: &str,
) -> Result<aws_sdk_bedrock::Client, Error> {
    use aws_config::BehaviorVersion;

    let region_provider = aws_sdk_bedrock::config::Region::new(region.to_string());

    match determine_auth_config(
        args.credentials.api_key.as_deref(),
        args.credentials.aws_access_key_id.as_deref(),
        args.credentials.aws_secret_access_key.as_deref(),
        args.credentials.aws_session_token.as_deref(),
    ) {
        BedrockAuthConfig::BearerToken(key) => {
            let config = aws_sdk_bedrock::config::Builder::new()
                .region(region_provider)
                .behavior_version(BehaviorVersion::latest())
                .token_provider(BearerTokenProvider::new(key))
                .build();
            Ok(aws_sdk_bedrock::Client::from_conf(config))
        }
        BedrockAuthConfig::IamCredentials { access_key_id, secret_access_key, session_token } => {
            let credentials = aws_credential_types::Credentials::new(
                access_key_id,
                secret_access_key,
                session_token,
                None,
                "windmill",
            );
            let config = aws_sdk_bedrock::config::Builder::new()
                .region(region_provider)
                .behavior_version(BehaviorVersion::latest())
                .credentials_provider(credentials)
                .build();
            Ok(aws_sdk_bedrock::Client::from_conf(config))
        }
        BedrockAuthConfig::Environment => {
            let config = aws_config::defaults(BehaviorVersion::latest())
                .region(region_provider)
                .load()
                .await;
            Ok(aws_sdk_bedrock::Client::new(&config))
        }
    }
}

async fn list_foundation_models(
    args: &ProxyBuildArgs<'_>,
    region: &str,
) -> Result<BedrockProxyResponse, Error> {
    let client = create_bedrock_control_client(args, region).await?;

    let response = client
        .list_foundation_models()
        .send()
        .await
        .map_err(|e| Error::internal_err(format!("Failed to list foundation models: {}", e)))?;

    let models: Vec<serde_json::Value> = response
        .model_summaries()
        .iter()
        .map(|m| {
            serde_json::json!({
                "modelId": m.model_id(),
                "modelName": m.model_name(),
                "providerName": m.provider_name(),
                "modelArn": m.model_arn(),
                "inputModalities": m.input_modalities().iter().map(|i| i.as_str()).collect::<Vec<_>>(),
                "outputModalities": m.output_modalities().iter().map(|o| o.as_str()).collect::<Vec<_>>(),
                "responseStreamingSupported": m.response_streaming_supported(),
                "inferenceTypesSupported": m.inference_types_supported().iter().map(|i| i.as_str()).collect::<Vec<_>>(),
            })
        })
        .collect();

    let body = serde_json::to_vec(&serde_json::json!({ "modelSummaries": models }))
        .map_err(|e| Error::internal_err(format!("Failed to serialize response: {}", e)))?;

    Ok(BedrockProxyResponse {
        status_code: StatusCode::OK,
        headers: json_response_headers(),
        body: BedrockProxyResponseBody::Fixed(Bytes::from(body)),
    })
}

async fn list_inference_profiles(
    args: &ProxyBuildArgs<'_>,
    region: &str,
) -> Result<BedrockProxyResponse, Error> {
    let client = create_bedrock_control_client(args, region).await?;

    let response =
        client.list_inference_profiles().send().await.map_err(|e| {
            Error::internal_err(format!("Failed to list inference profiles: {}", e))
        })?;

    let profiles: Vec<serde_json::Value> = response
        .inference_profile_summaries()
        .iter()
        .map(|p| {
            serde_json::json!({
                "inferenceProfileId": p.inference_profile_id(),
                "inferenceProfileName": p.inference_profile_name(),
                "inferenceProfileArn": p.inference_profile_arn(),
                "description": p.description(),
                "status": p.status().as_str(),
                "type": p.r#type().as_str(),
            })
        })
        .collect();

    let body = serde_json::to_vec(&serde_json::json!({ "inferenceProfileSummaries": profiles }))
        .map_err(|e| Error::internal_err(format!("Failed to serialize response: {}", e)))?;

    Ok(BedrockProxyResponse {
        status_code: StatusCode::OK,
        headers: json_response_headers(),
        body: BedrockProxyResponseBody::Fixed(Bytes::from(body)),
    })
}

async fn handle_bedrock_sdk_streaming(
    model: &str,
    body: &[u8],
    args: &ProxyBuildArgs<'_>,
    region: &str,
) -> Result<BedrockProxyResponse, Error> {
    let openai_req: OpenAIRequest = serde_json::from_slice(body)
        .map_err(|e| Error::internal_err(format!("Failed to parse OpenAI request: {}", e)))?;

    let bedrock_client = create_bedrock_client(args, region).await?;
    let enable_prompt_caching = bedrock_model_supports_prompt_caching(model);
    let (bedrock_messages, system_prompts) =
        openai_messages_to_bedrock(&openai_req.messages, enable_prompt_caching)?;
    // Adaptive thinking rejects sampling params; drop temperature when reasoning is on.
    let temperature = openai_req
        .reasoning_effort
        .is_none()
        .then_some(openai_req.temperature)
        .flatten();
    let inference_config = create_inference_config(temperature, openai_req.max_tokens);
    let tool_config = build_tool_config_from_request(
        openai_req.tools.as_deref(),
        openai_req.tool_choice.as_ref(),
        enable_prompt_caching,
    )?;

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

    if let Some(effort) = openai_req.reasoning_effort.as_deref() {
        request_builder =
            request_builder.additional_model_request_fields(bedrock_thinking_fields(effort));
    }

    tracing::debug!("Bedrock SDK streaming: sending converse_stream request");
    let stream_output = request_builder.send().await.map_err(|e| {
        let error_msg = format!("Bedrock SDK streaming error: {}", format_bedrock_error(&e));
        tracing::error!("Bedrock SDK streaming failed: {}", error_msg);
        Error::internal_err(error_msg)
    })?;
    tracing::debug!("Bedrock SDK streaming: stream established successfully");

    Ok(BedrockProxyResponse {
        status_code: StatusCode::OK,
        headers: event_stream_response_headers(),
        body: BedrockProxyResponseBody::Stream(
            sdk_stream_to_sse(stream_output.stream, model.to_string()).boxed(),
        ),
    })
}

/// Build the Converse `additionalModelRequestFields` enabling Claude adaptive
/// thinking at the given effort. `display: summarized` is billing-neutral on
/// Anthropic models and matches the direct-Anthropic chat path, which renders
/// summarized thinking in the UI.
fn bedrock_thinking_fields(effort: &str) -> aws_smithy_types::Document {
    json_to_document(serde_json::json!({
        "thinking": { "type": "adaptive", "display": "summarized" },
        "output_config": { "effort": effort }
    }))
}

pub fn sdk_stream_to_sse(
    stream: aws_sdk_bedrockruntime::primitives::event_stream::EventReceiver<
        aws_sdk_bedrockruntime::types::ConverseStreamOutput,
        aws_sdk_bedrockruntime::types::error::ConverseStreamOutputError,
    >,
    model: String,
) -> impl futures::Stream<Item = std::result::Result<Bytes, std::io::Error>> + Send {
    let id = format!("chatcmpl-{}", uuid::Uuid::new_v4().simple());
    let created = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    async_stream::stream! {
        let mut stream = stream;
        let mut state = BedrockSseStreamState::new(id, model, created);

        loop {
            match stream.recv().await {
                Ok(Some(event)) => {
                    for chunk in bedrock_sse_chunks_for_event(&event, &mut state) {
                        yield Ok(chunk);
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

        yield Ok(Bytes::from("data: [DONE]\n\n"));
    }
}

#[derive(Debug)]
struct BedrockSseStreamState {
    id: String,
    model: String,
    created: u64,
    tool_calls: HashMap<usize, (String, String, String)>,
    tool_block_indexes: HashMap<usize, usize>,
    next_tool_index: usize,
    /// Claude reasoning block accumulated from `ReasoningContent` deltas
    /// (text + signature, or redacted bytes). Attached to the first tool call
    /// of the turn so the frontend round-trips it for replay.
    reasoning: Option<BedrockExtraContent>,
    reasoning_attached: bool,
}

impl BedrockSseStreamState {
    fn new(id: String, model: String, created: u64) -> Self {
        Self {
            id,
            model,
            created,
            tool_calls: HashMap::new(),
            tool_block_indexes: HashMap::new(),
            next_tool_index: 0,
            reasoning: None,
            reasoning_attached: false,
        }
    }
}

fn bedrock_sse_chunks_for_event(
    event: &aws_sdk_bedrockruntime::types::ConverseStreamOutput,
    state: &mut BedrockSseStreamState,
) -> Vec<Bytes> {
    let mut chunks = Vec::new();

    if let Some(reasoning_text) = accumulate_reasoning_delta(event, state) {
        let chunk = serde_json::json!({
            "id": state.id,
            "object": "chat.completion.chunk",
            "created": state.created,
            "model": state.model,
            "choices": [{
                "index": 0,
                "delta": {
                    "reasoning_content": reasoning_text
                },
                "finish_reason": serde_json::Value::Null
            }]
        });

        chunks.push(Bytes::from(format!("data: {}\n\n", chunk)));
    }

    if let Some((block_index, tool_call)) =
        bedrock_stream_event_to_tool_start_with_block_index(event)
    {
        let index = state.next_tool_index;
        state.next_tool_index += 1;
        state.tool_block_indexes.insert(block_index, index);
        state.tool_calls.insert(
            index,
            (tool_call.id.clone(), tool_call.name.clone(), String::new()),
        );

        let mut tool_call_json = serde_json::json!({
            "index": index,
            "id": tool_call.id,
            "type": "function",
            "function": {
                "name": tool_call.name,
                "arguments": ""
            }
        });
        // Attach the turn's reasoning block to the first tool call so the
        // frontend echoes it back and the next request can replay it before
        // toolUse (required by Claude when thinking is enabled).
        if !state.reasoning_attached {
            if let Some(reasoning) = state.reasoning.as_ref() {
                tool_call_json["extra_content"] = serde_json::json!({ "bedrock": reasoning });
                state.reasoning_attached = true;
            }
        }

        let chunk = serde_json::json!({
            "id": state.id,
            "object": "chat.completion.chunk",
            "created": state.created,
            "model": state.model,
            "choices": [{
                "index": 0,
                "delta": {
                    "tool_calls": [tool_call_json]
                },
                "finish_reason": serde_json::Value::Null
            }]
        });

        chunks.push(Bytes::from(format!("data: {}\n\n", chunk)));
    }

    if let Some(text) = bedrock_stream_event_to_text(event) {
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

        chunks.push(Bytes::from(format!("data: {}\n\n", chunk)));
    }

    if let Some((block_index, input_delta)) =
        bedrock_stream_event_to_tool_delta_with_block_index(event)
    {
        if let Some(index) = state.tool_block_indexes.get(&block_index).copied() {
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

                chunks.push(Bytes::from(format!("data: {}\n\n", chunk)));
            }
        }
    }

    if let aws_sdk_bedrockruntime::types::ConverseStreamOutput::MessageStop(stop) = event {
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

        chunks.push(Bytes::from(format!("data: {}\n\n", chunk)));
    }

    chunks
}

/// Fold a `ReasoningContent` stream delta into the state's pending reasoning
/// block. Returns the text delta (for a `reasoning_content` SSE chunk) when the
/// event carried readable reasoning text. Shares the worker path's folding so
/// the two never drift.
fn accumulate_reasoning_delta(
    event: &aws_sdk_bedrockruntime::types::ConverseStreamOutput,
    state: &mut BedrockSseStreamState,
) -> Option<String> {
    bedrock_stream_event_to_reasoning_delta(event, &mut state.reasoning)
}

async fn handle_bedrock_sdk_non_streaming(
    model: &str,
    body: &[u8],
    args: &ProxyBuildArgs<'_>,
    region: &str,
) -> Result<BedrockProxyResponse, Error> {
    let openai_req: OpenAIRequest = serde_json::from_slice(body)
        .map_err(|e| Error::internal_err(format!("Failed to parse OpenAI request: {}", e)))?;

    let bedrock_client = create_bedrock_client(args, region).await?;
    let enable_prompt_caching = bedrock_model_supports_prompt_caching(model);
    let (bedrock_messages, system_prompts) =
        openai_messages_to_bedrock(&openai_req.messages, enable_prompt_caching)?;
    // Adaptive thinking rejects sampling params; drop temperature when reasoning is on.
    let temperature = openai_req
        .reasoning_effort
        .is_none()
        .then_some(openai_req.temperature)
        .flatten();
    let inference_config = create_inference_config(temperature, openai_req.max_tokens);
    let tool_config = build_tool_config_from_request(
        openai_req.tools.as_deref(),
        openai_req.tool_choice.as_ref(),
        enable_prompt_caching,
    )?;

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

    if let Some(effort) = openai_req.reasoning_effort.as_deref() {
        request_builder =
            request_builder.additional_model_request_fields(bedrock_thinking_fields(effort));
    }

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

    let id = format!("chatcmpl-{}", uuid::Uuid::new_v4().simple());
    let created = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let stop_reason = response.stop_reason().as_str();
    let finish_reason = match stop_reason {
        "end_turn" => "stop",
        "max_tokens" => "length",
        "tool_use" => "tool_calls",
        "stop_sequence" => "stop",
        "guardrail_intervened" | "content_filtered" => "content_filter",
        _ => "stop",
    };

    let mut text_content = String::new();
    let mut tool_calls: Vec<OpenAIToolCall> = Vec::new();
    let mut reasoning: Option<BedrockExtraContent> = None;

    if let Some(aws_sdk_bedrockruntime::types::ConverseOutput::Message(message)) = response.output()
    {
        for block in message.content() {
            match block {
                aws_sdk_bedrockruntime::types::ContentBlock::Text(text) => {
                    text_content.push_str(text);
                }
                aws_sdk_bedrockruntime::types::ContentBlock::ReasoningContent(rc) => {
                    let entry = reasoning.get_or_insert_with(Default::default);
                    match rc {
                        aws_sdk_bedrockruntime::types::ReasoningContentBlock::ReasoningText(rt) => {
                            entry
                                .reasoning_text
                                .get_or_insert_with(String::new)
                                .push_str(rt.text());
                            if let Some(signature) = rt.signature() {
                                entry.signature = Some(signature.to_string());
                            }
                        }
                        aws_sdk_bedrockruntime::types::ReasoningContentBlock::RedactedContent(
                            blob,
                        ) => {
                            entry.redacted_content = Some(base64::Engine::encode(
                                &base64::engine::general_purpose::STANDARD,
                                blob.as_ref(),
                            ));
                        }
                        _ => {}
                    }
                }
                aws_sdk_bedrockruntime::types::ContentBlock::ToolUse(tool_use) => {
                    let input_json = document_to_json(tool_use.input());
                    tool_calls.push(OpenAIToolCall {
                        id: tool_use.tool_use_id().to_string(),
                        function: OpenAIFunction {
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

    // Attach the turn's reasoning block to the first tool call so the frontend
    // round-trips it for replay (required by Claude when thinking is enabled).
    if let (Some(reasoning_block), Some(first_tool_call)) =
        (reasoning.as_ref(), tool_calls.first_mut())
    {
        first_tool_call.extra_content =
            Some(ExtraContent { bedrock: Some(reasoning_block.clone()), ..Default::default() });
    }
    let reasoning_content = reasoning
        .as_ref()
        .and_then(|r| r.reasoning_text.clone())
        .filter(|t| !t.is_empty());

    let mut message = if !tool_calls.is_empty() {
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
    if let Some(reasoning_content) = reasoning_content {
        message["reasoning_content"] = serde_json::Value::String(reasoning_content);
    }

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

    let body = serde_json::to_vec(&openai_resp)
        .map_err(|e| Error::internal_err(format!("Failed to serialize OpenAI response: {}", e)))?;

    Ok(BedrockProxyResponse {
        status_code: StatusCode::OK,
        headers: json_response_headers(),
        body: BedrockProxyResponseBody::Fixed(Bytes::from(body)),
    })
}

fn json_response_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert("content-type", "application/json".parse().unwrap());
    headers
}

fn event_stream_response_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert("content-type", "text/event-stream".parse().unwrap());
    headers.insert("cache-control", "no-cache".parse().unwrap());
    headers.insert("connection", "keep-alive".parse().unwrap());
    headers
}

fn document_to_json(doc: &aws_smithy_types::Document) -> serde_json::Value {
    match doc {
        aws_smithy_types::Document::Object(map) => {
            let mut json_map = serde_json::Map::new();
            for (key, value) in map {
                json_map.insert(key.clone(), document_to_json(value));
            }
            serde_json::Value::Object(json_map)
        }
        aws_smithy_types::Document::Array(values) => {
            serde_json::Value::Array(values.iter().map(document_to_json).collect())
        }
        aws_smithy_types::Document::Number(number) => match number {
            aws_smithy_types::Number::PosInt(number) => serde_json::Value::Number((*number).into()),
            aws_smithy_types::Number::NegInt(number) => serde_json::Value::Number((*number).into()),
            aws_smithy_types::Number::Float(number) => serde_json::json!(*number),
        },
        aws_smithy_types::Document::String(value) => serde_json::Value::String(value.clone()),
        aws_smithy_types::Document::Bool(value) => serde_json::Value::Bool(*value),
        aws_smithy_types::Document::Null => serde_json::Value::Null,
    }
}

// ============================================================================
// Query Builder
// ============================================================================

#[derive(Default)]
pub struct BedrockQueryBuilder;

impl BedrockQueryBuilder {
    /// Execute Bedrock request (streaming or non-streaming)
    pub async fn execute_request(
        &self,
        messages: &[OpenAIMessage],
        tools: Option<&[ToolDef]>,
        model: &str,
        temperature: Option<f32>,
        reasoning_effort: Option<&str>,
        max_tokens: Option<u32>,
        api_key: &str,
        region: &str,
        stream_event_sink: Option<Box<dyn StreamEventSink>>,
        client: &AuthedClient,
        workspace_id: &str,
        structured_output_tool_name: Option<&str>,
        aws_access_key_id: Option<&str>,
        aws_secret_access_key: Option<&str>,
        aws_session_token: Option<&str>,
    ) -> Result<ParsedResponse, Error> {
        let bedrock_client = if !api_key.is_empty() {
            BedrockClient::from_bearer_token(api_key.to_string(), region).await?
        } else if let (Some(access_key_id), Some(secret_access_key)) =
            (aws_access_key_id, aws_secret_access_key)
        {
            BedrockClient::from_credentials(
                access_key_id.to_string(),
                secret_access_key.to_string(),
                aws_session_token.map(str::to_string),
                region,
            )
            .await?
        } else {
            BedrockClient::from_env(region).await?
        };

        // Prepare messages: convert S3Objects to ImageUrls by downloading from S3
        let prepared_messages = prepare_messages_for_api(messages, client, workspace_id).await?;

        // Convert messages to Bedrock format (separates system prompts)
        let enable_prompt_caching = bedrock_model_supports_prompt_caching(model);
        let (bedrock_messages, system_prompts) =
            openai_messages_to_bedrock(&prepared_messages, enable_prompt_caching)?;

        // Adaptive thinking rejects sampling params; drop temperature when reasoning is on.
        let temperature = reasoning_effort.is_none().then_some(temperature).flatten();

        // Build inference configuration using shared helper
        let inference_config = create_inference_config(temperature, max_tokens.map(|t| t as i32));

        // Build tool configuration with optional ToolChoice
        let tool_config = build_tool_config(
            tools,
            structured_output_tool_name.is_some(),
            enable_prompt_caching,
        )?;

        self.execute_converse_stream(
            &bedrock_client,
            model,
            bedrock_messages,
            system_prompts,
            inference_config,
            tool_config,
            reasoning_effort,
            stream_event_sink,
        )
        .await
    }

    /// Execute streaming Bedrock request using shared stream parsing functions
    async fn execute_converse_stream(
        &self,
        bedrock_client: &BedrockClient,
        model: &str,
        bedrock_messages: Vec<aws_sdk_bedrockruntime::types::Message>,
        system_prompts: Vec<aws_sdk_bedrockruntime::types::SystemContentBlock>,
        inference_config: Option<aws_sdk_bedrockruntime::types::InferenceConfiguration>,
        tool_config: Option<aws_sdk_bedrockruntime::types::ToolConfiguration>,
        reasoning_effort: Option<&str>,
        stream_event_sink: Option<Box<dyn StreamEventSink>>,
    ) -> Result<ParsedResponse, Error> {
        tracing::debug!(
            "Worker Bedrock: executing converse_stream, messages={}, system_prompts={}, has_tools={}",
            bedrock_messages.len(),
            system_prompts.len(),
            tool_config.is_some()
        );

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

        if let Some(effort) = reasoning_effort {
            request_builder =
                request_builder.additional_model_request_fields(bedrock_thinking_fields(effort));
        }

        let mut stream = request_builder
            .send()
            .await
            .map_err(|e| {
                let error_msg =
                    format!("Bedrock streaming API error: {}", format_bedrock_error(&e));
                tracing::error!("Worker Bedrock: {}", error_msg);
                Error::internal_err(error_msg)
            })?
            .stream;

        tracing::debug!("Worker Bedrock: stream established, processing events");

        let mut accumulated_text = String::new();
        let mut events_str = String::new();
        let mut accumulated_tool_calls: HashMap<String, StreamingToolCall> = HashMap::new();
        let mut current_tool_use_id: Option<String> = None;
        let mut usage: Option<TokenUsage> = None;
        // Claude reasoning block for the turn (only populated when thinking is on),
        // attached to the first tool call for replay before toolUse.
        let mut reasoning: Option<crate::ai_types::BedrockExtraContent> = None;

        // Process stream events using shared parsing functions
        loop {
            match stream.recv().await {
                Ok(Some(event)) => {
                    // Fold reasoning deltas into the turn's reasoning block so it
                    // can be replayed before toolUse (required by Claude when
                    // thinking is enabled). No-op when thinking is off.
                    let _ = bedrock_stream_event_to_reasoning_delta(&event, &mut reasoning);

                    // Handle tool use start using shared parser
                    if let Some(tool_call) = bedrock_stream_event_to_tool_start(&event) {
                        current_tool_use_id = Some(tool_call.id.clone());
                        accumulated_tool_calls.insert(tool_call.id.clone(), tool_call);
                    }

                    // Handle text delta using shared parser
                    if let Some(text_delta) = bedrock_stream_event_to_text(&event) {
                        accumulated_text.push_str(&text_delta);
                        if let Some(processor) = stream_event_sink.as_ref() {
                            processor
                                .send(
                                    StreamingEvent::TokenDelta { content: text_delta },
                                    &mut events_str,
                                )
                                .await?;
                        }
                    }

                    // Handle tool use input delta using shared parser
                    if let Some(input_delta) = bedrock_stream_event_to_tool_delta(&event) {
                        if let Some(tool_id) = &current_tool_use_id {
                            if let Some(tool_call) = accumulated_tool_calls.get_mut(tool_id) {
                                tool_call.arguments.push_str(&input_delta);
                            }
                        }
                    }

                    // Handle content block stop using shared parser
                    if bedrock_stream_event_is_block_stop(&event) {
                        current_tool_use_id = None;
                    }

                    // Extract usage from Metadata event
                    if let aws_sdk_bedrockruntime::types::ConverseStreamOutput::Metadata(metadata) =
                        &event
                    {
                        if let Some(token_usage) = metadata.usage() {
                            usage = Some(
                                TokenUsage::new(
                                    Some(token_usage.input_tokens()),
                                    Some(token_usage.output_tokens()),
                                    Some(token_usage.total_tokens()),
                                )
                                .with_cache(
                                    token_usage
                                        .cache_read_input_tokens()
                                        .map(|v| i32::try_from(v).unwrap_or(i32::MAX)),
                                    token_usage
                                        .cache_write_input_tokens()
                                        .map(|v| i32::try_from(v).unwrap_or(i32::MAX)),
                                ),
                            );
                        }
                    }
                }
                Ok(None) => break,
                Err(e) => {
                    return Err(Error::internal_err(format!("Bedrock stream error: {}", e)));
                }
            }
        }

        // Send tool call events to stream processor
        if let Some(processor) = stream_event_sink.as_ref() {
            for tool_call in accumulated_tool_calls.values() {
                processor
                    .send(
                        StreamingEvent::ToolCallArguments {
                            call_id: tool_call.id.clone(),
                            function_name: tool_call.name.clone(),
                            arguments: tool_call.arguments.clone(),
                        },
                        &mut events_str,
                    )
                    .await?;
            }
        }

        let content = if accumulated_text.is_empty() {
            None
        } else {
            Some(accumulated_text)
        };

        let tool_calls = streaming_tool_calls_to_openai(
            accumulated_tool_calls.into_values().collect(),
            reasoning,
        );

        Ok(ParsedResponse::Text {
            content,
            tool_calls,
            events_str: if events_str.is_empty() {
                None
            } else {
                Some(events_str)
            },
            annotations: Vec::new(),
            used_websearch: false,
            usage,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aws_sdk_bedrockruntime::types::{
        ContentBlockDelta, ContentBlockDeltaEvent, ContentBlockStart, ContentBlockStartEvent,
        ContentBlockStopEvent, ConverseStreamOutput, ToolUseBlockDelta, ToolUseBlockStart,
    };

    fn sse_json(chunk: &Bytes) -> serde_json::Value {
        let chunk = std::str::from_utf8(chunk).expect("SSE chunk should be UTF-8");
        let payload = chunk
            .strip_prefix("data: ")
            .and_then(|chunk| chunk.strip_suffix("\n\n"))
            .expect("chunk should be SSE data");
        serde_json::from_str(payload).expect("chunk should contain JSON")
    }

    #[test]
    fn determine_auth_config_prioritizes_bearer_token() {
        let config = determine_auth_config(
            Some("bearer-token"),
            Some("AKIA123"),
            Some("secret"),
            Some("session-token"),
        );

        match config {
            BedrockAuthConfig::BearerToken(token) => assert_eq!(token, "bearer-token"),
            _ => panic!("expected bearer token auth config"),
        }
    }

    #[test]
    fn determine_auth_config_uses_iam_with_optional_session_token() {
        let config =
            determine_auth_config(None, Some("AKIA123"), Some("secret"), Some("session-token"));

        match config {
            BedrockAuthConfig::IamCredentials {
                access_key_id,
                secret_access_key,
                session_token,
            } => {
                assert_eq!(access_key_id, "AKIA123");
                assert_eq!(secret_access_key, "secret");
                assert_eq!(session_token.as_deref(), Some("session-token"));
            }
            _ => panic!("expected IAM auth config"),
        }
    }

    #[test]
    fn determine_auth_config_treats_empty_session_token_as_none() {
        let config = determine_auth_config(None, Some("AKIA123"), Some("secret"), Some(""));

        match config {
            BedrockAuthConfig::IamCredentials { session_token, .. } => {
                assert!(session_token.is_none());
            }
            _ => panic!("expected IAM auth config"),
        }
    }

    #[test]
    fn determine_auth_config_falls_back_to_environment() {
        let config = determine_auth_config(None, Some("AKIA123"), None, Some("session-token"));
        assert!(matches!(config, BedrockAuthConfig::Environment));
    }

    #[test]
    fn bedrock_sse_tool_indexes_ignore_text_block_stops() {
        let mut state =
            BedrockSseStreamState::new("chatcmpl-test".to_string(), "model".to_string(), 1);

        let text_delta = ConverseStreamOutput::ContentBlockDelta(
            ContentBlockDeltaEvent::builder()
                .content_block_index(0)
                .delta(ContentBlockDelta::Text("hello".to_string()))
                .build()
                .unwrap(),
        );
        assert_eq!(
            bedrock_sse_chunks_for_event(&text_delta, &mut state).len(),
            1
        );

        let text_stop = ConverseStreamOutput::ContentBlockStop(
            ContentBlockStopEvent::builder()
                .content_block_index(0)
                .build()
                .unwrap(),
        );
        assert!(bedrock_sse_chunks_for_event(&text_stop, &mut state).is_empty());

        let tool_start = ConverseStreamOutput::ContentBlockStart(
            ContentBlockStartEvent::builder()
                .content_block_index(1)
                .start(ContentBlockStart::ToolUse(
                    ToolUseBlockStart::builder()
                        .tool_use_id("call_1")
                        .name("lookup")
                        .build()
                        .unwrap(),
                ))
                .build()
                .unwrap(),
        );
        let start_chunks = bedrock_sse_chunks_for_event(&tool_start, &mut state);
        let start_json = sse_json(&start_chunks[0]);
        assert_eq!(
            start_json["choices"][0]["delta"]["tool_calls"][0]["index"],
            0
        );

        let tool_delta = ConverseStreamOutput::ContentBlockDelta(
            ContentBlockDeltaEvent::builder()
                .content_block_index(1)
                .delta(ContentBlockDelta::ToolUse(
                    ToolUseBlockDelta::builder()
                        .input("{\"city\":\"Paris\"}")
                        .build()
                        .unwrap(),
                ))
                .build()
                .unwrap(),
        );
        let delta_chunks = bedrock_sse_chunks_for_event(&tool_delta, &mut state);
        let delta_json = sse_json(&delta_chunks[0]);
        assert_eq!(
            delta_json["choices"][0]["delta"]["tool_calls"][0]["index"],
            0
        );
    }

    #[test]
    fn bedrock_thinking_fields_carry_adaptive_thinking_and_effort() {
        let fields = document_to_json(&bedrock_thinking_fields("xhigh"));
        assert_eq!(fields["thinking"]["type"], "adaptive");
        assert_eq!(fields["thinking"]["display"], "summarized");
        assert_eq!(fields["output_config"]["effort"], "xhigh");
    }

    fn reasoning_delta(
        block_index: i32,
        delta: aws_sdk_bedrockruntime::types::ReasoningContentBlockDelta,
    ) -> ConverseStreamOutput {
        ConverseStreamOutput::ContentBlockDelta(
            ContentBlockDeltaEvent::builder()
                .content_block_index(block_index)
                .delta(ContentBlockDelta::ReasoningContent(delta))
                .build()
                .unwrap(),
        )
    }

    #[test]
    fn bedrock_sse_streams_reasoning_and_attaches_block_to_first_tool_call() {
        use aws_sdk_bedrockruntime::types::ReasoningContentBlockDelta;

        let mut state =
            BedrockSseStreamState::new("chatcmpl-test".to_string(), "model".to_string(), 1);

        // Reasoning text streams as reasoning_content deltas.
        let chunks = bedrock_sse_chunks_for_event(
            &reasoning_delta(0, ReasoningContentBlockDelta::Text("let me ".to_string())),
            &mut state,
        );
        assert_eq!(
            sse_json(&chunks[0])["choices"][0]["delta"]["reasoning_content"],
            "let me "
        );
        bedrock_sse_chunks_for_event(
            &reasoning_delta(0, ReasoningContentBlockDelta::Text("think".to_string())),
            &mut state,
        );
        // Signature deltas accumulate silently (no chunk emitted).
        assert!(bedrock_sse_chunks_for_event(
            &reasoning_delta(
                0,
                ReasoningContentBlockDelta::Signature("sig-abc".to_string())
            ),
            &mut state,
        )
        .is_empty());

        // The first tool call carries the full reasoning block for replay.
        let tool_start = ConverseStreamOutput::ContentBlockStart(
            ContentBlockStartEvent::builder()
                .content_block_index(1)
                .start(ContentBlockStart::ToolUse(
                    ToolUseBlockStart::builder()
                        .tool_use_id("call_1")
                        .name("lookup")
                        .build()
                        .unwrap(),
                ))
                .build()
                .unwrap(),
        );
        let start_json = sse_json(&bedrock_sse_chunks_for_event(&tool_start, &mut state)[0]);
        let tool_call = &start_json["choices"][0]["delta"]["tool_calls"][0];
        assert_eq!(
            tool_call["extra_content"]["bedrock"]["reasoning_text"],
            "let me think"
        );
        assert_eq!(
            tool_call["extra_content"]["bedrock"]["signature"],
            "sig-abc"
        );

        // Subsequent tool calls don't repeat the block.
        let second_tool_start = ConverseStreamOutput::ContentBlockStart(
            ContentBlockStartEvent::builder()
                .content_block_index(2)
                .start(ContentBlockStart::ToolUse(
                    ToolUseBlockStart::builder()
                        .tool_use_id("call_2")
                        .name("lookup")
                        .build()
                        .unwrap(),
                ))
                .build()
                .unwrap(),
        );
        let second_json =
            sse_json(&bedrock_sse_chunks_for_event(&second_tool_start, &mut state)[0]);
        assert!(second_json["choices"][0]["delta"]["tool_calls"][0]
            .get("extra_content")
            .is_none());
    }

    #[test]
    fn bedrock_sse_accumulates_redacted_reasoning_bytes() {
        use aws_sdk_bedrockruntime::types::ReasoningContentBlockDelta;

        let mut state =
            BedrockSseStreamState::new("chatcmpl-test".to_string(), "model".to_string(), 1);
        for fragment in [b"ab".as_slice(), b"cd".as_slice()] {
            assert!(bedrock_sse_chunks_for_event(
                &reasoning_delta(
                    0,
                    ReasoningContentBlockDelta::RedactedContent(fragment.to_vec().into()),
                ),
                &mut state,
            )
            .is_empty());
        }

        let encoded = state.reasoning.unwrap().redacted_content.unwrap();
        let decoded =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, encoded).unwrap();
        assert_eq!(decoded, b"abcd");
    }
}
