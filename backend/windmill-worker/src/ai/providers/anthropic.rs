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

/// Custom tool for Anthropic native API (flat structure with type: "custom")
#[derive(Serialize, Debug)]
pub struct AnthropicCustomTool {
    pub r#type: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub input_schema: Box<RawValue>,
}

impl From<&ToolDef> for AnthropicCustomTool {
    fn from(tool: &ToolDef) -> Self {
        AnthropicCustomTool {
            r#type: "custom".to_string(),
            name: tool.function.name.clone(),
            description: tool.function.description.clone(),
            input_schema: tool.function.parameters.clone(),
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
    Text { text: String },
    #[serde(rename = "image")]
    Image { source: AnthropicImageSource },
    #[serde(rename = "tool_use")]
    ToolUse { id: String, name: String, input: Box<RawValue> },
    #[serde(rename = "tool_result")]
    ToolResult { tool_use_id: String, content: String },
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
}

/// Message format for Anthropic API requests
#[derive(Serialize, Debug)]
pub struct AnthropicMessage {
    pub role: String,
    pub content: Vec<AnthropicRequestContent>,
}

/// Anthropic-specific request structure
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
                        content.push(AnthropicRequestContent::Text { text });
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
                vec![AnthropicRequestContent::Text { text: text.clone() }]
            }
        }
        OpenAIContent::Parts(parts) => {
            let mut result = Vec::new();
            for part in parts {
                match part {
                    ContentPart::Text { text } => {
                        if !text.is_empty() {
                            result.push(AnthropicRequestContent::Text { text: text.clone() });
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
}

impl AnthropicQueryBuilder {
    pub fn new(provider_kind: AIProvider) -> Self {
        Self { provider_kind }
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
        let anthropic_messages = convert_messages_to_anthropic(&prepared_messages);

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

        let request = AnthropicRequest {
            model: args.model,
            system,
            messages: anthropic_messages,
            tools: if tools.is_empty() { None } else { Some(tools) },
            tool_choice,
            temperature: args.temperature,
            max_tokens: Some(args.max_tokens.unwrap_or(64000)),
            stream: true,
        };

        serde_json::to_string(&request)
            .map_err(|e| Error::internal_err(format!("Failed to serialize request: {}", e)))
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

    async fn parse_response(&self, response: reqwest::Response) -> Result<ParsedResponse, Error> {
        // Parse Anthropic native response format
        let response_text = response
            .text()
            .await
            .map_err(|e| Error::internal_err(format!("Failed to read response text: {}", e)))?;

        tracing::info!(
            "[AI_PROVIDER_RESPONSE] Anthropic raw response: {}",
            response_text
        );

        let anthropic_response: AnthropicResponse =
            serde_json::from_str(&response_text).map_err(|e| {
                Error::internal_err(format!(
                    "Failed to parse Anthropic response: {}. Raw response: {}",
                    e, response_text
                ))
            })?;

        // Extract text content and tool calls from content blocks
        let mut text_parts: Vec<String> = Vec::new();
        let mut tool_calls: Vec<super::openai::OpenAIToolCall> = Vec::new();

        for block in anthropic_response.content {
            match block {
                AnthropicContentBlock::Text { text, .. } => {
                    text_parts.push(text);
                }
                AnthropicContentBlock::ToolUse { id, name, input } => {
                    // Convert to OpenAI tool call format for compatibility
                    tool_calls.push(super::openai::OpenAIToolCall {
                        id,
                        function: super::openai::OpenAIFunction {
                            name,
                            arguments: serde_json::to_string(&input).unwrap_or_default(),
                        },
                        r#type: "function".to_string(),
                        extra_content: None,
                    });
                }
                // Skip server_tool_use and web_search_tool_result - they are internal to Anthropic
                AnthropicContentBlock::ServerToolUse { .. } => {}
                AnthropicContentBlock::WebSearchToolResult { .. } => {}
                AnthropicContentBlock::Unknown => {}
            }
        }

        let content = if text_parts.is_empty() {
            None
        } else {
            Some(text_parts.join(""))
        };

        Ok(ParsedResponse::Text {
            content,
            tool_calls,
            events_str: None,
            annotations: Vec::new(),
            used_websearch: false,
        })
    }

    async fn parse_streaming_response(
        &self,
        response: reqwest::Response,
        stream_event_processor: StreamEventProcessor,
    ) -> Result<ParsedResponse, Error> {
        let mut anthropic_sse_parser = AnthropicSSEParser::new(stream_event_processor);
        anthropic_sse_parser.parse_events(response).await?;

        let AnthropicSSEParser { accumulated_content, accumulated_tool_calls, events_str, .. } =
            anthropic_sse_parser;

        // Note: Tool call arguments events are already sent by the parser during streaming
        // when content_block_stop is received

        Ok(ParsedResponse::Text {
            content: if accumulated_content.is_empty() {
                None
            } else {
                Some(accumulated_content)
            },
            tool_calls: accumulated_tool_calls.into_values().collect(),
            events_str: Some(events_str),
            annotations: Vec::new(),
            used_websearch: false,
        })
    }

    fn get_endpoint(&self, base_url: &str, _model: &str, _output_type: &OutputType) -> String {
        format!("{}/messages", base_url)
    }

    fn get_auth_headers(
        &self,
        api_key: &str,
        _base_url: &str,
        _output_type: &OutputType,
    ) -> Vec<(&'static str, String)> {
        vec![
            ("x-api-key", api_key.to_string()),
            ("anthropic-version", "2023-06-01".to_string()),
        ]
    }
}
