use std::collections::HashMap;

use eventsource_stream::Eventsource;
use reqwest::Response;
use serde::Deserialize;
use serde_json;
use tokio_stream::StreamExt;
use windmill_common::{error::Error, utils::rd_string};

use crate::ai::{
    providers::openai::{ExtraContent, OpenAIFunction, OpenAIToolCall},
    query_builder::StreamEventProcessor,
    types::StreamingEvent,
};

#[derive(Deserialize)]
pub struct OpenAIChoiceDeltaToolCallFunction {
    pub name: Option<String>,
    pub arguments: Option<String>,
}

#[derive(Deserialize)]
pub struct OpenAIChoiceDeltaToolCall {
    pub index: Option<i64>,
    pub id: Option<String>,
    pub function: Option<OpenAIChoiceDeltaToolCallFunction>,
    /// Extra content for provider-specific metadata (e.g., Google Gemini thought signatures)
    pub extra_content: Option<ExtraContent>,
}

#[derive(Deserialize)]
pub struct OpenAIChoiceDelta {
    pub content: Option<String>,
    pub tool_calls: Option<Vec<OpenAIChoiceDeltaToolCall>>,
}

#[derive(Deserialize)]
pub struct OpenAIChoice {
    pub delta: Option<OpenAIChoiceDelta>,
}

#[derive(Deserialize)]
pub struct OpenAISSEEvent {
    pub choices: Option<Vec<OpenAIChoice>>,
}

lazy_static::lazy_static! {
    static ref DEBUG_SSE_STREAM: bool = std::env::var("DEBUG_SSE_STREAM")
        .unwrap_or("false".to_string())
        .parse::<bool>()
        .unwrap_or(false);
}
pub trait SSEParser {
    async fn parse_event_data(&mut self, data: &str) -> Result<(), Error>;

    async fn parse_events(&mut self, response: Response) -> Result<(), Error> {
        let mut stream = response.bytes_stream().eventsource();

        while let Some(event) = stream.next().await {
            match event {
                Ok(event) => {
                    if *DEBUG_SSE_STREAM {
                        tracing::info!("SSE event: {:?}", event);
                    }

                    self.parse_event_data(&event.data).await?;
                }
                Err(e) => {
                    tracing::error!("Failed to parse SSE event: {}", e);
                }
            }
        }

        Ok(())
    }
}

pub struct OpenAISSEParser {
    pub accumulated_content: String,
    pub accumulated_tool_calls: HashMap<i64, OpenAIToolCall>,
    pub events_str: String,
    pub stream_event_processor: StreamEventProcessor,
}

impl OpenAISSEParser {
    pub fn new(stream_event_processor: StreamEventProcessor) -> Self {
        Self {
            accumulated_content: String::new(),
            accumulated_tool_calls: HashMap::new(),
            events_str: String::new(),
            stream_event_processor,
        }
    }
}

impl SSEParser for OpenAISSEParser {
    async fn parse_event_data(&mut self, data: &str) -> Result<(), Error> {
        if data == "[DONE]" {
            return Ok(());
        }

        let event: Option<OpenAISSEEvent> = serde_json::from_str(data)
            .inspect_err(|e| {
                tracing::error!("Failed to parse SSE as an OpenAI event {}: {}", data, e);
            })
            .ok();

        if let Some(event) = event {
            if let Some(mut choices) = event.choices.filter(|s| !s.is_empty()) {
                if let Some(delta) = choices.remove(0).delta {
                    if let Some(content) = delta.content.filter(|s| !s.is_empty()) {
                        self.accumulated_content.push_str(&content);
                        let event = StreamingEvent::TokenDelta { content };
                        self.stream_event_processor
                            .send(event, &mut self.events_str)
                            .await?;
                    }

                    if let Some(tool_calls) = delta.tool_calls {
                        for (idx, tool_call) in tool_calls.into_iter().enumerate() {
                            let idx = tool_call.index.unwrap_or_else(|| idx as i64);

                            if let Some(function) = tool_call.function {
                                if let Some(existing_tool_call) =
                                    self.accumulated_tool_calls.get_mut(&idx)
                                {
                                    if let Some(arguments) = function.arguments {
                                        existing_tool_call.function.arguments += &arguments;
                                    }
                                    // Update extra_content if provided in this delta (for thought signatures)
                                    if let Some(extra) = tool_call.extra_content {
                                        existing_tool_call.extra_content = Some(extra);
                                    }
                                } else {
                                    let fun_name = function.name.unwrap_or_default();
                                    let call_id = tool_call.id.unwrap_or_else(|| rd_string(24));
                                    let event = StreamingEvent::ToolCall {
                                        call_id: call_id.clone(),
                                        function_name: fun_name.clone(),
                                    };
                                    self.stream_event_processor
                                        .send(event, &mut self.events_str)
                                        .await?;
                                    self.accumulated_tool_calls.insert(
                                        idx,
                                        OpenAIToolCall {
                                            id: call_id,
                                            function: OpenAIFunction {
                                                name: fun_name,
                                                arguments: function.arguments.unwrap_or_default(),
                                            },
                                            r#type: "function".to_string(),
                                            extra_content: tool_call.extra_content,
                                        },
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

// ============================================================================
// Anthropic SSE Parser
// ============================================================================

/// Content block start types for Anthropic streaming
#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum AnthropicContentBlockStart {
    #[serde(rename = "text")]
    Text {
        #[serde(default)]
        text: String,
    },
    #[serde(rename = "tool_use")]
    ToolUse { id: String, name: String },
    #[serde(other)]
    Unknown,
}

/// Delta types for Anthropic streaming content blocks
#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum AnthropicDelta {
    #[serde(rename = "text_delta")]
    TextDelta { text: String },
    #[serde(rename = "input_json_delta")]
    InputJsonDelta { partial_json: String },
    #[serde(other)]
    Unknown,
}

/// Anthropic SSE event structure
#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum AnthropicSSEEvent {
    #[serde(rename = "message_start")]
    MessageStart {},
    #[serde(rename = "content_block_start")]
    ContentBlockStart {
        index: usize,
        content_block: AnthropicContentBlockStart,
    },
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta { index: usize, delta: AnthropicDelta },
    #[serde(rename = "content_block_stop")]
    ContentBlockStop { index: usize },
    #[serde(rename = "message_delta")]
    MessageDelta {},
    #[serde(rename = "message_stop")]
    MessageStop {},
    #[serde(rename = "ping")]
    Ping {},
    #[serde(rename = "error")]
    Error {
        #[serde(default)]
        message: Option<String>,
    },
    #[serde(other)]
    Unknown,
}

/// Tracks state of a content block during streaming
#[derive(Debug)]
enum ContentBlockState {
    Text,
    ToolUse { id: String, name: String },
    Unknown,
}

/// Anthropic SSE Parser for streaming responses
pub struct AnthropicSSEParser {
    pub accumulated_content: String,
    pub accumulated_tool_calls: HashMap<i64, OpenAIToolCall>,
    pub events_str: String,
    pub stream_event_processor: StreamEventProcessor,
    /// Track content block types by index
    content_blocks: HashMap<usize, ContentBlockState>,
}

impl AnthropicSSEParser {
    pub fn new(stream_event_processor: StreamEventProcessor) -> Self {
        Self {
            accumulated_content: String::new(),
            accumulated_tool_calls: HashMap::new(),
            events_str: String::new(),
            stream_event_processor,
            content_blocks: HashMap::new(),
        }
    }
}

impl SSEParser for AnthropicSSEParser {
    async fn parse_event_data(&mut self, data: &str) -> Result<(), Error> {
        let event: Option<AnthropicSSEEvent> = serde_json::from_str(data)
            .inspect_err(|e| {
                tracing::error!("Failed to parse SSE as an Anthropic event {}: {}", data, e);
            })
            .ok();

        if let Some(event) = event {
            match event {
                AnthropicSSEEvent::ContentBlockStart { index, content_block } => {
                    match content_block {
                        AnthropicContentBlockStart::Text { text } => {
                            self.content_blocks.insert(index, ContentBlockState::Text);
                            // Some responses include initial text in the content_block_start
                            if !text.is_empty() {
                                self.accumulated_content.push_str(&text);
                                let event = StreamingEvent::TokenDelta { content: text };
                                self.stream_event_processor
                                    .send(event, &mut self.events_str)
                                    .await?;
                            }
                        }
                        AnthropicContentBlockStart::ToolUse { id, name } => {
                            self.content_blocks
                                .insert(index, ContentBlockState::ToolUse { id: id.clone(), name: name.clone() });
                            // Send tool call start event
                            let event = StreamingEvent::ToolCall {
                                call_id: id.clone(),
                                function_name: name.clone(),
                            };
                            self.stream_event_processor
                                .send(event, &mut self.events_str)
                                .await?;
                            // Initialize tool call accumulator
                            self.accumulated_tool_calls.insert(
                                index as i64,
                                OpenAIToolCall {
                                    id,
                                    function: OpenAIFunction { name, arguments: String::new() },
                                    r#type: "function".to_string(),
                                    extra_content: None,
                                },
                            );
                        }
                        AnthropicContentBlockStart::Unknown => {
                            self.content_blocks.insert(index, ContentBlockState::Unknown);
                        }
                    }
                }
                AnthropicSSEEvent::ContentBlockDelta { index, delta } => {
                    match delta {
                        AnthropicDelta::TextDelta { text } => {
                            // Only accumulate text if this is a text block (not server_tool_use results)
                            if let Some(ContentBlockState::Text) = self.content_blocks.get(&index) {
                                self.accumulated_content.push_str(&text);
                                let event = StreamingEvent::TokenDelta { content: text };
                                self.stream_event_processor
                                    .send(event, &mut self.events_str)
                                    .await?;
                            }
                        }
                        AnthropicDelta::InputJsonDelta { partial_json } => {
                            // Accumulate tool call arguments
                            if let Some(tool_call) =
                                self.accumulated_tool_calls.get_mut(&(index as i64))
                            {
                                tool_call.function.arguments.push_str(&partial_json);
                            }
                        }
                        AnthropicDelta::Unknown => {}
                    }
                }
                AnthropicSSEEvent::ContentBlockStop { index } => {
                    // Send tool call arguments event if this was a tool use block
                    if let Some(tool_call) = self.accumulated_tool_calls.get(&(index as i64)) {
                        let event = StreamingEvent::ToolCallArguments {
                            call_id: tool_call.id.clone(),
                            function_name: tool_call.function.name.clone(),
                            arguments: tool_call.function.arguments.clone(),
                        };
                        self.stream_event_processor
                            .send(event, &mut self.events_str)
                            .await?;
                    }
                }
                AnthropicSSEEvent::Error { message } => {
                    let error_msg = message.unwrap_or_else(|| "Unknown error".to_string());
                    tracing::error!("Anthropic streaming error: {}", error_msg);
                }
                // Ignore other events
                AnthropicSSEEvent::MessageStart {}
                | AnthropicSSEEvent::MessageDelta {}
                | AnthropicSSEEvent::MessageStop {}
                | AnthropicSSEEvent::Ping {}
                | AnthropicSSEEvent::Unknown => {}
            }
        }

        Ok(())
    }
}
