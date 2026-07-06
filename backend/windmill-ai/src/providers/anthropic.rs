use crate::{
    ai_google::parse_data_url,
    ai_providers::{AIPlatform, AIProvider},
    image_handler::prepare_messages_for_api,
    proxy::{add_user_to_body, ProxyBuildArgs, ProxyRequest},
    query_builder::{BuildRequestArgs, ParsedResponse, QueryBuilder, StreamEventSink},
    sse::{AnthropicSSEParser, SSEParser},
    types::*,
    utils::{extract_text_content, should_use_structured_output_tool, AI_HTTP_HEADERS},
};
use async_trait::async_trait;
use http::Method;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use windmill_common::{client::AuthedClient, error::Error};

/// Anthropic API version for standard API
const ANTHROPIC_VERSION_STANDARD: &str = "2023-06-01";
/// Anthropic API version for Google Vertex AI
const ANTHROPIC_VERSION_VERTEX: &str = "vertex-2023-10-16";

#[derive(Serialize, Debug, Clone)]
pub struct CacheControl {
    pub r#type: String,
}

impl CacheControl {
    pub fn ephemeral() -> Self {
        Self { r#type: "ephemeral".to_string() }
    }
}

/// Custom tool for Anthropic native API (flat structure with type: "custom")
#[derive(Serialize, Debug)]
pub struct AnthropicCustomTool {
    pub r#type: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub input_schema: Box<RawValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControl>,
}

impl From<&ToolDef> for AnthropicCustomTool {
    fn from(tool: &ToolDef) -> Self {
        AnthropicCustomTool {
            r#type: "custom".to_string(),
            name: tool.function.name.clone(),
            description: tool.function.description.clone(),
            input_schema: tool.function.parameters.clone(),
            cache_control: None,
        }
    }
}

/// Web search tool for Anthropic
#[derive(Serialize, Debug)]
pub struct AnthropicWebSearchTool {
    pub r#type: String,
    pub name: String,
}

/// Tool types for Anthropic API
#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum AnthropicTool {
    Custom(AnthropicCustomTool),
    WebSearch(AnthropicWebSearchTool),
}

/// Tool choice for Anthropic API to force tool usage
#[derive(Serialize, Debug)]
pub struct AnthropicToolChoice {
    pub r#type: String,
}

impl AnthropicToolChoice {
    /// Create a tool_choice that forces the model to use any tool
    pub fn any() -> Self {
        Self { r#type: "any".to_string() }
    }
}

/// Content block for Anthropic messages (for serialization to API)
#[derive(Serialize, Debug)]
#[serde(tag = "type")]
pub enum AnthropicRequestContent {
    #[serde(rename = "text")]
    Text {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        cache_control: Option<CacheControl>,
    },
    #[serde(rename = "thinking")]
    Thinking { thinking: String, signature: String },
    #[serde(rename = "redacted_thinking")]
    RedactedThinking { data: String },
    #[serde(rename = "image")]
    Image { source: AnthropicBase64Source },
    #[serde(rename = "document")]
    Document { source: AnthropicBase64Source },
    #[serde(rename = "tool_use")]
    ToolUse { id: String, name: String, input: Box<RawValue> },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        cache_control: Option<CacheControl>,
    },
}

/// Base64 source for Anthropic API (used by both Image and Document content blocks)
#[derive(Serialize, Debug)]
pub struct AnthropicBase64Source {
    pub r#type: String,
    pub media_type: String,
    pub data: String,
}

/// System content block for Anthropic API
#[derive(Serialize, Debug)]
pub struct AnthropicSystemContent {
    pub r#type: String,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControl>,
}

/// Message format for Anthropic API requests
#[derive(Serialize, Debug)]
pub struct AnthropicMessage {
    pub role: String,
    pub content: Vec<AnthropicRequestContent>,
}

/// Adaptive thinking config for Anthropic native API. `summarized` display
/// matches the chat proxy path (renders a summarized thinking stream).
#[derive(Serialize, Debug)]
pub struct AnthropicThinking {
    pub r#type: &'static str,
    pub display: &'static str,
}

impl AnthropicThinking {
    fn adaptive() -> Self {
        Self { r#type: "adaptive", display: "summarized" }
    }
}

/// Carries the reasoning effort token alongside adaptive thinking.
#[derive(Serialize, Debug)]
pub struct AnthropicOutputConfig {
    pub effort: String,
}

/// Anthropic-specific request structure for standard API
#[derive(Serialize)]
pub struct AnthropicRequest<'a> {
    pub model: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<Vec<AnthropicSystemContent>>,
    pub messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<AnthropicTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<AnthropicToolChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<AnthropicThinking>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_config: Option<AnthropicOutputConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    pub stream: bool,
}

/// Anthropic request structure for Google Vertex AI
/// Key differences from standard API:
/// - No model field (model is specified in the URL)
/// - anthropic_version is in the body instead of a header
#[derive(Serialize)]
pub struct AnthropicVertexRequest {
    pub anthropic_version: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<Vec<AnthropicSystemContent>>,
    pub messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<AnthropicTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<AnthropicToolChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<AnthropicThinking>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_config: Option<AnthropicOutputConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    pub stream: bool,
}

/// Convert OpenAI-format messages to Anthropic native format
fn convert_messages_to_anthropic(messages: &[OpenAIMessage]) -> Vec<AnthropicMessage> {
    let mut result: Vec<AnthropicMessage> = Vec::new();

    for msg in messages {
        match msg.role.as_str() {
            "system" => {
                // Skip - handled via args.system_prompt in build_text_request
            }
            "user" => {
                // Convert user messages
                let content = convert_content_to_anthropic(&msg.content);
                if !content.is_empty() {
                    result.push(AnthropicMessage { role: msg.role.clone(), content });
                }
            }
            "assistant" => {
                let mut content: Vec<AnthropicRequestContent> = Vec::new();

                // Replay the turn's thinking block first: when thinking is enabled,
                // Claude requires the thinking block (with its unmodified signature)
                // to precede tool_use in the assistant turn it was emitted in. It is
                // round-tripped on the tool call's extra_content (see AnthropicExtraContent).
                if let Some(reasoning) = msg
                    .tool_calls
                    .as_ref()
                    .and_then(|tcs| {
                        tcs.iter().find_map(|tc| {
                            tc.extra_content
                                .as_ref()
                                .and_then(|ec| ec.anthropic.as_ref())
                        })
                    })
                    .and_then(anthropic_reasoning_block_from_extra)
                {
                    content.push(reasoning);
                }

                // Add text content if present
                if let Some(ref c) = msg.content {
                    let text = extract_text_content(c);
                    if !text.is_empty() {
                        content.push(AnthropicRequestContent::Text { text, cache_control: None });
                    }
                }

                // Add tool_use blocks if present (echoing back what the assistant requested)
                if let Some(ref tool_calls) = msg.tool_calls {
                    for tc in tool_calls {
                        if tc.function.name.is_empty() {
                            continue;
                        }
                        // Pass arguments directly as RawValue to avoid round-trip serialization
                        let input = RawValue::from_string(tc.function.arguments.clone())
                            .unwrap_or_else(|_| RawValue::from_string("{}".to_string()).unwrap());
                        content.push(AnthropicRequestContent::ToolUse {
                            id: tc.id.clone(),
                            name: tc.function.name.clone(),
                            input,
                        });
                    }
                }

                if !content.is_empty() {
                    result.push(AnthropicMessage { role: "assistant".to_string(), content });
                }
            }
            "tool" => {
                // Tool results must be sent as user messages with tool_result content
                if let Some(ref tool_call_id) = msg.tool_call_id {
                    let content_text = msg
                        .content
                        .as_ref()
                        .map(|c| extract_text_content(c))
                        .unwrap_or_default();

                    result.push(AnthropicMessage {
                        role: "user".to_string(),
                        content: vec![AnthropicRequestContent::ToolResult {
                            tool_use_id: tool_call_id.clone(),
                            content: content_text,
                            cache_control: None,
                        }],
                    });
                }
            }
            _ => {
                // Unknown role, skip
            }
        }
    }

    result
}

/// Rebuild an Anthropic thinking content block from the round-tripped
/// [`AnthropicExtraContent`](crate::ai_types::AnthropicExtraContent). Returns
/// None when there is no replayable block (thinking text without a signature is
/// rejected by Anthropic, so it is dropped rather than sent).
fn anthropic_reasoning_block_from_extra(
    extra: &crate::ai_types::AnthropicExtraContent,
) -> Option<AnthropicRequestContent> {
    if let Some(data) = extra.redacted_thinking.as_deref() {
        return Some(AnthropicRequestContent::RedactedThinking { data: data.to_string() });
    }
    match (extra.thinking.as_deref(), extra.signature.as_deref()) {
        (Some(thinking), Some(signature)) => Some(AnthropicRequestContent::Thinking {
            thinking: thinking.to_string(),
            signature: signature.to_string(),
        }),
        _ => None,
    }
}

/// Convert OpenAI content to Anthropic content blocks
fn convert_content_to_anthropic(content: &Option<OpenAIContent>) -> Vec<AnthropicRequestContent> {
    let Some(content) = content else {
        return Vec::new();
    };

    match content {
        OpenAIContent::Text(text) => {
            if text.is_empty() {
                Vec::new()
            } else {
                vec![AnthropicRequestContent::Text { text: text.clone(), cache_control: None }]
            }
        }
        OpenAIContent::Parts(parts) => {
            let mut result = Vec::new();
            for part in parts {
                match part {
                    ContentPart::Text { text } => {
                        if !text.is_empty() {
                            result.push(AnthropicRequestContent::Text {
                                text: text.clone(),
                                cache_control: None,
                            });
                        }
                    }
                    ContentPart::ImageUrl { image_url } => {
                        if let Some((media_type, data)) = parse_data_url(&image_url.url) {
                            result.push(AnthropicRequestContent::Image {
                                source: AnthropicBase64Source {
                                    r#type: "base64".to_string(),
                                    media_type,
                                    data,
                                },
                            });
                        }
                    }
                    ContentPart::File { file } => {
                        if let Some((media_type, data)) = parse_data_url(&file.file_data) {
                            result.push(AnthropicRequestContent::Document {
                                source: AnthropicBase64Source {
                                    r#type: "base64".to_string(),
                                    media_type,
                                    data,
                                },
                            });
                        }
                    }
                    ContentPart::S3Object { .. } => {
                        // S3 objects should be converted to base64 by prepare_messages_for_api
                        // before reaching here, so we skip them
                    }
                }
            }
            result
        }
    }
}

/// Citation from Anthropic web search
#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct AnthropicCitation {
    #[serde(default)]
    pub start_index: Option<usize>,
    #[serde(default)]
    pub end_index: Option<usize>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
}

/// Web search result content from Anthropic
#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct AnthropicWebSearchContent {
    #[serde(default)]
    pub r#type: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
}

/// Anthropic content block - can be text, tool_use, server_tool_use, or web_search_tool_result
#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
#[allow(dead_code)]
pub enum AnthropicContentBlock {
    #[serde(rename = "text")]
    Text {
        text: String,
        #[serde(default)]
        citations: Vec<AnthropicCitation>,
    },
    #[serde(rename = "tool_use")]
    ToolUse { id: String, name: String, input: serde_json::Value },
    #[serde(rename = "server_tool_use")]
    ServerToolUse { id: String, name: String, input: serde_json::Value },
    #[serde(rename = "web_search_tool_result")]
    WebSearchToolResult { tool_use_id: String, content: Vec<AnthropicWebSearchContent> },
    #[serde(other)]
    Unknown,
}

/// Anthropic native API response
#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct AnthropicResponse {
    #[serde(default)]
    pub content: Vec<AnthropicContentBlock>,
    #[serde(default)]
    pub stop_reason: Option<String>,
}

/// Query builder for Anthropic using their native API format
pub struct AnthropicQueryBuilder {
    #[allow(dead_code)]
    provider_kind: AIProvider,
    platform: AIPlatform,
}

impl AnthropicQueryBuilder {
    pub fn new(provider_kind: AIProvider, platform: AIPlatform) -> Self {
        Self { provider_kind, platform }
    }

    fn is_vertex(&self) -> bool {
        self.platform == AIPlatform::GoogleVertexAi
    }

    /// Claude models hosted on Azure AI Foundry: the Anthropic Messages API is
    /// served under the resource's `/anthropic/v1` path rather than at the
    /// Anthropic public base URL.
    fn is_azure_foundry(&self) -> bool {
        matches!(self.provider_kind, AIProvider::AzureFoundry)
    }

    fn transform_proxy_body_for_vertex(body: &[u8]) -> Result<(String, Vec<u8>), Error> {
        let mut json_body: std::collections::HashMap<String, serde_json::Value> =
            serde_json::from_slice(body).map_err(|e| {
                Error::internal_err(format!("Failed to parse Anthropic request: {}", e))
            })?;

        let model = json_body
            .remove("model")
            .and_then(|v| v.as_str().map(str::to_string))
            .ok_or_else(|| {
                Error::BadRequest("Missing 'model' field in Anthropic request".to_string())
            })?;

        json_body.insert(
            "anthropic_version".to_string(),
            serde_json::Value::String(ANTHROPIC_VERSION_VERTEX.to_string()),
        );

        let transformed_body = serde_json::to_vec(&json_body).map_err(|e| {
            Error::internal_err(format!("Failed to serialize Vertex request: {}", e))
        })?;

        Ok((model, transformed_body))
    }

    fn build_anthropic_proxy_request(
        &self,
        args: &ProxyBuildArgs<'_>,
    ) -> Result<ProxyRequest, Error> {
        let credentials = args.credentials;
        let body = if let Some(user) = credentials.user.as_ref() {
            add_user_to_body(args.body, user)?
        } else {
            args.body.to_vec()
        };

        let base_url = credentials.base_url.trim_end_matches('/');
        let is_vertex = self.is_vertex();
        let is_anthropic_sdk = args.headers.get("X-Anthropic-SDK").is_some();

        let (url, body) = if is_vertex && *args.method != Method::GET {
            let (model, transformed_body) = Self::transform_proxy_body_for_vertex(&body)?;
            (
                format!("{}/{}:streamRawPredict", base_url, model),
                transformed_body,
            )
        } else if self.is_azure_foundry() {
            // Claude on Foundry is served under the resource's /anthropic/v1 path,
            // not the resource's /openai/v1 base. The Anthropic SDK sends the path
            // as "v1/messages", so drop its leading "v1/" before re-appending.
            let path = args.path.trim_start_matches("v1/");
            (
                AIProvider::build_azure_foundry_anthropic_url(base_url, path),
                body,
            )
        } else if is_anthropic_sdk {
            let truncated_base_url = base_url.trim_end_matches("/v1");
            (format!("{}/{}", truncated_base_url, args.path), body)
        } else {
            (format!("{}/{}", base_url, args.path), body)
        };

        let mut headers = vec![("content-type".to_string(), "application/json".to_string())];

        for (header_name, header_value) in args.headers.iter() {
            if header_name.as_str().starts_with("anthropic-") {
                if is_vertex && header_name.as_str() == "anthropic-version" {
                    continue;
                }
                headers.push((
                    header_name.as_str().to_string(),
                    header_value
                        .to_str()
                        .map_err(|e| {
                            Error::BadRequest(format!(
                                "Invalid Anthropic header value for {}: {}",
                                header_name, e
                            ))
                        })?
                        .to_string(),
                ));
            }
        }

        if let Some(api_key) = credentials.api_key.as_ref() {
            headers.push(("authorization".to_string(), format!("Bearer {}", api_key)));
            if !is_vertex {
                headers.push(("X-API-Key".to_string(), api_key.clone()));
            }
        }

        if let Some(access_token) = credentials.access_token.as_ref() {
            headers.push((
                "authorization".to_string(),
                format!("Bearer {}", access_token),
            ));
        }

        if let Some(org_id) = credentials.organization_id.as_ref() {
            headers.push(("OpenAI-Organization".to_string(), org_id.clone()));
        }

        for (header_name, header_value) in AI_HTTP_HEADERS.iter() {
            headers.push((header_name.clone(), header_value.clone()));
        }

        for (header_name, header_value) in &credentials.custom_headers {
            headers.push((header_name.clone(), header_value.clone()));
        }

        Ok(ProxyRequest { method: args.method.clone(), url, headers, body })
    }

    async fn build_text_request(
        &self,
        args: &BuildRequestArgs<'_>,
        client: &AuthedClient,
        workspace_id: &str,
    ) -> Result<String, Error> {
        // First prepare messages (handles S3 object to base64 conversion)
        let prepared_messages =
            prepare_messages_for_api(args.messages, client, workspace_id).await?;

        // Convert to Anthropic native message format
        let mut anthropic_messages = convert_messages_to_anthropic(&prepared_messages);

        // Build tools array using typed structs
        let mut tools: Vec<AnthropicTool> = Vec::new();

        // Add websearch tool if enabled (Anthropic format)
        if args.has_websearch {
            tools.push(AnthropicTool::WebSearch(AnthropicWebSearchTool {
                r#type: "web_search_20250305".to_string(),
                name: "web_search".to_string(),
            }));
        }

        // Add custom tools (convert from ToolDef to AnthropicCustomTool)
        if let Some(tool_defs) = args.tools {
            for tool in tool_defs {
                tools.push(AnthropicTool::Custom(AnthropicCustomTool::from(tool)));
            }
        }

        // Build system content from system_prompt, but None if system_prompt is empty string
        let system = match args.system_prompt {
            Some(s) if !s.is_empty() => Some(vec![AnthropicSystemContent {
                r#type: "text".to_string(),
                text: s.to_string(),
                cache_control: if self.is_vertex() {
                    None
                } else {
                    Some(CacheControl::ephemeral())
                },
            }]),
            _ => None,
        };

        // Check if we need to force tool usage for structured output
        let has_output_properties = args
            .output_schema
            .and_then(|schema| schema.properties.as_ref())
            .map(|props| !props.is_empty())
            .unwrap_or(false);

        let should_use_structured_output =
            should_use_structured_output_tool(&self.provider_kind, args.model);

        // Force tool usage when structured output is needed
        let tool_choice = if should_use_structured_output && has_output_properties {
            Some(AnthropicToolChoice::any())
        } else {
            None
        };

        let mut tools_option = if tools.is_empty() { None } else { Some(tools) };
        let max_tokens = Some(args.max_tokens.unwrap_or(64000));

        // Apply cache_control on the last custom tool
        if !self.is_vertex() {
            if let Some(ref mut tools_vec) = tools_option {
                if let Some(AnthropicTool::Custom(ref mut custom)) = tools_vec.last_mut() {
                    custom.cache_control = Some(CacheControl::ephemeral());
                }
            }
        }

        // Apply cache_control on the last content block of the last message
        if !self.is_vertex() {
            if let Some(last_msg) = anthropic_messages.last_mut() {
                if let Some(last_block) = last_msg.content.last_mut() {
                    match last_block {
                        AnthropicRequestContent::Text { cache_control, .. } => {
                            *cache_control = Some(CacheControl::ephemeral());
                        }
                        AnthropicRequestContent::ToolResult { cache_control, .. } => {
                            *cache_control = Some(CacheControl::ephemeral());
                        }
                        _ => {}
                    }
                }
            }
        }

        // Adaptive thinking rejects sampling params, so drop temperature when
        // reasoning is on (Anthropic returns a hard 400 otherwise).
        let (thinking, output_config, temperature) = match args.reasoning_effort {
            Some(effort) => (
                Some(AnthropicThinking::adaptive()),
                Some(AnthropicOutputConfig { effort: effort.to_string() }),
                None,
            ),
            None => (None, None, args.temperature),
        };

        // Build request based on platform
        if self.is_vertex() {
            // For Vertex AI: no model field, anthropic_version in body
            let request = AnthropicVertexRequest {
                anthropic_version: ANTHROPIC_VERSION_VERTEX,
                system,
                messages: anthropic_messages,
                tools: tools_option,
                tool_choice,
                temperature,
                thinking,
                output_config,
                max_tokens,
                stream: true,
            };
            serde_json::to_string(&request)
                .map_err(|e| Error::internal_err(format!("Failed to serialize request: {}", e)))
        } else {
            // For standard API: model in body, anthropic_version in header
            let request = AnthropicRequest {
                model: args.model,
                system,
                messages: anthropic_messages,
                tools: tools_option,
                tool_choice,
                temperature,
                thinking,
                output_config,
                max_tokens,
                stream: true,
            };
            serde_json::to_string(&request)
                .map_err(|e| Error::internal_err(format!("Failed to serialize request: {}", e)))
        }
    }
}

#[async_trait]
impl QueryBuilder for AnthropicQueryBuilder {
    fn supports_tools_with_output_type(&self, _output_type: &OutputType) -> bool {
        // Anthropic supports tools for text output
        matches!(_output_type, OutputType::Text)
    }

    fn build_proxy_request(&self, args: &ProxyBuildArgs<'_>) -> Result<ProxyRequest, Error> {
        self.build_anthropic_proxy_request(args)
    }

    async fn build_request(
        &self,
        args: &BuildRequestArgs<'_>,
        client: &AuthedClient,
        workspace_id: &str,
    ) -> Result<String, Error> {
        self.build_text_request(args, client, workspace_id).await
    }

    async fn parse_image_response(
        &self,
        _response: reqwest::Response,
    ) -> Result<ParsedResponse, Error> {
        Err(Error::internal_err(
            "Anthropic does not support image output".to_string(),
        ))
    }

    async fn parse_streaming_response(
        &self,
        response: reqwest::Response,
        stream_event_sink: Box<dyn StreamEventSink>,
    ) -> Result<ParsedResponse, Error> {
        let mut anthropic_sse_parser = AnthropicSSEParser::new(stream_event_sink);
        anthropic_sse_parser.parse_events(response).await?;

        let AnthropicSSEParser {
            accumulated_content,
            accumulated_tool_calls,
            events_str,
            annotations,
            used_websearch,
            usage: anthropic_usage,
            ..
        } = anthropic_sse_parser;

        // Note: Tool call arguments events are already sent by the parser during streaming
        // when content_block_stop is received

        // Convert Anthropic usage to TokenUsage
        let usage = anthropic_usage.map(|u| {
            TokenUsage::from_input_output(u.input_tokens, u.output_tokens)
                .with_cache(u.cache_read_input_tokens, u.cache_creation_input_tokens)
        });

        Ok(ParsedResponse::Text {
            content: if accumulated_content.is_empty() {
                None
            } else {
                Some(accumulated_content)
            },
            tool_calls: accumulated_tool_calls.into_values().collect(),
            events_str: Some(events_str),
            annotations,
            used_websearch,
            usage,
        })
    }

    fn get_endpoint(&self, base_url: &str, model: &str, _output_type: &OutputType) -> String {
        if self.is_vertex() {
            // For Vertex AI, the model is specified in the URL path
            // Expected base_url format: https://{region}-aiplatform.googleapis.com/v1/projects/{project}/locations/{location}/publishers/anthropic/models
            // We append the model and :streamRawPredict
            format!(
                "{}/{}:streamRawPredict",
                base_url.trim_end_matches('/'),
                model
            )
        } else if self.is_azure_foundry() {
            AIProvider::build_azure_foundry_anthropic_url(base_url, "messages")
        } else {
            format!("{}/messages", base_url)
        }
    }

    fn get_auth_headers(
        &self,
        api_key: &str,
        _base_url: &str,
        _output_type: &OutputType,
    ) -> Vec<(&'static str, String)> {
        if self.is_vertex() {
            // For Vertex AI, use Bearer token authentication
            // The api_key should be an OAuth2 access token
            vec![("Authorization", format!("Bearer {}", api_key))]
        } else {
            // Standard Anthropic API uses x-api-key and anthropic-version header
            vec![
                ("x-api-key", api_key.to_string()),
                ("anthropic-version", ANTHROPIC_VERSION_STANDARD.to_string()),
            ]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        credentials::ProviderCredentials, proxy::ProxyBuildArgs, query_builder::QueryBuilder,
    };
    use http::{HeaderMap, HeaderValue, Method};
    use std::collections::HashMap;

    fn credentials(platform: AIPlatform) -> ProviderCredentials {
        ProviderCredentials {
            provider: AIProvider::Anthropic,
            base_url: "https://api.anthropic.com/v1".to_string(),
            api_key: Some("api-key".to_string()),
            access_token: None,
            organization_id: None,
            user: None,
            region: None,
            aws_access_key_id: None,
            aws_secret_access_key: None,
            aws_session_token: None,
            platform,
            custom_headers: HashMap::new(),
        }
    }

    fn has_header(headers: &[(String, String)], name: &str, value: &str) -> bool {
        headers
            .iter()
            .any(|(header_name, header_value)| header_name == name && header_value == value)
    }

    #[test]
    fn builds_standard_anthropic_proxy_request() {
        let credentials = credentials(AIPlatform::Standard);
        let builder = AnthropicQueryBuilder::new(AIProvider::Anthropic, AIPlatform::Standard);
        let method = Method::POST;
        let mut headers = HeaderMap::new();
        headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
        let body = br#"{"model":"claude-sonnet-4","messages":[]}"#;

        let request = builder
            .build_proxy_request(&ProxyBuildArgs {
                method: &method,
                path: "messages",
                headers: &headers,
                body,
                credentials: &credentials,
            })
            .unwrap();

        assert_eq!(request.method, Method::POST);
        assert_eq!(request.url, "https://api.anthropic.com/v1/messages");
        assert_eq!(request.body, body.to_vec());
        assert!(has_header(
            &request.headers,
            "authorization",
            "Bearer api-key"
        ));
        assert!(has_header(&request.headers, "X-API-Key", "api-key"));
        assert!(has_header(
            &request.headers,
            "anthropic-version",
            "2023-06-01"
        ));
    }

    #[test]
    fn builds_vertex_anthropic_proxy_request() {
        let mut credentials = credentials(AIPlatform::GoogleVertexAi);
        credentials.base_url = "https://us-central1-aiplatform.googleapis.com/v1/projects/p/locations/us-central1/publishers/anthropic/models".to_string();
        credentials.user = Some("user-1".to_string());
        let builder = AnthropicQueryBuilder::new(AIProvider::Anthropic, AIPlatform::GoogleVertexAi);
        let method = Method::POST;
        let mut headers = HeaderMap::new();
        headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
        headers.insert("anthropic-beta", HeaderValue::from_static("some-beta"));

        let request = builder
            .build_proxy_request(&ProxyBuildArgs {
                method: &method,
                path: "messages",
                headers: &headers,
                body: br#"{"model":"claude-sonnet-4","messages":[]}"#,
                credentials: &credentials,
            })
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();

        assert_eq!(
            request.url,
            "https://us-central1-aiplatform.googleapis.com/v1/projects/p/locations/us-central1/publishers/anthropic/models/claude-sonnet-4:streamRawPredict"
        );
        assert_eq!(body["anthropic_version"], ANTHROPIC_VERSION_VERTEX);
        assert_eq!(body["user"], "user-1");
        assert!(body.get("model").is_none());
        assert!(has_header(
            &request.headers,
            "authorization",
            "Bearer api-key"
        ));
        assert!(!request
            .headers
            .iter()
            .any(|(header_name, _)| header_name == "X-API-Key"));
        assert!(!request
            .headers
            .iter()
            .any(|(header_name, _)| header_name == "anthropic-version"));
        assert!(has_header(&request.headers, "anthropic-beta", "some-beta"));
    }

    #[test]
    fn convert_messages_replays_thinking_block_before_tool_use() {
        let tool_call = crate::ai_types::OpenAIToolCall {
            id: "call_1".to_string(),
            function: crate::ai_types::OpenAIFunction {
                name: "lookup".to_string(),
                arguments: "{}".to_string(),
            },
            r#type: "function".to_string(),
            extra_content: Some(crate::ai_types::ExtraContent {
                anthropic: Some(crate::ai_types::AnthropicExtraContent {
                    thinking: Some("let me think".to_string()),
                    signature: Some("sig-abc".to_string()),
                    redacted_thinking: None,
                }),
                ..Default::default()
            }),
        };
        let assistant = OpenAIMessage {
            role: "assistant".to_string(),
            tool_calls: Some(vec![tool_call]),
            ..Default::default()
        };

        let messages = convert_messages_to_anthropic(&[assistant]);
        let content = &messages[0].content;
        match &content[0] {
            AnthropicRequestContent::Thinking { thinking, signature } => {
                assert_eq!(thinking, "let me think");
                assert_eq!(signature, "sig-abc");
            }
            other => panic!("expected thinking block first, got {:?}", other),
        }
        assert!(matches!(
            &content[1],
            AnthropicRequestContent::ToolUse { .. }
        ));
    }

    #[test]
    fn convert_messages_drops_thinking_without_signature() {
        let tool_call = crate::ai_types::OpenAIToolCall {
            id: "call_1".to_string(),
            function: crate::ai_types::OpenAIFunction {
                name: "lookup".to_string(),
                arguments: "{}".to_string(),
            },
            r#type: "function".to_string(),
            extra_content: Some(crate::ai_types::ExtraContent {
                anthropic: Some(crate::ai_types::AnthropicExtraContent {
                    thinking: Some("unsigned".to_string()),
                    signature: None,
                    redacted_thinking: None,
                }),
                ..Default::default()
            }),
        };
        let assistant = OpenAIMessage {
            role: "assistant".to_string(),
            tool_calls: Some(vec![tool_call]),
            ..Default::default()
        };

        let messages = convert_messages_to_anthropic(&[assistant]);
        // An unsigned thinking block can't be replayed, so only tool_use remains.
        assert!(matches!(
            messages[0].content[0],
            AnthropicRequestContent::ToolUse { .. }
        ));
    }

    #[test]
    fn anthropic_request_serializes_adaptive_thinking_without_temperature() {
        let request = AnthropicRequest {
            model: "claude-opus-4-8",
            system: None,
            messages: vec![],
            tools: None,
            tool_choice: None,
            temperature: None,
            thinking: Some(AnthropicThinking::adaptive()),
            output_config: Some(AnthropicOutputConfig { effort: "high".to_string() }),
            max_tokens: Some(64000),
            stream: true,
        };

        let body: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&request).unwrap()).unwrap();
        assert_eq!(body["thinking"]["type"], "adaptive");
        assert_eq!(body["thinking"]["display"], "summarized");
        assert_eq!(body["output_config"]["effort"], "high");
        // Sampling params are rejected alongside adaptive thinking.
        assert!(body.get("temperature").is_none());
    }

    #[test]
    fn anthropic_request_omits_thinking_when_reasoning_off() {
        let request = AnthropicRequest {
            model: "claude-opus-4-8",
            system: None,
            messages: vec![],
            tools: None,
            tool_choice: None,
            temperature: Some(0.5),
            thinking: None,
            output_config: None,
            max_tokens: Some(64000),
            stream: true,
        };

        let body: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&request).unwrap()).unwrap();
        assert!(body.get("thinking").is_none());
        assert!(body.get("output_config").is_none());
        assert_eq!(body["temperature"], 0.5);
    }

    #[test]
    fn rejects_vertex_proxy_request_without_model() {
        let credentials = credentials(AIPlatform::GoogleVertexAi);
        let builder = AnthropicQueryBuilder::new(AIProvider::Anthropic, AIPlatform::GoogleVertexAi);
        let method = Method::POST;
        let headers = HeaderMap::new();

        let err = builder
            .build_proxy_request(&ProxyBuildArgs {
                method: &method,
                path: "messages",
                headers: &headers,
                body: br#"{"messages":[]}"#,
                credentials: &credentials,
            })
            .unwrap_err();

        assert!(matches!(err, Error::BadRequest(message) if message.contains("Missing 'model'")));
    }

    #[test]
    fn builds_azure_foundry_anthropic_proxy_request() {
        // Foundry resource stored with a legacy /openai/v1 suffix; the Anthropic SDK
        // sends the path as "v1/messages". Both must resolve to the resource's
        // /anthropic/v1/messages surface.
        let mut credentials = credentials(AIPlatform::Standard);
        credentials.provider = AIProvider::AzureFoundry;
        credentials.base_url = "https://wm-test-ai.services.ai.azure.com/openai/v1".to_string();
        let builder = AnthropicQueryBuilder::new(AIProvider::AzureFoundry, AIPlatform::Standard);
        let method = Method::POST;
        let mut headers = HeaderMap::new();
        headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
        headers.insert("X-Anthropic-SDK", HeaderValue::from_static("true"));

        let request = builder
            .build_proxy_request(&ProxyBuildArgs {
                method: &method,
                path: "v1/messages",
                headers: &headers,
                body: br#"{"model":"claude-sonnet-5","messages":[]}"#,
                credentials: &credentials,
            })
            .unwrap();

        assert_eq!(
            request.url,
            "https://wm-test-ai.services.ai.azure.com/anthropic/v1/messages"
        );
        assert!(has_header(&request.headers, "X-API-Key", "api-key"));
    }
}
