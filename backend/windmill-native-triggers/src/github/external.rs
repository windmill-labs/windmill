use http::StatusCode;
use reqwest::Method;
use sqlx::PgConnection;
use std::collections::HashMap;
use windmill_common::{
    error::{Error, Result},
    BASE_URL, DB,
};

use crate::{
    generate_webhook_service_url,
    github::{
        routes, CreateWebhookResponse, GitHub, GithubOAuthData, GithubServiceConfig,
        GithubTriggerData, GithubWebhookApiResponse, GITHUB_API_BASE,
    },
    http_error_status, External, NativeTrigger, NativeTriggerData, PushArgsOwned, ServiceName,
};

#[async_trait::async_trait]
impl External for GitHub {
    type ServiceConfig = GithubServiceConfig;
    type TriggerData = GithubTriggerData;
    type OAuthData = GithubOAuthData;
    type CreateResponse = CreateWebhookResponse;

    const SERVICE_NAME: ServiceName = ServiceName::Github;
    const DISPLAY_NAME: &'static str = "GitHub";
    const SUPPORT_WEBHOOK: bool = true;
    const TOKEN_ENDPOINT: &'static str = "https://github.com/login/oauth/access_token";
    const REFRESH_ENDPOINT: &'static str = "https://github.com/login/oauth/access_token";
    const AUTH_ENDPOINT: &'static str = "https://github.com/login/oauth/authorize";

    async fn create(
        &self,
        w_id: &str,
        _oauth_data: &Self::OAuthData,
        webhook_token: &str,
        data: &NativeTriggerData<Self::ServiceConfig>,
        db: &DB,
        _tx: &mut PgConnection,
    ) -> Result<Self::CreateResponse> {
        let base_url = &**BASE_URL.load();

        // During create, we don't have external_id yet (GitHub assigns the hook ID)
        let webhook_url = generate_webhook_service_url(
            base_url,
            w_id,
            &data.script_path,
            data.is_flow,
            None,
            ServiceName::Github,
            webhook_token,
        );

        let url = format!(
            "{}/repos/{}/{}/hooks",
            GITHUB_API_BASE, data.service_config.owner, data.service_config.repo
        );

        let payload = serde_json::json!({
            "name": "web",
            "active": true,
            "events": data.service_config.events,
            "config": {
                "url": webhook_url,
                "content_type": "json",
                "insecure_ssl": "0"
            }
        });

        let response: CreateWebhookResponse = self
            .http_client_request(&url, Method::POST, w_id, db, None, Some(&payload))
            .await?;

        Ok(response)
    }

    async fn update(
        &self,
        w_id: &str,
        _oauth_data: &Self::OAuthData,
        external_id: &str,
        webhook_token: &str,
        data: &NativeTriggerData<Self::ServiceConfig>,
        db: &DB,
        _tx: &mut PgConnection,
    ) -> Result<serde_json::Value> {
        let base_url = &**BASE_URL.load();

        // Now we have external_id, include it in the webhook URL
        let webhook_url = generate_webhook_service_url(
            base_url,
            w_id,
            &data.script_path,
            data.is_flow,
            Some(external_id),
            ServiceName::Github,
            webhook_token,
        );

        let url = format!(
            "{}/repos/{}/{}/hooks/{}",
            GITHUB_API_BASE, data.service_config.owner, data.service_config.repo, external_id
        );

        let payload = serde_json::json!({
            "active": true,
            "events": data.service_config.events,
            "config": {
                "url": webhook_url,
                "content_type": "json",
                "insecure_ssl": "0"
            }
        });

        let _: serde_json::Value = self
            .http_client_request(&url, Method::PATCH, w_id, db, None, Some(&payload))
            .await?;

        // Return the resolved service_config
        serde_json::to_value(&data.service_config)
            .map_err(|e| Error::internal_err(format!("Failed to serialize service config: {}", e)))
    }

    async fn get(
        &self,
        w_id: &str,
        _oauth_data: &Self::OAuthData,
        external_id: &str,
        db: &DB,
        _tx: &mut PgConnection,
    ) -> Result<Option<Self::TriggerData>> {
        // We need owner/repo to construct the API URL — fetch from DB
        let (owner, repo) = match self.get_owner_repo_from_db(db, w_id, external_id).await? {
            Some(pair) => pair,
            None => return Ok(None),
        };

        let data = self
            .get_webhook(w_id, &owner, &repo, external_id, db)
            .await?;
        Ok(Some(data))
    }

    async fn delete(
        &self,
        w_id: &str,
        _oauth_data: &Self::OAuthData,
        external_id: &str,
        db: &DB,
        _tx: &mut PgConnection,
    ) -> Result<()> {
        // We need owner/repo to construct the API URL — fetch from DB.
        // Missing row means the trigger was already removed; other DB errors propagate.
        let (owner, repo) = match self.get_owner_repo_from_db(db, w_id, external_id).await? {
            Some(pair) => pair,
            None => return Ok(()),
        };

        let url = format!(
            "{}/repos/{}/{}/hooks/{}",
            GITHUB_API_BASE, owner, repo, external_id
        );

        // Swallow 404 only (webhook may already be deleted on GitHub); propagate
        // other errors so callers don't think cleanup succeeded when it didn't.
        let result: std::result::Result<serde_json::Value, _> = self
            .http_client_request::<_, ()>(&url, Method::DELETE, w_id, db, None, None)
            .await;

        match result {
            Ok(_) => Ok(()),
            Err(e) if http_error_status(&e) == Some(StatusCode::NOT_FOUND) => Ok(()),
            Err(e) => Err(e),
        }
    }

    async fn maintain_triggers(
        &self,
        db: &DB,
        workspace_id: &str,
        triggers: &[NativeTrigger],
        _oauth_data: &Self::OAuthData,
        synced: &mut Vec<crate::sync::TriggerSyncInfo>,
        errors: &mut Vec<crate::sync::SyncError>,
    ) {
        // GitHub webhooks don't expire, but we verify they still exist
        for trigger in triggers {
            let config: GithubServiceConfig = match trigger
                .service_config
                .as_ref()
                .and_then(|v| serde_json::from_value(v.clone()).ok())
            {
                Some(c) => c,
                None => {
                    errors.push(crate::sync::SyncError {
                        resource_path: trigger.script_path.clone(),
                        error_message: "Invalid service config".to_string(),
                        error_type: "config_error".to_string(),
                    });
                    continue;
                }
            };

            let url = format!(
                "{}/repos/{}/{}/hooks/{}",
                GITHUB_API_BASE, config.owner, config.repo, trigger.external_id
            );

            let result: std::result::Result<GithubWebhookApiResponse, _> = self
                .http_client_request::<_, ()>(&url, Method::GET, workspace_id, db, None, None)
                .await;

            match result {
                Ok(_response) => {
                    // Webhook still exists — clear any previous error
                    if trigger.error.is_some() {
                        synced.push(crate::sync::TriggerSyncInfo {
                            external_id: trigger.external_id.clone(),
                            script_path: trigger.script_path.clone(),
                            action: crate::sync::SyncAction::ErrorCleared,
                        });
                    }
                }
                Err(e) if http_error_status(&e) == Some(StatusCode::NOT_FOUND) => {
                    errors.push(crate::sync::SyncError {
                        resource_path: trigger.script_path.clone(),
                        error_message: "Webhook no longer exists on GitHub".to_string(),
                        error_type: "not_found".to_string(),
                    });
                }
                Err(e) => {
                    errors.push(crate::sync::SyncError {
                        resource_path: trigger.script_path.clone(),
                        error_message: format!("Failed to verify GitHub webhook: {}", e),
                        error_type: "api_error".to_string(),
                    });
                }
            }
        }
    }

    async fn prepare_webhook(
        &self,
        _db: &DB,
        _w_id: &str,
        headers: HashMap<String, String>,
        body: String,
        _script_path: &str,
        _is_flow: bool,
    ) -> Result<PushArgsOwned> {
        use serde_json::value::to_raw_value;

        let mut args = HashMap::new();

        let event = headers.get("x-github-event").cloned().unwrap_or_default();

        let delivery_id = headers
            .get("x-github-delivery")
            .cloned()
            .unwrap_or_default();

        let payload: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();

        args.insert(
            "event".to_string(),
            to_raw_value(&event).map_err(|e| Error::internal_err(e.to_string()))?,
        );
        args.insert(
            "delivery_id".to_string(),
            to_raw_value(&delivery_id).map_err(|e| Error::internal_err(e.to_string()))?,
        );
        args.insert(
            "payload".to_string(),
            to_raw_value(&payload).map_err(|e| Error::internal_err(e.to_string()))?,
        );

        Ok(PushArgsOwned { extra: None, args })
    }

    fn external_id_and_metadata_from_response(
        &self,
        resp: &Self::CreateResponse,
    ) -> (String, Option<serde_json::Value>) {
        (resp.id.to_string(), None)
    }

    fn additional_routes(&self) -> axum::Router {
        routes::github_routes(self.clone())
    }
}

impl GitHub {
    /// Fetch owner/repo from the stored service_config in the DB.
    /// Returns `Ok(None)` if the trigger row is missing; propagates other DB errors.
    async fn get_owner_repo_from_db(
        &self,
        db: &DB,
        w_id: &str,
        external_id: &str,
    ) -> Result<Option<(String, String)>> {
        let config = sqlx::query_scalar!(
            r#"
            SELECT service_config
            FROM native_trigger
            WHERE external_id = $1 AND service_name = $2 AND workspace_id = $3
            "#,
            external_id,
            ServiceName::Github as ServiceName,
            w_id
        )
        .fetch_optional(db)
        .await?
        .flatten();

        let Some(config) = config else {
            return Ok(None);
        };

        let owner = config
            .get("owner")
            .and_then(|v| v.as_str())
            .map(String::from)
            .unwrap_or_default();
        let repo = config
            .get("repo")
            .and_then(|v| v.as_str())
            .map(String::from)
            .unwrap_or_default();

        Ok(Some((owner, repo)))
    }

    /// Fetch a webhook from GitHub API.
    async fn get_webhook(
        &self,
        w_id: &str,
        owner: &str,
        repo: &str,
        external_id: &str,
        db: &DB,
    ) -> Result<GithubTriggerData> {
        let url = format!(
            "{}/repos/{}/{}/hooks/{}",
            GITHUB_API_BASE, owner, repo, external_id
        );

        let response: GithubWebhookApiResponse = self
            .http_client_request::<_, ()>(&url, Method::GET, w_id, db, None, None)
            .await?;

        Ok(GithubTriggerData {
            id: response.id,
            active: response.active,
            events: response.events,
            owner: owner.to_string(),
            repo: repo.to_string(),
        })
    }
}
