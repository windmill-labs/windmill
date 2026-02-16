use async_trait::async_trait;
use reqwest::Method;
use serde_json::value::RawValue;
use sqlx::PgConnection;
use std::collections::HashMap;
use windmill_common::{
    error::{Error, Result},
    worker::to_raw_value,
    BASE_URL, DB,
};
use windmill_queue::PushArgsOwned;

use crate::{
    generate_webhook_service_url, get_token_by_prefix,
    sync::{SyncAction, SyncError, TriggerSyncInfo},
    update_native_trigger_error, update_native_trigger_service_config, External, NativeTrigger,
    NativeTriggerData, ServiceName,
};

use super::{
    endpoints, routes, CreateWatchResponse, Google, GoogleOAuthData, GoogleServiceConfig,
    GoogleTriggerType, StopChannelRequest, WatchRequest,
};

#[async_trait]
impl External for Google {
    type ServiceConfig = GoogleServiceConfig;
    // Google has no "get channel" API, so TriggerData is never constructed.
    // The trait default for get() returns Ok(None).
    type TriggerData = ();
    type OAuthData = GoogleOAuthData;
    type CreateResponse = CreateWatchResponse;

    const SERVICE_NAME: ServiceName = ServiceName::Google;
    const DISPLAY_NAME: &'static str = "Google";
    const SUPPORT_WEBHOOK: bool = true;
    const TOKEN_ENDPOINT: &'static str = "https://oauth2.googleapis.com/token";
    const REFRESH_ENDPOINT: &'static str = "https://oauth2.googleapis.com/token";
    const AUTH_ENDPOINT: &'static str = "https://accounts.google.com/o/oauth2/v2/auth";

    async fn create(
        &self,
        w_id: &str,
        _oauth_data: &Self::OAuthData,
        webhook_token: &str,
        data: &NativeTriggerData<Self::ServiceConfig>,
        db: &DB,
        _tx: &mut PgConnection,
    ) -> Result<Self::CreateResponse> {
        let channel_id = uuid::Uuid::new_v4().to_string();
        self.create_watch_channel(w_id, &channel_id, webhook_token, data, db)
            .await
    }

    async fn update(
        &self,
        w_id: &str,
        oauth_data: &Self::OAuthData,
        external_id: &str,
        webhook_token: &str,
        data: &NativeTriggerData<Self::ServiceConfig>,
        db: &DB,
        tx: &mut PgConnection,
    ) -> Result<serde_json::Value> {
        // Google doesn't support updating watch channels — delete old, create new.
        let _ = self.delete(w_id, oauth_data, external_id, db, tx).await;

        // Reuse the same channel ID so external_id stays permanent
        let resp = self
            .create_watch_channel(w_id, external_id, webhook_token, data, db)
            .await?;

        self.service_config_from_create_response(data, &resp)
            .ok_or_else(|| {
                Error::InternalErr(
                    "Failed to build service_config from create response".to_string(),
                )
            })
    }

    async fn delete(
        &self,
        w_id: &str,
        _oauth_data: &Self::OAuthData,
        external_id: &str,
        db: &DB,
        tx: &mut PgConnection,
    ) -> Result<()> {
        // Get the stored trigger to find the google_resource_id and trigger_type
        let trigger = sqlx::query_scalar!(
            r#"
            SELECT service_config
            FROM native_trigger
            WHERE external_id = $1 AND service_name = $2 AND workspace_id = $3
            "#,
            external_id,
            ServiceName::Google as ServiceName,
            w_id
        )
        .fetch_optional(&mut *tx)
        .await?;

        let config = trigger.flatten();
        if config.is_none() {
            return Ok(());
        }
        let config = config.unwrap();

        let (google_resource_id, url) = super::parse_stop_channel_params(&config);

        if !google_resource_id.is_empty() {
            let stop_request =
                StopChannelRequest { id: external_id.to_string(), resource_id: google_resource_id };

            // Stop the channel (ignore errors - channel may have already expired)
            let result: std::result::Result<serde_json::Value, _> = self
                .http_client_request(&url, Method::POST, w_id, db, None, Some(&stop_request))
                .await;

            if let Err(e) = result {
                tracing::warn!("Failed to stop Google channel {}: {}", external_id, e);
            }
        }

        Ok(())
    }

    async fn maintain_triggers(
        &self,
        db: &DB,
        workspace_id: &str,
        triggers: &[NativeTrigger],
        _oauth_data: &Self::OAuthData,
        synced: &mut Vec<TriggerSyncInfo>,
        errors: &mut Vec<SyncError>,
    ) {
        renew_expiring_channels(self, db, workspace_id, triggers, synced, errors).await;
    }

    async fn prepare_webhook(
        &self,
        _db: &DB,
        _w_id: &str,
        headers: HashMap<String, String>,
        _body: String,
        _script_path: &str,
        _is_flow: bool,
    ) -> Result<PushArgsOwned> {
        // Google sends notification info in headers (same format for Drive and Calendar)
        let payload = serde_json::json!({
            "channel_id": headers.get("x-goog-channel-id").cloned().unwrap_or_default(),
            "resource_id": headers.get("x-goog-resource-id").cloned().unwrap_or_default(),
            "resource_state": headers.get("x-goog-resource-state").cloned().unwrap_or_default(),
            "resource_uri": headers.get("x-goog-resource-uri").cloned().unwrap_or_default(),
            "message_number": headers.get("x-goog-message-number").cloned().unwrap_or_default(),
            "channel_expiration": headers.get("x-goog-channel-expiration").cloned().unwrap_or_default(),
            "changed": headers.get("x-goog-changed").cloned().unwrap_or_default(),
            "channel_token": headers.get("x-goog-channel-token").cloned().unwrap_or_default(),
        });

        let mut args: HashMap<String, Box<RawValue>> = HashMap::new();
        args.insert("payload".to_string(), to_raw_value(&payload));

        Ok(PushArgsOwned { extra: None, args })
    }

    fn external_id_and_metadata_from_response(
        &self,
        resp: &Self::CreateResponse,
    ) -> (String, Option<serde_json::Value>) {
        let metadata = serde_json::json!({
            "googleResourceId": resp.resource_id,
            "expiration": resp.expiration,
        });
        (resp.id.clone(), Some(metadata))
    }

    fn service_config_from_create_response(
        &self,
        data: &NativeTriggerData<Self::ServiceConfig>,
        resp: &Self::CreateResponse,
    ) -> Option<serde_json::Value> {
        let mut config = data.service_config.clone();
        config.google_resource_id = Some(resp.resource_id.clone());
        config.expiration = Some(resp.expiration.clone());
        serde_json::to_value(&config).ok()
    }

    fn additional_routes(&self) -> axum::Router {
        routes::google_routes(self.clone())
    }
}

// Helper methods for creating trigger type-specific watches
impl Google {
    /// Build a webhook URL and watch request, then register the channel with Google.
    /// Used by both `create()` (new UUID) and `update()` (reuse existing external_id).
    async fn create_watch_channel(
        &self,
        w_id: &str,
        channel_id: &str,
        webhook_token: &str,
        data: &NativeTriggerData<GoogleServiceConfig>,
        db: &DB,
    ) -> Result<CreateWatchResponse> {
        let base_url = &*BASE_URL.read().await;
        let webhook_url = generate_webhook_service_url(
            base_url,
            w_id,
            &data.script_path,
            data.is_flow,
            Some(channel_id),
            ServiceName::Google,
            webhook_token,
        );

        tracing::info!(
            "Creating Google {} watch channel '{}' with webhook URL: {}",
            data.service_config.trigger_type,
            channel_id,
            webhook_url
        );

        let expiration_ms = chrono::Utc::now().timestamp_millis()
            + (data.service_config.max_expiration_hours() as i64 * 3600 * 1000);
        let mut watch_request = WatchRequest::new(channel_id.to_string(), webhook_url);
        watch_request.expiration = Some(expiration_ms);

        match data.service_config.trigger_type {
            GoogleTriggerType::Drive => {
                self.create_drive_watch(w_id, &data.service_config, &watch_request, db)
                    .await
            }
            GoogleTriggerType::Calendar => {
                self.create_calendar_watch(w_id, &data.service_config, &watch_request, db)
                    .await
            }
        }
    }

    async fn create_drive_watch(
        &self,
        w_id: &str,
        config: &GoogleServiceConfig,
        watch_request: &WatchRequest,
        db: &DB,
    ) -> Result<CreateWatchResponse> {
        match config.resource_id.as_deref().filter(|s| !s.is_empty()) {
            Some(resource_id) => {
                // Specific file: use files.watch
                let url = format!("{}/files/{}/watch", endpoints::DRIVE_API_BASE, resource_id);

                self.http_client_request(&url, Method::POST, w_id, db, None, Some(watch_request))
                    .await
            }
            None => {
                // All changes: use changes.watch
                let token_url = format!("{}/changes/startPageToken", endpoints::DRIVE_API_BASE);
                let token_response: serde_json::Value = self
                    .http_client_request::<_, ()>(&token_url, Method::GET, w_id, db, None, None)
                    .await?;

                let start_page_token = token_response
                    .get("startPageToken")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        Error::InternalErr("Failed to get startPageToken".to_string())
                    })?;

                let watch_body = serde_json::to_value(watch_request)?;
                let watch_url = format!(
                    "{}/changes/watch?pageToken={}",
                    endpoints::DRIVE_API_BASE,
                    start_page_token
                );

                self.http_client_request(
                    &watch_url,
                    Method::POST,
                    w_id,
                    db,
                    None,
                    Some(&watch_body),
                )
                .await
            }
        }
    }

    async fn create_calendar_watch(
        &self,
        w_id: &str,
        config: &GoogleServiceConfig,
        watch_request: &WatchRequest,
        db: &DB,
    ) -> Result<CreateWatchResponse> {
        let calendar_id = config.calendar_id.as_ref().ok_or_else(|| {
            Error::BadRequest("calendar_id is required for Calendar triggers".into())
        })?;

        let url = format!(
            "{}/calendars/{}/events/watch",
            endpoints::CALENDAR_API_BASE,
            urlencoding::encode(calendar_id)
        );

        self.http_client_request(&url, Method::POST, w_id, db, None, Some(watch_request))
            .await
    }

    /// Renew an expiring Google watch channel.
    /// Stops the old channel and creates a new one with the same channel ID.
    /// Returns the updated service_config with new expiration.
    pub async fn renew_channel(
        &self,
        w_id: &str,
        trigger: &NativeTrigger,
        db: &DB,
    ) -> Result<serde_json::Value> {
        let config: GoogleServiceConfig = trigger
            .service_config
            .as_ref()
            .map(|v| serde_json::from_value(v.clone()))
            .transpose()?
            .ok_or_else(|| Error::InternalErr("Missing service config".to_string()))?;

        let webhook_token = get_token_by_prefix(db, &trigger.webhook_token_prefix)
            .await?
            .ok_or_else(|| Error::InternalErr("Webhook token not found".to_string()))?;

        let base_url = &*BASE_URL.read().await;
        // Reuse the same channel ID so external_id stays permanent
        let channel_id = trigger.external_id.clone();
        let webhook_url = generate_webhook_service_url(
            base_url,
            w_id,
            &trigger.script_path,
            trigger.is_flow,
            Some(&channel_id),
            ServiceName::Google,
            &webhook_token,
        );

        tracing::info!(
            "Renewing Google {} watch channel '{}' with webhook URL: {}",
            config.trigger_type,
            channel_id,
            webhook_url
        );

        let expiration_ms = chrono::Utc::now().timestamp_millis()
            + (config.max_expiration_hours() as i64 * 3600 * 1000);
        let mut watch_request = WatchRequest::new(channel_id.clone(), webhook_url);
        watch_request.expiration = Some(expiration_ms);

        // Best-effort stop old channel before creating a new one
        let old_google_resource_id = trigger
            .service_config
            .as_ref()
            .and_then(|c| c.get("googleResourceId"))
            .and_then(|r| r.as_str())
            .unwrap_or_default();

        if !old_google_resource_id.is_empty() {
            let stop_request = StopChannelRequest {
                id: channel_id.clone(),
                resource_id: old_google_resource_id.to_string(),
            };
            let url = match config.trigger_type {
                GoogleTriggerType::Calendar => {
                    format!("{}/channels/stop", endpoints::CALENDAR_API_BASE)
                }
                GoogleTriggerType::Drive => {
                    format!("{}/channels/stop", endpoints::DRIVE_API_BASE)
                }
            };
            let result: std::result::Result<serde_json::Value, _> = self
                .http_client_request(&url, Method::POST, w_id, db, None, Some(&stop_request))
                .await;
            if let Err(e) = result {
                tracing::warn!(
                    "Failed to stop old Google channel {} during renewal: {}",
                    channel_id,
                    e
                );
            }
        }

        // Create new watch channel with the same channel ID
        let resp = match config.trigger_type {
            GoogleTriggerType::Drive => {
                self.create_drive_watch(w_id, &config, &watch_request, db)
                    .await?
            }
            GoogleTriggerType::Calendar => {
                self.create_calendar_watch(w_id, &config, &watch_request, db)
                    .await?
            }
        };

        // Build the updated service_config with new expiration
        let mut new_config = config;
        new_config.google_resource_id = Some(resp.resource_id);
        new_config.expiration = Some(resp.expiration);

        serde_json::to_value(&new_config)
            .map_err(|e| Error::internal_err(format!("Failed to serialize config: {}", e)))
    }
}

/// Renewal window: renew Drive channels with <1 hour remaining, Calendar with <1 day remaining.
pub fn should_renew_channel(service_config: &serde_json::Value) -> bool {
    let expiration_ms = service_config
        .get("expiration")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0);

    if expiration_ms == 0 {
        return false;
    }

    let now_ms = chrono::Utc::now().timestamp_millis();
    let remaining_ms = expiration_ms - now_ms;

    let trigger_type = service_config
        .get("triggerType")
        .and_then(|t| t.as_str())
        .unwrap_or("drive");

    let renewal_window_ms: i64 = match trigger_type {
        "calendar" => 24 * 60 * 60 * 1000, // 1 day for Calendar (7 day expiry)
        _ => 60 * 60 * 1000,               // 1 hour for Drive (24h expiry)
    };

    remaining_ms < renewal_window_ms
}

async fn renew_expiring_channels(
    handler: &Google,
    db: &DB,
    workspace_id: &str,
    triggers: &[NativeTrigger],
    synced: &mut Vec<TriggerSyncInfo>,
    errors: &mut Vec<SyncError>,
) {
    for trigger in triggers {
        let Some(config) = &trigger.service_config else {
            continue;
        };

        if !should_renew_channel(config) {
            continue;
        }

        tracing::info!(
            "Renewing expiring Google channel {} for script_path '{}' in workspace '{}'",
            trigger.external_id,
            trigger.script_path,
            workspace_id
        );

        match handler.renew_channel(workspace_id, trigger, db).await {
            Ok(new_config) => {
                match update_native_trigger_service_config(
                    db,
                    workspace_id,
                    ServiceName::Google,
                    &trigger.external_id,
                    &new_config,
                )
                .await
                {
                    Ok(()) => {
                        tracing::info!(
                            "Renewed Google channel {} for '{}'",
                            trigger.external_id,
                            trigger.script_path
                        );
                        synced.push(TriggerSyncInfo {
                            external_id: trigger.external_id.clone(),
                            script_path: trigger.script_path.clone(),
                            action: SyncAction::ConfigUpdated,
                        });
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to update DB after renewing Google channel {}: {}",
                            trigger.external_id,
                            e
                        );
                        errors.push(SyncError {
                            resource_path: format!("workspace:{}", workspace_id),
                            error_message: format!(
                                "Failed to update DB after channel renewal for {}: {}",
                                trigger.external_id, e
                            ),
                            error_type: "channel_renewal_error".to_string(),
                        });
                    }
                }
            }
            Err(e) => {
                tracing::error!(
                    "Failed to renew Google channel {} for '{}': {}",
                    trigger.external_id,
                    trigger.script_path,
                    e
                );

                let _ = update_native_trigger_error(
                    db,
                    workspace_id,
                    ServiceName::Google,
                    &trigger.external_id,
                    Some(&format!("Channel renewal failed: {}", e)),
                )
                .await;

                errors.push(SyncError {
                    resource_path: format!("workspace:{}", workspace_id),
                    error_message: format!(
                        "Channel renewal failed for {}: {}",
                        trigger.external_id, e
                    ),
                    error_type: "channel_renewal_error".to_string(),
                });
            }
        }
    }
}
