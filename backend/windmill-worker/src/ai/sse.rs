use reqwest::Response;
use serde::Deserialize;
use serde_json;
use windmill_common::error::Error;

/// Parse Server-Sent Events from a streaming HTTP response
pub struct SSEParser {
    response: Response,
}

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

impl SSEParser {
    pub fn new(response: Response) -> Self {
        Self { response }
    }

    /// Parse SSE stream and yield streaming events
    pub async fn parse_events<F>(self, mut event_handler: F) -> Result<(), Error>
    where
        F: FnMut(OpenAISSEEvent) -> Result<(), Error>,
    {
        // Get the response body as bytes
        let body_bytes = self
            .response
            .bytes()
            .await
            .map_err(|e| Error::internal_err(format!("Failed to read response body: {}", e)))?;

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

                let event: OpenAISSEEvent = serde_json::from_str(data).map_err(|e| {
                    Error::internal_err(format!("Failed to parse SSE chunk: {}", e))
                })?;

                event_handler(event)?;
            }
        }

        Ok(())
    }
}
