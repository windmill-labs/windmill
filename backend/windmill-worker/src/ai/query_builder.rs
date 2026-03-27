use async_trait::async_trait;
use windmill_common::{error::Error, worker::Connection};
use windmill_queue::MiniPulledJob;

use crate::{
    ai::{
        providers::{
            anthropic::AnthropicQueryBuilder, google_ai::GoogleAIQueryBuilder,
            openai::OpenAIQueryBuilder, openrouter::OpenRouterQueryBuilder,
            other::OtherQueryBuilder,
        },
        types::*,
    },
    job_logger::append_result_stream,
};

// Re-export from windmill_ai
pub use windmill_ai::query_builder::{
    BuildRequestArgs, ParsedResponse, QueryBuilder, StreamEventSink,
};

/// Factory function to create the appropriate query builder for a provider
pub fn create_query_builder(provider: &ProviderWithResource) -> Box<dyn QueryBuilder> {
    use windmill_ai::ai_providers::AIProvider;

    match provider.kind {
        AIProvider::GoogleAI => Box::new(GoogleAIQueryBuilder::new(
            provider.get_platform().clone(),
        )),
        AIProvider::OpenAI => Box::new(OpenAIQueryBuilder::new(provider.kind.clone())),
        AIProvider::Anthropic => Box::new(AnthropicQueryBuilder::new(
            provider.kind.clone(),
            provider.get_platform().clone(),
            provider.get_enable_1m_context(),
        )),
        AIProvider::OpenRouter => Box::new(OpenRouterQueryBuilder::new()),
        _ => Box::new(OtherQueryBuilder::new(provider.kind.clone())),
    }
}

/// Processes streaming events by persisting them to the database.
/// Implements StreamEventSink so it can be passed to QueryBuilder methods.
pub struct StreamEventProcessor {
    tx: Option<tokio::sync::mpsc::Sender<String>>,
    pub handle: Option<tokio::task::JoinHandle<()>>,
}

impl Clone for StreamEventProcessor {
    fn clone(&self) -> Self {
        Self { tx: self.tx.clone(), handle: None }
    }
}

impl StreamEventProcessor {
    /// Create a new StreamEventProcessor that persists events to the database.
    pub fn new(conn: &Connection, job: &MiniPulledJob) -> Self {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(100);
        let conn = conn.clone();
        let job_id = job.id.clone();
        let workspace_id = job.workspace_id.clone();
        let handle = tokio::spawn(async move {
            let mut offset = -1;
            while let Some(event) = rx.recv().await {
                offset += 1;
                match tokio::time::timeout(
                    std::time::Duration::from_secs(20),
                    append_result_stream(&conn, &workspace_id, &job_id, &event, offset),
                )
                .await
                {
                    Ok(res) => {
                        if let Err(err) = res {
                            tracing::error!("Failed to save stream event: {}", err);
                        }
                    }
                    Err(err) => {
                        tracing::error!("Did not manage to save stream event after 20 seconds, stopping stream event processor: {}", err);
                        break;
                    }
                }
            }
        });

        Self { tx: Some(tx), handle: Some(handle) }
    }

    /// Create a silent StreamEventProcessor that only accumulates events locally
    /// without persisting them to the database. Used when streaming is disabled.
    pub fn new_silent() -> Self {
        Self { tx: None, handle: None }
    }

    pub fn to_handle(self) -> Option<tokio::task::JoinHandle<()>> {
        self.handle
    }
}

#[async_trait]
impl StreamEventSink for StreamEventProcessor {
    async fn send(&self, event: StreamingEvent, events_str: &mut String) -> Result<(), Error> {
        // In silent mode, skip both persistence AND accumulation (no overhead)
        let Some(ref tx) = self.tx else {
            return Ok(());
        };

        match serde_json::to_string(&event) {
            Ok(event_json) => {
                let event_json = format!("{}\n", event_json);
                events_str.push_str(&event_json);

                if let Err(err) = tx
                    .send(event_json.clone())
                    .await
                    .map_err(|e| Error::internal_err(format!("Failed to send event: {}", e)))
                {
                    tracing::error!(
                        "Failed to send event to stream event processor, skiping event: {}",
                        err
                    );
                }

                Ok(())
            }
            Err(e) => Err(Error::internal_err(format!(
                "Failed to serialize streaming event {:#?}, error is: {}",
                event, e
            ))),
        }
    }
}
