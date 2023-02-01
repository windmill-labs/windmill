use std::time::Duration;

use axum::{
    async_trait,
    extract::{FromRequestParts, OriginalUri},
    http::request::Parts,
    Extension,
};
use hyper::StatusCode;
use serde::Serialize;
use tokio::{select, sync::mpsc, time::interval};

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
    pub fn new(mut shutdown_rx: tokio::sync::broadcast::Receiver<()>, db: DB) -> Self {
        let (tx, mut rx) = mpsc::unbounded_channel::<(String, WebhookMessage)>();
        let _process = tokio::spawn(async move {
            let client = reqwest::Client::builder()
                // TODO: investigate pool timeouts and such if TCP load is high
                .timeout(Duration::from_secs(5))
                .build()
                .unwrap();
            let cache = retainer::Cache::new();
            let mut cache_purge_interval = interval(Duration::from_secs(30));

            loop {
                select! {
                    biased;
                    _ = shutdown_rx.recv() => break,
                    r = rx.recv() => match r {
                        Some((workspace_id, message)) => {
                            let url_guard = match cache.get(&workspace_id).await {
                                Some(guard) => {
                                    guard
                                },
                                None => {
                                    let Ok(webook_opt) =
                                        sqlx::query_scalar!(
                                            "SELECT webhook FROM workspace_settings WHERE workspace_id = $1",
                                            workspace_id
                                        )
                                        .fetch_one(
                                            &db,
                                        )
                                        .await else {
                                            tracing::error!("Webhook Message to send - but cannot get workspace settings! Workspace: {workspace_id}");
                                            continue;
                                        };
                                    cache.insert(workspace_id.clone(), webook_opt, Duration::from_secs(30)).await;
                                    cache.get(&workspace_id).await.unwrap()
                                }
                            };
                            let webook_opt = url_guard.value();
                            if let Some(url) = webook_opt {
                                let timer = WEBHOOK_REQUEST_COUNT.start_timer();
                                let _ = client.post(url).json(&message).send().await;
                                timer.stop_and_record();
                                drop(url_guard);
                            }
                        },
                        None => break,
                    },
                    _ = futures::future::poll_fn(|cx| cache_purge_interval.poll_tick(cx)) => {
                        tracing::trace!("Puring Webhook Cache");
                        cache.purge(10, 0.50).await;
                    },
                }
            }
        });

        Self { channel: tx }
    }
}

#[derive(Clone)]
pub struct WebhookUtil {
    workspace_id: Option<String>,
    shared: Extension<WebhookShared>,
}

impl WebhookUtil {
    pub fn send_message(&self, message: WebhookMessage) {
        let Some(workspace_id) = &self.workspace_id else {
            return;
        };
        let _ = self.shared.channel.send((workspace_id.clone(), message));
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

        let shared = Extension::<WebhookShared>::from_request_parts(parts, state)
            .await
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Could not aquire shared process while retrieving webhook".to_owned(),
                )
            })?;

        Ok(Self { workspace_id, shared })
    }
}
