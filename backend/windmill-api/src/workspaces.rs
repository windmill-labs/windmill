/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

#[cfg(feature = "enterprise")]
use std::str::FromStr;

use crate::BASE_URL;
use crate::db::ApiAuthed;
use crate::{
    apps::AppWithLastVersion,
    db::DB,
    folders::Folder,
    resources::{Resource, ResourceType},
    users::{WorkspaceInvite, VALID_USERNAME, send_email_if_possible},
    utils::require_super_admin,
    variables::build_crypt,
    webhook_util::{InstanceEvent, WebhookShared}
};
#[cfg(feature = "enterprise")]
use axum::response::Redirect;
use axum::{
    body::StreamBody,
    extract::{Extension, Path, Query},
    headers,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use chrono::Utc;
use magic_crypt::MagicCryptTrait;
#[cfg(feature = "enterprise")]
use stripe::CustomerId;
use uuid::Uuid;
use windmill_audit::{audit_log, ActionKind};
use windmill_common::db::UserDB;
use windmill_common::schedule::Schedule;
use windmill_common::users::username_to_permissioned_as;
use windmill_common::{
    error::{to_anyhow, Error, JsonResult, Result},
    flows::Flow,
    scripts::{Schema, Script, ScriptLang},
    utils::{paginate, rd_string, require_admin, Pagination},
    variables::ExportableListableVariable,
    oauth2::WORKSPACE_SLACK_BOT_TOKEN_PATH,
};
use windmill_queue::QueueTransaction;

use hyper::{header, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map};
use sqlx::{FromRow, Postgres, Transaction};
use tempfile::TempDir;
use tokio::fs::File;
use tokio_util::io::ReaderStream;

pub fn workspaced_service() -> Router {
    let router = Router::new()
        .route("/list_pending_invites", get(list_pending_invites))
        .route("/update", post(edit_workspace))
        .route("/archive", post(archive_workspace))
        .route("/invite_user", post(invite_user))
        .route("/add_user", post(add_user))
        .route("/delete_invite", post(delete_invite))
        .route("/get_settings", get(get_settings))
        .route("/get_deploy_to", get(get_deploy_to))
        .route("/edit_slack_command", post(edit_slack_command))
        .route("/run_slack_message_test_job", post(run_slack_message_test_job))
        .route("/edit_webhook", post(edit_webhook))
        .route("/edit_auto_invite", post(edit_auto_invite))
        .route("/edit_deploy_to", post(edit_deploy_to))
        .route("/tarball", get(tarball_workspace))
        .route("/premium_info", get(premium_info))
        .route("/edit_copilot_config", post(edit_copilot_config))
        .route("/get_copilot_info", get(get_copilot_info) )
        .route("/edit_error_handler", post(edit_error_handler));

    #[cfg(feature = "enterprise")]
    { 
        if std::env::var("STRIPE_KEY").is_err() {
            return router;
        } else {
            tracing::info!("stripe enabled");
            
            return router
            .route("/checkout", get(stripe_checkout))
            .route("/billing_portal", get(stripe_portal));
        }
    }

    #[cfg(not(feature = "enterprise"))]
    router
}
pub fn global_service() -> Router {
    Router::new()
        .route("/list_as_superadmin", get(list_workspaces_as_super_admin))
        .route("/list", get(list_workspaces))
        .route("/users", get(user_workspaces))
        .route("/create", post(create_workspace))
        .route("/exists", post(exists_workspace))
        .route("/exists_username", post(exists_username))
        .route("/allowed_domain_auto_invite", get(is_allowed_auto_domain))
        .route("/unarchive/:workspace", post(unarchive_workspace))
        .route("/delete/:workspace", delete(delete_workspace))
}

#[derive(FromRow, Serialize)]
struct Workspace {
    id: String,
    name: String,
    owner: String,
    deleted: bool,
    premium: bool,
}

#[derive(FromRow, Serialize, Debug)]
pub struct WorkspaceSettings {
    pub workspace_id: String,
    pub slack_team_id: Option<String>,
    pub slack_name: Option<String>,
    pub slack_command_script: Option<String>,
    pub slack_email: String,
    pub auto_invite_domain: Option<String>,
    pub auto_invite_operator: Option<bool>,
    pub customer_id: Option<String>,
    pub plan: Option<String>,
    pub webhook: Option<String>,
    pub deploy_to: Option<String>,
    pub openai_resource_path: Option<String>,
    pub code_completion_enabled: bool,
    pub error_handler: Option<String>,
    pub error_handler_extra_args: Option<serde_json::Value>,
    pub error_handler_muted_on_cancel: Option<bool>,
}

#[derive(FromRow, Serialize, Debug)]
pub struct Usage {
    pub workspace_id: String,
    pub slack_team_id: Option<String>,
    pub slack_name: Option<String>,
    pub slack_command_script: Option<String>,
    pub slack_email: String,
}

#[derive(sqlx::Type, Serialize, Deserialize, Debug)]
#[sqlx(type_name = "WORKSPACE_KEY_KIND", rename_all = "lowercase")]
pub enum WorkspaceKeyKind {
    Cloud,
}

#[derive(Deserialize)]
struct EditCommandScript {
    slack_command_script: Option<String>,
}

#[derive(Deserialize)]
struct RunSlackMessageTestJobRequest {
    hub_script_path: String,
    channel: String,
    test_msg: String
}

#[derive(Serialize)]
struct RunSlackMessageTestJobResponse {
    job_uuid: String,
}

#[cfg(feature = "enterprise")]
#[derive(Deserialize)]
struct EditDeployTo {
    deploy_to: Option<String>,
}

#[derive(Deserialize)]
struct EditAutoInvite {
    operator: Option<bool>,
}

#[derive(Deserialize)]
struct EditWebhook {
    webhook: Option<String>,
}

#[derive(Deserialize)]
struct EditCopilotConfig {
    openai_resource_path: Option<String>,
    code_completion_enabled: bool,
}

#[derive(Deserialize)]
struct CreateWorkspace {
    id: String,
    name: String,
    username: String,
}

#[derive(Deserialize)]
struct EditWorkspace {
    name: String,
    owner: String,
}

#[derive(Serialize)]
struct WorkspaceList {
    pub email: String,
    pub workspaces: Vec<UserWorkspace>,
}

#[derive(Serialize)]
struct UserWorkspace {
    pub id: String,
    pub name: String,
    pub username: String,
}

#[derive(Deserialize)]
struct WorkspaceId {
    pub id: String,
}

#[derive(Deserialize)]
struct ValidateUsername {
    pub id: String,
    pub username: String,
}

#[derive(Deserialize)]
pub struct NewWorkspaceInvite {
    pub email: String,
    pub is_admin: bool,
    pub operator: bool,
}

#[derive(Deserialize)]
pub struct NewWorkspaceUser {
    pub email: String,
    pub username: String,
    pub is_admin: bool,
    pub operator: bool,
}

#[derive(Deserialize)]
pub struct EditErrorHandler {
    pub error_handler: Option<String>,
    pub error_handler_extra_args: Option<serde_json::Value>,
    pub error_handler_muted_on_cancel: Option<bool>,
}

async fn list_pending_invites(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
) -> JsonResult<Vec<WorkspaceInvite>> {
    require_admin(authed.is_admin, &authed.username)?;
    let mut tx = user_db.begin(&authed).await?;
    let rows = sqlx::query_as!(
        WorkspaceInvite,
        "SELECT * from workspace_invite WHERE workspace_id = $1",
        w_id
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(rows))
}

#[derive(Serialize, FromRow)]
pub struct PremiumWorkspaceInfo {
    pub premium: bool,
    pub usage: Option<i32>,
}
async fn premium_info(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> JsonResult<PremiumWorkspaceInfo> {
    require_admin(authed.is_admin, &authed.username)?;
    let mut tx = db.begin().await?;
    let row = sqlx::query_as::<_, PremiumWorkspaceInfo>(
        "SELECT premium, usage.usage FROM workspace LEFT JOIN usage ON usage.id = $1 AND month_ = EXTRACT(YEAR FROM current_date) * 12 + EXTRACT(MONTH FROM current_date) AND usage.is_workspace IS true WHERE workspace.id = $1",
    )
    .bind(w_id)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(row))
}

#[cfg(feature = "enterprise")]
#[derive(Deserialize)]
struct PlanQuery {
    plan: String,
}

#[cfg(feature = "enterprise")]
async fn stripe_checkout(
    authed: ApiAuthed,
    Path(w_id): Path<String>,
    Query(plan): Query<PlanQuery>,
) -> Result<Redirect> {
    // #[cfg(feature = "enterprise")]
    {
        require_admin(authed.is_admin, &authed.username)?;

        let client = stripe::Client::new(std::env::var("STRIPE_KEY").expect("STRIPE_KEY"));
        let base_url = BASE_URL.read().await.clone();
        let success_rd = format!("{}/workspace_settings/checkout?success=true", base_url);
        let failure_rd = format!("{}/workspace_settings/checkout?success=false", base_url);
        let checkout_session = {
            let mut params = stripe::CreateCheckoutSession::new(&failure_rd, &success_rd);
            params.mode = Some(stripe::CheckoutSessionMode::Subscription);
            params.line_items = match plan.plan.as_str() {
                "team" => Some(vec![
                    stripe::CreateCheckoutSessionLineItems {
                        quantity: Some(1),
                        price: Some("price_1NCNOgGU3NdFi9eLuG4fZuEP".to_string()),
                        ..Default::default()
                    },
                    stripe::CreateCheckoutSessionLineItems {
                        quantity: None,
                        price: Some("price_1NCNCpGU3NdFi9eLbiE6Ca42".to_string()),
                        ..Default::default()
                    },
                ]),
                _ => Err(Error::BadRequest("invalid plan".to_string()))?,
            };
            params.customer_email = Some(&authed.email);
            params.client_reference_id = Some(&w_id);
            stripe::CheckoutSession::create(&client, params)
                .await
                .unwrap()
        };
        let uri = checkout_session
            .url
            .ok_or_else(|| Error::InternalErr(format!("stripe checkout redirect issue")))?;
        Ok(Redirect::to(&uri))
    }
}

#[cfg(feature = "enterprise")]
async fn stripe_portal(
    authed: ApiAuthed,
    Path(w_id): Path<String>,
    Extension(db): Extension<DB>,
) -> Result<Redirect> {
    require_admin(authed.is_admin, &authed.username)?;
    let customer_id = sqlx::query_scalar!(
        "SELECT customer_id FROM workspace_settings WHERE workspace_id = $1",
        w_id
    )
    .fetch_one(&db)
    .await?
    .ok_or_else(|| Error::InternalErr(format!("no customer id for workspace {}", w_id)))?;
    let client = stripe::Client::new(std::env::var("STRIPE_KEY").expect("STRIPE_KEY"));
    let success_rd = format!("{}/workspace_settings?tab=premium", BASE_URL.read().await.clone());
    let portal_session = {
        let customer_id = CustomerId::from_str(&customer_id).unwrap();
        let mut params = stripe::CreateBillingPortalSession::new(customer_id);
        params.return_url = Some(&success_rd);
        stripe::BillingPortalSession::create(&client, params)
            .await
            .map_err(to_anyhow)?
    };
    Ok(Redirect::to(&portal_session.url))
}

// async fn stripe_usage(
//     authed: ApiAuthed,
//     Path(w_id): Path<String>,
//     Extension(db): Extension<DB>,
//     Extension(base_url): Extension<Arc<BaseUrl>>,
// ) -> Result<Redirect> {
//     require_admin(authed.is_admin, &authed.username)?;
//     let customer_id = sqlx::query_scalar!(
//         "SELECT customer_id FROM workspace_settings WHERE workspace_id = $1",
//         w_id
//     )
//     .fetch_one(&db)
//     .await?
//     .ok_or_else(|| Error::InternalErr(format!("no customer id for workspace {}", w_id)))?;
//     let client = stripe::Client::new(std::env::var("STRIPE_KEY").expect("STRIPE_KEY"));
//     let success_rd = format!("{}/workspace_settings?tab=premium", base_url.0);
//     let portal_session = {
//         let customer_id = CustomerId::from_str(&customer_id).unwrap();
//         let subscriptions = stripe::Subscription::list(
//             &client,
//             stripe::ListSubscriptions { customer: Some(customer_id), ..Default::default() },
//         )
//         .await
//         .map_err(to_anyhow)?
//         .data[0];
//         let getUsage =
//             stripe::SubscriptionItem::list(
//                 &client,
//                 stripe::ListSubscriptionItems {
//                     subscription: subscription.id,
//                     ..Default::default()
//                 },
//             )
//             .await
//             .map_err(to_anyhow)
//         };
//         let mut params = stripe::ListSubscriptionItems::new(customer_id);
//         params.return_url = Some(&success_rd);
//         stripe::BillingPortalSession::create(&client, params)
//             .await
//             .map_err(to_anyhow)?
//     };
// }

async fn exists_workspace(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Json(WorkspaceId { id }): Json<WorkspaceId>,
) -> JsonResult<bool> {
    let mut tx = user_db.begin(&authed).await?;
    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM workspace WHERE workspace.id = $1)",
        id
    )
    .fetch_one(&mut *tx)
    .await?
    .unwrap_or(false);
    tx.commit().await?;
    Ok(Json(exists))
}

async fn list_workspaces(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
) -> JsonResult<Vec<Workspace>> {
    let mut tx = user_db.begin(&authed).await?;
    let workspaces = sqlx::query_as!(
        Workspace,
        "SELECT workspace.* FROM workspace, usr WHERE usr.workspace_id = workspace.id AND \
         usr.email = $1 AND deleted = false",
        authed.email
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(workspaces))
}

async fn get_settings(
    authed: ApiAuthed,
    Path(w_id): Path<String>,
    Extension(user_db): Extension<UserDB>,
) -> JsonResult<WorkspaceSettings> {
    let mut tx = user_db.begin(&authed).await?;
    let settings = sqlx::query_as!(
        WorkspaceSettings,
        "SELECT * FROM workspace_settings WHERE workspace_id = $1",
        &w_id
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| Error::InternalErr(format!("getting settings: {e}")))?;

    tx.commit().await?;
    Ok(Json(settings))
}

#[derive(Serialize)]
struct DeployTo {
    deploy_to: Option<String>,
}
async fn get_deploy_to(
    authed: ApiAuthed,
    Path(w_id): Path<String>,
    Extension(user_db): Extension<UserDB>,
) -> JsonResult<DeployTo> {
    let mut tx = user_db.begin(&authed).await?;
    let settings = sqlx::query_as!(
        DeployTo,
        "SELECT deploy_to FROM workspace_settings WHERE workspace_id = $1",
        &w_id
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| Error::InternalErr(format!("getting deploy_to: {e}")))?;

    tx.commit().await?;
    Ok(Json(settings))
}

async fn edit_slack_command(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    ApiAuthed { is_admin, username, .. }: ApiAuthed,
    Json(es): Json<EditCommandScript>,
) -> Result<String> {
    require_admin(is_admin, &username)?;
    let mut tx = db.begin().await?;
    sqlx::query!(
        "UPDATE workspace_settings SET slack_command_script = $1 WHERE workspace_id = $2",
        es.slack_command_script,
        &w_id
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed.username,
        "workspaces.edit_command_script",
        ActionKind::Update,
        &w_id,
        Some(&authed.email),
        Some(
            [(
                "script",
                es.slack_command_script
                    .unwrap_or("NO_SCRIPT".to_string())
                    .as_str(),
            )]
            .into(),
        ),
    )
    .await?;
    tx.commit().await?;

    Ok(format!("Edit command script {}", &w_id))
}

async fn run_slack_message_test_job(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,
    Path(w_id): Path<String>,
    Json(req): Json<RunSlackMessageTestJobRequest>,
) -> JsonResult<RunSlackMessageTestJobResponse> {
    let mut fake_result = Map::new();
    fake_result.insert("error".to_string(), json!(req.test_msg));
    fake_result.insert("success_result".to_string(), json!(req.test_msg));

    let mut extra_args = Map::new();
    extra_args.insert("channel".to_string(), json!(req.channel));
    extra_args.insert("slack".to_string(), json!(format!("$res:{WORKSPACE_SLACK_BOT_TOKEN_PATH}")));

    let tx: QueueTransaction<'_, _> = (rsmq.clone(), db.begin().await?).into();
    let (uuid, tx) = windmill_queue::handle_on_failure(
        &db, 
        tx,
        Uuid::parse_str("00000000-0000-0000-0000-000000000000")?,
        "slack_message_test", 
        "slack_message_test", 
        false, 
        w_id.as_str(),
        &format!("script/{}", req.hub_script_path.as_str()), 
        sqlx::types::Json(&fake_result),
        0, 
        Utc::now(), 
        Some(json!(extra_args)),
        authed.email.as_str(), 
        None, // Note: we could mark it as high priority to return result quickly to the user
    ).await?;
    tx.commit().await?;

    Ok(Json(RunSlackMessageTestJobResponse {
        job_uuid: uuid.to_string(),
    }))
}

#[cfg(feature = "enterprise")]
async fn edit_deploy_to(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    ApiAuthed { is_admin, username, .. }: ApiAuthed,
    Json(es): Json<EditDeployTo>,
) -> Result<String> {
    require_admin(is_admin, &username)?;

    let mut tx = db.begin().await?;
    sqlx::query!(
        "UPDATE workspace_settings SET deploy_to = $1 WHERE workspace_id = $2",
        es.deploy_to,
        &w_id
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed.username,
        "workspaces.edit_deploy_to",
        ActionKind::Update,
        &w_id,
        Some(&authed.email),
        Some(
            [(
                "script",
                es.deploy_to.unwrap_or("NO_DEPLOY_TO".to_string()).as_str(),
            )]
            .into(),
        ),
    )
    .await?;
    tx.commit().await?;

    Ok(format!("Edit deploy to for {}", &w_id))
}

#[cfg(not(feature = "enterprise"))]
async fn edit_deploy_to() -> Result<String> {
    return Err(Error::BadRequest(
        "Deploy to is only available on enterprise".to_string(),
    ));
}

const BANNED_DOMAINS: &str = include_str!("../banned_domains.txt");

async fn is_allowed_auto_domain(ApiAuthed { email, .. }: ApiAuthed) -> JsonResult<bool> {
    let domain = email.split('@').last().unwrap();
    return Ok(Json(!BANNED_DOMAINS.contains(domain)));
}

async fn edit_auto_invite(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    ApiAuthed { is_admin, email, username, .. }: ApiAuthed,
    Json(ea): Json<EditAutoInvite>,
) -> Result<String> {
    require_admin(is_admin, &username)?;
    let domain = email.split('@').last().unwrap();

    let mut tx = db.begin().await?;

    if let Some(operator) = ea.operator {
        if BANNED_DOMAINS.contains(domain) {
            return Err(Error::BadRequest(format!(
                "Domain {} is not allowed",
                domain
            )));
        }

        sqlx::query!(
            "UPDATE workspace_settings SET auto_invite_domain = $1, auto_invite_operator = $2 WHERE workspace_id = $3",
            domain,
            operator,
            &w_id
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            "INSERT INTO workspace_invite
        (workspace_id, email, is_admin, operator)
        SELECT $1::text, email, false, $3 FROM password WHERE email LIKE CONCAT('%', $2::text) AND NOT EXISTS (
            SELECT 1 FROM usr WHERE workspace_id = $1::text AND email = password.email
        )
        ON CONFLICT DO NOTHING",
            &w_id,
            &domain,
            operator
        )
        .execute(&mut *tx)
        .await?;
    } else {
        sqlx::query!(
            "UPDATE workspace_settings SET auto_invite_domain = NULL, auto_invite_operator = NULL WHERE workspace_id = $1",
            &w_id,
        )
        .execute(&mut *tx)
        .await?;
    }
    audit_log(
        &mut *tx,
        &authed.username,
        "workspaces.edit_auto_invite_domain",
        ActionKind::Update,
        &w_id,
        Some(&authed.email),
        Some([("operator", &format!("{:?}", ea.operator)[..])].into()),
    )
    .await?;
    tx.commit().await?;

    Ok(format!(
        "Edit auto-invite for workspace {} to {}",
        &w_id, domain
    ))
}

async fn edit_webhook(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    ApiAuthed { is_admin, username, .. }: ApiAuthed,
    Json(ew): Json<EditWebhook>,
) -> Result<String> {
    require_admin(is_admin, &username)?;

    let mut tx = db.begin().await?;

    if let Some(webhook) = &ew.webhook {
        sqlx::query!(
            "UPDATE workspace_settings SET webhook = $1 WHERE workspace_id = $2",
            webhook,
            &w_id
        )
        .execute(&mut *tx)
        .await?;
    } else {
        sqlx::query!(
            "UPDATE workspace_settings SET webhook = NULL WHERE workspace_id = $1",
            &w_id,
        )
        .execute(&mut *tx)
        .await?;
    }
    audit_log(
        &mut *tx,
        &authed.username,
        "workspaces.edit_webhook",
        ActionKind::Update,
        &w_id,
        Some(&authed.email),
        Some([("webhook", &format!("{:?}", ew.webhook)[..])].into()),
    )
    .await?;
    tx.commit().await?;

    Ok(format!("Edit webhook for workspace {}", &w_id))
}

async fn edit_copilot_config(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    ApiAuthed { is_admin, username, .. }: ApiAuthed,
    Json(eo): Json<EditCopilotConfig>,
) -> Result<String> {
    require_admin(is_admin, &username)?;

    let mut tx = db.begin().await?;

    if let Some(openai_resource_path) = &eo.openai_resource_path {
        sqlx::query!(
            "UPDATE workspace_settings SET openai_resource_path = $1, code_completion_enabled = $2 WHERE workspace_id = $3",
            openai_resource_path,
            eo.code_completion_enabled,
            &w_id
        )
        .execute(&mut *tx)
        .await?;
    } else {
        sqlx::query!(
            "UPDATE workspace_settings SET openai_resource_path = NULL, code_completion_enabled = $1 WHERE workspace_id = $2",
            eo.code_completion_enabled,
            &w_id,
        )
        .execute(&mut *tx)
        .await?;
    }
    audit_log(
        &mut *tx,
        &authed.username,
        "workspaces.edit_copilot_config",
        ActionKind::Update,
        &w_id,
        Some(&authed.email),
            Some([("openai_resource_path", &format!("{:?}", eo.openai_resource_path)[..]), ("code_completion_enabled", &format!("{:?}", eo.code_completion_enabled)[..])].into()),
    )
    .await?;
    tx.commit().await?;

    Ok(format!("Edit copilot config for workspace {}", &w_id))
}

#[derive(Serialize)]
struct CopilotInfo {
    pub exists_openai_resource_path: bool,
    pub code_completion_enabled: bool,
}
async fn get_copilot_info(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> JsonResult<CopilotInfo> {

    let mut tx = db.begin().await?;
    let record = sqlx::query!(
        "SELECT openai_resource_path, code_completion_enabled FROM workspace_settings WHERE workspace_id = $1",
        &w_id
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| Error::InternalErr(format!("getting openai_resource_path and code_completion_enabled: {e}")))?;
    tx.commit().await?;


    Ok(Json(CopilotInfo {
        exists_openai_resource_path: record.openai_resource_path.is_some(),
        code_completion_enabled: record.code_completion_enabled,
    }))
}


async fn edit_error_handler(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    ApiAuthed { is_admin, username, .. }: ApiAuthed,
    Json(ee): Json<EditErrorHandler>,
) -> Result<String> {
    require_admin(is_admin, &username)?;

    let mut tx = db.begin().await?;
    
    sqlx::query_as!(
        Group,
        "INSERT INTO group_ (workspace_id, name, summary, extra_perms) VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING",
        w_id,
        "error_handler",
        "The group the error handler acts on belhalf of",
        serde_json::json!({username_to_permissioned_as(&authed.username): true})
    )
    .execute(&mut *tx)
    .await?;

    if let Some(error_handler) = &ee.error_handler {
        sqlx::query!(
            "UPDATE workspace_settings SET error_handler = $1, error_handler_extra_args = $2, error_handler_muted_on_cancel = $3 WHERE workspace_id = $4",
            error_handler,
            ee.error_handler_extra_args,
            ee.error_handler_muted_on_cancel.unwrap_or(false),
            &w_id
        )
        .execute(&mut *tx)
        .await?;
    } else {
        sqlx::query!(
            "UPDATE workspace_settings SET error_handler = NULL, error_handler_extra_args = NULL WHERE workspace_id = $1",
            &w_id,
        )
        .execute(&mut *tx)
        .await?;
    }
    audit_log(
        &mut *tx,
        &authed.username,
        "workspaces.edit_error_handler",
        ActionKind::Update,
        &w_id,
        Some(&authed.email),
        Some([("error_handler", &format!("{:?}", ee.error_handler)[..])].into()),
    )
    .await?;
    tx.commit().await?;

    Ok(format!("Edit error_handler for workspace {}", &w_id))

}

async fn list_workspaces_as_super_admin(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Query(pagination): Query<Pagination>,
    ApiAuthed { email, .. }: ApiAuthed,
) -> JsonResult<Vec<Workspace>> {
    require_super_admin(&db, &email).await?;
    let (per_page, offset) = paginate(pagination);

    let mut tx = user_db.begin(&authed).await?;
    let workspaces = sqlx::query_as!(
        Workspace,
        "SELECT * FROM workspace LIMIT $1 OFFSET $2",
        per_page as i32,
        offset as i32
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(workspaces))
}

async fn user_workspaces(
    Extension(db): Extension<DB>,
    ApiAuthed { email, .. }: ApiAuthed,
) -> JsonResult<WorkspaceList> {
    let mut tx = db.begin().await?;
    let workspaces = sqlx::query_as!(
        UserWorkspace,
        "SELECT workspace.id, workspace.name, usr.username
     FROM workspace, usr WHERE usr.workspace_id = workspace.id AND usr.email = $1 AND deleted = \
         false",
        email
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(WorkspaceList { email, workspaces }))
}

async fn check_name_conflict<'c>(tx: &mut Transaction<'c, Postgres>, w_id: &str) -> Result<()> {
    let exists = sqlx::query_scalar!("SELECT EXISTS(SELECT 1 FROM workspace WHERE id = $1)", w_id)
        .fetch_one(&mut **tx)
        .await?
        .unwrap_or(false);
    if exists {
        return Err(windmill_common::error::Error::BadRequest(format!(
            "Workspace {} already exists",
            w_id
        )));
    }
    return Ok(());
}

lazy_static::lazy_static! {

    pub static ref CREATE_WORKSPACE_REQUIRE_SUPERADMIN: bool = std::env::var("CREATE_WORKSPACE_REQUIRE_SUPERADMIN").is_ok_and(|x| x.parse::<bool>().unwrap_or(true));

}

async fn _check_nb_of_workspaces(db: &DB) -> Result<()> {
    let nb_workspaces = sqlx::query_scalar!("SELECT COUNT(*) FROM workspace WHERE id != 'admins' AND deleted = false",)
        .fetch_one(db)
        .await?;
    if nb_workspaces.unwrap_or(0) >= 3 {
        return Err(Error::BadRequest(
            "You have reached the maximum number of workspaces (3 outside of default worskapce 'admins') without an enterprise license. Archive/delete another workspace to create a new one"
                .to_string(),
        ));
    }
    return Ok(());
}


async fn create_workspace(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Json(nw): Json<CreateWorkspace>,
) -> Result<String> {

    if *CREATE_WORKSPACE_REQUIRE_SUPERADMIN {
        require_super_admin(&db, &authed.email).await?;
    }

    #[cfg(not(feature = "enterprise"))]
    _check_nb_of_workspaces(&db).await?;

    let mut tx: Transaction<'_, Postgres> = db.begin().await?;

    check_name_conflict(&mut tx, &nw.id).await?;
    sqlx::query!(
        "INSERT INTO workspace
            (id, name, owner)
            VALUES ($1, $2, $3)",
        nw.id,
        nw.name,
        authed.email,
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "INSERT INTO workspace_settings
            (workspace_id)
            VALUES ($1)",
        nw.id
    )
    .execute(&mut *tx)
    .await?;
    let key = rd_string(64);
    sqlx::query!(
        "INSERT INTO workspace_key
            (workspace_id, kind, key)
            VALUES ($1, 'cloud', $2)",
        nw.id,
        &key
    )
    .execute(&mut *tx)
    .await?;

    // let mc = magic_crypt::new_magic_crypt!(key, 256);
    // sqlx::query!(
    //     "INSERT INTO variable
    //         (workspace_id, path, value, is_secret, description)
    //         VALUES ($1, 'g/all/pretty_secret', $2, true, 'This item is secret'),
    //             ($3, 'g/all/not_secret', $4, false, 'This item is not secret')",
    //     nw.id,
    //     crate::variables::encrypt(&mc, "pretty secret value"),
    //     nw.id,
    //     "finland does not actually exist",
    // )
    // .execute(&mut tx)
    // .await?;

    sqlx::query!(
        "INSERT INTO usr
            (workspace_id, email, username, is_admin)
            VALUES ($1, $2, $3, true)",
        nw.id,
        authed.email,
        nw.username,
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "INSERT INTO group_
            VALUES ($1, 'all', 'The group that always contains all users of this workspace')",
        nw.id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "INSERT INTO usr_to_group
            VALUES ($1, 'all', $2)",
        nw.id,
        nw.username
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "INSERT INTO folder (workspace_id, name, display_name, owners, extra_perms) VALUES ($1, 'app_themes', 'App Themes', ARRAY[]::TEXT[], '{\"g/all\": false}') ON CONFLICT DO NOTHING",
        nw.id,
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "INSERT INTO folder (workspace_id, name, display_name, owners, extra_perms) VALUES ($1, 'app_custom', 'App Custom Components', ARRAY[]::TEXT[], '{\"g/all\": false}') ON CONFLICT DO NOTHING",
        nw.id,
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "INSERT INTO folder (workspace_id, name, display_name, owners, extra_perms) VALUES ($1, 'app_groups', 'App Groups', ARRAY[]::TEXT[], '{\"g/all\": false}') ON CONFLICT DO NOTHING",
        nw.id,
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "INSERT INTO resource (workspace_id, path, value, description, resource_type) VALUES ($1, 'f/app_themes/theme_0', '{\"name\": \"Default Theme\", \"value\": \"\"}', 'The default app theme', 'app_theme') ON CONFLICT DO NOTHING",
        nw.id,
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed.username,
        "workspaces.create",
        ActionKind::Create,
        &nw.id,
        Some(nw.name.as_str()),
        None,
    )
    .await?;
    tx.commit().await?;
    Ok(format!("Created workspace {}", &nw.id))
}

async fn edit_workspace(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    ApiAuthed { is_admin, username, .. }: ApiAuthed,
    Json(ew): Json<EditWorkspace>,
) -> Result<String> {
    require_admin(is_admin, &username)?;
    let mut tx = db.begin().await?;
    sqlx::query!(
        "UPDATE workspace SET name = $1, owner = $2 WHERE id = $3",
        ew.name,
        ew.owner,
        &w_id
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed.username,
        "workspaces.update",
        ActionKind::Update,
        &w_id,
        Some(&authed.email),
        None,
    )
    .await?;
    tx.commit().await?;

    Ok(format!("Updated workspace {}", &w_id))
}

async fn archive_workspace(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    ApiAuthed { is_admin, username, email, .. }: ApiAuthed,
) -> Result<String> {
    require_admin(is_admin, &username)?;
    let mut tx = db.begin().await?;
    sqlx::query!("UPDATE workspace SET deleted = true WHERE id = $1", &w_id)
        .execute(&mut *tx)
        .await?;

    audit_log(
        &mut *tx,
        &username,
        "workspaces.archive",
        ActionKind::Update,
        &w_id,
        Some(&email),
        None,
    )
    .await?;
    tx.commit().await?;

    Ok(format!("Archived workspace {}", &w_id))
}

async fn unarchive_workspace(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    ApiAuthed { is_admin, username, email, .. }: ApiAuthed,
) -> Result<String> {
    require_admin(is_admin, &username)?;
    let mut tx = db.begin().await?;
    sqlx::query!("UPDATE workspace SET deleted = false WHERE id = $1", &w_id)
        .execute(&mut *tx)
        .await?;

    audit_log(
        &mut *tx,
        &username,
        "workspaces.unarchive",
        ActionKind::Update,
        &w_id,
        Some(&email),
        None,
    )
    .await?;
    tx.commit().await?;

    Ok(format!("Unarchived workspace {}", &w_id))
}

async fn delete_workspace(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    ApiAuthed { username, email, .. }: ApiAuthed,
) -> Result<String> {
    let w_id = match w_id.as_str() {
        "starter" => Err(Error::BadRequest(
            "starter workspace cannot be deleted".to_string(),
        )),
        "admins" => Err(Error::BadRequest(
            "admins workspace cannot be deleted".to_string(),
        )),
        _ => Ok(w_id),
    }?;
    let mut tx = db.begin().await?;
    require_super_admin(&db, &email).await?;

    sqlx::query!("DELETE FROM script WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query!("DELETE FROM flow WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query!("DELETE FROM app WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query!("DELETE FROM variable WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query!("DELETE FROM resource WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query!("DELETE FROM schedule WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query!("DELETE FROM completed_job WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query!("DELETE FROM usr WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query!("DELETE FROM resource_type WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query!(
        "DELETE FROM workspace_invite WHERE workspace_id = $1",
        &w_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!("DELETE FROM usr_to_group WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query!("DELETE FROM group_ WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query!("DELETE FROM folder WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query!("DELETE FROM workspace_key WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query!(
        "DELETE FROM workspace_settings WHERE workspace_id = $1",
        &w_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!("DELETE FROM workspace WHERE id = $1", &w_id)
        .execute(&mut *tx)
        .await?;

    audit_log(
        &mut *tx,
        &username,
        "workspaces.delete",
        ActionKind::Delete,
        &w_id,
        Some(&email),
        None,
    )
    .await?;
    tx.commit().await?;

    Ok(format!("Deleted workspace {}", &w_id))
}

pub async fn invite_user_to_all_auto_invite_worspaces(db: &DB, email: &str) -> Result<()> {
    let mut tx = db.begin().await?;
    let domain = email.split('@').last().unwrap();
    let workspaces = sqlx::query!(
        "SELECT workspace_id, auto_invite_operator FROM workspace_settings WHERE auto_invite_domain = $1",
        domain
    )
    .fetch_all(&mut *tx)
    .await?;
    for r in workspaces {
        sqlx::query!(
            "INSERT INTO workspace_invite
                (workspace_id, email, is_admin, operator)
                VALUES ($1, $2, false, $3)
                ON CONFLICT DO NOTHING",
            r.workspace_id,
            email,
            r.auto_invite_operator
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

async fn invite_user(
    ApiAuthed { username, is_admin, .. }: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(webhook): Extension<WebhookShared>,
    Path(w_id): Path<String>,
    Json(mut nu): Json<NewWorkspaceInvite>,
) -> Result<(StatusCode, String)> {
    require_admin(is_admin, &username)?;

    nu.email = nu.email.to_lowercase();

    let mut tx = db.begin().await?;

    sqlx::query!(
        "INSERT INTO workspace_invite
            (workspace_id, email, is_admin, operator)
            VALUES ($1, $2, $3, $4)",
        &w_id,
        nu.email,
        nu.is_admin,
        nu.operator
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    send_email_if_possible(
        &format!("Invited to Windmill's workspace: {w_id}"),
        &format!(
            "You have been granted access to Windmill's workspace {w_id}

If you do not have an account on {}, login with SSO or ask an admin to create an account for you.",
            BASE_URL.read().await.clone()
        ),
        &nu.email,
    );
    
    webhook.send_instance_event(InstanceEvent::UserInvitedWorkspace {
        email: nu.email.clone(),
        workspace: w_id,
    });

    Ok((
        StatusCode::CREATED,
        format!("user with email {} invited", nu.email),
    ))
}

async fn add_user(
    ApiAuthed { username, email, is_admin, .. }: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(webhook): Extension<WebhookShared>,
    Path(w_id): Path<String>,
    Json(mut nu): Json<NewWorkspaceUser>,
) -> Result<(StatusCode, String)> {
    require_admin(is_admin, &username)?;
    nu.email = nu.email.to_lowercase();

    let mut tx = db.begin().await?;
    if !VALID_USERNAME.is_match(&nu.username) {
        return Err(windmill_common::error::Error::BadRequest(format!(
            "Usermame can only contain alphanumeric characters and underscores and must start with a letter"
        )));
    }

    sqlx::query!(
        "INSERT INTO usr
            (workspace_id, email, username, is_admin, operator)
            VALUES ($1, $2, $3, $4, $5)",
        &w_id,
        nu.email,
        nu.username,
        nu.is_admin,
        nu.operator
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query_as!(
        Group,
        "INSERT INTO usr_to_group (workspace_id, usr, group_) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
        &w_id,
        nu.username,
        "all",
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    send_email_if_possible(
        &format!("Added to Windmill's workspace: {w_id}"),
        &format!(
            "You have been granted access to Windmill's workspace {w_id} by {email}

If you do not have an account on {}, login with SSO or ask an admin to create an account for you.",
            BASE_URL.read().await.clone()
        ),
        &nu.email,
    );

    webhook.send_instance_event(InstanceEvent::UserAddedWorkspace {
        workspace: w_id.clone(),
        email: nu.email.clone(),
    });

    Ok((
        StatusCode::CREATED,
        format!("user with email {} added", nu.email),
    ))
}

async fn delete_invite(
    ApiAuthed { username, is_admin, .. }: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(nu): Json<NewWorkspaceInvite>,
) -> Result<(StatusCode, String)> {
    require_admin(is_admin, &username)?;

    let mut tx = db.begin().await?;

    sqlx::query!(
        "DELETE FROM workspace_invite WHERE
        workspace_id = $1 AND email = $2 AND is_admin = $3 AND operator = $4",
        &w_id,
        nu.email,
        nu.is_admin,
        nu.operator
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok((
        StatusCode::CREATED,
        format!("invite to email {} deleted", nu.email),
    ))
}

async fn exists_username(
    Extension(db): Extension<DB>,
    Json(vu): Json<ValidateUsername>,
) -> Result<String> {
    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM usr WHERE username = $1 AND workspace_id = $2)",
        vu.username,
        vu.id
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(true);

    if exists {
        return Err(Error::BadRequest("username already taken".to_string()));
    }

    Ok("valid username".to_string())
}

#[derive(Serialize)]
struct ScriptMetadata {
    summary: String,
    description: String,
    schema: Option<Schema>,
    is_template: bool,
    lock: Vec<String>,
    kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    envs: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    concurrent_limit: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    concurrency_time_window_s: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cache_ttl: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dedicated_worker: Option<bool>,
    #[serde(skip_serializing_if = "is_none_or_false")]
    ws_error_handler_muted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    priority: Option<i16>,    
    #[serde(skip_serializing_if = "Option::is_none")]
    tag: Option<String>,    
}

pub fn is_none_or_false(val: &Option<bool>) -> bool {
    match val {
        Some(val) => !val,
        None => true,
    }
}

enum ArchiveImpl {
    Zip(async_zip::write::ZipFileWriter<File>),
    Tar(tokio_tar::Builder<File>),
}

impl ArchiveImpl {
    async fn write_to_archive(&mut self, content: &str, path: &str) -> Result<()> {
        match self {
            ArchiveImpl::Tar(t) => {
                let bytes = content.as_bytes();
                let mut header = tokio_tar::Header::new_gnu();
                header.set_size(bytes.len() as u64);
                header.set_mtime(0);
                header.set_uid(0);
                header.set_gid(0);
                header.set_mode(0o777);
                header.set_cksum();
                t.append_data(&mut header, path, bytes).await?;
            }
            ArchiveImpl::Zip(z) => {
                let header = async_zip::ZipEntryBuilder::new(
                    path.to_owned(),
                    async_zip::Compression::Deflate,
                )
                .last_modification_date(Default::default())
                .unix_permissions(0o777)
                .build();
                z.write_entry_whole(header, content.as_bytes())
                    .await
                    .map_err(to_anyhow)?;
            }
        }
        Ok(())
    }
    async fn finish(self) -> Result<()> {
        match self {
            ArchiveImpl::Tar(t) => t.into_inner().await?,
            ArchiveImpl::Zip(z) => z.close().await.map_err(to_anyhow)?,
        }
        .sync_all()
        .await?;

        Ok(())
    }
}

#[derive(Deserialize)]
struct ArchiveQueryParams {
    archive_type: Option<String>,
    plain_secret: Option<bool>,
    plain_secrets: Option<bool>,
    skip_secrets: Option<bool>,
    skip_variables: Option<bool>,
    skip_resources: Option<bool>,
    include_schedules: Option<bool>,
}

#[inline]
pub fn to_string_without_metadata<T>(value: &T, preserve_extra_perms: bool) -> Result<String>
where
    T: ?Sized + Serialize,
{
    let value = serde_json::to_value(value).map_err(to_anyhow)?;
    value
        .as_object()
        .map(|obj| {
            let mut obj = obj.clone();
            for key in [
                "workspace_id",
                "path",
                "name",
                "versions",
                "id",
                "created_at",
                "updated_at",
                "created_by",
                "updated_by",
                "edited_at",
                "edited_by",
                "archived",
                "has_draft",
                "draft_only",
                "error"
            ] {
                if obj.contains_key(key) {
                    obj.remove(key);
                }
            }

            if !preserve_extra_perms && obj.contains_key("extra_perms") {
                obj.remove("extra_perms");
            }

            serde_json::to_string_pretty(&obj).ok()
        })
        .flatten()
        .ok_or_else(|| Error::BadRequest("Impossible to serialize value".to_string()))
}

async fn tarball_workspace(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Query(ArchiveQueryParams {
        archive_type,
        plain_secret,
        plain_secrets,
        skip_resources,
        skip_secrets,
        skip_variables,
        include_schedules,
    }): Query<ArchiveQueryParams>,
) -> Result<([(headers::HeaderName, String); 2], impl IntoResponse)> {
    require_admin(authed.is_admin, &authed.username)?;

    let tmp_dir = TempDir::new_in("/tmp/windmill/")?;

    let name = match archive_type.as_deref() {
        Some("tar") | None => Ok(format!("windmill-{w_id}.tar")),
        Some("zip") => Ok(format!("windmill-{w_id}.zip")),
        Some(t) => Err(Error::BadRequest(format!("Invalid Archive Type {t}"))),
    }?;
    let file_path = tmp_dir.path().join(&name);
    let file = File::create(&file_path).await?;
    let mut archive = match archive_type.as_deref() {
        Some("tar") | None => Ok(ArchiveImpl::Tar(tokio_tar::Builder::new(file))),
        Some("zip") => Ok(ArchiveImpl::Zip(async_zip::write::ZipFileWriter::new(file))),
        Some(t) => Err(Error::BadRequest(format!("Invalid Archive Type {t}"))),
    }?;
    {
        let folders = sqlx::query_as::<_, Folder>("SELECT * FROM folder WHERE workspace_id = $1")
            .bind(&w_id)
            .fetch_all(&db)
            .await?;

        for folder in folders {
            archive
                .write_to_archive(
                    &to_string_without_metadata(&folder, true).unwrap(),
                    &format!("f/{}/folder.meta.json", folder.name),
                )
                .await?;
        }
    }

    {
        let scripts = sqlx::query_as::<_, Script>(
            "SELECT * FROM script as o WHERE workspace_id = $1 AND archived = false
            AND created_at = (select max(created_at) from script where path = o.path AND \
             workspace_id = $1)",
        )
        .bind(&w_id)
        .fetch_all(&db)
        .await?;

        for script in scripts {
            let ext = match script.language {
                ScriptLang::Python3 => "py",
                ScriptLang::Deno => "ts",
                ScriptLang::Go => "go",
                ScriptLang::Bash => "sh",
                ScriptLang::Powershell => "ps1",
                ScriptLang::Postgresql => "pg.sql",
                ScriptLang::Mysql => "my.sql",
                ScriptLang::Bigquery => "bq.sql",
                ScriptLang::Snowflake => "sf.sql",
                ScriptLang::Mssql => "ms.sql",
                ScriptLang::Graphql => "gql",
                ScriptLang::Nativets => "fetch.ts",
                ScriptLang::Bun => "bun.ts",
            };
            archive
                .write_to_archive(&script.content, &format!("{}.{}", script.path, ext))
                .await?;

            let lock = script
                .lock
                .unwrap_or_else(|| "".to_string())
                .lines()
                .map(|x| x.to_string())
                .collect();
            let metadata = ScriptMetadata {
                summary: script.summary,
                description: script.description,
                schema: script.schema,
                is_template: script.is_template,
                kind: script.kind.to_string(),
                lock,
                envs: script.envs,
                concurrent_limit: script.concurrent_limit,
                concurrency_time_window_s: script.concurrency_time_window_s,
                cache_ttl: script.cache_ttl,
                dedicated_worker: script.dedicated_worker,
                ws_error_handler_muted: script.ws_error_handler_muted,
                priority: script.priority,
                tag: script.tag,
                
            };
            let metadata_str = serde_json::to_string_pretty(&metadata).unwrap();
            archive
                .write_to_archive(&metadata_str, &format!("{}.script.json", script.path))
                .await?;
        }
    }

    if !skip_resources.unwrap_or(false) {
        let resources = sqlx::query_as!(
            Resource,
            "SELECT * FROM resource WHERE workspace_id = $1 AND resource_type != 'state' AND resource_type != 'cache'",
            &w_id
        )
        .fetch_all(&db)
        .await?;

        for resource in resources {
            let resource_str = &to_string_without_metadata(&resource, false).unwrap();
            archive
                .write_to_archive(&resource_str, &format!("{}.resource.json", resource.path))
                .await?;
        }
    }

    if !skip_resources.unwrap_or(false) {
        let resource_types = sqlx::query_as!(
            ResourceType,
            "SELECT * FROM resource_type WHERE workspace_id = $1",
            &w_id
        )
        .fetch_all(&db)
        .await?;

        for resource_type in resource_types {
            let resource_str = &to_string_without_metadata(&resource_type, false).unwrap();
            archive
                .write_to_archive(
                    &resource_str,
                    &format!("{}.resource-type.json", resource_type.name),
                )
                .await?;
        }
    }

    {
        let flows = sqlx::query_as::<_, Flow>(
            "SELECT * FROM flow WHERE workspace_id = $1 AND archived = false",
        )
        .bind(&w_id)
        .fetch_all(&db)
        .await?;

        for flow in flows {
            let flow_str = &to_string_without_metadata(&flow, false).unwrap();
            archive
                .write_to_archive(&flow_str, &format!("{}.flow.json", flow.path))
                .await?;
        }
    }

    if !skip_variables.unwrap_or(false) {
        let variables = sqlx::query_as::<_, ExportableListableVariable>(
            if !skip_secrets.unwrap_or(false) { 
                "SELECT *, false as is_expired FROM variable WHERE workspace_id = $1" 
            } else {
                "SELECT *, false as is_expired FROM variable WHERE workspace_id = $1 AND is_secret = false" 
            }
        )
        .bind(&w_id)
        .fetch_all(&db)
        .await?;

        let mc = build_crypt(&mut db.begin().await?, &w_id).await?;

        for mut var in variables {
            if plain_secret.or(plain_secrets).unwrap_or(false)
                && var.value.is_some()
                && var.is_secret
            {
                var.value = Some(
                    mc.decrypt_base64_to_string(var.value.unwrap())
                        .map_err(|e| Error::InternalErr(e.to_string()))?,
                );
            }
            let var_str = &to_string_without_metadata(&var, false).unwrap();
            archive
                .write_to_archive(&var_str, &format!("{}.variable.json", var.path))
                .await?;
        }
    }

    {
        let apps = sqlx::query_as!(
            AppWithLastVersion,
            "SELECT app.id, app.path, app.summary, app.versions, app.policy,
            app.extra_perms, app_version.value, 
            app_version.created_at, app_version.created_by from app, app_version 
            WHERE app.workspace_id = $1 AND app_version.id = app.versions[array_upper(app.versions, 1)]",
            &w_id
        )
        .fetch_all(&db)
        .await?;

        for app in apps {
            let app_str = &to_string_without_metadata(&app, false).unwrap();
            archive
                .write_to_archive(&app_str, &format!("{}.app.json", app.path))
                .await?;
        }
    }

    if include_schedules.unwrap_or(false) {
        let schedules = sqlx::query_as!(
            Schedule,
            "SELECT * FROM schedule
            WHERE workspace_id = $1",
            &w_id
        )
        .fetch_all(&db)
        .await?;

        for schedule in schedules {
            let app_str = &to_string_without_metadata(&schedule, false).unwrap();
            archive
                .write_to_archive(&app_str, &format!("{}.schedule.json", schedule.path))
                .await?;
        }
    }

    archive.finish().await?;

    let file = tokio::fs::File::open(&file_path).await?;

    let stream = ReaderStream::new(file);
    let body = StreamBody::new(stream);

    let headers = [
        (header::CONTENT_TYPE, "application/x-tar".to_string()),
        (
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{name}\""),
        ),
    ];
    Ok((headers, body))
}
