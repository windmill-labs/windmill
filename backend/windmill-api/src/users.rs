/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

// Re-export everything from windmill-api-users
pub use windmill_api_users::users::*;

use std::collections::HashMap;
use std::sync::Arc;

use crate::db::ApiAuthed;
use crate::secret_backend_ext::rename_vault_secrets_with_prefix;
use argon2::Argon2;
use axum::{
    extract::{Extension, Path},
    routing::{get, post},
    Json, Router,
};
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use windmill_api_auth::require_super_admin;
use windmill_audit::audit_oss::audit_log;
use windmill_audit::ActionKind;
use windmill_common::audit::AuditAuthor;
use windmill_common::usernames::VALID_USERNAME;
use windmill_common::utils::require_admin;
use windmill_common::{
    error::{self, Error, JsonResult, Result},
    DB,
};
use windmill_git_sync::handle_deployment_metadata;

/// Wraps the subcrate's workspaced_service with offboarding routes.
pub fn workspaced_service() -> Router {
    windmill_api_users::users::workspaced_service()
        .route("/offboard_preview/{user}", get(offboard_preview))
        .route("/offboard/{user}", post(offboard_workspace_user))
}

/// Wraps the subcrate's global_service with routes that depend on windmill-api internals.
pub fn global_service() -> Router {
    windmill_api_users::users::global_service()
        .route("/setpassword", post(set_password))
        .route("/set_password_of/{user}", post(set_password_of_user))
        .route("/create", post(create_user))
        .route("/rename/{user}", post(rename_user))
        .route("/onboarding", post(submit_onboarding_data))
        .route("/offboard_preview/{user}", get(global_offboard_preview))
        .route("/offboard/{user}", post(offboard_global_user))
}

/// Wraps the subcrate's make_unauthed_service with routes that depend on windmill-api internals.
pub fn make_unauthed_service() -> Router {
    windmill_api_users::users::make_unauthed_service()
        .route("/reset_password", post(reset_password))
}

async fn create_user(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(webhook): Extension<windmill_common::webhook::WebhookShared>,
    Extension(argon2): Extension<Arc<Argon2<'_>>>,
    Json(nu): Json<NewUser>,
) -> Result<(StatusCode, String)> {
    crate::users_oss::create_user(authed, db, webhook, argon2, nu).await
}

async fn submit_onboarding_data(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Json(data): Json<crate::users_oss::OnboardingData>,
) -> Result<String> {
    crate::users_oss::submit_onboarding_data(authed, Extension(db), Json(data)).await
}

async fn set_password(
    Extension(db): Extension<DB>,
    Extension(argon2): Extension<Arc<Argon2<'_>>>,
    authed: ApiAuthed,
    Json(ep): Json<EditPassword>,
) -> Result<String> {
    let email = authed.email.clone();
    crate::users_oss::set_password(db, argon2, authed, &email, ep).await
}

async fn set_password_of_user(
    Extension(db): Extension<DB>,
    Extension(argon2): Extension<Arc<Argon2<'_>>>,
    Path(email): Path<String>,
    authed: ApiAuthed,
    Json(ep): Json<EditPassword>,
) -> Result<String> {
    require_super_admin(&db, &authed.email).await?;
    crate::users_oss::set_password(db, argon2, authed, &email, ep).await
}

#[derive(Deserialize)]
struct RenameUser {
    new_username: String,
}

async fn rename_user(
    authed: ApiAuthed,
    Path(user_email): Path<String>,
    Extension(db): Extension<DB>,
    Json(ru): Json<RenameUser>,
) -> Result<String> {
    require_super_admin(&db, &authed.email).await?;

    let mut tx = db.begin().await?;

    let username_conflict = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM usr WHERE username = $1 and email != $2 UNION SELECT 1 FROM password WHERE username = $1 UNION SELECT 1 FROM pending_user WHERE username = $1)",
        &ru.new_username,
        &user_email
    )
    .fetch_one(&mut *tx)
    .await?
    .unwrap_or(false);

    if username_conflict {
        return Err(Error::BadRequest(format!(
            "username {} already used by another user",
            &ru.new_username
        )));
    }

    if !VALID_USERNAME.is_match(&ru.new_username) {
        return Err(windmill_common::error::Error::BadRequest(format!(
            "Usermame can only contain alphanumeric characters and underscores and must start with a letter"
        )));
    }

    sqlx::query!(
        "UPDATE password SET username = $1 WHERE email = $2",
        ru.new_username,
        user_email
    )
    .execute(&mut *tx)
    .await?;

    let workspace_usernames = sqlx::query!(
        "SELECT workspace_id, username FROM usr WHERE email = $1",
        &user_email
    )
    .fetch_all(&mut *tx)
    .await?;

    for w_u in workspace_usernames {
        if ru.new_username == w_u.username {
            continue;
        }
        update_username_in_workpsace(
            &mut tx,
            &db,
            &user_email,
            &w_u.username,
            &ru.new_username,
            &w_u.workspace_id,
        )
        .await?;
    }

    audit_log(
        &mut *tx,
        &authed,
        "users.rename",
        ActionKind::Update,
        "global",
        Some(&user_email),
        None,
    )
    .await?;
    tx.commit().await?;
    Ok(format!(
        "updated username of user {} to {}",
        &user_email, &ru.new_username
    ))
}

// ---- Offboarding types ----

#[derive(Serialize, Deserialize)]
struct OffboardPreview {
    scripts: i64,
    flows: i64,
    apps: i64,
    resources: i64,
    variables: i64,
    schedules: i64,
    triggers: i64,
    tokens: i64,
}

#[derive(Deserialize)]
struct OffboardRequest {
    reassign_to: String,
    /// Required when reassign_to is a folder (f/...). The username whose identity
    /// will be used as permissioned_as for schedules and triggers.
    new_operator: Option<String>,
    #[serde(default = "default_true")]
    delete_user: bool,
    reassign_tokens_to: Option<String>,
}

fn default_true() -> bool {
    true
}

#[derive(Serialize)]
struct OffboardResponse {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    conflicts: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    summary: Option<OffboardSummary>,
}

#[derive(Serialize)]
struct OffboardSummary {
    scripts_reassigned: i64,
    flows_reassigned: i64,
    apps_reassigned: i64,
    resources_reassigned: i64,
    variables_reassigned: i64,
    schedules_reassigned: i64,
    triggers_reassigned: i64,
    tokens_handled: i64,
    drafts_deleted: i64,
}

#[derive(Serialize)]
struct GlobalOffboardPreview {
    workspaces: Vec<WorkspaceOffboardPreview>,
}

#[derive(Serialize)]
struct WorkspaceOffboardPreview {
    workspace_id: String,
    username: String,
    preview: OffboardPreview,
}

#[derive(Deserialize)]
struct GlobalOffboardRequest {
    #[serde(default)]
    reassignments: HashMap<String, WorkspaceReassignment>,
    #[serde(default = "default_true")]
    delete_user: bool,
}

#[derive(Deserialize)]
struct WorkspaceReassignment {
    reassign_to: String,
    new_operator: Option<String>,
    reassign_tokens_to: Option<String>,
}

// ---- Offboarding preview helpers ----

async fn get_offboard_preview(
    db: impl sqlx::PgExecutor<'_> + Copy,
    w_id: &str,
    username: &str,
) -> Result<OffboardPreview> {
    let user_prefix = format!("u/{}/%", username);
    let user_owner = format!("u/{}", username);

    let scripts = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM script WHERE path LIKE $1 AND workspace_id = $2 AND NOT archived AND NOT deleted",
        &user_prefix, w_id
    ).fetch_one(db).await?.unwrap_or(0);

    let flows = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM flow WHERE path LIKE $1 AND workspace_id = $2 AND NOT archived",
        &user_prefix,
        w_id
    )
    .fetch_one(db)
    .await?
    .unwrap_or(0);

    let apps = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM app WHERE path LIKE $1 AND workspace_id = $2",
        &user_prefix,
        w_id
    )
    .fetch_one(db)
    .await?
    .unwrap_or(0);

    let resources = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM resource WHERE path LIKE $1 AND workspace_id = $2",
        &user_prefix,
        w_id
    )
    .fetch_one(db)
    .await?
    .unwrap_or(0);

    let variables = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM variable WHERE path LIKE $1 AND workspace_id = $2",
        &user_prefix,
        w_id
    )
    .fetch_one(db)
    .await?
    .unwrap_or(0);

    let schedules = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM schedule WHERE path LIKE $1 AND workspace_id = $2",
        &user_prefix,
        w_id
    )
    .fetch_one(db)
    .await?
    .unwrap_or(0);

    // Count all trigger types (excluding native_trigger which has no user path)
    let triggers = sqlx::query_scalar!(
        r#"SELECT COALESCE(SUM(cnt), 0) as "count!: i64" FROM (
            SELECT COUNT(*) as cnt FROM http_trigger WHERE path LIKE $1 AND workspace_id = $2
            UNION ALL SELECT COUNT(*) FROM websocket_trigger WHERE path LIKE $1 AND workspace_id = $2
            UNION ALL SELECT COUNT(*) FROM kafka_trigger WHERE path LIKE $1 AND workspace_id = $2
            UNION ALL SELECT COUNT(*) FROM postgres_trigger WHERE path LIKE $1 AND workspace_id = $2
            UNION ALL SELECT COUNT(*) FROM mqtt_trigger WHERE path LIKE $1 AND workspace_id = $2
            UNION ALL SELECT COUNT(*) FROM nats_trigger WHERE path LIKE $1 AND workspace_id = $2
            UNION ALL SELECT COUNT(*) FROM sqs_trigger WHERE path LIKE $1 AND workspace_id = $2
            UNION ALL SELECT COUNT(*) FROM gcp_trigger WHERE path LIKE $1 AND workspace_id = $2
            UNION ALL SELECT COUNT(*) FROM email_trigger WHERE path LIKE $1 AND workspace_id = $2
        ) t"#,
        &user_prefix, w_id
    ).fetch_one(db).await?;

    let tokens = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM token WHERE owner = $1 AND workspace_id = $2",
        &user_owner,
        w_id
    )
    .fetch_one(db)
    .await?
    .unwrap_or(0);

    Ok(OffboardPreview { scripts, flows, apps, resources, variables, schedules, triggers, tokens })
}

// ---- Workspace-level offboarding endpoints ----

async fn offboard_preview(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, username)): Path<(String, String)>,
) -> JsonResult<OffboardPreview> {
    require_admin(authed.is_admin, &authed.username)?;
    let preview = get_offboard_preview(&db, &w_id, &username).await?;
    Ok(Json(preview))
}

async fn offboard_workspace_user(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, username)): Path<(String, String)>,
    Json(req): Json<OffboardRequest>,
) -> JsonResult<OffboardResponse> {
    require_admin(authed.is_admin, &authed.username)?;

    // Validate reassign_to format
    let (target_kind, target_name) = parse_reassign_target(&req.reassign_to)?;

    // Validate target exists
    validate_target(&db, &w_id, target_kind, target_name).await?;

    // Resolve permissioned_as: when target is a user, use that user; when folder, require new_operator
    let new_permissioned_as = resolve_new_permissioned_as(
        target_kind,
        &req.reassign_to,
        req.new_operator.as_deref(),
        &db,
        &w_id,
    )
    .await?;

    // Validate token reassignment target if specified
    if let Some(ref token_target) = req.reassign_tokens_to {
        validate_token_target(&db, &w_id, token_target).await?;
    }

    // Check for path conflicts
    let conflicts = check_path_conflicts(&db, &w_id, &username, &req.reassign_to).await?;
    if !conflicts.is_empty() {
        return Ok(Json(OffboardResponse { conflicts, summary: None }));
    }

    let email = sqlx::query_scalar!(
        "SELECT email FROM usr WHERE username = $1 AND workspace_id = $2",
        &username,
        &w_id
    )
    .fetch_optional(&db)
    .await?
    .ok_or_else(|| {
        Error::NotFound(format!(
            "user {} not found in workspace {}",
            &username, &w_id
        ))
    })?;

    let mut tx = db.begin().await?;

    let summary = offboard_user_from_workspace(
        &mut tx,
        &db,
        &w_id,
        &username,
        &req.reassign_to,
        &new_permissioned_as,
        req.reassign_tokens_to.as_deref(),
    )
    .await?;

    if req.delete_user {
        delete_workspace_user_internal(&w_id, &username, &email, &mut tx, Some(&authed)).await?;
    } else {
        // Still audit the reassignment
        audit_log(
            &mut *tx,
            &authed,
            "users.offboard_reassign",
            ActionKind::Update,
            &w_id,
            Some(&username),
            Some([("reassign_to", req.reassign_to.as_str())].into()),
        )
        .await?;
    }

    tx.commit().await?;

    if req.delete_user {
        handle_deployment_metadata(
            &authed.email,
            &authed.username,
            &db,
            &w_id,
            windmill_git_sync::DeployedObject::User { email: email.clone() },
            Some(format!(
                "Offboarded user '{}' from workspace (objects reassigned to {})",
                &email, &req.reassign_to
            )),
            true,
            None,
        )
        .await?;
    }

    Ok(Json(OffboardResponse {
        conflicts: vec![],
        summary: Some(summary),
    }))
}

// ---- Instance-level offboarding endpoints ----

async fn global_offboard_preview(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(email): Path<String>,
) -> JsonResult<GlobalOffboardPreview> {
    require_super_admin(&db, &authed.email).await?;

    let workspaces = sqlx::query!(
        "SELECT workspace_id, username FROM usr WHERE email = $1",
        &email
    )
    .fetch_all(&db)
    .await?;

    let mut previews = Vec::new();
    for w in workspaces {
        let preview = get_offboard_preview(&db, &w.workspace_id, &w.username).await?;
        previews.push(WorkspaceOffboardPreview {
            workspace_id: w.workspace_id,
            username: w.username,
            preview,
        });
    }

    Ok(Json(GlobalOffboardPreview { workspaces: previews }))
}

async fn offboard_global_user(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(email): Path<String>,
    Json(req): Json<GlobalOffboardRequest>,
) -> Result<String> {
    require_super_admin(&db, &authed.email).await?;

    let workspaces = sqlx::query!(
        "SELECT workspace_id, username FROM usr WHERE email = $1",
        &email
    )
    .fetch_all(&db)
    .await?;

    // Validate all targets and resolve permissioned_as upfront
    let mut resolved_permissioned_as: HashMap<String, String> = HashMap::new();
    for (w_id, reassignment) in &req.reassignments {
        let (target_kind, target_name) = parse_reassign_target(&reassignment.reassign_to)?;
        validate_target(&db, w_id, target_kind, target_name).await?;
        let perm_as = resolve_new_permissioned_as(
            target_kind,
            &reassignment.reassign_to,
            reassignment.new_operator.as_deref(),
            &db,
            w_id,
        )
        .await?;
        resolved_permissioned_as.insert(w_id.clone(), perm_as);
        if let Some(ref token_target) = reassignment.reassign_tokens_to {
            validate_token_target(&db, w_id, token_target).await?;
        }
    }

    // Check for conflicts in all workspaces
    for ws in &workspaces {
        if let Some(reassignment) = req.reassignments.get(&ws.workspace_id) {
            let conflicts = check_path_conflicts(
                &db,
                &ws.workspace_id,
                &ws.username,
                &reassignment.reassign_to,
            )
            .await?;
            if !conflicts.is_empty() {
                return Err(Error::BadRequest(format!(
                    "Path conflicts in workspace '{}': {}",
                    &ws.workspace_id,
                    conflicts.join(", ")
                )));
            }
        }
    }

    let mut tx = db.begin().await?;

    // Process each workspace
    for ws in &workspaces {
        if let Some(reassignment) = req.reassignments.get(&ws.workspace_id) {
            let perm_as = resolved_permissioned_as
                .get(&ws.workspace_id)
                .ok_or_else(|| {
                    Error::InternalErr("missing resolved permissioned_as".to_string())
                })?;
            offboard_user_from_workspace(
                &mut tx,
                &db,
                &ws.workspace_id,
                &ws.username,
                &reassignment.reassign_to,
                perm_as,
                reassignment.reassign_tokens_to.as_deref(),
            )
            .await?;
        }

        if req.delete_user {
            delete_workspace_user_internal(&ws.workspace_id, &ws.username, &email, &mut tx, None)
                .await?;
        }
    }

    if req.delete_user {
        // Delete instance-level records (same as existing delete_user)
        sqlx::query!("DELETE FROM token WHERE email = $1", &email)
            .execute(&mut *tx)
            .await?;
        sqlx::query!("DELETE FROM password WHERE email = $1", &email)
            .execute(&mut *tx)
            .await?;
        sqlx::query!("DELETE FROM workspace_invite WHERE email = $1", &email)
            .execute(&mut *tx)
            .await?;
        sqlx::query!("DELETE FROM email_to_igroup WHERE email = $1", &email)
            .execute(&mut *tx)
            .await?;
    }

    audit_log(
        &mut *tx,
        &authed,
        if req.delete_user {
            "users.offboard_delete"
        } else {
            "users.offboard_reassign"
        },
        if req.delete_user {
            ActionKind::Delete
        } else {
            ActionKind::Update
        },
        "global",
        Some(&email),
        None,
    )
    .await?;

    tx.commit().await?;

    if req.delete_user {
        Ok(format!("user {} offboarded and deleted", &email))
    } else {
        Ok(format!("user {} objects reassigned", &email))
    }
}

// ---- Offboarding helpers ----

fn parse_reassign_target(target: &str) -> Result<(&str, &str)> {
    if let Some(name) = target.strip_prefix("u/") {
        if name.is_empty() {
            return Err(Error::BadRequest(
                "empty username in reassign_to".to_string(),
            ));
        }
        Ok(("user", name))
    } else if let Some(name) = target.strip_prefix("f/") {
        if name.is_empty() {
            return Err(Error::BadRequest("empty folder in reassign_to".to_string()));
        }
        Ok(("folder", name))
    } else {
        Err(Error::BadRequest(
            "reassign_to must start with 'u/' (user) or 'f/' (folder)".to_string(),
        ))
    }
}

async fn validate_target(db: &DB, w_id: &str, kind: &str, name: &str) -> Result<()> {
    match kind {
        "user" => {
            let exists = sqlx::query_scalar!(
                "SELECT EXISTS(SELECT 1 FROM usr WHERE username = $1 AND workspace_id = $2)",
                name,
                w_id
            )
            .fetch_one(db)
            .await?
            .unwrap_or(false);
            if !exists {
                return Err(Error::NotFound(format!(
                    "target user '{}' not found in workspace '{}'",
                    name, w_id
                )));
            }
        }
        "folder" => {
            let exists = sqlx::query_scalar!(
                "SELECT EXISTS(SELECT 1 FROM folder WHERE name = $1 AND workspace_id = $2)",
                name,
                w_id
            )
            .fetch_one(db)
            .await?
            .unwrap_or(false);
            if !exists {
                return Err(Error::NotFound(format!(
                    "target folder '{}' not found in workspace '{}'",
                    name, w_id
                )));
            }
        }
        _ => unreachable!(),
    }
    Ok(())
}

async fn validate_token_target(db: &DB, w_id: &str, target: &str) -> Result<()> {
    if let Some(name) = target.strip_prefix("u/") {
        let exists = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM usr WHERE username = $1 AND workspace_id = $2)",
            name,
            w_id
        )
        .fetch_one(db)
        .await?
        .unwrap_or(false);
        if !exists {
            return Err(Error::NotFound(format!(
                "token target user '{}' not found in workspace '{}'",
                name, w_id
            )));
        }
    } else if let Some(name) = target.strip_prefix("g/") {
        let exists = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM group_ WHERE name = $1 AND workspace_id = $2)",
            name,
            w_id
        )
        .fetch_one(db)
        .await?
        .unwrap_or(false);
        if !exists {
            return Err(Error::NotFound(format!(
                "token target group '{}' not found in workspace '{}'",
                name, w_id
            )));
        }
    } else {
        return Err(Error::BadRequest(
            "reassign_tokens_to must start with 'u/' (user) or 'g/' (group)".to_string(),
        ));
    }
    Ok(())
}

/// Resolve the permissioned_as value for schedules/triggers.
/// When target is a user, permissioned_as = "u/{user}".
/// When target is a folder, new_operator must be provided — a username to run as.
async fn resolve_new_permissioned_as(
    target_kind: &str,
    reassign_to: &str,
    new_operator: Option<&str>,
    db: &DB,
    w_id: &str,
) -> Result<String> {
    match target_kind {
        "user" => Ok(reassign_to.to_string()), // already "u/{username}"
        "folder" => {
            let operator = new_operator.ok_or_else(|| {
                Error::BadRequest(
                    "new_operator is required when reassigning to a folder".to_string(),
                )
            })?;
            // Validate that the operator user exists in this workspace
            let exists = sqlx::query_scalar!(
                "SELECT EXISTS(SELECT 1 FROM usr WHERE username = $1 AND workspace_id = $2)",
                operator,
                w_id
            )
            .fetch_one(db)
            .await?
            .unwrap_or(false);
            if !exists {
                return Err(Error::NotFound(format!(
                    "new_operator user '{}' not found in workspace '{}'",
                    operator, w_id
                )));
            }
            Ok(format!("u/{}", operator))
        }
        _ => unreachable!(),
    }
}

async fn check_path_conflicts(
    db: &DB,
    w_id: &str,
    username: &str,
    reassign_to: &str,
) -> Result<Vec<String>> {
    let new_prefix = format!("{}/", reassign_to);

    // Check each table that has path-based objects
    let mut conflicts = Vec::new();

    let tables_and_queries = [
        (
            "script",
            r#"
            SELECT REGEXP_REPLACE(s1.path, '^u/' || $1 || '/', $3) as new_path
            FROM script s1
            WHERE s1.path LIKE ('u/' || $1 || '/%') AND s1.workspace_id = $2 AND NOT s1.archived AND NOT s1.deleted
            AND EXISTS (
                SELECT 1 FROM script s2
                WHERE s2.path = REGEXP_REPLACE(s1.path, '^u/' || $1 || '/', $3)
                AND s2.workspace_id = $2 AND NOT s2.archived AND NOT s2.deleted
            )
        "#,
        ),
        (
            "flow",
            r#"
            SELECT REGEXP_REPLACE(f1.path, '^u/' || $1 || '/', $3) as new_path
            FROM flow f1
            WHERE f1.path LIKE ('u/' || $1 || '/%') AND f1.workspace_id = $2 AND NOT f1.archived
            AND EXISTS (
                SELECT 1 FROM flow f2
                WHERE f2.path = REGEXP_REPLACE(f1.path, '^u/' || $1 || '/', $3)
                AND f2.workspace_id = $2 AND NOT f2.archived
            )
        "#,
        ),
        (
            "app",
            r#"
            SELECT REGEXP_REPLACE(a1.path, '^u/' || $1 || '/', $3) as new_path
            FROM app a1
            WHERE a1.path LIKE ('u/' || $1 || '/%') AND a1.workspace_id = $2
            AND EXISTS (
                SELECT 1 FROM app a2
                WHERE a2.path = REGEXP_REPLACE(a1.path, '^u/' || $1 || '/', $3)
                AND a2.workspace_id = $2
            )
        "#,
        ),
        (
            "resource",
            r#"
            SELECT REGEXP_REPLACE(r1.path, '^u/' || $1 || '/', $3) as new_path
            FROM resource r1
            WHERE r1.path LIKE ('u/' || $1 || '/%') AND r1.workspace_id = $2
            AND EXISTS (
                SELECT 1 FROM resource r2
                WHERE r2.path = REGEXP_REPLACE(r1.path, '^u/' || $1 || '/', $3)
                AND r2.workspace_id = $2
            )
        "#,
        ),
        (
            "variable",
            r#"
            SELECT REGEXP_REPLACE(v1.path, '^u/' || $1 || '/', $3) as new_path
            FROM variable v1
            WHERE v1.path LIKE ('u/' || $1 || '/%') AND v1.workspace_id = $2
            AND EXISTS (
                SELECT 1 FROM variable v2
                WHERE v2.path = REGEXP_REPLACE(v1.path, '^u/' || $1 || '/', $3)
                AND v2.workspace_id = $2
            )
        "#,
        ),
        (
            "schedule",
            r#"
            SELECT REGEXP_REPLACE(s1.path, '^u/' || $1 || '/', $3) as new_path
            FROM schedule s1
            WHERE s1.path LIKE ('u/' || $1 || '/%') AND s1.workspace_id = $2
            AND EXISTS (
                SELECT 1 FROM schedule s2
                WHERE s2.path = REGEXP_REPLACE(s1.path, '^u/' || $1 || '/', $3)
                AND s2.workspace_id = $2
            )
        "#,
        ),
    ];

    for (table_name, _query_template) in &tables_and_queries {
        // We can't use the query_template directly with sqlx::query! macro,
        // so we use a single unified dynamic query
        let rows: Vec<String> = sqlx::query_scalar(&format!(
            "SELECT REGEXP_REPLACE(t1.path, '^u/' || $1 || '/', $3) \
             FROM {table} t1 \
             WHERE t1.path LIKE ('u/' || $1 || '/%') AND t1.workspace_id = $2 \
             AND EXISTS ( \
                 SELECT 1 FROM {table} t2 \
                 WHERE t2.path = REGEXP_REPLACE(t1.path, '^u/' || $1 || '/', $3) \
                 AND t2.workspace_id = $2 \
             )",
            table = table_name
        ))
        .bind(username)
        .bind(w_id)
        .bind(&new_prefix)
        .fetch_all(db)
        .await?;

        for path in rows {
            conflicts.push(format!("{}: {}", table_name, path));
        }
    }

    Ok(conflicts)
}

/// Core offboarding logic: reassign all user-scoped objects to the target.
async fn offboard_user_from_workspace<'c>(
    tx: &mut sqlx::Transaction<'c, sqlx::Postgres>,
    db: &DB,
    w_id: &str,
    username: &str,
    reassign_to: &str,
    new_permissioned_as: &str,
    reassign_tokens_to: Option<&str>,
) -> Result<OffboardSummary> {
    // reassign_to is "u/{name}" or "f/{name}"
    let new_prefix = reassign_to.to_string();

    // ---- scripts ----
    let scripts_reassigned = sqlx::query_scalar!(
        r#"WITH updated AS (
            UPDATE script SET path = REGEXP_REPLACE(path, 'u/' || $2 || '/(.*)', $1 || '/\1')
            WHERE path LIKE ('u/' || $2 || '/%') AND workspace_id = $3
            RETURNING 1
        ) SELECT COUNT(*) FROM updated"#,
        &new_prefix,
        username,
        w_id
    )
    .fetch_one(&mut **tx)
    .await?
    .unwrap_or(0);

    // ---- flows ----
    // Flows use INSERT + DELETE pattern due to path being a primary key
    let flows_reassigned = sqlx::query_scalar!(
        r#"WITH inserted AS (
            INSERT INTO flow
                (workspace_id, path, summary, description, archived, extra_perms, dependency_job, draft_only, tag, ws_error_handler_muted, dedicated_worker, timeout, visible_to_runner_only, on_behalf_of_email, concurrency_key, versions, value, schema, edited_by, edited_at)
            SELECT workspace_id, REGEXP_REPLACE(path, 'u/' || $2 || '/(.*)', $1 || '/\1'), summary, description, archived, extra_perms, dependency_job, draft_only, tag, ws_error_handler_muted, dedicated_worker, timeout, visible_to_runner_only, on_behalf_of_email, concurrency_key, versions, value, schema, edited_by, edited_at
                FROM flow
                WHERE path LIKE ('u/' || $2 || '/%') AND workspace_id = $3
            RETURNING 1
        ) SELECT COUNT(*) FROM inserted"#,
        &new_prefix, username, w_id
    ).fetch_one(&mut **tx).await?.unwrap_or(0);

    sqlx::query!(
        r#"UPDATE flow_version SET path = REGEXP_REPLACE(path, 'u/' || $2 || '/(.*)', $1 || '/\1') WHERE path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        &new_prefix, username, w_id
    ).execute(&mut **tx).await?;

    sqlx::query!(
        r#"UPDATE flow_node SET path = REGEXP_REPLACE(path, 'u/' || $2 || '/(.*)', $1 || '/\1') WHERE path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        &new_prefix, username, w_id
    ).execute(&mut **tx).await?;

    sqlx::query!(
        "DELETE FROM flow WHERE path LIKE ('u/' || $1 || '/%') AND workspace_id = $2",
        username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    // ---- apps ----
    let apps_reassigned = sqlx::query_scalar!(
        r#"WITH updated AS (
            UPDATE app SET path = REGEXP_REPLACE(path, 'u/' || $2 || '/(.*)', $1 || '/\1')
            WHERE path LIKE ('u/' || $2 || '/%') AND workspace_id = $3
            RETURNING 1
        ) SELECT COUNT(*) FROM updated"#,
        &new_prefix,
        username,
        w_id
    )
    .fetch_one(&mut **tx)
    .await?
    .unwrap_or(0);

    // Update app policy.on_behalf_of
    sqlx::query!(
        "UPDATE app SET policy = jsonb_set(policy, ARRAY['on_behalf_of'], to_jsonb($1::text)) WHERE policy->>'on_behalf_of' = ('u/' || $2) AND workspace_id = $3",
        &new_permissioned_as, username, w_id
    ).execute(&mut **tx).await?;

    // ---- resources ----
    let resources_reassigned = sqlx::query_scalar!(
        r#"WITH updated AS (
            UPDATE resource SET path = REGEXP_REPLACE(path, 'u/' || $2 || '/(.*)', $1 || '/\1')
            WHERE path LIKE ('u/' || $2 || '/%') AND workspace_id = $3
            RETURNING 1
        ) SELECT COUNT(*) FROM updated"#,
        &new_prefix,
        username,
        w_id
    )
    .fetch_one(&mut **tx)
    .await?
    .unwrap_or(0);

    // ---- variables (with Vault secret handling) ----
    let old_var_prefix = format!("u/{}/", username);
    let new_var_prefix = format!("{}/", reassign_to);

    let vault_secrets: Vec<(String, String)> = sqlx::query!(
        r#"SELECT path, value FROM variable
           WHERE path LIKE ('u/' || $1 || '/%')
           AND workspace_id = $2
           AND is_secret = true
           AND value LIKE '$vault:%'"#,
        username,
        w_id
    )
    .fetch_all(&mut **tx)
    .await?
    .into_iter()
    .map(|r| (r.path, r.value))
    .collect();

    let vault_updates =
        rename_vault_secrets_with_prefix(db, w_id, &old_var_prefix, &new_var_prefix, vault_secrets)
            .await?;

    for (old_path, new_value) in vault_updates {
        sqlx::query!(
            "UPDATE variable SET value = $1 WHERE path = $2 AND workspace_id = $3",
            new_value,
            old_path,
            w_id
        )
        .execute(&mut **tx)
        .await?;
    }

    let variables_reassigned = sqlx::query_scalar!(
        r#"WITH updated AS (
            UPDATE variable SET path = REGEXP_REPLACE(path, 'u/' || $2 || '/(.*)', $1 || '/\1')
            WHERE path LIKE ('u/' || $2 || '/%') AND workspace_id = $3
            RETURNING 1
        ) SELECT COUNT(*) FROM updated"#,
        &new_prefix,
        username,
        w_id
    )
    .fetch_one(&mut **tx)
    .await?
    .unwrap_or(0);

    // ---- schedules ----
    let schedules_reassigned = sqlx::query_scalar!(
        r#"WITH updated AS (
            UPDATE schedule SET
                path = REGEXP_REPLACE(path, 'u/' || $2 || '/(.*)', $1 || '/\1'),
                script_path = REGEXP_REPLACE(script_path, 'u/' || $2 || '/(.*)', $1 || '/\1'),
                permissioned_as = $4
            WHERE path LIKE ('u/' || $2 || '/%') AND workspace_id = $3
            RETURNING 1
        ) SELECT COUNT(*) FROM updated"#,
        &new_prefix,
        username,
        w_id,
        &new_permissioned_as
    )
    .fetch_one(&mut **tx)
    .await?
    .unwrap_or(0);

    // Also update schedules where only script_path references the old user (schedule itself may be elsewhere)
    sqlx::query!(
        r#"UPDATE schedule SET script_path = REGEXP_REPLACE(script_path, 'u/' || $2 || '/(.*)', $1 || '/\1')
        WHERE script_path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        &new_prefix, username, w_id
    ).execute(&mut **tx).await?;

    // Also update permissioned_as on schedules that run as this user but aren't under their path
    sqlx::query!(
        "UPDATE schedule SET permissioned_as = $1 WHERE permissioned_as = ('u/' || $2) AND workspace_id = $3",
        &new_permissioned_as, username, w_id
    ).execute(&mut **tx).await?;

    // ---- triggers (all 9 types with path/permissioned_as) ----
    let trigger_tables = [
        "http_trigger",
        "websocket_trigger",
        "kafka_trigger",
        "postgres_trigger",
        "mqtt_trigger",
        "nats_trigger",
        "sqs_trigger",
        "gcp_trigger",
        "email_trigger",
    ];

    let mut triggers_reassigned: i64 = 0;
    for table in &trigger_tables {
        // Update path, script_path, and permissioned_as
        let count: i64 = sqlx::query_scalar(&format!(
            "WITH updated AS ( \
                UPDATE {table} SET \
                    path = REGEXP_REPLACE(path, 'u/' || $1 || '/(.*)', $2 || '/\\1'), \
                    script_path = REGEXP_REPLACE(script_path, 'u/' || $1 || '/(.*)', $2 || '/\\1'), \
                    permissioned_as = $4 \
                WHERE path LIKE ('u/' || $1 || '/%') AND workspace_id = $3 \
                RETURNING 1 \
            ) SELECT COUNT(*) FROM updated"
        ))
        .bind(username)
        .bind(&new_prefix)
        .bind(w_id)
        .bind(&new_permissioned_as)
        .fetch_one(&mut **tx)
        .await?;
        triggers_reassigned += count;

        // Also update script_path references and permissioned_as for triggers not under user's path
        sqlx::query(&format!(
            "UPDATE {table} SET script_path = REGEXP_REPLACE(script_path, 'u/' || $1 || '/(.*)', $2 || '/\\1') \
             WHERE script_path LIKE ('u/' || $1 || '/%') AND workspace_id = $3"
        ))
        .bind(username)
        .bind(&new_prefix)
        .bind(w_id)
        .execute(&mut **tx)
        .await?;

        sqlx::query(&format!(
            "UPDATE {table} SET permissioned_as = $1 WHERE permissioned_as = ('u/' || $2) AND workspace_id = $3"
        ))
        .bind(&new_permissioned_as)
        .bind(username)
        .bind(w_id)
        .execute(&mut **tx)
        .await?;
    }

    // ---- Clean up extra_perms: remove u/{username} key from all tables ----
    let extra_perms_tables = [
        "script",
        "flow",
        "app",
        "resource",
        "variable",
        "schedule",
        "group_",
        "folder",
        "raw_app",
        "http_trigger",
        "websocket_trigger",
        "kafka_trigger",
        "postgres_trigger",
        "mqtt_trigger",
        "nats_trigger",
        "sqs_trigger",
        "gcp_trigger",
        "email_trigger",
    ];
    for table in &extra_perms_tables {
        sqlx::query(&format!(
            "UPDATE {table} SET extra_perms = extra_perms - ('u/' || $1) \
             WHERE extra_perms ? ('u/' || $1) AND workspace_id = $2"
        ))
        .bind(username)
        .bind(w_id)
        .execute(&mut **tx)
        .await?;
    }

    // ---- Clean up folder owners ----
    sqlx::query!(
        "UPDATE folder SET owners = array_remove(owners, 'u/' || $1) WHERE ('u/' || $1) = ANY(owners) AND workspace_id = $2",
        username, w_id
    ).execute(&mut **tx).await?;

    // ---- Related path tables ----
    sqlx::query!(
        r#"UPDATE workspace_integrations SET resource_path = REGEXP_REPLACE(resource_path, 'u/' || $2 || '/(.*)', $1 || '/\1') WHERE resource_path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        &new_prefix, username, w_id
    ).execute(&mut **tx).await?;

    sqlx::query!(
        r#"UPDATE workspace_runnable_dependencies SET flow_path = REGEXP_REPLACE(flow_path, 'u/' || $2 || '/(.*)', $1 || '/\1') WHERE flow_path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        &new_prefix, username, w_id
    ).execute(&mut **tx).await?;

    sqlx::query!(
        r#"UPDATE workspace_runnable_dependencies SET app_path = REGEXP_REPLACE(app_path, 'u/' || $2 || '/(.*)', $1 || '/\1') WHERE app_path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        &new_prefix, username, w_id
    ).execute(&mut **tx).await?;

    sqlx::query!(
        r#"UPDATE workspace_runnable_dependencies SET runnable_path = REGEXP_REPLACE(runnable_path, 'u/' || $2 || '/(.*)', $1 || '/\1') WHERE runnable_path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        &new_prefix, username, w_id
    ).execute(&mut **tx).await?;

    sqlx::query!(
        r#"UPDATE asset SET usage_path = REGEXP_REPLACE(usage_path, 'u/' || $2 || '/(.*)', $1 || '/\1') WHERE usage_path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        &new_prefix, username, w_id
    ).execute(&mut **tx).await?;

    // ---- Drafts ----
    let drafts_deleted = sqlx::query_scalar!(
        r#"WITH deleted AS (
            DELETE FROM draft WHERE path LIKE ('u/' || $1 || '/%') AND workspace_id = $2
            RETURNING 1
        ) SELECT COUNT(*) FROM deleted"#,
        username,
        w_id
    )
    .fetch_one(&mut **tx)
    .await?
    .unwrap_or(0);

    // ---- Personal data: favorites, inputs, captures ----
    sqlx::query!(
        "DELETE FROM favorite WHERE usr = $1 AND workspace_id = $2",
        username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        "DELETE FROM input WHERE created_by = $1 AND workspace_id = $2",
        username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        "DELETE FROM capture WHERE created_by = $1 AND workspace_id = $2",
        username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    // ---- Tokens ----
    let user_owner = format!("u/{}", username);
    let tokens_handled = if let Some(token_target) = reassign_tokens_to {
        // Reassign tokens to the specified target
        let count = sqlx::query_scalar!(
            r#"WITH updated AS (
                UPDATE token SET owner = $1 WHERE owner = $2 AND workspace_id = $3
                RETURNING 1
            ) SELECT COUNT(*) FROM updated"#,
            token_target,
            &user_owner,
            w_id
        )
        .fetch_one(&mut **tx)
        .await?
        .unwrap_or(0);
        count
    } else {
        // Revoke tokens
        let count = sqlx::query_scalar!(
            r#"WITH deleted AS (
                DELETE FROM token WHERE owner = $1 AND workspace_id = $2
                RETURNING 1
            ) SELECT COUNT(*) FROM deleted"#,
            &user_owner,
            w_id
        )
        .fetch_one(&mut **tx)
        .await?
        .unwrap_or(0);
        count
    };

    Ok(OffboardSummary {
        scripts_reassigned,
        flows_reassigned,
        apps_reassigned,
        resources_reassigned,
        variables_reassigned,
        schedules_reassigned,
        triggers_reassigned,
        tokens_handled,
        drafts_deleted,
    })
}

async fn update_username_in_workpsace<'c>(
    tx: &mut sqlx::Transaction<'c, sqlx::Postgres>,
    db: &DB,
    email: &str,
    old_username: &str,
    new_username: &str,
    w_id: &str,
) -> error::Result<()> {
    // ---- instance and workspace users ----
    sqlx::query!(
        "UPDATE usr SET username = $1 WHERE email = $2",
        new_username,
        email
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        "UPDATE usr_to_group SET usr = $1 WHERE usr = $2",
        new_username,
        old_username
    )
    .execute(&mut **tx)
    .await?;

    // ---- v2_job ----
    sqlx::query!(
        r#"UPDATE v2_job SET runnable_path = REGEXP_REPLACE(runnable_path,'u/' || $2 || '/(.*)','u/' || $1 || '/\1') WHERE runnable_path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        r#"UPDATE v2_job SET trigger = REGEXP_REPLACE(trigger,'u/' || $2 || '/(.*)','u/' || $1 || '/\1') WHERE trigger LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        "UPDATE v2_job SET permissioned_as = ('u/' || $1) WHERE permissioned_as = ('u/' || $2) AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        "UPDATE v2_job SET created_by = $1 WHERE created_by = $2 AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    // ---- v2_job_queue ----
    sqlx::query!(
        "UPDATE v2_job_queue SET canceled_by = $1 WHERE canceled_by = $2 AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    // ---- v2_job_completed ----
    sqlx::query!(
        "UPDATE v2_job_completed SET canceled_by = $1 WHERE canceled_by = $2 AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    // ---- resources----
    sqlx::query!(
        r#"UPDATE resource SET created_by = $1 WHERE created_by = $2 AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        r#"UPDATE resource_type SET created_by = $1 WHERE created_by = $2 AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        r#"UPDATE resource SET path = REGEXP_REPLACE(path,'u/' || $2 || '/(.*)','u/' || $1 || '/\1') WHERE path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    ).execute(&mut **tx)
    .await?;

    sqlx::query!(
        "UPDATE resource SET extra_perms = extra_perms - ('u/' || $2) || jsonb_build_object(('u/' || $1), extra_perms->('u/' || $2)) WHERE extra_perms ? ('u/' || $2) AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    // ---- variables ----

    // Handle Vault secret renames before updating paths in DB
    let old_prefix = format!("u/{}/", old_username);
    let new_prefix = format!("u/{}/", new_username);

    // Fetch all Vault-stored secret variables under this user's path
    let vault_secrets: Vec<(String, String)> = sqlx::query!(
        r#"SELECT path, value FROM variable
           WHERE path LIKE ('u/' || $1 || '/%')
           AND workspace_id = $2
           AND is_secret = true
           AND value LIKE '$vault:%'"#,
        old_username,
        w_id
    )
    .fetch_all(&mut **tx)
    .await?
    .into_iter()
    .map(|r| (r.path, r.value))
    .collect();

    // Rename secrets in Vault and get the new values
    let vault_updates =
        rename_vault_secrets_with_prefix(db, w_id, &old_prefix, &new_prefix, vault_secrets).await?;

    // Update the values in the DB for renamed Vault secrets (using OLD path, before path update)
    for (old_path, new_value) in vault_updates {
        sqlx::query!(
            "UPDATE variable SET value = $1 WHERE path = $2 AND workspace_id = $3",
            new_value,
            old_path,
            w_id
        )
        .execute(&mut **tx)
        .await?;
    }

    // Now update the paths in the database
    sqlx::query!(
        r#"UPDATE variable SET path = REGEXP_REPLACE(path,'u/' || $2 || '/(.*)','u/' || $1 || '/\1') WHERE path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    ).execute(&mut **tx)
    .await?;

    sqlx::query!(
        r#"UPDATE workspace_integrations SET resource_path = REGEXP_REPLACE(resource_path, 'u/' || $2 || '/(.*)', 'u/' || $1 || '/\1') WHERE resource_path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        "UPDATE variable SET extra_perms = extra_perms - ('u/' || $2) || jsonb_build_object(('u/' || $1), extra_perms->('u/' || $2)) WHERE extra_perms ? ('u/' || $2) AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    // ---- scripts ----
    sqlx::query!(
        r#"UPDATE script SET path = REGEXP_REPLACE(path,'u/' || $2 || '/(.*)','u/' || $1 || '/\1') WHERE path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        "UPDATE script SET created_by = $1 WHERE created_by = $2 AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await
    .unwrap();

    sqlx::query!(
        "UPDATE script SET extra_perms = extra_perms - ('u/' || $2) || jsonb_build_object(('u/' || $1), extra_perms->('u/' || $2)) WHERE extra_perms ? ('u/' || $2) AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    // ---- flows ----
    sqlx::query!(
        r#"INSERT INTO flow
            (workspace_id, path, summary, description, archived, extra_perms, dependency_job, draft_only, tag, ws_error_handler_muted, dedicated_worker, timeout, visible_to_runner_only, on_behalf_of_email, concurrency_key, versions, value, schema, edited_by, edited_at)
        SELECT workspace_id, REGEXP_REPLACE(path,'u/' || $2 || '/(.*)','u/' || $1 || '/\1'), summary, description, archived, extra_perms, dependency_job, draft_only, tag, ws_error_handler_muted, dedicated_worker, timeout, visible_to_runner_only, on_behalf_of_email, concurrency_key, versions, value, schema, edited_by, edited_at
            FROM flow
            WHERE path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    ).execute(&mut **tx)
    .await?;

    sqlx::query!(
        r#"UPDATE flow_version SET path = REGEXP_REPLACE(path,'u/' || $2 || '/(.*)','u/' || $1 || '/\1') WHERE path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        r#"UPDATE workspace_runnable_dependencies SET flow_path = REGEXP_REPLACE(flow_path,'u/' || $2 || '/(.*)','u/' || $1 || '/\1') WHERE flow_path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    ).execute(&mut **tx)
    .await?;

    sqlx::query!(
        r#"UPDATE workspace_runnable_dependencies SET app_path = REGEXP_REPLACE(app_path,'u/' || $2 || '/(.*)','u/' || $1 || '/\1') WHERE app_path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    ).execute(&mut **tx)
    .await?;

    sqlx::query!(
        r#"UPDATE workspace_runnable_dependencies SET runnable_path = REGEXP_REPLACE(runnable_path,'u/' || $2 || '/(.*)','u/' || $1 || '/\1') WHERE runnable_path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    ).execute(&mut **tx)
    .await?;

    sqlx::query!(
        r#"UPDATE asset SET usage_path = REGEXP_REPLACE(usage_path,'u/' || $2 || '/(.*)','u/' || $1 || '/\1') WHERE usage_path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        r#"UPDATE flow_node SET path = REGEXP_REPLACE(path,'u/' || $2 || '/(.*)','u/' || $1 || '/\1') WHERE path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        "DELETE FROM flow WHERE path LIKE ('u/' || $1 || '/%') AND workspace_id = $2",
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        "UPDATE flow SET extra_perms = extra_perms - ('u/' || $2) || jsonb_build_object(('u/' || $1), extra_perms->('u/' || $2)) WHERE extra_perms ? ('u/' || $2) AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    // ---- draft ----
    sqlx::query!(
        r#"UPDATE draft SET path = REGEXP_REPLACE(path,'u/' || $2 || '/(.*)','u/' || $1 || '/\1') WHERE path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        r#"UPDATE draft SET value = to_json(jsonb_set(to_jsonb(value), ARRAY['path'], to_jsonb(REGEXP_REPLACE(value->>'path','u/' || $2 || '/(.*)','u/' || $1 || '/\1')))) WHERE value->>'path' LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    ).execute(&mut **tx)
    .await?;

    // ---- app ----
    sqlx::query!(
        r#"UPDATE app SET path = REGEXP_REPLACE(path,'u/' || $2 || '/(.*)','u/' || $1 || '/\1') WHERE path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        "UPDATE app SET policy = jsonb_set(policy, ARRAY['on_behalf_of'], to_jsonb('u/' || $1)) WHERE policy->>'on_behalf_of' = ('u/' || $2) AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        "UPDATE app SET extra_perms = extra_perms - ('u/' || $2) || jsonb_build_object(('u/' || $1), extra_perms->('u/' || $2)) WHERE extra_perms ? ('u/' || $2) AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    // ---- app_version ----

    sqlx::query!(
        "UPDATE app_version SET created_by = $1 WHERE created_by = $2 AND EXISTS (SELECT 1 FROM app WHERE workspace_id = $3 AND app.id = app_version.app_id)",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    // ---- schedules ----

    sqlx::query!(
        r#"UPDATE schedule SET path = REGEXP_REPLACE(path,'u/' || $2 || '/(.*)','u/' || $1 || '/\1') WHERE path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        r#"UPDATE schedule SET script_path = REGEXP_REPLACE(script_path,'u/' || $2 || '/(.*)','u/' || $1 || '/\1') WHERE script_path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        "UPDATE schedule SET edited_by = $1 WHERE edited_by = $2 AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await
    .unwrap();

    sqlx::query!(
        "UPDATE schedule SET extra_perms = extra_perms - ('u/' || $2) || jsonb_build_object(('u/' || $1), extra_perms->('u/' || $2)) WHERE extra_perms ? ('u/' || $2) AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    // ---- group_ ----

    sqlx::query!(
        "UPDATE group_ SET extra_perms = extra_perms - ('u/' || $2) || jsonb_build_object(('u/' || $1), extra_perms->('u/' || $2)) WHERE extra_perms ? ('u/' || $2) AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    // ---- folders ----

    sqlx::query!(
        "UPDATE folder SET created_by = $1 WHERE created_by = $2 AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        "UPDATE folder SET owners = ARRAY_REPLACE(owners, 'u/' || $2, 'u/' || $1) WHERE  ('u/' || $2) = ANY(owners) AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        "UPDATE folder SET extra_perms = extra_perms - ('u/' || $2) || jsonb_build_object(('u/' || $1), extra_perms->('u/' || $2)) WHERE extra_perms ? ('u/' || $2) AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    // ---- input ----

    sqlx::query!(
        "UPDATE input SET created_by = $1 WHERE created_by = $2 AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await
    .unwrap();

    // ---- favorite ----

    sqlx::query!(
        "UPDATE favorite SET usr = $1 WHERE usr = $2 AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await
    .unwrap();

    // ---- capture ----

    sqlx::query!(
        "UPDATE capture SET created_by = $1 WHERE created_by = $2 AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await
    .unwrap();

    // ---- token ----

    sqlx::query!(
        "UPDATE token SET owner = ('u/' || $1) WHERE owner = ('u/' || $2) AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await
    .unwrap();

    sqlx::query!(
        r#"UPDATE token SET scopes = array(select regexp_replace(unnest(scopes), 'run:([^/]+)/u/' || $2 || '/(.+)', 'run:\1/u/' || $1 || '/\2')) WHERE EXISTS (SELECT 1 FROM UNNEST(scopes) scope WHERE scope LIKE ('run:%/u/' || $2 || '/%')) AND workspace_id = $3"#,
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    // ---- raw_app ----

    sqlx::query!(
        "UPDATE raw_app SET extra_perms = extra_perms - ('u/' || $2) || jsonb_build_object(('u/' || $1), extra_perms->('u/' || $2)) WHERE extra_perms ? ('u/' || $2) AND workspace_id = $3",
        new_username,
        old_username,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    Ok(())
}

/// Reset password using a token
async fn reset_password(
    Extension(db): Extension<DB>,
    Extension(argon2): Extension<Arc<Argon2<'_>>>,
    Json(req): Json<ResetPassword>,
) -> Result<Json<PasswordResetResponse>> {
    let mut tx = db.begin().await?;

    // Find the token and verify it's not expired
    let magic_link = sqlx::query!(
        "SELECT email FROM magic_link WHERE token = $1 AND expiration > NOW()",
        &req.token
    )
    .fetch_optional(&mut *tx)
    .await?;

    let email = match magic_link {
        Some(link) => link.email,
        None => {
            return Err(Error::BadRequest(
                "Invalid or expired password reset token".to_string(),
            ))
        }
    };

    // Hash the new password
    let password_hash = crate::users_oss::hash_password(argon2, req.new_password)?;

    // Update the password
    let rows_updated = sqlx::query!(
        "UPDATE password SET password_hash = $1 WHERE email = $2 AND login_type = 'password'",
        &password_hash,
        &email
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();

    if rows_updated == 0 {
        return Err(Error::BadRequest(
            "Unable to update password. User may not exist or may use a different login method."
                .to_string(),
        ));
    }

    // Delete the used token and any other tokens for this email
    sqlx::query!("DELETE FROM magic_link WHERE email = $1", &email)
        .execute(&mut *tx)
        .await?;

    // Invalidate all existing sessions for this user
    sqlx::query!(
        "DELETE FROM token WHERE email = $1 AND label = 'session'",
        &email
    )
    .execute(&mut *tx)
    .await?;

    // Audit log
    let audit_author = AuditAuthor {
        email: email.clone(),
        username: email.clone(),
        username_override: None,
        token_prefix: None,
    };

    audit_log(
        &mut *tx,
        &audit_author,
        "users.password_reset",
        ActionKind::Update,
        "global",
        Some(&email),
        None,
    )
    .await?;

    tx.commit().await?;

    Ok(Json(PasswordResetResponse {
        message: "Password has been reset successfully. You can now log in with your new password."
            .to_string(),
    }))
}
