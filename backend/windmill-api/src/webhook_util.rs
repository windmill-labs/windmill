use std::time::Duration;

use axum::{
    async_trait,
    extract::{FromRequestParts, OriginalUri},
    http::request::Parts,
    Extension,
};
use hyper::StatusCode;
use serde::Serialize;
use tokio::{select, sync::mpsc};

use crate::db::DB;

lazy_static::lazy_static! {
    // TODO: these aren't synced, they should be moved into the queue abstraction once/if that happens.
    static ref WEBHOOK_REQUEST_COUNT: prometheus::Histogram = prometheus::register_histogram!(
        "webhook_request",
        "Histogram of webhook requests made"
    )
    .unwrap();
}

#[derive(Serialize)]
#[serde(tag = "type")]
pub enum WebhookMessage {
    // See https://serde.rs/enum-representations.html#internally-tagged for how this looks in JSON
    CreateApp { workspace: String, path: String },
    DeleteApp { workspace: String, path: String },
    UpdateApp { workspace: String, old_path: String, new_path: String },
    CreateFlow { workspace: String, path: String },
    UpdateFlow { workspace: String, old_path: String, new_path: String },
    ArchiveFlow { workspace: String, path: String },
    CreateFolder { workspace: String, name: String },
    UpdateFolder { workspace: String, name: String },
    DeleteFolder { workspace: String, name: String },
    DeleteResource { workspace: String, path: String },
    CreateResource { workspace: String, path: String },
    UpdateResource { workspace: String, old_path: String, new_path: String },
    CreateResourceType { name: String },
    DeleteResourceType { name: String },
    UpdateResourceType { name: String },
    CreateScript { workspace: String, path: String, hash: String },
    UpdateScript { workspace: String, path: String, hash: String },
    DeleteScript { workspace: String, hash: String },
    CreateVariable { workspace: String, path: String },
    UpdateVariable { workspace: String, old_path: String, new_path: String },
    DeleteVariable { workspace: String, path: String },
}

#[derive(Clone)]
pub struct WebhookShared {
    pub channel: mpsc::UnboundedSender<(String, WebhookMessage)>,
}

impl WebhookShared {
    pub fn new(mut shutdown_rx: tokio::sync::broadcast::Receiver<()>) -> Self {
        let (tx, mut rx) = mpsc::unbounded_channel::<(String, WebhookMessage)>();
        let _process = tokio::spawn(async move {
            let client = reqwest::Client::builder()
                // TODO: investigate pool timeouts and such if TCP load is high
                .timeout(Duration::from_secs(5))
                .build()
                .unwrap();
            loop {
                select! {
                    biased;
                    _ = shutdown_rx.recv() => break,
                    r = rx.recv() => match r {
                        Some((url, message)) => {
                            let timer = WEBHOOK_REQUEST_COUNT.start_timer();
                            let _ = client.post(url).json(&message).send().await;
                            timer.stop_and_record();
                        },
                        None => break,
                    }
                }
            }
        });

        Self { channel: tx }
    }
}

#[derive(Clone)]
pub struct WebhookUtil {
    webhook: Option<String>,
    shared: Extension<WebhookShared>,
}

impl WebhookUtil {
    pub fn send_message(&self, message: WebhookMessage) {
        let Some(webhook) = &self.webhook else {
            return;
        };
        let _ = self.shared.channel.send((webhook.clone(), message));
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for WebhookUtil
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> std::result::Result<Self, Self::Rejection> {
        let original_uri = OriginalUri::from_request_parts(parts, state)
            .await
            .ok()
            .map(|x| x.0)
            .unwrap_or_default();
        let path_vec: Vec<&str> = original_uri.path().split("/").collect();
        let workspace_id = if path_vec.len() >= 4 && path_vec[0] == "" && path_vec[2] == "w" {
            Some(path_vec[3].to_owned())
        } else {
            None
        };

        let webhook = sqlx::query_scalar!(
            "SELECT webhook FROM workspace_settings WHERE workspace_id = $1",
            workspace_id
        )
        .fetch_one(
            &Extension::<DB>::from_request_parts(parts, state)
                .await
                .map_err(|_| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Could not aquire DB while retrieving webhook".to_owned(),
                    )
                })?
                .0,
        )
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Could not execute DB query {:?}", e),
            )
        })?;
        let shared = Extension::<WebhookShared>::from_request_parts(parts, state)
            .await
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Could not aquire shared process while retrieving webhook".to_owned(),
                )
            })?;

        Ok(Self { webhook, shared })
    }
}
