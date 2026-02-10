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

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ToolChoice {
    #[allow(dead_code)]
    Auto,
    Required,
}

/// Stream options for OpenAI API to include usage in streaming responses
#[derive(Serialize)]
pub struct StreamOptions {
    pub include_usage: bool,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_options: Option<StreamOptions>,
}

/// Result from building a request, includes both with and without stream_options
pub struct BuiltRequests {
    pub with_usage: String,
    pub without_usage: String,
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

    /// Build both request variants (with and without stream_options) to enable retry on incompatible providers
    pub async fn build_text_requests(
        &self,
        args: &BuildRequestArgs<'_>,
        client: &AuthedClient,
        workspace_id: &str,
    ) -> Result<BuiltRequests, Error> {
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
            let mut strict_schema = args.output_schema.unwrap().clone();
            strict_schema.make_strict();
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

        // Build request with stream_options for usage tracking
        let request_with_usage = OpenAICompletionRequest {
            model: args.model,
            messages: &prepared_messages,
            tools: args.tools,
            temperature: args.temperature,
            max_completion_tokens: args.max_tokens,
            response_format: response_format.clone(),
            tool_choice: tool_choice.clone(),
            stream: true,
            stream_options: Some(StreamOptions { include_usage: true }),
        };

        // Build request without stream_options for providers that don't support it
        let request_without_usage = OpenAICompletionRequest {
            model: args.model,
            messages: &prepared_messages,
            tools: args.tools,
            temperature: args.temperature,
            max_completion_tokens: args.max_tokens,
            response_format,
            tool_choice,
            stream: true,
            stream_options: None,
        };

        let with_usage = serde_json::to_string(&request_with_usage)
            .map_err(|e| Error::internal_err(format!("Failed to serialize request: {}", e)))?;
        let without_usage = serde_json::to_string(&request_without_usage)
            .map_err(|e| Error::internal_err(format!("Failed to serialize request: {}", e)))?;

        Ok(BuiltRequests { with_usage, without_usage })
    }

    async fn build_text_request(
        &self,
        args: &BuildRequestArgs<'_>,
        client: &AuthedClient,
        workspace_id: &str,
    ) -> Result<String, Error> {
        // Default to returning the request with usage tracking
        Ok(self
            .build_text_requests(args, client, workspace_id)
            .await?
            .with_usage)
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

    async fn build_request_without_usage(
        &self,
        args: &BuildRequestArgs<'_>,
        client: &AuthedClient,
        workspace_id: &str,
    ) -> Result<String, Error> {
        Ok(self
            .build_text_requests(args, client, workspace_id)
            .await?
            .without_usage)
    }

    fn supports_retry_without_usage(&self) -> bool {
        true
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
            usage: openai_usage,
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

        // Convert OpenAI Chat Completions usage to TokenUsage
        let usage = openai_usage
            .map(|u| TokenUsage::new(u.prompt_tokens, u.completion_tokens, u.total_tokens));

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
            usage,
        })
    }

    fn get_endpoint(&self, base_url: &str, _model: &str, _output_type: &OutputType) -> String {
        if self.provider_kind.is_azure_openai(base_url) {
            AIProvider::build_azure_openai_url(base_url, "chat/completions")
        } else {
            format!("{}/chat/completions", base_url)
        }
    }

    fn get_auth_headers(
        &self,
        api_key: &str,
        base_url: &str,
        _output_type: &OutputType,
    ) -> Vec<(&'static str, String)> {
        if self.provider_kind.is_azure_openai(base_url) {
            vec![("api-key", api_key.to_string())]
        } else {
            vec![("Authorization", format!("Bearer {}", api_key))]
        }
    }
}
