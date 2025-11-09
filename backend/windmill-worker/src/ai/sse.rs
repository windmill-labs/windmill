use std::collections::HashMap;

use reqwest::Response;
use serde::Deserialize;
use serde_json;
use tokio_stream::StreamExt;
use windmill_common::{error::Error, utils::rd_string};

use crate::ai::{
    providers::openai::{OpenAIFunction, OpenAIToolCall},
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
        let mut stream = response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result
                .map_err(|e| Error::internal_err(format!("Failed to read chunk: {}", e)))?;

            // Convert chunk to string and add to buffer
            let chunk_str = String::from_utf8_lossy(&chunk);
            if *DEBUG_SSE_STREAM {
                tracing::info!("SSE chunk: {}", chunk_str);
            }
            buffer.push_str(&chunk_str);

            // Process complete lines from buffer
            while let Some(newline_pos) = buffer.find("\n\n") {
                let line = buffer.drain(..newline_pos + 2).collect::<String>();
                let line = line.trim_end_matches('\n');

                // Skip empty lines and comments
                if line.is_empty() || line.starts_with(':') {
                    continue;
                }

                // Parse SSE data field
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        // OpenAI sends [DONE] to indicate end of stream
                        return Ok(());
                    }

                    self.parse_event_data(data).await?;
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
        let event: OpenAISSEEvent = serde_json::from_str(data).map_err(|e| {
            Error::internal_err(format!("Failed to parse SSE chunk {}: {}", data, e))
        })?;

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
                            if let Some(tool_call) = self.accumulated_tool_calls.get_mut(&idx) {
                                if let Some(arguments) = function.arguments {
                                    tool_call.function.arguments += &arguments;
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
                                    },
                                );
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
