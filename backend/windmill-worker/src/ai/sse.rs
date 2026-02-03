use std::collections::HashMap;

use eventsource_stream::Eventsource;
use reqwest::Response;
use serde::Deserialize;
use serde_json;
use tokio_stream::StreamExt;
use windmill_common::{error::Error, utils::rd_string};

use crate::ai::{
    query_builder::StreamEventProcessor,
    types::{StreamingEvent, UrlCitation},
};

use windmill_common::ai_types::{ExtraContent, GoogleExtraContent, OpenAIFunction, OpenAIToolCall};

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

/// OpenAI Chat Completions API usage information (from final chunk with stream_options.include_usage)
#[derive(Deserialize, Debug, Clone, Default)]
pub struct OpenAIChatUsage {
    #[serde(default)]
    pub prompt_tokens: Option<i32>,
    #[serde(default)]
    pub completion_tokens: Option<i32>,
    #[serde(default)]
    pub total_tokens: Option<i32>,
}

#[derive(Deserialize)]
pub struct OpenAISSEEvent {
    pub choices: Option<Vec<OpenAIChoice>>,
    #[serde(default)]
    pub usage: Option<OpenAIChatUsage>,
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
        let mut consecutive_errors = 0;

        while let Some(event) = stream.next().await {
            match event {
                Ok(event) => {
                    consecutive_errors = 0;
                    if *DEBUG_SSE_STREAM {
                        tracing::info!("SSE event: {:?}", event);
                    }

                    self.parse_event_data(&event.data).await?;
                }
                Err(e) => {
                    consecutive_errors += 1;
                    tracing::error!("Failed to parse SSE event: {}", e);
                    if consecutive_errors >= 5 {
                        tracing::error!(
                            "Breaking SSE stream after {} consecutive parsing errors",
                            consecutive_errors
                        );
                        return Err(Error::InternalErr(format!(
                            "SSE stream terminated after {} consecutive parsing errors",
                            consecutive_errors
                        )));
                    }
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
    /// Token usage from final chunk (when stream_options.include_usage is true)
    pub usage: Option<OpenAIChatUsage>,
}

impl OpenAISSEParser {
    pub fn new(stream_event_processor: StreamEventProcessor) -> Self {
        Self {
            accumulated_content: String::new(),
            accumulated_tool_calls: HashMap::new(),
            events_str: String::new(),
            stream_event_processor,
            usage: None,
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
            // Extract usage from final chunk (when stream_options.include_usage is true)
            if let Some(usage) = event.usage {
                self.usage = Some(usage);
            }

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
                                            extra_content: None,
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
    #[serde(rename = "server_tool_use")]
    ServerToolUse {
        #[allow(unused)]
        id: String,
        name: String,
    },
    #[serde(other)]
    Unknown,
}

/// Citation from Anthropic web search delta events
#[derive(Deserialize, Debug)]
pub struct AnthropicCitationDelta {
    #[allow(dead_code)]
    pub r#type: String,
    pub url: String,
    #[serde(default)]
    pub title: Option<String>,
}

/// Delta types for Anthropic streaming content blocks
#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum AnthropicDelta {
    #[serde(rename = "text_delta")]
    TextDelta { text: String },
    #[serde(rename = "input_json_delta")]
    InputJsonDelta { partial_json: String },
    #[serde(rename = "citations_delta")]
    CitationsDelta { citation: AnthropicCitationDelta },
    #[serde(other)]
    Unknown,
}

/// Anthropic usage information from message_delta event
#[derive(Deserialize, Debug, Clone)]
pub struct AnthropicUsage {
    #[serde(default)]
    pub input_tokens: Option<i32>,
    #[serde(default)]
    pub output_tokens: Option<i32>,
    #[serde(default)]
    pub cache_creation_input_tokens: Option<i32>,
    #[serde(default)]
    pub cache_read_input_tokens: Option<i32>,
}

/// Anthropic SSE event structure
#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum AnthropicSSEEvent {
    #[serde(rename = "message_start")]
    MessageStart {},
    #[serde(rename = "content_block_start")]
    ContentBlockStart { index: usize, content_block: AnthropicContentBlockStart },
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta { index: usize, delta: AnthropicDelta },
    #[serde(rename = "content_block_stop")]
    ContentBlockStop { index: usize },
    #[serde(rename = "message_delta")]
    MessageDelta {
        #[serde(default)]
        usage: Option<AnthropicUsage>,
    },
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
#[allow(dead_code)]
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
    /// Collected URL citation annotations from web search
    pub annotations: Vec<UrlCitation>,
    /// Whether web search was used in this response
    pub used_websearch: bool,
    /// Token usage from message_delta event
    pub usage: Option<AnthropicUsage>,
}

impl AnthropicSSEParser {
    pub fn new(stream_event_processor: StreamEventProcessor) -> Self {
        Self {
            accumulated_content: String::new(),
            accumulated_tool_calls: HashMap::new(),
            events_str: String::new(),
            stream_event_processor,
            content_blocks: HashMap::new(),
            annotations: Vec::new(),
            used_websearch: false,
            usage: None,
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
                            self.content_blocks.insert(
                                index,
                                ContentBlockState::ToolUse { id: id.clone(), name: name.clone() },
                            );
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
                        AnthropicContentBlockStart::ServerToolUse { name, .. } => {
                            // Detect websearch tool usage
                            if name == "web_search" {
                                self.used_websearch = true;
                            }
                            self.content_blocks
                                .insert(index, ContentBlockState::Unknown);
                        }
                        AnthropicContentBlockStart::Unknown => {
                            self.content_blocks
                                .insert(index, ContentBlockState::Unknown);
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
                        AnthropicDelta::CitationsDelta { citation } => {
                            // Collect URL citations from web search
                            self.annotations.push(UrlCitation {
                                start_index: 0, // Anthropic doesn't provide start/end indices
                                end_index: 0,
                                url: citation.url,
                                title: citation.title,
                            });
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
                AnthropicSSEEvent::MessageDelta { usage } => {
                    if let Some(u) = usage {
                        self.usage = Some(u);
                    }
                }
                // Ignore other events
                AnthropicSSEEvent::MessageStart {}
                | AnthropicSSEEvent::MessageStop {}
                | AnthropicSSEEvent::Ping {}
                | AnthropicSSEEvent::Unknown => {}
            }
        }

        Ok(())
    }
}

// ============================================================================
// Gemini SSE Parser
// ============================================================================

/// Gemini streaming response part - can be text or function call
#[derive(Deserialize, Debug)]
pub struct GeminiSSEPart {
    #[serde(default)]
    pub text: Option<String>,
    #[serde(rename = "functionCall")]
    pub function_call: Option<GeminiSSEFunctionCall>,
    /// Thought signature for Gemini 3+ models - required for function calling
    #[serde(rename = "thoughtSignature")]
    pub thought_signature: Option<String>,
}

/// Function call in Gemini streaming response
#[derive(Deserialize, Debug)]
pub struct GeminiSSEFunctionCall {
    pub name: String,
    pub args: serde_json::Value,
}

/// Content in Gemini streaming candidate
#[derive(Deserialize, Debug)]
pub struct GeminiSSEContent {
    pub parts: Option<Vec<GeminiSSEPart>>,
}

/// Web reference in Gemini grounding chunk
#[derive(Deserialize, Debug)]
pub struct GeminiGroundingChunkWeb {
    pub uri: String,
    #[serde(default)]
    pub title: Option<String>,
}

/// Grounding chunk from Gemini web search
#[derive(Deserialize, Debug)]
pub struct GeminiGroundingChunk {
    pub web: Option<GeminiGroundingChunkWeb>,
}

/// Grounding metadata from Gemini web search
#[derive(Deserialize, Debug)]
pub struct GeminiGroundingMetadata {
    #[serde(rename = "groundingChunks", default)]
    pub grounding_chunks: Vec<GeminiGroundingChunk>,
    #[serde(rename = "webSearchQueries", default)]
    pub web_search_queries: Vec<String>,
}

/// Candidate in Gemini streaming response
#[derive(Deserialize, Debug)]
pub struct GeminiSSECandidate {
    pub content: Option<GeminiSSEContent>,
    #[serde(rename = "finishReason")]
    #[allow(dead_code)]
    pub finish_reason: Option<String>,
    #[serde(rename = "groundingMetadata")]
    pub grounding_metadata: Option<GeminiGroundingMetadata>,
}

/// Gemini usage metadata from SSE response
#[derive(Deserialize, Debug, Clone)]
pub struct GeminiUsageMetadata {
    #[serde(rename = "promptTokenCount", default)]
    pub prompt_token_count: Option<i32>,
    #[serde(rename = "candidatesTokenCount", default)]
    pub candidates_token_count: Option<i32>,
    #[serde(rename = "totalTokenCount", default)]
    pub total_token_count: Option<i32>,
}

/// Gemini SSE event structure
#[derive(Deserialize, Debug)]
pub struct GeminiSSEEvent {
    pub candidates: Option<Vec<GeminiSSECandidate>>,
    #[serde(rename = "usageMetadata")]
    pub usage_metadata: Option<GeminiUsageMetadata>,
}

/// Gemini SSE Parser for streaming responses
pub struct GeminiSSEParser {
    pub accumulated_content: String,
    pub accumulated_tool_calls: HashMap<i64, OpenAIToolCall>,
    pub events_str: String,
    pub stream_event_processor: StreamEventProcessor,
    tool_call_index: i64,
    /// Collected URL citation annotations from web search
    pub annotations: Vec<UrlCitation>,
    /// Whether web search was used in this response
    pub used_websearch: bool,
    /// Token usage from usageMetadata
    pub usage: Option<GeminiUsageMetadata>,
}

impl GeminiSSEParser {
    pub fn new(stream_event_processor: StreamEventProcessor) -> Self {
        Self {
            accumulated_content: String::new(),
            accumulated_tool_calls: HashMap::new(),
            events_str: String::new(),
            stream_event_processor,
            tool_call_index: 0,
            annotations: Vec::new(),
            used_websearch: false,
            usage: None,
        }
    }
}

impl SSEParser for GeminiSSEParser {
    async fn parse_event_data(&mut self, data: &str) -> Result<(), Error> {
        let event: Option<GeminiSSEEvent> = serde_json::from_str(data)
            .inspect_err(|e| {
                tracing::error!("Failed to parse SSE as a Gemini event {}: {}", data, e);
            })
            .ok();

        if let Some(event) = event {
            if let Some(candidates) = event.candidates {
                for candidate in candidates {
                    if let Some(content) = candidate.content {
                        if let Some(parts) = content.parts {
                            for part in parts {
                                // Handle text content
                                if let Some(text) = part.text {
                                    if !text.is_empty() {
                                        self.accumulated_content.push_str(&text);
                                        let event = StreamingEvent::TokenDelta { content: text };
                                        self.stream_event_processor
                                            .send(event, &mut self.events_str)
                                            .await?;
                                    }
                                }

                                // Handle function calls
                                if let Some(function_call) = part.function_call {
                                    let call_id = format!("call_{}", rd_string(24));
                                    let idx = self.tool_call_index;
                                    self.tool_call_index += 1;

                                    // Send tool call start event
                                    let event = StreamingEvent::ToolCall {
                                        call_id: call_id.clone(),
                                        function_name: function_call.name.clone(),
                                    };
                                    self.stream_event_processor
                                        .send(event, &mut self.events_str)
                                        .await?;

                                    // Build extra_content with thought_signature if present
                                    let extra_content =
                                        part.thought_signature.map(|sig| ExtraContent {
                                            google: Some(GoogleExtraContent {
                                                thought_signature: Some(sig),
                                            }),
                                        });

                                    // Store accumulated tool call
                                    self.accumulated_tool_calls.insert(
                                        idx,
                                        OpenAIToolCall {
                                            id: call_id,
                                            function: OpenAIFunction {
                                                name: function_call.name,
                                                arguments: serde_json::to_string(
                                                    &function_call.args,
                                                )
                                                .unwrap_or_else(|_| "{}".to_string()),
                                            },
                                            r#type: "function".to_string(),
                                            extra_content,
                                        },
                                    );
                                }
                            }
                        }
                    }

                    // Handle grounding metadata (web search results)
                    if let Some(ref grounding_metadata) = candidate.grounding_metadata {
                        // Set used_websearch if there are search queries or grounding chunks
                        if !grounding_metadata.web_search_queries.is_empty()
                            || !grounding_metadata.grounding_chunks.is_empty()
                        {
                            self.used_websearch = true;
                        }

                        // Extract citations from grounding chunks
                        for chunk in &grounding_metadata.grounding_chunks {
                            if let Some(ref web) = chunk.web {
                                self.annotations.push(UrlCitation {
                                    start_index: 0, // Gemini doesn't provide character indices
                                    end_index: 0,
                                    url: web.uri.clone(),
                                    title: web.title.clone(),
                                });
                            }
                        }
                    }
                }
            }

            // Extract usage metadata
            if let Some(usage_metadata) = event.usage_metadata {
                self.usage = Some(usage_metadata);
            }
        }

        Ok(())
    }
}

// ============================================================================
// OpenAI Responses API SSE Parser
// ============================================================================

/// Output item structure for OpenAI Responses API streaming
#[derive(Deserialize, Debug)]
pub struct ResponsesOutputItem {
    pub id: Option<String>,
    pub r#type: Option<String>,
    pub name: Option<String>,
    pub call_id: Option<String>,
}

/// URL citation annotation from OpenAI Responses API SSE events
#[derive(Deserialize, Debug)]
pub struct OpenAIUrlCitationEvent {
    #[allow(dead_code)]
    pub r#type: String,
    pub start_index: usize,
    pub end_index: usize,
    pub url: String,
    pub title: Option<String>,
}

/// OpenAI Responses API usage information
#[derive(Deserialize, Debug, Clone)]
pub struct OpenAIResponsesUsage {
    #[serde(default)]
    pub input_tokens: Option<i32>,
    #[serde(default)]
    pub output_tokens: Option<i32>,
    #[serde(default)]
    pub total_tokens: Option<i32>,
}

/// OpenAI Responses API response object (from response.completed event)
#[derive(Deserialize, Debug)]
pub struct OpenAIResponsesResponse {
    #[serde(default)]
    pub usage: Option<OpenAIResponsesUsage>,
}

/// SSE event types for OpenAI Responses API streaming
/// Based on frontend implementation: openai-responses.ts:220-302
#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum OpenAIResponsesSSEEvent {
    /// Text content delta
    #[serde(rename = "response.output_text.delta")]
    OutputTextDelta { delta: String },

    /// New output item added (e.g., function_call start)
    #[serde(rename = "response.output_item.added")]
    OutputItemAdded { item: ResponsesOutputItem },

    /// Function call arguments delta (streaming arguments)
    #[serde(rename = "response.function_call_arguments.delta")]
    FunctionCallArgumentsDelta { item_id: String, delta: String },

    /// Function call arguments complete
    #[serde(rename = "response.function_call_arguments.done")]
    FunctionCallArgumentsDone { item_id: String, arguments: String },

    /// Response complete
    #[serde(rename = "response.done")]
    Done {},

    /// Response completed with full response object (contains usage)
    #[serde(rename = "response.completed")]
    Completed { response: OpenAIResponsesResponse },

    /// Response created
    #[serde(rename = "response.created")]
    Created {},

    /// Response in progress
    #[serde(rename = "response.in_progress")]
    InProgress {},

    /// Output item done
    #[serde(rename = "response.output_item.done")]
    OutputItemDone {},

    /// Content part added
    #[serde(rename = "response.content_part.added")]
    ContentPartAdded {},

    /// Content part done
    #[serde(rename = "response.content_part.done")]
    ContentPartDone {},

    /// Annotation added (e.g., URL citation from web search)
    #[serde(rename = "response.output_text.annotation.added")]
    AnnotationAdded { annotation: OpenAIUrlCitationEvent },

    /// Catch-all for unknown event types
    #[serde(other)]
    Other,
}

/// OpenAI Responses API SSE Parser for streaming responses
pub struct OpenAIResponsesSSEParser {
    pub accumulated_content: String,
    pub accumulated_tool_calls: HashMap<String, OpenAIToolCall>,
    /// Maps item_id -> (name, call_id) for function calls
    tool_call_metadata: HashMap<String, (String, String)>,
    /// Maps item_id -> accumulated arguments
    tool_call_arguments: HashMap<String, String>,
    pub events_str: String,
    pub stream_event_processor: StreamEventProcessor,
    /// Collected URL citation annotations from web search
    pub annotations: Vec<UrlCitation>,
    /// Whether web search was used in this response
    pub used_websearch: bool,
    /// Token usage from response.completed event
    pub usage: Option<OpenAIResponsesUsage>,
}

impl OpenAIResponsesSSEParser {
    pub fn new(stream_event_processor: StreamEventProcessor) -> Self {
        Self {
            accumulated_content: String::new(),
            accumulated_tool_calls: HashMap::new(),
            tool_call_metadata: HashMap::new(),
            tool_call_arguments: HashMap::new(),
            events_str: String::new(),
            stream_event_processor,
            annotations: Vec::new(),
            used_websearch: false,
            usage: None,
        }
    }
}

impl SSEParser for OpenAIResponsesSSEParser {
    async fn parse_event_data(&mut self, data: &str) -> Result<(), Error> {
        let event: Option<OpenAIResponsesSSEEvent> = serde_json::from_str(data)
            .inspect_err(|e| {
                tracing::error!(
                    "Failed to parse SSE as an OpenAI Responses event {}: {}",
                    data,
                    e
                );
            })
            .ok();

        if let Some(event) = event {
            match event {
                OpenAIResponsesSSEEvent::OutputTextDelta { delta } => {
                    if !delta.is_empty() {
                        self.accumulated_content.push_str(&delta);
                        let event = StreamingEvent::TokenDelta { content: delta };
                        self.stream_event_processor
                            .send(event, &mut self.events_str)
                            .await?;
                    }
                }

                OpenAIResponsesSSEEvent::OutputItemAdded { item } => {
                    match item.r#type.as_deref() {
                        // Handle function_call type items
                        Some("function_call") => {
                            if let (Some(item_id), Some(name), Some(call_id)) =
                                (item.id, item.name, item.call_id)
                            {
                                // Store metadata for this function call
                                self.tool_call_metadata
                                    .insert(item_id.clone(), (name.clone(), call_id.clone()));
                                self.tool_call_arguments
                                    .insert(item_id.clone(), String::new());

                                // Send tool call start event
                                let event = StreamingEvent::ToolCall {
                                    call_id: call_id.clone(),
                                    function_name: name.clone(),
                                };
                                self.stream_event_processor
                                    .send(event, &mut self.events_str)
                                    .await?;
                            }
                        }
                        // Handle web_search_call type items
                        Some("web_search_call") => {
                            self.used_websearch = true;
                        }
                        _ => {}
                    }
                }

                OpenAIResponsesSSEEvent::FunctionCallArgumentsDelta { item_id, delta } => {
                    // Accumulate arguments for this function call
                    if let Some(args) = self.tool_call_arguments.get_mut(&item_id) {
                        args.push_str(&delta);
                    }
                }

                OpenAIResponsesSSEEvent::FunctionCallArgumentsDone { item_id, arguments } => {
                    // Function call is complete - create the tool call
                    if let Some((name, call_id)) = self.tool_call_metadata.get(&item_id) {
                        // Send tool call arguments event
                        let event = StreamingEvent::ToolCallArguments {
                            call_id: call_id.clone(),
                            function_name: name.clone(),
                            arguments: arguments.clone(),
                        };
                        self.stream_event_processor
                            .send(event, &mut self.events_str)
                            .await?;

                        // Store the complete tool call
                        self.accumulated_tool_calls.insert(
                            item_id.clone(),
                            OpenAIToolCall {
                                id: call_id.clone(),
                                function: OpenAIFunction { name: name.clone(), arguments },
                                r#type: "function".to_string(),
                                extra_content: None,
                            },
                        );
                    }
                }

                OpenAIResponsesSSEEvent::AnnotationAdded { annotation } => {
                    // Collect URL citation annotations from web search
                    self.annotations.push(UrlCitation {
                        start_index: annotation.start_index,
                        end_index: annotation.end_index,
                        url: annotation.url,
                        title: annotation.title,
                    });
                }

                OpenAIResponsesSSEEvent::Completed { response } => {
                    // Extract usage from response.completed event
                    if let Some(usage) = response.usage {
                        self.usage = Some(usage);
                    }
                }

                // Ignore other event types
                OpenAIResponsesSSEEvent::Done {}
                | OpenAIResponsesSSEEvent::Created {}
                | OpenAIResponsesSSEEvent::InProgress {}
                | OpenAIResponsesSSEEvent::OutputItemDone {}
                | OpenAIResponsesSSEEvent::ContentPartAdded {}
                | OpenAIResponsesSSEEvent::ContentPartDone {}
                | OpenAIResponsesSSEEvent::Other => {}
            }
        }

        Ok(())
    }
}
