use crate::db::{ApiAuthed, DB};

use axum::{body::Bytes, extract::Path, response::IntoResponse, routing::post, Extension, Router};
use bytes;
use futures;
use http::{HeaderMap, Method};
use quick_cache::sync::Cache;
use reqwest::{Client, RequestBuilder};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use std::collections::HashMap;
use uuid;
use windmill_audit::{audit_oss::audit_log, ActionKind};
use windmill_common::ai_providers::{AIProvider, ProviderConfig, ProviderModel, AZURE_API_VERSION};
use windmill_common::error::{to_anyhow, Error, Result};
use windmill_common::utils::configure_client;
use windmill_common::variables::get_variable_or_self;

lazy_static::lazy_static! {
    static ref HTTP_CLIENT: Client = configure_client(reqwest::ClientBuilder::new()
        .timeout(std::time::Duration::from_secs(60 * 5))
        .user_agent("windmill/beta"))
        .build().unwrap();

    static ref OPENAI_AZURE_BASE_PATH: Option<String> = std::env::var("OPENAI_AZURE_BASE_PATH").ok();

    pub static ref AI_REQUEST_CACHE: Cache<(String, AIProvider), ExpiringAIRequestConfig> = Cache::new(500);

    /// Parse AI_HTTP_HEADERS environment variable into a vector of (header_name, header_value) tuples
    /// Format: "header1: value1, header2: value2"
    static ref AI_HTTP_HEADERS: Vec<(String, String)> = {
        std::env::var("AI_HTTP_HEADERS")
            .ok()
            .map(|headers_str| {
                headers_str
                    .split(',')
                    .filter_map(|header| {
                        let parts: Vec<&str> = header.splitn(2, ':').collect();
                        if parts.len() == 2 {
                            let name = parts[0].trim().to_string();
                            let value = parts[1].trim().to_string();
                            if !name.is_empty() && !value.is_empty() {
                                Some((name, value))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    };
}

#[derive(Deserialize, Debug)]
struct AIOAuthResource {
    client_id: String,
    client_secret: String,
    token_url: String,
    user: Option<String>,
}

#[derive(Deserialize, Debug)]
struct AIStandardResource {
    #[serde(alias = "baseUrl")]
    base_url: Option<String>,
    #[serde(alias = "apiKey")]
    api_key: Option<String>,
    organization_id: Option<String>,
    region: Option<String>,
}

#[derive(Deserialize, Debug)]
struct OAuthTokens {
    access_token: String,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum AIResource {
    OAuth(AIOAuthResource),
    Standard(AIStandardResource),
}

#[derive(Deserialize, Clone, Debug)]
struct AIRequestConfig {
    pub base_url: String,
    pub api_key: Option<String>,
    pub access_token: Option<String>,
    pub organization_id: Option<String>,
    pub user: Option<String>,
}

impl AIRequestConfig {
    pub async fn new(
        provider: &AIProvider,
        db: &DB,
        w_id: &str,
        resource: AIResource,
    ) -> Result<Self> {
        let (api_key, access_token, organization_id, base_url, user) = match resource {
            AIResource::Standard(resource) => {
                let base_url = provider
                    .get_base_url(resource.base_url, resource.region, db)
                    .await?;
                let api_key = if let Some(api_key) = resource.api_key {
                    Some(get_variable_or_self(api_key, db, w_id).await?)
                } else {
                    None
                };
                let organization_id = if let Some(organization_id) = resource.organization_id {
                    Some(get_variable_or_self(organization_id, db, w_id).await?)
                } else {
                    None
                };

                (api_key, None, organization_id, base_url, None)
            }
            AIResource::OAuth(resource) => {
                let user = if let Some(user) = resource.user.clone() {
                    Some(get_variable_or_self(user, db, w_id).await?)
                } else {
                    None
                };
                let token = Self::get_token_using_oauth(resource, db, w_id).await?;
                let base_url = provider.get_base_url(None, None, db).await?;

                (None, Some(token), None, base_url, user)
            }
        };

        Ok(Self { base_url, organization_id, api_key, access_token, user })
    }

    async fn get_token_using_oauth(
        mut resource: AIOAuthResource,
        db: &DB,
        w_id: &str,
    ) -> Result<String> {
        resource.client_id = get_variable_or_self(resource.client_id, db, w_id).await?;
        resource.client_secret = get_variable_or_self(resource.client_secret, db, w_id).await?;
        resource.token_url = get_variable_or_self(resource.token_url, db, w_id).await?;
        let mut params = HashMap::new();
        params.insert("grant_type", "client_credentials");
        params.insert("scope", "https://cognitiveservices.azure.com/.default");
        let response = HTTP_CLIENT
            .post(resource.token_url)
            .form(&params)
            .basic_auth(resource.client_id, Some(resource.client_secret))
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|err| {
                Error::internal_err(format!(
                    "Failed to get access token using credentials flow: {}",
                    err
                ))
            })?;
        let response = response.json::<OAuthTokens>().await.map_err(|err| {
            Error::internal_err(format!(
                "Failed to parse access token from credentials flow: {}",
                err
            ))
        })?;
        Ok(response.access_token)
    }

    pub fn prepare_request(
        self,
        provider: &AIProvider,
        path: &str,
        method: Method,
        headers: HeaderMap,
        body: Bytes,
    ) -> Result<RequestBuilder> {
        let body = if let Some(user) = self.user {
            Self::add_user_to_body(body, user)?
        } else {
            body
        };

        let base_url = self.base_url.trim_end_matches('/');
        println!("HERE base_url: {}", base_url);

        let is_azure = provider.is_azure_openai(base_url);
        let is_anthropic = matches!(provider, AIProvider::Anthropic);
        let is_anthropic_sdk = headers.get("X-Anthropic-SDK").is_some();
        let is_bedrock = matches!(provider, AIProvider::AWSBedrock);

        // Handle AWS Bedrock transformation
        let (url, body) = if is_bedrock && method != Method::GET {
            let (model, transformed_body, is_streaming) = Self::transform_openai_to_bedrock(&body)?;
            let endpoint = if is_streaming {
                "converse-stream"
            } else {
                "converse"
            };
            let bedrock_url = format!("{}/model/{}/{}", base_url, model, endpoint);
            (bedrock_url, transformed_body)
        } else if is_bedrock && (path == "foundation-models" || path == "inference-profiles") {
            // AWS Bedrock foundation-models and inference-profiles endpoints use different base URL (without -runtime)
            let bedrock_base_url = base_url.replace("bedrock-runtime.", "bedrock.");
            let bedrock_url = format!("{}/{}", bedrock_base_url, path);
            (bedrock_url, body)
        } else if is_azure && method != Method::GET {
            let model = AIProvider::extract_model_from_body(&body)?;
            let azure_url = AIProvider::build_azure_openai_url(base_url, &model, path);
            (azure_url, body)
        } else if is_anthropic_sdk {
            let truncated_base_url = base_url.trim_end_matches("/v1");
            let anthropic_url = format!("{}/{}", truncated_base_url, path);
            (anthropic_url, body)
        } else {
            let default_url = format!("{}/{}", base_url, path);
            (default_url, body)
        };

        tracing::debug!("AI request URL: {}", url);

        let mut request = HTTP_CLIENT
            .request(method, url)
            .header("content-type", "application/json");

        for (header_name, header_value) in headers.iter() {
            if header_name.to_string().starts_with("anthropic-") {
                request = request.header(header_name, header_value);
            }
        }

        request = request.body(body);

        if is_azure {
            request = request.query(&[("api-version", AZURE_API_VERSION)])
        }

        if let Some(api_key) = self.api_key {
            if is_azure {
                request = request.header("api-key", api_key.clone())
            } else {
                request = request.header("authorization", format!("Bearer {}", api_key.clone()))
            }
            if is_anthropic {
                request = request.header("X-API-Key", api_key);
            }
        }

        if let Some(access_token) = self.access_token {
            request = request.header("authorization", format!("Bearer {}", access_token))
        }

        if let Some(org_id) = self.organization_id {
            request = request.header("OpenAI-Organization", org_id);
        }

        // Apply custom headers from AI_HTTP_HEADERS environment variable
        for (header_name, header_value) in AI_HTTP_HEADERS.iter() {
            request = request.header(header_name.as_str(), header_value.as_str());
        }

        Ok(request)
    }

    fn add_user_to_body(body: Bytes, user: String) -> Result<Bytes> {
        tracing::debug!("Adding user to request body");
        let mut json_body: HashMap<String, Box<RawValue>> = serde_json::from_slice(&body)
            .map_err(|e| Error::internal_err(format!("Failed to parse request body: {}", e)))?;

        let user_json_string = serde_json::Value::String(user).to_string(); // makes sure to escape characters

        json_body.insert(
            "user".to_string(),
            RawValue::from_string(user_json_string)
                .map_err(|e| Error::internal_err(format!("Failed to parse user: {}", e)))?,
        );

        Ok(serde_json::to_vec(&json_body)
            .map_err(|e| Error::internal_err(format!("Failed to reserialize request body: {}", e)))?
            .into())
    }

    /// Transform OpenAI format request to AWS Bedrock Converse format
    /// Returns: (model_id, transformed_body, is_streaming)
    fn transform_openai_to_bedrock(body: &[u8]) -> Result<(String, Bytes, bool)> {
        use serde_json::Value;

        // Parse the OpenAI request
        let openai_req: Value = serde_json::from_slice(body)
            .map_err(|e| Error::internal_err(format!("Failed to parse OpenAI request: {}", e)))?;

        // Extract model and streaming flag
        let model = openai_req["model"]
            .as_str()
            .ok_or_else(|| Error::BadRequest("Missing 'model' field in request".to_string()))?
            .to_string();

        let is_streaming = openai_req["stream"].as_bool().unwrap_or(false);

        // Build Bedrock request
        let mut bedrock_req = serde_json::json!({});

        // Transform messages
        if let Some(messages) = openai_req["messages"].as_array() {
            let mut system_messages = Vec::new();
            let mut conversation_messages = Vec::new();

            for msg in messages {
                let role = msg["role"].as_str().unwrap_or("");

                match role {
                    "system" => {
                        // Extract system messages to separate array
                        if let Some(content) = msg["content"].as_str() {
                            system_messages.push(serde_json::json!({"text": content}));
                        }
                    }
                    "user" | "assistant" => {
                        // Normalize content to array format
                        let mut content = if let Some(text) = msg["content"].as_str() {
                            // Simple string → array of content blocks
                            vec![serde_json::json!({"text": text})]
                        } else if let Some(content_array) = msg["content"].as_array() {
                            // Already an array - transform each item
                            content_array
                                .iter()
                                .filter_map(|item| {
                                    if let Some(text) = item["text"].as_str() {
                                        Some(serde_json::json!({"text": text}))
                                    } else if item["type"].as_str() == Some("text") {
                                        Some(serde_json::json!({"text": item["text"]}))
                                    } else if item["type"].as_str() == Some("image_url") {
                                        // Transform image_url format if needed
                                        // For now, pass through - may need more sophisticated handling
                                        Some(item.clone())
                                    } else {
                                        None
                                    }
                                })
                                .collect()
                        } else {
                            vec![]
                        };

                        // Handle tool_calls for assistant messages (OpenAI → Bedrock toolUse)
                        if role == "assistant" {
                            if let Some(tool_calls) = msg["tool_calls"].as_array() {
                                for tool_call in tool_calls {
                                    if tool_call["type"].as_str() == Some("function") {
                                        let tool_use_id = tool_call["id"].as_str().unwrap_or("");
                                        let function_name =
                                            tool_call["function"]["name"].as_str().unwrap_or("");
                                        let arguments_str = tool_call["function"]["arguments"]
                                            .as_str()
                                            .unwrap_or("{}");

                                        // Parse arguments JSON string to object
                                        let input = serde_json::from_str::<Value>(arguments_str)
                                            .map_err(|e| {
                                                Error::internal_err(format!(
                                                    "Failed to parse tool call arguments: {}",
                                                    e
                                                ))
                                            })?;

                                        content.push(serde_json::json!({
                                            "toolUse": {
                                                "toolUseId": tool_use_id,
                                                "name": function_name,
                                                "input": input
                                            }
                                        }));
                                    }
                                }
                            }
                        }

                        // Only add message if it has content
                        if !content.is_empty() {
                            conversation_messages.push(serde_json::json!({
                                "role": role,
                                "content": content
                            }));
                        }
                    }
                    "tool" => {
                        // Transform tool response to Bedrock format
                        let tool_call_id = msg["tool_call_id"].as_str().unwrap_or("");
                        let content = msg["content"].as_str().unwrap_or("");

                        // Try to parse content as JSON
                        // Bedrock requires json field to be an object, not a primitive or array
                        let tool_result_content =
                            if let Ok(json_content) = serde_json::from_str::<Value>(content) {
                                if json_content.is_object() {
                                    vec![serde_json::json!({"json": json_content})]
                                } else {
                                    // Wrap primitives and arrays in an object
                                    vec![serde_json::json!({"json": {"result": json_content}})]
                                }
                            } else {
                                vec![serde_json::json!({"text": content})]
                            };

                        conversation_messages.push(serde_json::json!({
                            "role": "user",
                            "content": [{
                                "toolResult": {
                                    "toolUseId": tool_call_id,
                                    "content": tool_result_content
                                }
                            }]
                        }));
                    }
                    _ => {}
                }
            }

            if !system_messages.is_empty() {
                bedrock_req["system"] = Value::Array(system_messages);
            }
            bedrock_req["messages"] = Value::Array(conversation_messages);
        }

        // Transform inference parameters
        let mut inference_config = serde_json::json!({});
        if let Some(max_tokens) = openai_req["max_tokens"].as_i64() {
            inference_config["maxTokens"] = Value::Number(max_tokens.into());
        }
        if let Some(temperature) = openai_req["temperature"].as_f64() {
            inference_config["temperature"] = serde_json::json!(temperature);
        }
        if let Some(top_p) = openai_req["top_p"].as_f64() {
            inference_config["topP"] = serde_json::json!(top_p);
        }
        if let Some(stop) = openai_req["stop"].as_array() {
            let stop_sequences: Vec<String> = stop
                .iter()
                .filter_map(|s| s.as_str().map(|s| s.to_string()))
                .collect();
            if !stop_sequences.is_empty() {
                inference_config["stopSequences"] =
                    Value::Array(stop_sequences.into_iter().map(Value::String).collect());
            }
        }
        if !inference_config.as_object().unwrap().is_empty() {
            bedrock_req["inferenceConfig"] = inference_config;
        }

        // Transform tools if present
        if let Some(tools) = openai_req["tools"].as_array() {
            let mut bedrock_tools = Vec::new();

            for tool in tools {
                if tool["type"].as_str() == Some("function") {
                    if let Some(function) = tool["function"].as_object() {
                        bedrock_tools.push(serde_json::json!({
                            "toolSpec": {
                                "name": function.get("name"),
                                "description": function.get("description")
                                    .and_then(|v| v.as_str())
                                    .filter(|s| !s.is_empty())
                                    .unwrap_or("Tool function"),
                                "inputSchema": {
                                    "json": function.get("parameters")
                                }
                            }
                        }));
                    }
                }
            }

            if !bedrock_tools.is_empty() {
                let mut tool_config = serde_json::json!({
                    "tools": bedrock_tools
                });

                // Transform tool_choice
                if let Some(tool_choice) = openai_req.get("tool_choice") {
                    if tool_choice == "auto" {
                        tool_config["toolChoice"] = serde_json::json!({"auto": {}});
                    } else if tool_choice == "required" {
                        tool_config["toolChoice"] = serde_json::json!({"any": {}});
                    } else if let Some(obj) = tool_choice.as_object() {
                        if obj.get("type").and_then(|v| v.as_str()) == Some("function") {
                            if let Some(function) = obj.get("function").and_then(|v| v.as_object())
                            {
                                if let Some(name) = function.get("name").and_then(|v| v.as_str()) {
                                    tool_config["toolChoice"] = serde_json::json!({
                                        "tool": {"name": name}
                                    });
                                }
                            }
                        }
                    }
                }

                bedrock_req["toolConfig"] = tool_config;
            }
        }

        let transformed_body = serde_json::to_vec(&bedrock_req)
            .map_err(|e| {
                Error::internal_err(format!("Failed to serialize Bedrock request: {}", e))
            })?
            .into();

        Ok((model, transformed_body, is_streaming))
    }

    /// Transform AWS Bedrock Converse response to OpenAI format
    async fn transform_bedrock_to_openai(
        response: reqwest::Response,
        model: String,
    ) -> Result<Bytes> {
        use serde_json::Value;

        let bedrock_resp: Value = response
            .json()
            .await
            .map_err(|e| Error::internal_err(format!("Failed to parse Bedrock response: {}", e)))?;

        // Generate unique ID and timestamp
        let id = format!("chatcmpl-{}", uuid::Uuid::new_v4().simple());
        let created = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Extract stop reason and map to finish_reason
        let stop_reason = bedrock_resp["stopReason"].as_str().unwrap_or("end_turn");
        let finish_reason = match stop_reason {
            "end_turn" => "stop",
            "max_tokens" => "length",
            "tool_use" => "tool_calls",
            "stop_sequence" => "stop",
            "guardrail_intervened" | "content_filtered" => "content_filter",
            _ => "stop",
        };

        // Extract message content
        let message_content = &bedrock_resp["output"]["message"]["content"];
        let mut text_content = String::new();
        let mut tool_calls = Vec::new();

        if let Some(content_array) = message_content.as_array() {
            for (_index, block) in content_array.iter().enumerate() {
                if let Some(text) = block["text"].as_str() {
                    text_content.push_str(text);
                } else if let Some(tool_use) = block.get("toolUse") {
                    // Transform tool use to OpenAI tool_calls format
                    let tool_call_id = tool_use["toolUseId"].as_str().unwrap_or("");
                    let name = tool_use["name"].as_str().unwrap_or("");
                    let input = &tool_use["input"];

                    tool_calls.push(serde_json::json!({
                        "id": tool_call_id,
                        "type": "function",
                        "function": {
                            "name": name,
                            "arguments": serde_json::to_string(input).unwrap_or_default()
                        }
                    }));
                }
            }
        }

        // Build the message
        let message = if !tool_calls.is_empty() {
            serde_json::json!({
                "role": "assistant",
                "content": if text_content.is_empty() { Value::Null } else { Value::String(text_content) },
                "tool_calls": tool_calls
            })
        } else {
            serde_json::json!({
                "role": "assistant",
                "content": text_content
            })
        };

        // Extract usage information
        let usage = if let Some(usage_data) = bedrock_resp.get("usage") {
            serde_json::json!({
                "prompt_tokens": usage_data["inputTokens"].as_i64().unwrap_or(0),
                "completion_tokens": usage_data["outputTokens"].as_i64().unwrap_or(0),
                "total_tokens": usage_data["totalTokens"].as_i64().unwrap_or(0)
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
            .map_err(|e| {
                Error::internal_err(format!("Failed to serialize OpenAI response: {}", e))
            })?
            .into();

        Ok(response_body)
    }

    /// Transform AWS Bedrock streaming response to OpenAI SSE format
    /// Bedrock uses AWS event stream binary format, not SSE
    fn transform_bedrock_stream_to_openai(
        stream: impl futures::Stream<Item = std::result::Result<bytes::Bytes, reqwest::Error>>
            + Send
            + 'static,
        model: String,
    ) -> impl futures::Stream<Item = std::result::Result<bytes::Bytes, std::io::Error>> + Send {
        use futures::stream::StreamExt;
        use serde_json::Value;
        use std::collections::HashMap;

        let id = format!("chatcmpl-{}", uuid::Uuid::new_v4().simple());
        let created = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // State to track partial tool calls and binary buffer
        struct StreamState {
            id: String,
            model: String,
            created: u64,
            tool_calls: HashMap<usize, (String, String, String)>, // index -> (id, name, args)
            buffer: Vec<u8>, // Binary buffer for AWS event stream
        }

        let state = std::sync::Arc::new(tokio::sync::Mutex::new(StreamState {
            id: id.clone(),
            model: model.clone(),
            created,
            tool_calls: HashMap::new(),
            buffer: Vec::new(),
        }));

        stream
            .then(move |chunk_result| {
                let state = state.clone();
                async move {
                    match chunk_result {
                        Ok(chunk) => {
                            let mut state = state.lock().await;
                            state.buffer.extend_from_slice(&chunk);

                            let mut events = Vec::new();

                            // Parse AWS event stream messages from buffer
                            loop {
                                // Need at least 12 bytes for prelude (8) + prelude CRC (4)
                                if state.buffer.len() < 12 {
                                    break;
                                }

                                // Read prelude: total_length (4 bytes) + headers_length (4 bytes)
                                let total_length = u32::from_be_bytes([
                                    state.buffer[0],
                                    state.buffer[1],
                                    state.buffer[2],
                                    state.buffer[3],
                                ]) as usize;

                                // Check if we have the complete message
                                if state.buffer.len() < total_length {
                                    break;
                                }

                                let headers_length = u32::from_be_bytes([
                                    state.buffer[4],
                                    state.buffer[5],
                                    state.buffer[6],
                                    state.buffer[7],
                                ]) as usize;

                                // Skip prelude CRC (4 bytes after prelude)
                                let headers_start = 12;
                                let payload_start = headers_start + headers_length;
                                let payload_end = total_length - 4; // Exclude message CRC

                                // Parse headers to extract event type
                                let mut event_type = None;
                                let mut pos = headers_start;
                                while pos < payload_start {
                                    if pos + 1 > state.buffer.len() {
                                        break;
                                    }
                                    let name_len = state.buffer[pos] as usize;
                                    pos += 1;

                                    if pos + name_len > state.buffer.len() {
                                        break;
                                    }
                                    let name = String::from_utf8_lossy(&state.buffer[pos..pos + name_len]).to_string();
                                    pos += name_len;

                                    if pos + 3 > state.buffer.len() {
                                        break;
                                    }
                                    let value_type = state.buffer[pos];
                                    pos += 1;
                                    let value_len = u16::from_be_bytes([state.buffer[pos], state.buffer[pos + 1]]) as usize;
                                    pos += 2;

                                    if pos + value_len > state.buffer.len() {
                                        break;
                                    }

                                    if value_type == 7 && name == ":event-type" {
                                        event_type = Some(String::from_utf8_lossy(&state.buffer[pos..pos + value_len]).to_string());
                                    }
                                    pos += value_len;
                                }

                                // Extract JSON payload (copy to avoid borrow issues)
                                let payload = state.buffer[payload_start..payload_end].to_vec();

                                // Remove processed message from buffer
                                state.buffer.drain(0..total_length);

                                // Process the event
                                if let Some(evt_type) = event_type {
                                    if let Ok(payload_str) = std::str::from_utf8(&payload) {
                                        if let Ok(parsed_data) = serde_json::from_str::<Value>(payload_str) {
                                        // Transform based on event type
                                        match evt_type.as_str() {
                                            "messageStart" => {
                                                // No output for messageStart
                                            }
                                            "contentBlockStart" => {
                                                let index = parsed_data["contentBlockIndex"].as_u64().unwrap_or(0) as usize;

                                                if let Some(tool_use) = parsed_data["start"].get("toolUse") {
                                                    let tool_id = tool_use["toolUseId"].as_str().unwrap_or("").to_string();
                                                    let name = tool_use["name"].as_str().unwrap_or("").to_string();

                                                    state.tool_calls.insert(index, (tool_id.clone(), name.clone(), String::new()));

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
                                                                    "id": tool_id,
                                                                    "type": "function",
                                                                    "function": {
                                                                        "name": name,
                                                                        "arguments": ""
                                                                    }
                                                                }]
                                                            },
                                                            "finish_reason": Value::Null
                                                        }]
                                                    });

                                                    events.push(Ok(bytes::Bytes::from(format!("data: {}\n\n", chunk))));
                                                }
                                            }
                                            "contentBlockDelta" => {
                                                let index = parsed_data["contentBlockIndex"].as_u64().unwrap_or(0) as usize;

                                                if let Some(text) = parsed_data["delta"]["text"].as_str() {
                                                    // Text content delta
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
                                                            "finish_reason": Value::Null
                                                        }]
                                                    });

                                                    events.push(Ok(bytes::Bytes::from(format!("data: {}\n\n", chunk))));
                                                } else if let Some(tool_use_input) = parsed_data["delta"]["toolUse"]["input"].as_str() {
                                                    // Tool use arguments delta
                                                    if let Some((_tool_id, _name, ref mut args)) = state.tool_calls.get_mut(&index) {
                                                        args.push_str(tool_use_input);

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
                                                                            "arguments": tool_use_input
                                                                        }
                                                                    }]
                                                                },
                                                                "finish_reason": Value::Null
                                                            }]
                                                        });

                                                        events.push(Ok(bytes::Bytes::from(format!("data: {}\n\n", chunk))));
                                                    }
                                                }
                                            }
                                            "contentBlockStop" => {
                                                // No output needed
                                            }
                                            "messageStop" => {
                                                let stop_reason = parsed_data["stopReason"].as_str().unwrap_or("end_turn");
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

                                                events.push(Ok(bytes::Bytes::from(format!("data: {}\n\n", chunk))));
                                            }
                                            "metadata" => {
                                                // Could include usage info here if needed
                                            }
                                            _ => {}
                                        }
                                        }
                                    }
                                }
                            } // end loop

                            events
                        }
                        Err(e) => {
                            vec![Err(std::io::Error::new(
                                std::io::ErrorKind::Other,
                                e.to_string(),
                            ))]
                        }
                    }
                }
            })
            .flat_map(|events| futures::stream::iter(events))
            .chain(futures::stream::iter(vec![
                // Send [DONE] at the end
                Ok(bytes::Bytes::from("data: [DONE]\n\n"))
            ]))
    }
}

#[derive(Clone, Debug)]
pub struct ExpiringAIRequestConfig {
    config: AIRequestConfig,
    expires_at: std::time::Instant,
}

impl ExpiringAIRequestConfig {
    fn new(config: AIRequestConfig) -> Self {
        Self { config, expires_at: std::time::Instant::now() + std::time::Duration::from_secs(60) }
    }
    fn is_expired(&self) -> bool {
        self.expires_at < std::time::Instant::now()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AIConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub providers: Option<HashMap<AIProvider, ProviderConfig>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_model: Option<ProviderModel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_completion_model: Option<ProviderModel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_prompts: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens_per_model: Option<HashMap<String, i32>>,
}

pub fn global_service() -> Router {
    Router::new().route("/proxy/*ai", post(global_proxy).get(global_proxy))
}

pub fn workspaced_service() -> Router {
    Router::new().route("/proxy/*ai", post(proxy).get(proxy))
}

async fn global_proxy(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(ai_path): Path<String>,
    method: Method,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let provider = headers
        .get("X-Provider")
        .map(|v| v.to_str().unwrap_or("").to_string());
    let api_key = headers
        .get("X-API-Key")
        .map(|v| v.to_str().unwrap_or("").to_string());

    let provider = match provider {
        Some(provider) => AIProvider::try_from(provider.as_str())?,
        None => return Err(Error::BadRequest("Provider is required".to_string())),
    };

    let Some(api_key) = api_key else {
        return Err(Error::BadRequest("API key is required".to_string()));
    };

    let base_url = provider.get_base_url(None, None, &db).await?;

    let url = format!("{}/{}", base_url, ai_path);

    let mut request = HTTP_CLIENT
        .request(method, url)
        .header("content-type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key));

    // Apply custom headers from AI_HTTP_HEADERS environment variable
    for (header_name, header_value) in AI_HTTP_HEADERS.iter() {
        request = request.header(header_name.as_str(), header_value.as_str());
    }

    let request = request.body(body);

    let response = request.send().await.map_err(to_anyhow)?;

    let mut tx = db.begin().await?;

    audit_log(
        &mut *tx,
        &authed,
        "ai.global_request",
        ActionKind::Execute,
        "global",
        Some(&authed.email),
        None,
    )
    .await?;
    tx.commit().await?;

    if response.error_for_status_ref().is_err() {
        let err_msg = response.text().await.unwrap_or("".to_string());
        return Err(Error::AIError(err_msg));
    }

    let status_code = response.status();
    let headers = response.headers().clone();
    let stream = response.bytes_stream();
    Ok((status_code, headers, axum::body::Body::from_stream(stream)))
}

async fn proxy(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, ai_path)): Path<(String, String)>,
    method: Method,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let provider = headers
        .get("X-Provider")
        .map(|v| v.to_str().unwrap_or("").to_string());

    let provider = match provider {
        Some(provider) => AIProvider::try_from(provider.as_str())?,
        None => return Err(Error::BadRequest("Provider is required".to_string())),
    };

    let workspace_cache = AI_REQUEST_CACHE.get(&(w_id.clone(), provider.clone()));

    let forced_resource_path = headers
        .get("X-Resource-Path")
        .map(|v| v.to_str().unwrap_or("").to_string());
    let request_config = match workspace_cache {
        Some(request_cache) if !request_cache.is_expired() && forced_resource_path.is_none() => {
            request_cache.config
        }
        _ => {
            let (resource_path, save_to_cache) = if let Some(resource_path) = forced_resource_path {
                // forced resource path
                (resource_path, false)
            } else {
                let ai_config = sqlx::query_scalar!(
                    "SELECT ai_config FROM workspace_settings WHERE workspace_id = $1",
                    &w_id
                )
                .fetch_one(&db)
                .await?;

                if ai_config.is_none() {
                    return Err(Error::internal_err(
                        "AI resource not configured".to_string(),
                    ));
                }

                let mut ai_config = serde_json::from_value::<AIConfig>(ai_config.unwrap())
                    .map_err(|e| Error::BadRequest(e.to_string()))?;

                let provider_config = ai_config
                    .providers
                    .as_mut()
                    .map(|providers| providers.remove(&provider))
                    .flatten()
                    .ok_or_else(|| {
                        Error::BadRequest(format!("Provider {:?} not configured", provider))
                    })?;

                if provider_config.resource_path.is_empty() {
                    return Err(Error::BadRequest("Resource path is empty".to_string()));
                }

                (provider_config.resource_path, true)
            };

            let resource= sqlx::query_scalar!(
                "SELECT value as \"value: sqlx::types::Json<Box<RawValue>>\" FROM resource WHERE path = $1 AND workspace_id = $2",
                &resource_path,
                &w_id
            )
            .fetch_optional(&db)
            .await?
            .ok_or_else(|| Error::NotFound(format!("Could not find the resource {}, update the resource path in the workspace settings", resource_path)))?
            .ok_or_else(|| Error::BadRequest(format!("Empty resource value for {}", resource_path)))?;

            let resource = serde_json::from_str::<AIResource>(resource.0.get())
                .map_err(|e| Error::BadRequest(e.to_string()))?;

            let request_config = AIRequestConfig::new(&provider, &db, &w_id, resource).await?;
            if save_to_cache {
                AI_REQUEST_CACHE.insert(
                    (w_id.clone(), provider.clone()),
                    ExpiringAIRequestConfig::new(request_config.clone()),
                );
            }
            request_config
        }
    };

    // Extract model and streaming flag for Bedrock transformation (only for POST requests)
    let (model_for_transform, is_streaming) =
        if matches!(provider, AIProvider::AWSBedrock) && method == Method::POST {
            let parsed: serde_json::Value = serde_json::from_slice(&body)
                .map_err(|e| Error::internal_err(format!("Failed to parse request body: {}", e)))?;
            let model = parsed["model"].as_str().unwrap_or("").to_string();
            let is_streaming = parsed["stream"].as_bool().unwrap_or(false);
            (Some(model), is_streaming)
        } else {
            (None, false)
        };

    let request = request_config.prepare_request(&provider, &ai_path, method, headers, body)?;

    let response = request.send().await.map_err(to_anyhow)?;

    let mut tx = db.begin().await?;

    audit_log(
        &mut *tx,
        &authed,
        "ai.request",
        ActionKind::Execute,
        &w_id,
        Some(&authed.email),
        Some([("ai_config_path", &format!("{:?}", ai_path)[..])].into()),
    )
    .await?;
    tx.commit().await?;

    if response.error_for_status_ref().is_err() {
        let err_msg = response.text().await.unwrap_or("".to_string());
        return Err(Error::AIError(err_msg));
    }

    // Transform Bedrock responses back to OpenAI format
    if matches!(provider, AIProvider::AWSBedrock) && model_for_transform.is_some() {
        let model = model_for_transform.unwrap();

        if is_streaming {
            // Transform streaming response
            use http::StatusCode;

            let mut response_headers = HeaderMap::new();
            response_headers.insert("content-type", "text/event-stream".parse().unwrap());
            response_headers.insert("cache-control", "no-cache".parse().unwrap());
            response_headers.insert("connection", "keep-alive".parse().unwrap());

            let stream = response.bytes_stream();
            let transformed_stream =
                AIRequestConfig::transform_bedrock_stream_to_openai(stream, model);

            Ok((
                StatusCode::OK,
                response_headers,
                axum::body::Body::from_stream(transformed_stream),
            ))
        } else {
            // Transform non-streaming response
            let transformed_body =
                AIRequestConfig::transform_bedrock_to_openai(response, model).await?;

            let mut response_headers = HeaderMap::new();
            response_headers.insert("content-type", "application/json".parse().unwrap());

            Ok((
                http::StatusCode::OK,
                response_headers,
                axum::body::Body::from(transformed_body),
            ))
        }
    } else {
        // Pass through for other providers
        let status_code = response.status();
        let headers = response.headers().clone();
        let stream = response.bytes_stream();
        Ok((status_code, headers, axum::body::Body::from_stream(stream)))
    }
}
