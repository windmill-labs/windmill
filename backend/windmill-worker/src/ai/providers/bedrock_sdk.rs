use crate::ai::{
    image_handler::prepare_messages_for_api,
    query_builder::{ParsedResponse, StreamEventProcessor},
    types::StreamingEvent,
};
use std::collections::HashMap;
use windmill_common::{
    ai_types::{OpenAIMessage, ToolDef},
    aws_bedrock::{
        bedrock_response_to_openai, bedrock_stream_event_is_block_stop,
        bedrock_stream_event_to_text, bedrock_stream_event_to_tool_delta,
        bedrock_stream_event_to_tool_start, create_inference_config, openai_messages_to_bedrock,
        openai_tools_to_bedrock, streaming_tool_calls_to_openai, BedrockClient, StreamingToolCall,
    },
    client::AuthedClient,
    error::Error,
};

pub struct BedrockQueryBuilder;

impl BedrockQueryBuilder {
    pub fn new() -> Self {
        Self
    }

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
        should_stream: bool,
        stream_event_processor: Option<StreamEventProcessor>,
        client: &AuthedClient,
        workspace_id: &str,
        structured_output_tool_name: Option<&str>,
    ) -> Result<ParsedResponse, Error> {
        // Create Bedrock client with bearer token authentication
        let bedrock_client = BedrockClient::from_bearer_token(api_key.to_string(), region).await?;

        // Prepare messages: convert S3Objects to ImageUrls by downloading from S3
        let prepared_messages = prepare_messages_for_api(messages, client, workspace_id).await?;

        // Convert messages to Bedrock format (separates system prompts)
        let (bedrock_messages, system_prompts) = openai_messages_to_bedrock(&prepared_messages)?;

        // Build inference configuration
        let inference_config = create_inference_config(temperature, max_tokens.map(|t| t as i32));

        // Build tool configuration with optional ToolChoice
        let tool_config = self.build_tool_config(tools, structured_output_tool_name.is_some())?;

        if should_stream {
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
        } else {
            self.execute_converse(
                &bedrock_client,
                model,
                bedrock_messages,
                system_prompts,
                inference_config,
                tool_config,
            )
            .await
        }
    }

    /// Build tool configuration with optional ToolChoice for structured output
    fn build_tool_config(
        &self,
        tools: Option<&[ToolDef]>,
        force_tool_use: bool,
    ) -> Result<Option<aws_sdk_bedrockruntime::types::ToolConfiguration>, Error> {
        if let Some(tools) = tools {
            let bedrock_tools = openai_tools_to_bedrock(tools)?;
            let mut tool_config_builder =
                aws_sdk_bedrockruntime::types::ToolConfiguration::builder()
                    .set_tools(Some(bedrock_tools));

            // For structured output, force the model to use the tool
            if force_tool_use {
                tool_config_builder = tool_config_builder.tool_choice(
                    aws_sdk_bedrockruntime::types::ToolChoice::Any(
                        aws_sdk_bedrockruntime::types::AnyToolChoice::builder().build(),
                    ),
                );
            }

            Ok(Some(tool_config_builder.build().map_err(|e| {
                Error::internal_err(format!("Failed to build tool configuration: {}", e))
            })?))
        } else {
            Ok(None)
        }
    }

    /// Execute non-streaming Bedrock request
    async fn execute_converse(
        &self,
        bedrock_client: &BedrockClient,
        model: &str,
        bedrock_messages: Vec<aws_sdk_bedrockruntime::types::Message>,
        system_prompts: Vec<aws_sdk_bedrockruntime::types::SystemContentBlock>,
        inference_config: Option<aws_sdk_bedrockruntime::types::InferenceConfiguration>,
        tool_config: Option<aws_sdk_bedrockruntime::types::ToolConfiguration>,
    ) -> Result<ParsedResponse, Error> {
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

        // Execute the request
        let response = request_builder
            .send()
            .await
            .map_err(|e| Error::internal_err(format!("Bedrock API error: {}", e)))?;

        // Convert response back to OpenAI format
        let (content, tool_calls) = bedrock_response_to_openai(&response)?;

        Ok(ParsedResponse::Text { content, tool_calls, events_str: None })
    }

    /// Execute streaming Bedrock request
    pub async fn execute_converse_stream(
        &self,
        bedrock_client: &BedrockClient,
        model: &str,
        bedrock_messages: Vec<aws_sdk_bedrockruntime::types::Message>,
        system_prompts: Vec<aws_sdk_bedrockruntime::types::SystemContentBlock>,
        inference_config: Option<aws_sdk_bedrockruntime::types::InferenceConfiguration>,
        tool_config: Option<aws_sdk_bedrockruntime::types::ToolConfiguration>,
        stream_event_processor: Option<StreamEventProcessor>,
    ) -> Result<ParsedResponse, Error> {
        // Build streaming request
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

        // Execute streaming request
        let mut stream = request_builder
            .send()
            .await
            .map_err(|e| Error::internal_err(format!("Bedrock streaming API error: {}", e)))?
            .stream;

        let mut accumulated_text = String::new();
        let mut events_str = String::new();
        let mut accumulated_tool_calls: HashMap<String, StreamingToolCall> = HashMap::new();
        let mut current_tool_use_id: Option<String> = None;

        // Process stream events
        loop {
            match stream.recv().await {
                Ok(Some(event)) => {
                    // Handle tool use start
                    if let Some(tool_call) = bedrock_stream_event_to_tool_start(&event) {
                        current_tool_use_id = Some(tool_call.id.clone());
                        accumulated_tool_calls.insert(tool_call.id.clone(), tool_call);
                    }

                    // Handle text delta
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

                    // Handle tool use input delta
                    if let Some(input_delta) = bedrock_stream_event_to_tool_delta(&event) {
                        if let Some(tool_id) = &current_tool_use_id {
                            if let Some(tool_call) = accumulated_tool_calls.get_mut(tool_id) {
                                tool_call.arguments.push_str(&input_delta);
                            }
                        }
                    }

                    // Handle content block stop
                    if bedrock_stream_event_is_block_stop(&event) {
                        current_tool_use_id = None;
                    }
                }
                Ok(None) => break, // Stream ended
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
        })
    }
}
