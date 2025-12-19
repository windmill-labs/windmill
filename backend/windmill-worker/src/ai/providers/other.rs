use async_trait::async_trait;
use serde::Serialize;
use serde_json;
use windmill_common::{ai_providers::AIProvider, client::AuthedClient, error::Error};

use crate::ai::{
    image_handler::prepare_messages_for_api,
    query_builder::{BuildRequestArgs, ParsedResponse, QueryBuilder, StreamEventProcessor},
    sse::{OpenAISSEParser, SSEParser},
    types::*,
    utils::should_use_structured_output_tool,
};

#[derive(Serialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum ToolChoice {
    #[allow(dead_code)]
    Auto,
    Required,
}

#[derive(Serialize)]
pub struct OpenAICompletionRequest<'a> {
    pub model: &'a str,
    pub messages: &'a [OpenAIMessage],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<&'a [ToolDef]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
    pub stream: bool,
}

/// Query builder for providers using the OpenAI-compatible completion endpoint
/// (Mistral, DeepSeek, Groq, TogetherAI, CustomAI, etc.)
pub struct OtherQueryBuilder {
    provider_kind: AIProvider,
}

impl OtherQueryBuilder {
    pub fn new(provider_kind: AIProvider) -> Self {
        Self { provider_kind }
    }

    async fn build_text_request(
        &self,
        args: &BuildRequestArgs<'_>,
        client: &AuthedClient,
        workspace_id: &str,
    ) -> Result<String, Error> {
        let prepared_messages =
            prepare_messages_for_api(args.messages, client, workspace_id).await?;

        // Check if we need to add response_format for structured output
        let has_output_properties = args
            .output_schema
            .and_then(|schema| schema.properties.as_ref())
            .map(|props| !props.is_empty())
            .unwrap_or(false);

        let should_use_structured_output_tool =
            should_use_structured_output_tool(&self.provider_kind, args.model);

        // Only use response_format for providers that support it (not Claude/Anthropic)
        // Claude models use a tool for structured output instead
        let response_format = if has_output_properties
            && args.output_schema.is_some()
            && !should_use_structured_output_tool
        {
            let schema = args.output_schema.unwrap();
            let strict_schema = schema.clone().make_strict();
            Some(ResponseFormat {
                r#type: "json_schema".to_string(),
                json_schema: JsonSchemaFormat {
                    name: "structured_output".to_string(),
                    schema: strict_schema,
                    strict: Some(true),
                },
            })
        } else {
            None
        };

        // Force usage of structured output tool for Claude models when structured output is provided
        let tool_choice = if should_use_structured_output_tool && has_output_properties {
            Some(ToolChoice::Required)
        } else {
            None
        };

        let request = OpenAICompletionRequest {
            model: args.model,
            messages: &prepared_messages,
            tools: args.tools,
            temperature: args.temperature,
            max_completion_tokens: args.max_tokens,
            response_format,
            tool_choice,
            stream: true,
        };

        serde_json::to_string(&request)
            .map_err(|e| Error::internal_err(format!("Failed to serialize request: {}", e)))
    }
}

#[async_trait]
impl QueryBuilder for OtherQueryBuilder {
    fn supports_tools_with_output_type(&self, _output_type: &OutputType) -> bool {
        // Most providers support tools for text output
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
            "Image response not supported".to_string(),
        ))
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
            annotations: None,
            used_websearch: false,
        })
    }

    fn get_endpoint(&self, base_url: &str, _model: &str, _output_type: &OutputType) -> String {
        format!("{}/chat/completions", base_url)
    }

    fn get_auth_headers(
        &self,
        api_key: &str,
        _base_url: &str,
        _output_type: &OutputType,
    ) -> Vec<(&'static str, String)> {
        vec![("Authorization", format!("Bearer {}", api_key))]
    }
}
