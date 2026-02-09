/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

// Re-export everything from windmill-api-workspaces
pub use windmill_api_workspaces::workspaces::*;

use crate::ai::{AIConfig, AI_REQUEST_CACHE};
use crate::db::ApiAuthed;
use crate::teams_oss::{
    connect_teams, edit_teams_command, run_teams_message_test_job,
    workspaces_list_available_teams_channels, workspaces_list_available_teams_ids,
};

use axum::{
    extract::{Extension, Path},
    routing::{get, post},
    Json, Router,
};
use windmill_audit::audit_oss::audit_log;
use windmill_audit::ActionKind;
use windmill_common::{
    error::{Error, JsonResult, Result},
    utils::require_admin,
    DB,
};
use windmill_git_sync::{handle_deployment_metadata, DeployedObject};

#[cfg(feature = "enterprise")]
use axum::extract::Query;
#[cfg(feature = "enterprise")]
use serde::Deserialize;
#[cfg(feature = "enterprise")]
use windmill_common::utils::require_admin_or_devops;

/// Wraps the subcrate's workspaced_service with routes that depend on windmill-api internals.
pub fn workspaced_service() -> Router {
    let router = windmill_api_workspaces::workspaces::workspaced_service()
        .route("/edit_teams_command", post(edit_teams_command))
        .route(
            "/available_teams_ids",
            get(workspaces_list_available_teams_ids),
        )
        .route(
            "/available_teams_channels",
            get(workspaces_list_available_teams_channels),
        )
        .route("/connect_teams", post(connect_teams))
        .route(
            "/run_teams_message_test_job",
            post(run_teams_message_test_job),
        )
        .route("/tarball", get(crate::workspaces_export::tarball_workspace))
        .route("/edit_copilot_config", post(edit_copilot_config))
        .route("/get_copilot_info", get(get_copilot_info))
        .route("/critical_alerts", get(get_critical_alerts))
        .route(
            "/critical_alerts/:id/acknowledge",
            post(acknowledge_critical_alert),
        )
        .route(
            "/critical_alerts/acknowledge_all",
            post(acknowledge_all_critical_alerts),
        )
        .route("/critical_alerts/mute", post(mute_critical_alerts));

    #[cfg(all(feature = "stripe", feature = "enterprise"))]
    {
        crate::stripe_oss::add_stripe_routes(router)
    }

    #[cfg(not(feature = "stripe"))]
    router
}

async fn edit_copilot_config(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    ApiAuthed { is_admin, username, .. }: ApiAuthed,
    Json(ai_config): Json<AIConfig>,
) -> Result<String> {
    require_admin(is_admin, &username)?;

    if let Some(ref custom_prompts) = ai_config.custom_prompts {
        for (mode, prompt) in custom_prompts.iter() {
            if prompt.len() > MAX_CUSTOM_PROMPT_LENGTH {
                return Err(Error::BadRequest(format!(
                    "Custom prompt for mode '{}' exceeds maximum length of {} characters (current: {})",
                    mode,
                    MAX_CUSTOM_PROMPT_LENGTH,
                    prompt.len()
                )));
            }
        }
    }

    let mut tx = db.begin().await?;

    sqlx::query!(
        "UPDATE workspace_settings SET ai_config = $1 WHERE workspace_id = $2",
        sqlx::types::Json(&ai_config) as sqlx::types::Json<&AIConfig>,
        &w_id
    )
    .execute(&mut *tx)
    .await?;

    if let Some(ref providers) = ai_config.providers {
        for provider in providers.keys() {
            AI_REQUEST_CACHE.remove(&(w_id.clone(), provider.clone()));
        }
    }

    audit_log(
        &mut *tx,
        &authed,
        "workspaces.edit_copilot_config",
        ActionKind::Update,
        &w_id,
        Some(&authed.email),
        Some([("ai_config", &format!("{:?}", ai_config)[..])].into()),
    )
    .await?;
    tx.commit().await?;

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        DeployedObject::Settings { setting_type: "ai_config".to_string() },
        Some("AI configuration updated".to_string()),
        false,
        None,
    )
    .await?;

    Ok(format!("Edit copilot config for workspace {}", &w_id))
}

async fn get_copilot_info(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> JsonResult<AIConfig> {
    let mut tx = db.begin().await?;
    let copilot_info = sqlx::query_scalar!(
        "SELECT ai_config as \"ai_config: sqlx::types::Json<AIConfig>\" FROM workspace_settings WHERE workspace_id = $1",
        &w_id
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        Error::internal_err(format!(
            "getting ai config: {e:#}"
        ))
    })?;
    tx.commit().await?;

    if let Some(sqlx::types::Json(copilot_info)) = copilot_info {
        Ok(Json(copilot_info))
    } else {
        Ok(Json(AIConfig {
            providers: None,
            default_model: None,
            code_completion_model: None,
            custom_prompts: None,
            max_tokens_per_model: None,
        }))
    }
}

#[cfg(feature = "enterprise")]
pub async fn get_critical_alerts(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    authed: ApiAuthed,
    Query(params): Query<crate::utils::AlertQueryParams>,
) -> JsonResult<serde_json::Value> {
    require_admin_or_devops(authed.is_admin, &authed.username, &authed.email, &db).await?;

    crate::utils::get_critical_alerts(db, params, Some(w_id)).await
}

#[cfg(not(feature = "enterprise"))]
pub async fn get_critical_alerts() -> Error {
    Error::NotFound("Critical Alerts require EE".to_string())
}

#[cfg(feature = "enterprise")]
pub async fn acknowledge_critical_alert(
    Extension(db): Extension<DB>,
    Path((w_id, id)): Path<(String, i32)>,
    authed: ApiAuthed,
) -> Result<String> {
    require_admin_or_devops(authed.is_admin, &authed.username, &authed.email, &db).await?;
    crate::utils::acknowledge_critical_alert(db, Some(w_id), id).await
}

#[cfg(not(feature = "enterprise"))]
pub async fn acknowledge_critical_alert() -> Error {
    Error::NotFound("Critical Alerts require EE".to_string())
}

#[cfg(feature = "enterprise")]
pub async fn acknowledge_all_critical_alerts(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    authed: ApiAuthed,
) -> Result<String> {
    require_admin(authed.is_admin, &authed.username)?;
    crate::utils::acknowledge_all_critical_alerts(db, Some(w_id)).await
}

#[cfg(not(feature = "enterprise"))]
pub async fn acknowledge_all_critical_alerts() -> Error {
    Error::NotFound("Critical Alerts require EE".to_string())
}

#[cfg(feature = "enterprise")]
#[derive(Deserialize)]
pub struct MuteCriticalAlertRequest {
    pub mute_critical_alerts: Option<bool>,
}

#[cfg(feature = "enterprise")]
async fn mute_critical_alerts(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    authed: ApiAuthed,
    Json(m_r): Json<MuteCriticalAlertRequest>,
) -> Result<String> {
    require_admin(authed.is_admin, &authed.username)?;

    let mute_alerts = m_r.mute_critical_alerts.unwrap_or(false);

    if mute_alerts {
        sqlx::query!(
            "UPDATE alerts SET acknowledged_workspace = true, acknowledged = true WHERE workspace_id = $1",
            &w_id
        )
    .execute(&db)
    .await?;
    }

    sqlx::query!(
        "UPDATE workspace_settings SET mute_critical_alerts = $1 WHERE workspace_id = $2",
        mute_alerts,
        &w_id
    )
    .execute(&db)
    .await?;

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        DeployedObject::Settings { setting_type: "critical_alerts".to_string() },
        None,
        false,
        None,
    )
    .await?;

    Ok(format!(
        "Updated mute criticital alert ui settings for workspace: {}",
        &w_id
    ))
}

#[cfg(not(feature = "enterprise"))]
pub async fn mute_critical_alerts() -> Error {
    Error::NotFound("Critical Alerts require EE".to_string())
}
