use async_trait::async_trait;
use serde::Serialize;
use windmill_common::{ai_providers::AIProvider, client::AuthedClient, error::Error};

use crate::ai::{
    image_handler::prepare_messages_for_api,
    query_builder::{BuildRequestArgs, ParsedResponse, QueryBuilder, StreamEventProcessor},
    sse::{OpenAISSEParser, SSEParser},
    types::*,
};

use super::other::OpenAIResponse;

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
                "type": "web_search",
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
            max_tokens: args.max_tokens,
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
        // Parse Anthropic response (similar to OpenAI format)
        let openai_response: OpenAIResponse = response
            .json()
            .await
            .map_err(|e| Error::internal_err(format!("Failed to parse response: {}", e)))?;

        let first_choice = openai_response
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| Error::internal_err("No response from API"))?;

        Ok(ParsedResponse::Text {
            content: first_choice.message.content.map(|c| match c {
                OpenAIContent::Text(text) => text,
                OpenAIContent::Parts(parts) => {
                    // Extract text from parts
                    parts
                        .into_iter()
                        .filter_map(|part| match part {
                            ContentPart::Text { text } => Some(text),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join(" ")
                }
            }),
            tool_calls: first_choice.message.tool_calls.unwrap_or_default(),
            events_str: None,
        })
    }

    async fn parse_streaming_response(
        &self,
        response: reqwest::Response,
        stream_event_processor: StreamEventProcessor,
    ) -> Result<ParsedResponse, Error> {
        let mut openai_sse_parser = OpenAISSEParser::new(stream_event_processor);
        openai_sse_parser.parse_events(response).await?;

        let OpenAISSEParser {
            accumulated_content,
            accumulated_tool_calls,
            mut events_str,
            stream_event_processor,
        } = openai_sse_parser;

        // Process streaming events with error handling

        for tool_call in accumulated_tool_calls.values() {
            let event = StreamingEvent::ToolCallArguments {
                call_id: tool_call.id.clone(),
                function_name: tool_call.function.name.clone(),
                arguments: tool_call.function.arguments.clone(),
            };
            stream_event_processor.send(event, &mut events_str).await?;
        }

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
