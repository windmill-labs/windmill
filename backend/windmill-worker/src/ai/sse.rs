use crate::ai::types::StreamingEvent;
use reqwest::Response;
use serde_json;
use std::collections::HashMap;
use windmill_common::error::Error;

/// Parse Server-Sent Events from a streaming HTTP response
pub struct SSEParser {
    response: Response,
}

impl SSEParser {
    pub fn new(response: Response) -> Self {
        Self { response }
    }

    /// Parse SSE stream and yield streaming events
    pub async fn parse_events<F>(self, mut event_handler: F) -> Result<(), Error>
    where
        F: FnMut(StreamingEvent) -> Result<(), Error>,
    {
        // Get the response body as bytes
        let body_bytes = self.response.bytes().await.map_err(|e| {
            Error::internal_err(format!("Failed to read response body: {}", e))
        })?;

        // Split by lines and process
        let body_str = String::from_utf8_lossy(&body_bytes);
        for line in body_str.lines() {
            // Skip empty lines and comments
            if line.is_empty() || line.starts_with(':') {
                continue;
            }

            // Parse SSE data field
            if let Some(data) = line.strip_prefix("data: ") {
                if data == "[DONE]" {
                    // OpenAI sends [DONE] to indicate end of stream
                    break;
                }

                // Try to parse as JSON
                if let Ok(event) = Self::parse_openai_chunk(data) {
                    if let Some(event) = event {
                        event_handler(event)?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Parse OpenAI streaming chunk into StreamingEvent
    fn parse_openai_chunk(data: &str) -> Result<Option<StreamingEvent>, Error> {
        let chunk: serde_json::Value = serde_json::from_str(data).map_err(|e| {
            Error::internal_err(format!("Failed to parse SSE chunk: {}", e))
        })?;

        let choices = chunk["choices"].as_array().ok_or_else(|| {
            Error::internal_err("SSE chunk missing choices array".to_string())
        })?;

        if choices.is_empty() {
            return Ok(None);
        }

        let choice = &choices[0];
        let delta = &choice["delta"];

        // Check for content delta
        if let Some(content) = delta["content"].as_str() {
            if !content.is_empty() {
                return Ok(Some(StreamingEvent::TokenDelta {
                    content: content.to_string(),
                }));
            }
        }

        // Check for tool calls
        if let Some(tool_calls) = delta["tool_calls"].as_array() {
            for tool_call in tool_calls {
                let _index = tool_call["index"].as_u64().unwrap_or(0);
                
                if let Some(id) = tool_call["id"].as_str() {
                    if let Some(function) = tool_call["function"].as_object() {
                        if let Some(name) = function["name"].as_str() {
                            return Ok(Some(StreamingEvent::ToolCallStart {
                                call_id: id.to_string(),
                                function_name: name.to_string(),
                            }));
                        }
                    }
                }

                // Check for function arguments delta
                if let Some(function) = tool_call["function"].as_object() {
                    if let Some(arguments) = function["arguments"].as_str() {
                        if !arguments.is_empty() {
                            // For now, we'll accumulate arguments and send complete when done
                            // This is a simplified implementation - a full implementation would
                            // track partial arguments and send ToolCallComplete when done
                        }
                    }
                }
            }
        }

        // Check for finish reason
        if let Some(finish_reason) = choice["finish_reason"].as_str() {
            if finish_reason == "stop" || finish_reason == "tool_calls" {
                return Ok(Some(StreamingEvent::MessageComplete));
            }
        }

        Ok(None)
    }
}

/// Helper to accumulate tool call arguments during streaming
pub struct ToolCallAccumulator {
    calls: HashMap<String, ToolCallState>,
}

#[derive(Debug)]
struct ToolCallState {
    function_name: String,
    arguments: String,
    complete: bool,
}

impl ToolCallAccumulator {
    pub fn new() -> Self {
        Self {
            calls: HashMap::new(),
        }
    }

    /// Add a tool call start event
    pub fn start_call(&mut self, call_id: String, function_name: String) {
        self.calls.insert(
            call_id,
            ToolCallState {
                function_name,
                arguments: String::new(),
                complete: false,
            },
        );
    }

    /// Add arguments to a tool call
    pub fn add_arguments(&mut self, call_id: &str, arguments: &str) {
        if let Some(state) = self.calls.get_mut(call_id) {
            state.arguments.push_str(arguments);
        }
    }

    /// Mark a tool call as complete and return the completed call
    pub fn complete_call(&mut self, call_id: &str) -> Option<StreamingEvent> {
        if let Some(mut state) = self.calls.remove(call_id) {
            state.complete = true;
            Some(StreamingEvent::ToolCallComplete {
                call_id: call_id.to_string(),
                function_name: state.function_name,
                arguments: state.arguments,
            })
        } else {
            None
        }
    }

    /// Get all completed tool calls and clear the accumulator
    pub fn get_completed_calls(&mut self) -> Vec<StreamingEvent> {
        let completed: Vec<_> = self
            .calls
            .drain()
            .filter(|(_, state)| state.complete)
            .map(|(call_id, state)| StreamingEvent::ToolCallComplete {
                call_id,
                function_name: state.function_name,
                arguments: state.arguments,
            })
            .collect();
        completed
    }
}