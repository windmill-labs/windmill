use std::time::Duration;

use quick_cache::sync::Cache;
use serde::Serialize;
use tokio::{select, sync::mpsc};

#[cfg(feature = "prometheus")]
use crate::METRICS_ENABLED;

use crate::db::DB;
use crate::oauth2::InstanceEvent;
use crate::utils::configure_client;

#[cfg(feature = "prometheus")]
lazy_static::lazy_static! {
    // TODO: these aren't synced, they should be moved into the queue abstraction once/if that happens.
    static ref WEBHOOK_REQUEST_COUNT: prometheus::Histogram = prometheus::register_histogram!(
        "webhook_request",
        "Histogram of webhook requests made"
    )
    .unwrap();

}

lazy_static::lazy_static! {

    pub static ref INSTANCE_EVENTS_WEBHOOK: Option<String> = std::env::var("INSTANCE_EVENTS_WEBHOOK").ok();

    pub static ref WEBHOOK_CACHE: Cache<String, Option<String>> = Cache::new(100);

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
            let client = configure_client(
                reqwest::Client::builder()
                    .connect_timeout(Duration::from_secs(5))
                    // TODO: investigate pool timeouts and such if TCP load is high
                    .timeout(Duration::from_secs(5)),
            )
            .build()
            .unwrap();

            loop {
                select! {
                    biased;
                    _ = shutdown_rx.recv() => break,
                    r = rx.recv() => match r {
                        Some(WebhookPayload::WorkspaceEvent(workspace_id, message)) => {
                            let webhook_opt = match WEBHOOK_CACHE.get(&workspace_id) {
                                Some(guard) => {
                                    guard
                                },
                                None => {
                                    let Ok(mut webhook_opt) =
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
                                    if webhook_opt.as_ref().is_some_and(|x| x.is_empty()) {
                                        webhook_opt = None;
                                    }
                                    WEBHOOK_CACHE.insert(workspace_id, webhook_opt.clone());
                                    webhook_opt
                                }
                            };
                            if let Some(url) = webhook_opt {
                                #[cfg(feature = "prometheus")]
                                let timer = if METRICS_ENABLED.load(std::sync::atomic::Ordering::Relaxed) { Some(WEBHOOK_REQUEST_COUNT.start_timer()) } else { None };
                                tracing::info!("Sending webhook message to {}", url);
                                let _ = client.post(url).json(&message).send().await;
                                #[cfg(feature = "prometheus")]
                                timer.map(|x| x.stop_and_record());
                            }
                        },
                        Some(WebhookPayload::InstanceEvent(event)) => {
                            #[cfg(feature = "prometheus")]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webhook_message_create_script() {
        let msg = WebhookMessage::CreateScript {
            workspace: "demo".to_string(),
            path: "f/test/script".to_string(),
            hash: "abc123".to_string(),
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "CreateScript");
        assert_eq!(json["workspace"], "demo");
        assert_eq!(json["path"], "f/test/script");
        assert_eq!(json["hash"], "abc123");
    }

    #[test]
    fn test_webhook_message_update_flow() {
        let msg = WebhookMessage::UpdateFlow {
            workspace: "staging".to_string(),
            old_path: "f/old/flow".to_string(),
            new_path: "f/new/flow".to_string(),
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "UpdateFlow");
        assert_eq!(json["old_path"], "f/old/flow");
        assert_eq!(json["new_path"], "f/new/flow");
    }

    #[test]
    fn test_webhook_message_delete_resource() {
        let msg = WebhookMessage::DeleteResource {
            workspace: "prod".to_string(),
            path: "u/admin/db".to_string(),
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "DeleteResource");
        assert_eq!(json["workspace"], "prod");
        assert_eq!(json["path"], "u/admin/db");
    }

    #[test]
    fn test_webhook_message_create_folder() {
        let msg = WebhookMessage::CreateFolder {
            workspace: "demo".to_string(),
            name: "shared".to_string(),
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "CreateFolder");
        assert_eq!(json["name"], "shared");
    }

    #[test]
    fn test_webhook_message_resource_type() {
        let msg = WebhookMessage::CreateResourceType { name: "postgresql".to_string() };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "CreateResourceType");
        assert_eq!(json["name"], "postgresql");
        // Should NOT have workspace field
        assert!(json.get("workspace").is_none());
    }

    #[test]
    fn test_webhook_message_all_variants_have_type_tag() {
        let messages: Vec<WebhookMessage> = vec![
            WebhookMessage::CreateApp { workspace: "w".into(), path: "p".into() },
            WebhookMessage::DeleteApp { workspace: "w".into(), path: "p".into() },
            WebhookMessage::UpdateApp {
                workspace: "w".into(),
                old_path: "o".into(),
                new_path: "n".into(),
            },
            WebhookMessage::CreateFlow { workspace: "w".into(), path: "p".into() },
            WebhookMessage::UpdateFlow {
                workspace: "w".into(),
                old_path: "o".into(),
                new_path: "n".into(),
            },
            WebhookMessage::ArchiveFlow { workspace: "w".into(), path: "p".into() },
            WebhookMessage::DeleteFlow { workspace: "w".into(), path: "p".into() },
            WebhookMessage::CreateFolder { workspace: "w".into(), name: "n".into() },
            WebhookMessage::UpdateFolder { workspace: "w".into(), name: "n".into() },
            WebhookMessage::DeleteFolder { workspace: "w".into(), name: "n".into() },
            WebhookMessage::DeleteResource { workspace: "w".into(), path: "p".into() },
            WebhookMessage::CreateResource { workspace: "w".into(), path: "p".into() },
            WebhookMessage::UpdateResource {
                workspace: "w".into(),
                old_path: "o".into(),
                new_path: "n".into(),
            },
            WebhookMessage::CreateResourceType { name: "n".into() },
            WebhookMessage::DeleteResourceType { name: "n".into() },
            WebhookMessage::UpdateResourceType { name: "n".into() },
            WebhookMessage::CreateScript {
                workspace: "w".into(),
                path: "p".into(),
                hash: "h".into(),
            },
            WebhookMessage::UpdateScript {
                workspace: "w".into(),
                path: "p".into(),
                hash: "h".into(),
            },
            WebhookMessage::DeleteScript { workspace: "w".into(), hash: "h".into() },
            WebhookMessage::DeleteScriptPath { workspace: "w".into(), path: "p".into() },
            WebhookMessage::CreateVariable { workspace: "w".into(), path: "p".into() },
            WebhookMessage::UpdateVariable {
                workspace: "w".into(),
                old_path: "o".into(),
                new_path: "n".into(),
            },
            WebhookMessage::DeleteVariable { workspace: "w".into(), path: "p".into() },
        ];

        for msg in &messages {
            let json = serde_json::to_value(msg).unwrap();
            assert!(
                json.get("type").is_some(),
                "Missing 'type' tag in: {}",
                serde_json::to_string(msg).unwrap()
            );
        }
    }

    #[test]
    fn test_webhook_message_type_tags_are_variant_names() {
        let msg = WebhookMessage::CreateApp { workspace: "w".into(), path: "p".into() };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "CreateApp");

        let msg = WebhookMessage::DeleteVariable { workspace: "w".into(), path: "p".into() };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "DeleteVariable");
    }
}
