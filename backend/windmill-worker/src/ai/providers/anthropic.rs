use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use windmill_common::{ai_providers::AIProvider, client::AuthedClient, error::Error};

use crate::ai::{
    image_handler::prepare_messages_for_api,
    query_builder::{BuildRequestArgs, ParsedResponse, QueryBuilder, StreamEventProcessor},
    sse::{AnthropicSSEParser, SSEParser},
    types::*,
    utils::{extract_text_content, parse_data_url, should_use_structured_output_tool},
};

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
    #[serde(rename = "image")]
    Image { source: AnthropicImageSource },
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

/// Image source for Anthropic API
#[derive(Serialize, Debug)]
pub struct AnthropicImageSource {
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
                        // Handle base64 images
                        if let Some((media_type, data)) = parse_data_url(&image_url.url) {
                            result.push(AnthropicRequestContent::Image {
                                source: AnthropicImageSource {
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
    platform: AnthropicPlatform,
    enable_1m_context: bool,
}

impl AnthropicQueryBuilder {
    pub fn new(
        provider_kind: AIProvider,
        platform: AnthropicPlatform,
        enable_1m_context: bool,
    ) -> Self {
        Self { provider_kind, platform, enable_1m_context }
    }

    fn is_vertex(&self) -> bool {
        self.platform == AnthropicPlatform::GoogleVertexAi
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

        // Build request based on platform
        if self.is_vertex() {
            // For Vertex AI: no model field, anthropic_version in body
            let request = AnthropicVertexRequest {
                anthropic_version: ANTHROPIC_VERSION_VERTEX,
                system,
                messages: anthropic_messages,
                tools: tools_option,
                tool_choice,
                temperature: args.temperature,
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
                temperature: args.temperature,
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
        stream_event_processor: StreamEventProcessor,
    ) -> Result<ParsedResponse, Error> {
        let mut anthropic_sse_parser = AnthropicSSEParser::new(stream_event_processor);
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
            let mut headers = vec![
                ("x-api-key", api_key.to_string()),
                ("anthropic-version", ANTHROPIC_VERSION_STANDARD.to_string()),
            ];
            if self.enable_1m_context {
                headers.push(("anthropic-beta", "context-1m-2025-08-07".to_string()));
            }
            headers
        }
    }
}
