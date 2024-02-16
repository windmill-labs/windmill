use std::time::Duration;

use quick_cache::sync::Cache;
use serde::Serialize;
use tokio::{select, sync::mpsc};
use windmill_common::METRICS_ENABLED;

use crate::db::DB;
use crate::oauth2_ee::InstanceEvent;

lazy_static::lazy_static! {
    // TODO: these aren't synced, they should be moved into the queue abstraction once/if that happens.
    static ref WEBHOOK_REQUEST_COUNT: prometheus::Histogram = prometheus::register_histogram!(
        "webhook_request",
        "Histogram of webhook requests made"
    )
    .unwrap();

    pub static ref INSTANCE_EVENTS_WEBHOOK: Option<String> = std::env::var("INSTANCE_EVENTS_WEBHOOK").ok();

}

pub enum WebhookPayload {
    WorkspaceEvent(String, WebhookMessage),
    InstanceEvent(InstanceEvent),
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
    DeleteFlow { workspace: String, path: String },
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
    DeleteScriptPath { workspace: String, path: String },
    CreateVariable { workspace: String, path: String },
    UpdateVariable { workspace: String, old_path: String, new_path: String },
    DeleteVariable { workspace: String, path: String },
}

#[derive(Clone)]
pub struct WebhookShared {
    pub channel: mpsc::UnboundedSender<WebhookPayload>,
}

impl WebhookShared {
    pub fn new(mut shutdown_rx: tokio::sync::broadcast::Receiver<()>, db: DB) -> Self {
        let (tx, mut rx) = mpsc::unbounded_channel::<WebhookPayload>();
        let _process = tokio::spawn(async move {
            let client = reqwest::Client::builder()
                // TODO: investigate pool timeouts and such if TCP load is high
                .timeout(Duration::from_secs(5))
                .build()
                .unwrap();
            let cache = Cache::new(100);

            loop {
                select! {
                    biased;
                    _ = shutdown_rx.recv() => break,
                    r = rx.recv() => match r {
                        Some(WebhookPayload::WorkspaceEvent(workspace_id, message)) => {
                            let webhook_opt = match cache.get(&workspace_id) {
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
                                    cache.insert(workspace_id, webook_opt.clone());
                                    webook_opt
                                }
                            };
                            if let Some(url) = webhook_opt {
                                let timer = if METRICS_ENABLED.load(std::sync::atomic::Ordering::Relaxed) { Some(WEBHOOK_REQUEST_COUNT.start_timer()) } else { None };
                                let _ = client.post(url).json(&message).send().await;
                                timer.map(|x| x.stop_and_record());
                            }
                        },
                        Some(WebhookPayload::InstanceEvent(event)) => {
                            if METRICS_ENABLED.load(std::sync::atomic::Ordering::Relaxed) { Some(WEBHOOK_REQUEST_COUNT.start_timer()) } else { None };
                            let r = client.post(INSTANCE_EVENTS_WEBHOOK.as_ref().unwrap()).json(&event).send().await;
                            if let Err(e) = r {
                                tracing::error!("Error sending instance event: {}", e);
                            }
                        },
                        None => break,
                    },
                }
            }
        });

        Self { channel: tx }
    }

    pub fn send_message(&self, workspace_id: String, message: WebhookMessage) {
        let _ = self.channel.send(WebhookPayload::WorkspaceEvent(
            workspace_id.clone(),
            message,
        ));
    }

    pub fn send_instance_event(&self, event: InstanceEvent) {
        if INSTANCE_EVENTS_WEBHOOK.is_none() {
            return;
        }
        let _ = self.channel.send(WebhookPayload::InstanceEvent(event));
    }
}
