use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use windmill_common::{ai_providers::AIProvider, client::AuthedClient, error::Error};

use crate::ai::{
    image_handler::prepare_messages_for_api,
    query_builder::{BuildRequestArgs, ParsedResponse, QueryBuilder, StreamEventProcessor},
    sse::{AnthropicSSEParser, SSEParser},
    types::*,
};

/// Anthropic-specific request structure
#[derive(Serialize)]
pub struct AnthropicRequest<'a> {
    pub model: &'a str,
    pub messages: &'a [OpenAIMessage],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    pub stream: bool,
}

/// Anthropic content block - can be text, tool_use, server_tool_use, or web_search_tool_result
#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum AnthropicContentBlock {
    #[serde(rename = "text")]
    Text {
        text: String,
        #[serde(default)]
        citations: Vec<serde_json::Value>,
    },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(rename = "server_tool_use")]
    ServerToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(rename = "web_search_tool_result")]
    WebSearchToolResult {
        tool_use_id: String,
        content: Vec<serde_json::Value>,
    },
    #[serde(other)]
    Unknown,
}

/// Anthropic native API response
#[derive(Deserialize, Debug)]
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
        stream: bool,
    ) -> Result<String, Error> {
        let prepared_messages =
            prepare_messages_for_api(args.messages, client, workspace_id).await?;

        // Build tools array
        let mut tools: Vec<serde_json::Value> = Vec::new();

        // Add websearch tool if enabled (Anthropic format)
        if args.has_websearch {
            tools.push(serde_json::json!({
                "type": "web_search_20250305",
                "name": "web_search"
            }));
        }

        // Add custom tools
        if let Some(tool_defs) = args.tools {
            for tool in tool_defs {
                tools.push(serde_json::to_value(tool).map_err(|e| {
                    Error::internal_err(format!("Failed to serialize tool: {}", e))
                })?);
            }
        }

        let request = AnthropicRequest {
            model: args.model,
            messages: &prepared_messages,
            tools: if tools.is_empty() { None } else { Some(tools) },
            temperature: args.temperature,
            max_tokens: Some(args.max_tokens.unwrap_or(64000)),
            stream,
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

    fn supports_streaming(&self) -> bool {
        // Anthropic supports streaming
        true
    }

    async fn build_request(
        &self,
        args: &BuildRequestArgs<'_>,
        client: &AuthedClient,
        workspace_id: &str,
        stream: bool,
    ) -> Result<String, Error> {
        self.build_text_request(args, client, workspace_id, stream)
            .await
    }

    async fn parse_response(&self, response: reqwest::Response) -> Result<ParsedResponse, Error> {
        // Parse Anthropic native response format
        let response_text = response
            .text()
            .await
            .map_err(|e| Error::internal_err(format!("Failed to read response text: {}", e)))?;

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

        Ok(ParsedResponse::Text { content, tool_calls, events_str: None })
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
            ..
        } = anthropic_sse_parser;

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
        })
    }

    fn get_endpoint(
        &self,
        base_url: &str,
        _model: &str,
        _output_type: &OutputType,
        _stream: bool,
    ) -> String {
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
