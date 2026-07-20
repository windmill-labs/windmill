use std::collections::HashMap;

use eventsource_stream::Eventsource;
use reqwest::Response;
use serde::Deserialize;
use tokio_stream::StreamExt;
use windmill_common::{error::Error, utils::rd_string};

use crate::{
    ai_google::{parse_gemini_sse_event, GeminiUsageMetadata},
    ai_types::UrlCitation,
    ai_types::{
        AnthropicExtraContent, ExtraContent, GoogleExtraContent, OpenAIFunction, OpenAIToolCall,
    },
    query_builder::StreamEventSink,
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
}

#[derive(Deserialize)]
pub struct OpenAIChoiceDelta {
    pub content: Option<String>,
    /// Reasoning summary streamed by providers that expose it (e.g. DeepSeek's
    /// `reasoning_content`). Rendered as a "thinking" affordance.
    #[serde(default)]
    pub reasoning_content: Option<String>,
    pub tool_calls: Option<Vec<OpenAIChoiceDeltaToolCall>>,
}

#[derive(Deserialize)]
pub struct OpenAIChoice {
    pub delta: Option<OpenAIChoiceDelta>,
}

/// Nested prompt token details returned by the Chat Completions API.
/// `cached_tokens` is the portion of `prompt_tokens` served from cache (a subset, not additive).
#[derive(Deserialize, Debug, Clone, Default)]
pub struct OpenAIPromptTokensDetails {
    #[serde(default)]
    pub cached_tokens: Option<i32>,
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
    #[serde(default)]
    pub prompt_tokens_details: Option<OpenAIPromptTokensDetails>,
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
#[allow(async_fn_in_trait)]
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
    pub stream_event_processor: Box<dyn StreamEventSink>,
    /// Token usage from final chunk (when stream_options.include_usage is true)
    pub usage: Option<OpenAIChatUsage>,
}

impl OpenAISSEParser {
    pub fn new(stream_event_processor: Box<dyn StreamEventSink>) -> Self {
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
                    if let Some(reasoning) = delta.reasoning_content.filter(|s| !s.is_empty()) {
                        let event = StreamingEvent::ReasoningTokenDelta { content: reasoning };
                        self.stream_event_processor
                            .send(event, &mut self.events_str)
                            .await?;
                    }

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
    #[serde(rename = "thinking")]
    Thinking {
        #[serde(default)]
        thinking: String,
    },
    #[serde(rename = "redacted_thinking")]
    RedactedThinking {
        #[serde(default)]
        data: String,
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
    #[serde(rename = "thinking_delta")]
    ThinkingDelta { thinking: String },
    #[serde(rename = "signature_delta")]
    SignatureDelta { signature: String },
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
    Thinking,
    ToolUse { id: String, name: String },
    Unknown,
}

/// Anthropic SSE Parser for streaming responses
pub struct AnthropicSSEParser {
    pub accumulated_content: String,
    pub accumulated_tool_calls: HashMap<i64, OpenAIToolCall>,
    pub events_str: String,
    pub stream_event_processor: Box<dyn StreamEventSink>,
    /// Track content block types by index
    content_blocks: HashMap<usize, ContentBlockState>,
    /// Collected URL citation annotations from web search
    pub annotations: Vec<UrlCitation>,
    /// Whether web search was used in this response
    pub used_websearch: bool,
    /// Token usage from message_delta event
    pub usage: Option<AnthropicUsage>,
    /// Claude thinking block accumulated from `thinking`/`signature` deltas
    /// (or a redacted block). Attached to the first tool call of the turn so it
    /// can be replayed before `tool_use` (required by Claude when thinking is on).
    pending_reasoning: Option<AnthropicExtraContent>,
    reasoning_attached: bool,
}

impl AnthropicSSEParser {
    pub fn new(stream_event_processor: Box<dyn StreamEventSink>) -> Self {
        Self {
            accumulated_content: String::new(),
            accumulated_tool_calls: HashMap::new(),
            events_str: String::new(),
            stream_event_processor,
            content_blocks: HashMap::new(),
            annotations: Vec::new(),
            used_websearch: false,
            usage: None,
            pending_reasoning: None,
            reasoning_attached: false,
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
                            // Attach the turn's thinking block to the first tool call so it
                            // can be replayed before tool_use on the next request.
                            let extra_content = if self.reasoning_attached {
                                None
                            } else {
                                self.reasoning_attached = true;
                                self.pending_reasoning.take().map(|anthropic| ExtraContent {
                                    anthropic: Some(anthropic),
                                    ..Default::default()
                                })
                            };
                            // Initialize tool call accumulator
                            self.accumulated_tool_calls.insert(
                                index as i64,
                                OpenAIToolCall {
                                    id,
                                    function: OpenAIFunction { name, arguments: String::new() },
                                    r#type: "function".to_string(),
                                    extra_content,
                                },
                            );
                        }
                        AnthropicContentBlockStart::Thinking { thinking } => {
                            self.content_blocks
                                .insert(index, ContentBlockState::Thinking);
                            let entry = self.pending_reasoning.get_or_insert_with(Default::default);
                            if !thinking.is_empty() {
                                entry
                                    .thinking
                                    .get_or_insert_with(String::new)
                                    .push_str(&thinking);
                                self.stream_event_processor
                                    .send(
                                        StreamingEvent::ReasoningTokenDelta { content: thinking },
                                        &mut self.events_str,
                                    )
                                    .await?;
                            }
                        }
                        AnthropicContentBlockStart::RedactedThinking { data } => {
                            // Redacted blocks arrive whole (no deltas).
                            self.content_blocks
                                .insert(index, ContentBlockState::Unknown);
                            self.pending_reasoning
                                .get_or_insert_with(Default::default)
                                .redacted_thinking = Some(data);
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
                        AnthropicDelta::ThinkingDelta { thinking } => {
                            if let Some(ContentBlockState::Thinking) =
                                self.content_blocks.get(&index)
                            {
                                self.pending_reasoning
                                    .get_or_insert_with(Default::default)
                                    .thinking
                                    .get_or_insert_with(String::new)
                                    .push_str(&thinking);
                                self.stream_event_processor
                                    .send(
                                        StreamingEvent::ReasoningTokenDelta { content: thinking },
                                        &mut self.events_str,
                                    )
                                    .await?;
                            }
                        }
                        AnthropicDelta::SignatureDelta { signature } => {
                            if let Some(ContentBlockState::Thinking) =
                                self.content_blocks.get(&index)
                            {
                                self.pending_reasoning
                                    .get_or_insert_with(Default::default)
                                    .signature
                                    .get_or_insert_with(String::new)
                                    .push_str(&signature);
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

/// Accumulates Gemini streaming events and converts them into the shared
/// [`OpenAIToolCall`] / [`StreamingEvent`] representation.
///
/// The actual SSE parsing is delegated to [`parse_gemini_sse_event`] from
/// `windmill_ai::ai_google` so the logic can be shared with the API proxy.
pub struct GeminiSSEParser {
    pub accumulated_content: String,
    pub accumulated_tool_calls: HashMap<i64, OpenAIToolCall>,
    pub events_str: String,
    pub stream_event_processor: Box<dyn StreamEventSink>,
    tool_call_index: i64,
    pub annotations: Vec<UrlCitation>,
    pub used_websearch: bool,
    pub usage: Option<GeminiUsageMetadata>,
}

impl GeminiSSEParser {
    pub fn new(stream_event_processor: Box<dyn StreamEventSink>) -> Self {
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
        let Some(parsed) = parse_gemini_sse_event(data)? else {
            return Ok(());
        };

        if let Some(reasoning) = parsed.reasoning.filter(|s| !s.is_empty()) {
            self.stream_event_processor
                .send(
                    StreamingEvent::ReasoningTokenDelta { content: reasoning },
                    &mut self.events_str,
                )
                .await?;
        }

        if let Some(text) = parsed.text {
            self.accumulated_content.push_str(&text);
            self.stream_event_processor
                .send(
                    StreamingEvent::TokenDelta { content: text },
                    &mut self.events_str,
                )
                .await?;
        }

        for tool_call in parsed.tool_calls {
            let call_id = format!("call_{}", rd_string(24));
            let idx = self.tool_call_index;
            self.tool_call_index += 1;

            self.stream_event_processor
                .send(
                    StreamingEvent::ToolCall {
                        call_id: call_id.clone(),
                        function_name: tool_call.name.clone(),
                    },
                    &mut self.events_str,
                )
                .await?;

            let extra_content = tool_call.thought_signature.map(|sig| ExtraContent {
                google: Some(GoogleExtraContent { thought_signature: Some(sig) }),
                ..Default::default()
            });

            self.accumulated_tool_calls.insert(
                idx,
                OpenAIToolCall {
                    id: call_id,
                    function: OpenAIFunction {
                        name: tool_call.name,
                        arguments: serde_json::to_string(&tool_call.args)
                            .unwrap_or_else(|_| "{}".to_string()),
                    },
                    r#type: "function".to_string(),
                    extra_content,
                },
            );
        }

        self.annotations.extend(parsed.annotations);
        if parsed.used_websearch {
            self.used_websearch = true;
        }
        if let Some(usage) = parsed.usage {
            self.usage = Some(usage);
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

/// Nested input token details returned by the Responses API.
/// `cached_tokens` is the portion of `input_tokens` served from cache (a subset, not additive).
#[derive(Deserialize, Debug, Clone, Default)]
pub struct OpenAIInputTokensDetails {
    #[serde(default)]
    pub cached_tokens: Option<i32>,
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
    #[serde(default)]
    pub input_tokens_details: Option<OpenAIInputTokensDetails>,
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
    pub stream_event_processor: Box<dyn StreamEventSink>,
    /// Collected URL citation annotations from web search
    pub annotations: Vec<UrlCitation>,
    /// Whether web search was used in this response
    pub used_websearch: bool,
    /// Token usage from response.completed event
    pub usage: Option<OpenAIResponsesUsage>,
}

impl OpenAIResponsesSSEParser {
    pub fn new(stream_event_processor: Box<dyn StreamEventSink>) -> Self {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reasoning_token_delta_serializes_with_snake_case_tag() {
        let event = StreamingEvent::ReasoningTokenDelta { content: "hmm".to_string() };
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&event).unwrap()).unwrap();
        assert_eq!(json["type"], "reasoning_token_delta");
        assert_eq!(json["content"], "hmm");
    }

    #[test]
    fn openai_chat_usage_parses_cached_prompt_tokens() {
        // Payload shape returned by OpenAI and Azure OpenAI Chat Completions.
        // cached_tokens lives under prompt_tokens_details and is a subset of prompt_tokens.
        let usage: OpenAIChatUsage = serde_json::from_str(
            r#"{"prompt_tokens":4819,"completion_tokens":1,"total_tokens":4820,"prompt_tokens_details":{"cached_tokens":4736,"audio_tokens":0}}"#,
        )
        .unwrap();
        assert_eq!(
            usage.prompt_tokens_details.and_then(|d| d.cached_tokens),
            Some(4736)
        );
    }

    #[test]
    fn openai_responses_usage_parses_cached_input_tokens() {
        // Payload shape returned by the OpenAI Responses API.
        let usage: OpenAIResponsesUsage = serde_json::from_str(
            r#"{"input_tokens":4819,"input_tokens_details":{"cache_write_tokens":0,"cached_tokens":4736},"output_tokens":2,"total_tokens":4821}"#,
        )
        .unwrap();
        assert_eq!(
            usage.input_tokens_details.and_then(|d| d.cached_tokens),
            Some(4736)
        );
    }

    #[test]
    fn openai_delta_parses_reasoning_content() {
        // DeepSeek and similar stream reasoning under `reasoning_content`.
        let delta: OpenAIChoiceDelta =
            serde_json::from_str(r#"{"reasoning_content":"let me think"}"#).unwrap();
        assert_eq!(delta.reasoning_content.as_deref(), Some("let me think"));
    }

    #[test]
    fn parses_anthropic_thinking_events() {
        let start: AnthropicSSEEvent = serde_json::from_str(
            r#"{"type":"content_block_start","index":0,"content_block":{"type":"thinking","thinking":"x"}}"#,
        )
        .unwrap();
        assert!(matches!(
            start,
            AnthropicSSEEvent::ContentBlockStart {
                content_block: AnthropicContentBlockStart::Thinking { .. },
                ..
            }
        ));

        let thinking_delta: AnthropicSSEEvent = serde_json::from_str(
            r#"{"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"more"}}"#,
        )
        .unwrap();
        assert!(matches!(
            thinking_delta,
            AnthropicSSEEvent::ContentBlockDelta {
                delta: AnthropicDelta::ThinkingDelta { .. },
                ..
            }
        ));

        let signature_delta: AnthropicSSEEvent = serde_json::from_str(
            r#"{"type":"content_block_delta","index":0,"delta":{"type":"signature_delta","signature":"sig"}}"#,
        )
        .unwrap();
        assert!(matches!(
            signature_delta,
            AnthropicSSEEvent::ContentBlockDelta {
                delta: AnthropicDelta::SignatureDelta { .. },
                ..
            }
        ));

        let redacted: AnthropicSSEEvent = serde_json::from_str(
            r#"{"type":"content_block_start","index":0,"content_block":{"type":"redacted_thinking","data":"abc"}}"#,
        )
        .unwrap();
        assert!(matches!(
            redacted,
            AnthropicSSEEvent::ContentBlockStart {
                content_block: AnthropicContentBlockStart::RedactedThinking { .. },
                ..
            }
        ));
    }
}
