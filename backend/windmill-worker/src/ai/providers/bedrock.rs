//! AWS Bedrock provider for the AI agent.
//!
//! Uses shared SDK code from windmill_common::ai_bedrock for:
//! - BedrockClient (SDK wrapper with auth)
//! - Message conversion (OpenAI format -> Bedrock format)
//! - Stream event parsing
//! - Helper utilities

use crate::ai::{
    image_handler::prepare_messages_for_api,
    query_builder::{ParsedResponse, StreamEventProcessor},
    types::StreamingEvent,
    types::TokenUsage,
    types::{OpenAIMessage, ToolDef},
};
use std::collections::HashMap;
use windmill_common::{client::AuthedClient, error::Error};

// Re-export from shared module for use by other parts of the worker
use windmill_common::ai_bedrock::{
    bedrock_stream_event_is_block_stop, bedrock_stream_event_to_text,
    bedrock_stream_event_to_tool_delta, bedrock_stream_event_to_tool_start, build_tool_config,
    create_inference_config, format_bedrock_error, openai_messages_to_bedrock,
    streaming_tool_calls_to_openai, StreamingToolCall,
};
pub use windmill_common::ai_bedrock::{check_env_credentials, BedrockClient};

// ============================================================================
// Query Builder (Worker-specific orchestration)
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
        max_tokens: Option<u32>,
        api_key: &str,
        region: &str,
        stream_event_processor: Option<StreamEventProcessor>,
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
        let (bedrock_messages, system_prompts) = openai_messages_to_bedrock(&prepared_messages)?;

        // Build inference configuration using shared helper
        let inference_config = create_inference_config(temperature, max_tokens.map(|t| t as i32));

        // Build tool configuration with optional ToolChoice
        let tool_config = build_tool_config(tools, structured_output_tool_name.is_some())?;

        self.execute_converse_stream(
            &bedrock_client,
            model,
            bedrock_messages,
            system_prompts,
            inference_config,
            tool_config,
            stream_event_processor,
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
        stream_event_processor: Option<StreamEventProcessor>,
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

        // Process stream events using shared parsing functions
        loop {
            match stream.recv().await {
                Ok(Some(event)) => {
                    // Handle tool use start using shared parser
                    if let Some(tool_call) = bedrock_stream_event_to_tool_start(&event) {
                        current_tool_use_id = Some(tool_call.id.clone());
                        accumulated_tool_calls.insert(tool_call.id.clone(), tool_call);
                    }

                    // Handle text delta using shared parser
                    if let Some(text_delta) = bedrock_stream_event_to_text(&event) {
                        accumulated_text.push_str(&text_delta);
                        if let Some(processor) = stream_event_processor.as_ref() {
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
        if let Some(processor) = stream_event_processor.as_ref() {
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

        let tool_calls =
            streaming_tool_calls_to_openai(accumulated_tool_calls.into_values().collect());

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
