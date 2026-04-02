use std::collections::HashMap;

use crate::db::ApiAuthed;
use crate::secret_backend_ext::rename_vault_secrets_with_prefix;
use axum::{
    extract::{Extension, Path},
    Json,
};
use serde::{Deserialize, Serialize};
use windmill_api_auth::require_super_admin;
use windmill_api_users::users::delete_workspace_user_internal;
use windmill_audit::audit_oss::audit_log;
use windmill_audit::ActionKind;
use windmill_common::utils::require_admin;
use windmill_common::{
    error::{Error, JsonResult, Result},
    DB,
};
use windmill_git_sync::handle_deployment_metadata;

// ---- Types ----

#[derive(Serialize, Deserialize)]
pub(crate) struct OffboardPreview {
    /// Objects under u/{username}/ that will be reassigned
    owned: OffboardAffectedPaths,
    /// Objects NOT under the user's path but that execute on behalf of this user
    /// (will have their permissioned_as/on_behalf_of updated)
    executing_on_behalf: OffboardAffectedPaths,
    /// Tokens owned by this user (will be deleted)
    tokens: i64,
    /// HTTP triggers under the user's path (webhook URLs will change)
    http_triggers: i64,
    /// Email triggers under the user's path (email addresses will change)
    email_triggers: i64,
}

#[derive(Serialize, Deserialize, Default)]
struct OffboardAffectedPaths {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    scripts: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    flows: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    apps: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    resources: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    variables: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    schedules: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    triggers: Vec<String>,
}

#[derive(Deserialize)]
pub(crate) struct OffboardRequest {
    reassign_to: String,
    /// Required when reassign_to is a folder (f/...). The username whose identity
    /// will be used as permissioned_as for schedules and triggers.
    new_operator: Option<String>,
    #[serde(default = "default_true")]
    delete_user: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Serialize)]
pub(crate) struct OffboardResponse {
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
    drafts_deleted: i64,
}

#[derive(Serialize)]
pub(crate) struct GlobalOffboardPreview {
    workspaces: Vec<WorkspaceOffboardPreview>,
}

#[derive(Serialize)]
struct WorkspaceOffboardPreview {
    workspace_id: String,
    username: String,
    preview: OffboardPreview,
}

#[derive(Deserialize)]
pub(crate) struct GlobalOffboardRequest {
    #[serde(default)]
    reassignments: HashMap<String, WorkspaceReassignment>,
    #[serde(default = "default_true")]
    delete_user: bool,
}

#[derive(Deserialize)]
struct WorkspaceReassignment {
    reassign_to: String,
    new_operator: Option<String>,
}

// ---- Preview helpers ----

async fn get_offboard_preview(
    db: impl sqlx::PgExecutor<'_> + Copy,
    w_id: &str,
    username: &str,
    email: &str,
) -> Result<OffboardPreview> {
    let user_prefix = format!("u/{}/%", username);
    let user_owner = format!("u/{}", username);

    // ---- Owned objects (under u/{username}/) ----
    let scripts = sqlx::query_scalar!(
        "SELECT path FROM script WHERE path LIKE $1 AND workspace_id = $2 AND NOT archived AND NOT deleted",
        &user_prefix, w_id
    ).fetch_all(db).await?;

    let flows = sqlx::query_scalar!(
        "SELECT path FROM flow WHERE path LIKE $1 AND workspace_id = $2 AND NOT archived",
        &user_prefix,
        w_id
    )
    .fetch_all(db)
    .await?;

    let apps = sqlx::query_scalar!(
        "SELECT path FROM app WHERE path LIKE $1 AND workspace_id = $2",
        &user_prefix,
        w_id
    )
    .fetch_all(db)
    .await?;

    let resources = sqlx::query_scalar!(
        "SELECT path FROM resource WHERE path LIKE $1 AND workspace_id = $2",
        &user_prefix,
        w_id
    )
    .fetch_all(db)
    .await?;

    let variables = sqlx::query_scalar!(
        "SELECT path FROM variable WHERE path LIKE $1 AND workspace_id = $2",
        &user_prefix,
        w_id
    )
    .fetch_all(db)
    .await?;

    let schedules = sqlx::query_scalar!(
        "SELECT path FROM schedule WHERE path LIKE $1 AND workspace_id = $2",
        &user_prefix,
        w_id
    )
    .fetch_all(db)
    .await?;

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
    let mut triggers = Vec::new();
    for table in &trigger_tables {
        let paths: Vec<String> = sqlx::query_scalar(&format!(
            "SELECT path FROM {table} WHERE path LIKE $1 AND workspace_id = $2"
        ))
        .bind(&user_prefix)
        .bind(w_id)
        .fetch_all(db)
        .await?;
        triggers.extend(paths);
    }

    // ---- Tokens ----
    let tokens = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM token WHERE owner = $1 AND workspace_id = $2",
        &user_owner,
        w_id
    )
    .fetch_one(db)
    .await?
    .unwrap_or(0);

    // ---- Operator references (not under user's path) ----
    let op_scripts = sqlx::query_scalar!(
        "SELECT path FROM script WHERE on_behalf_of_email = $1 AND NOT path LIKE $2 AND workspace_id = $3 AND NOT archived AND NOT deleted",
        email, &user_prefix, w_id
    ).fetch_all(db).await?;

    let op_flows = sqlx::query_scalar!(
        "SELECT path FROM flow WHERE on_behalf_of_email = $1 AND NOT path LIKE $2 AND workspace_id = $3 AND NOT archived",
        email, &user_prefix, w_id
    ).fetch_all(db).await?;

    let op_apps = sqlx::query_scalar!(
        "SELECT path FROM app WHERE policy->>'on_behalf_of' = $1 AND NOT path LIKE $2 AND workspace_id = $3",
        &user_owner, &user_prefix, w_id
    ).fetch_all(db).await?;

    let op_schedules = sqlx::query_scalar!(
        "SELECT path FROM schedule WHERE permissioned_as = $1 AND NOT path LIKE $2 AND workspace_id = $3",
        &user_owner, &user_prefix, w_id
    ).fetch_all(db).await?;

    let mut op_triggers = Vec::new();
    for table in &trigger_tables {
        let paths: Vec<String> = sqlx::query_scalar(&format!(
            "SELECT path FROM {table} WHERE permissioned_as = $1 AND NOT path LIKE $2 AND workspace_id = $3"
        ))
        .bind(&user_owner)
        .bind(&user_prefix)
        .bind(w_id)
        .fetch_all(db)
        .await?;
        op_triggers.extend(paths);
    }

    // ---- Specific trigger warnings ----
    let http_triggers = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM http_trigger WHERE path LIKE $1 AND workspace_id = $2",
        &user_prefix,
        w_id
    )
    .fetch_one(db)
    .await?
    .unwrap_or(0);

    let email_triggers = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM email_trigger WHERE path LIKE $1 AND workspace_id = $2",
        &user_prefix,
        w_id
    )
    .fetch_one(db)
    .await?
    .unwrap_or(0);

    Ok(OffboardPreview {
        owned: OffboardAffectedPaths {
            scripts,
            flows,
            apps,
            resources,
            variables,
            schedules,
            triggers,
        },
        executing_on_behalf: OffboardAffectedPaths {
            scripts: op_scripts,
            flows: op_flows,
            apps: op_apps,
            schedules: op_schedules,
            triggers: op_triggers,
            ..Default::default()
        },
        tokens,
        http_triggers,
        email_triggers,
    })
}

// ---- Workspace-level endpoints ----

pub(crate) async fn offboard_preview(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, username)): Path<(String, String)>,
) -> JsonResult<OffboardPreview> {
    require_admin(authed.is_admin, &authed.username)?;
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
    let preview = get_offboard_preview(&db, &w_id, &username, &email).await?;
    Ok(Json(preview))
}

pub(crate) async fn offboard_workspace_user(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, username)): Path<(String, String)>,
    Json(req): Json<OffboardRequest>,
) -> JsonResult<OffboardResponse> {
    require_admin(authed.is_admin, &authed.username)?;

    let (target_kind, target_name) = parse_reassign_target(&req.reassign_to)?;
    validate_target(&db, &w_id, target_kind, target_name).await?;

    let new_permissioned_as = resolve_new_permissioned_as(
        target_kind,
        &req.reassign_to,
        req.new_operator.as_deref(),
        &db,
        &w_id,
    )
    .await?;

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
        &email,
        &req.reassign_to,
        &new_permissioned_as,
    )
    .await?;

    if req.delete_user {
        delete_workspace_user_internal(&w_id, &username, &email, &mut tx, Some(&authed)).await?;
    } else {
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

// ---- Instance-level endpoints ----

pub(crate) async fn global_offboard_preview(
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
        let preview = get_offboard_preview(&db, &w.workspace_id, &w.username, &email).await?;
        previews.push(WorkspaceOffboardPreview {
            workspace_id: w.workspace_id,
            username: w.username,
            preview,
        });
    }

    Ok(Json(GlobalOffboardPreview { workspaces: previews }))
}

pub(crate) async fn offboard_global_user(
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
                &email,
                &reassignment.reassign_to,
                perm_as,
            )
            .await?;
        }

        if req.delete_user {
            delete_workspace_user_internal(&ws.workspace_id, &ws.username, &email, &mut tx, None)
                .await?;
        }
    }

    if req.delete_user {
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

// ---- Validation helpers ----

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
        "user" => Ok(reassign_to.to_string()),
        "folder" => {
            let operator = new_operator.ok_or_else(|| {
                Error::BadRequest(
                    "new_operator is required when reassigning to a folder".to_string(),
                )
            })?;
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
    let mut conflicts = Vec::new();

    let tables = ["script", "flow", "app", "resource", "variable", "schedule"];

    for table_name in &tables {
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

// ---- Core reassignment logic ----

async fn offboard_user_from_workspace<'c>(
    tx: &mut sqlx::Transaction<'c, sqlx::Postgres>,
    db: &DB,
    w_id: &str,
    username: &str,
    email: &str,
    reassign_to: &str,
    new_permissioned_as: &str,
) -> Result<OffboardSummary> {
    let new_prefix = reassign_to.to_string();

    // Resolve the new operator's email for on_behalf_of_email on scripts/flows
    let new_operator_username = new_permissioned_as
        .strip_prefix("u/")
        .unwrap_or(new_permissioned_as);
    let new_operator_email = sqlx::query_scalar!(
        "SELECT email FROM usr WHERE username = $1 AND workspace_id = $2",
        new_operator_username,
        w_id
    )
    .fetch_optional(&mut **tx)
    .await?;

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

    if let Some(ref new_email) = new_operator_email {
        sqlx::query!(
            "UPDATE script SET on_behalf_of_email = $1 WHERE on_behalf_of_email = $2 AND workspace_id = $3",
            new_email,
            email,
            w_id
        )
        .execute(&mut **tx)
        .await?;
    }

    // ---- flows ----
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

    if let Some(ref new_email) = new_operator_email {
        sqlx::query!(
            "UPDATE flow SET on_behalf_of_email = $1 WHERE on_behalf_of_email = $2 AND workspace_id = $3",
            new_email,
            email,
            w_id
        )
        .execute(&mut **tx)
        .await?;
    }

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

    sqlx::query!(
        r#"UPDATE schedule SET script_path = REGEXP_REPLACE(script_path, 'u/' || $2 || '/(.*)', $1 || '/\1')
        WHERE script_path LIKE ('u/' || $2 || '/%') AND workspace_id = $3"#,
        &new_prefix, username, w_id
    ).execute(&mut **tx).await?;

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

    // ---- Clean up extra_perms ----
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

    // ---- Personal data ----
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
    sqlx::query!(
        "DELETE FROM token WHERE owner = $1 AND workspace_id = $2",
        &user_owner,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    Ok(OffboardSummary {
        scripts_reassigned,
        flows_reassigned,
        apps_reassigned,
        resources_reassigned,
        variables_reassigned,
        schedules_reassigned,
        triggers_reassigned,
        drafts_deleted,
    })
}
