use crate::db::ApiAuthed;

use crate::workspaces::{
    check_w_id_conflict, CREATE_WORKSPACE_REQUIRE_SUPERADMIN,
    MIGRATE_JOBS_WORKSPACE_REQUIRE_SUPERADMIN, WM_FORK_PREFIX,
};
use crate::{db::DB, utils::require_super_admin};

use axum::extract::Query;
use axum::{
    extract::{Extension, Path},
    Json,
};

use sqlx::{Postgres, Transaction};
use windmill_audit::audit_oss::audit_log;
use windmill_audit::ActionKind;

use windmill_common::db::UserDB;
use windmill_common::error::JsonResult;
use windmill_common::worker::CLOUD_HOSTED;

use windmill_common::{
    auth::is_super_admin_email,
    error::{Error, Result},
    utils::require_admin,
};

use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct MigrateJobRequest {
    source_workspace_id: String,
    target_workspace_id: String,
    #[serde(default = "default_batch_size")]
    batch_size: i64,
}

#[derive(Deserialize)]
pub struct MigrateWorkspaceRequest {
    source_workspace_id: String,
    target_workspace_id: String,
    target_workspace_name: String,
}

fn default_batch_size() -> i64 {
    DEFAULT_BATCH_SIZE
}

#[derive(Deserialize)]
pub(crate) struct DeleteWorkspaceQuery {
    pub(crate) only_delete_forks: Option<bool>,
}

async fn delete_workspace_tables(tx: &mut Transaction<'_, Postgres>, w_id: &str) -> Result<()> {
    sqlx::query!("DELETE FROM ai_agent_memory WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query!(
        "DELETE FROM flow_conversation WHERE workspace_id = $1",
        &w_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!("DELETE FROM workspace_env WHERE workspace_id = $1", w_id)
        .execute(&mut **tx)
        .await?;

    sqlx::query!("DELETE FROM dependency_map WHERE workspace_id = $1", w_id)
        .execute(&mut **tx)
        .await?;
    sqlx::query!("DELETE FROM v2_job_queue WHERE workspace_id = $1", w_id)
        .execute(&mut **tx)
        .await?;
    sqlx::query!("DELETE FROM v2_job WHERE workspace_id = $1", w_id)
        .execute(&mut **tx)
        .await?;
    sqlx::query!("DELETE FROM capture WHERE workspace_id = $1", w_id)
        .execute(&mut **tx)
        .await?;

    // capture_config has on delete cascade

    sqlx::query!("DELETE FROM draft WHERE workspace_id = $1", w_id)
        .execute(&mut **tx)
        .await?;
    sqlx::query!("DELETE FROM script WHERE workspace_id = $1", w_id)
        .execute(&mut **tx)
        .await?;
    sqlx::query!("DELETE FROM flow WHERE workspace_id = $1", w_id)
        .execute(&mut **tx)
        .await?;
    sqlx::query!("DELETE FROM app WHERE workspace_id = $1", w_id)
        .execute(&mut **tx)
        .await?;
    sqlx::query!("DELETE FROM raw_app WHERE workspace_id = $1", w_id)
        .execute(&mut **tx)
        .await?;
    sqlx::query!("DELETE FROM input WHERE workspace_id = $1", w_id)
        .execute(&mut **tx)
        .await?;
    sqlx::query!("DELETE FROM variable WHERE workspace_id = $1", w_id)
        .execute(&mut **tx)
        .await?;
    sqlx::query!("DELETE FROM resource WHERE workspace_id = $1", w_id)
        .execute(&mut **tx)
        .await?;

    sqlx::query!("DELETE FROM schedule WHERE workspace_id = $1", w_id)
        .execute(&mut **tx)
        .await?;

    sqlx::query!("DELETE FROM v2_job_completed WHERE workspace_id = $1", w_id)
        .execute(&mut **tx)
        .await?;

    sqlx::query!("DELETE FROM job_stats WHERE workspace_id = $1", w_id)
        .execute(&mut **tx)
        .await?;

    sqlx::query!(
        "DELETE FROM deployment_metadata WHERE workspace_id = $1",
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!("DELETE FROM usr WHERE workspace_id = $1", w_id)
        .execute(&mut **tx)
        .await?;

    sqlx::query!("DELETE FROM resource_type WHERE workspace_id = $1", w_id)
        .execute(&mut **tx)
        .await?;

    sqlx::query!("DELETE FROM workspace_invite WHERE workspace_id = $1", w_id)
        .execute(&mut **tx)
        .await?;

    sqlx::query!("DELETE FROM usr_to_group WHERE workspace_id = $1", w_id)
        .execute(&mut **tx)
        .await?;

    sqlx::query!("DELETE FROM group_ WHERE workspace_id = $1", w_id)
        .execute(&mut **tx)
        .await?;

    sqlx::query!("DELETE FROM folder WHERE workspace_id = $1", w_id)
        .execute(&mut **tx)
        .await?;

    sqlx::query!("DELETE FROM account WHERE workspace_id = $1", w_id)
        .execute(&mut **tx)
        .await?;

    sqlx::query!("DELETE FROM workspace_key WHERE workspace_id = $1", w_id)
        .execute(&mut **tx)
        .await?;

    sqlx::query!(
        "DELETE FROM workspace_settings WHERE workspace_id = $1",
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!("DELETE FROM token WHERE workspace_id = $1", w_id)
        .execute(&mut **tx)
        .await?;

    sqlx::query!("DELETE FROM http_trigger WHERE workspace_id = $1", w_id)
        .execute(&mut **tx)
        .await?;

    sqlx::query!(
        "DELETE FROM websocket_trigger WHERE workspace_id = $1",
        w_id
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!("DELETE FROM kafka_trigger WHERE workspace_id = $1", w_id)
        .execute(&mut **tx)
        .await?;

    // NATS triggers have on delete cascade

    sqlx::query!("DELETE FROM workspace WHERE id = $1", w_id)
        .execute(&mut **tx)
        .await?;

    Ok(())
}

pub(crate) async fn delete_workspace(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    authed: ApiAuthed,
    Query(dwq): Query<DeleteWorkspaceQuery>,
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

    if dwq.only_delete_forks.unwrap_or(false) && !w_id.starts_with(WM_FORK_PREFIX) {
        return Err(Error::BadRequest(
            "Cannot delete this workspace because it is not a workspace fork.".to_string(),
        ));
    }

    let mut tx = db.begin().await?;
    if !(w_id.starts_with(WM_FORK_PREFIX) && is_workspace_owner(&authed, &w_id, &mut tx).await?) {
        require_super_admin(&db, &authed.email).await?;
    }

    delete_workspace_tables(&mut tx, &w_id).await?;

    audit_log(
        &mut *tx,
        &authed,
        "workspaces.delete",
        ActionKind::Delete,
        &w_id,
        Some(&authed.email),
        None,
    )
    .await?;
    tx.commit().await?;

    Ok(format!("Deleted workspace {}", &w_id))
}

async fn is_workspace_owner(
    authed: &ApiAuthed,
    w_id: &str,
    tx: &mut Transaction<'_, Postgres>,
) -> Result<bool> {
    let owner = sqlx::query_scalar!("SELECT owner FROM workspace WHERE id = $1", w_id)
        .fetch_optional(&mut **tx)
        .await?;
    Ok(owner.map(|o| o == authed.email).unwrap_or(false))
}

#[inline]
pub async fn is_allowed_to_migrate(db: &DB, authed: &ApiAuthed, predicate: bool) -> Result<()> {
    if *CLOUD_HOSTED && !is_super_admin_email(db, &authed.email).await? {
        return Err(Error::BadRequest(
            "This feature is not available on the cloud".to_string(),
        ));
    }

    if predicate {
        require_super_admin(db, &authed.email).await?;
    } else {
        require_admin(authed.is_admin, &authed.username)?;
    }

    Ok(())
}

pub async fn migrate_workspace(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Json(req): Json<MigrateWorkspaceRequest>,
) -> Result<String> {
    is_allowed_to_migrate(&db, &authed, *CREATE_WORKSPACE_REQUIRE_SUPERADMIN).await?;

    let mut tx = db.begin().await?;

    check_w_id_conflict(&mut tx, &req.target_workspace_id).await?;

    sqlx::query!(
        r#"
            INSERT INTO
                workspace (id, name, owner, deleted, premium, parent_workspace_id)
            SELECT $1, $2, owner, deleted, premium, $3
            FROM 
                workspace 
            WHERE 
                id = $4
        "#,
        &req.target_workspace_id,
        &req.target_workspace_name,
        &req.source_workspace_id,
        &req.source_workspace_id,
    )
    .execute(&mut *tx)
    .await?;

    migrate_workspace_data_tables(&mut tx, &req.source_workspace_id, &req.target_workspace_id)
        .await?;

    audit_log(
        &mut *tx,
        &authed,
        "workspace.migrate",
        ActionKind::Update,
        &req.target_workspace_id,
        Some(&authed.email),
        Some(
            [
                ("source", req.source_workspace_id.as_str()),
                ("target", req.target_workspace_id.as_str()),
            ]
            .into(),
        ),
    )
    .await?;

    tx.commit().await?;

    Ok(format!(
        "Migrated from {} to {}",
        &req.source_workspace_id, &req.target_workspace_id
    ))
}

async fn migrate_workspace_data_tables(
    tx: &mut Transaction<'_, Postgres>,
    source: &str,
    target: &str,
) -> Result<()> {
    let non_auth_tables = [
        "account",
        "app",
        "audit",
        "capture",
        "capture_config",
        "http_trigger",
        "websocket_trigger",
        "kafka_trigger",
        "nats_trigger",
        "dependency_map",
        "deployment_metadata",
        "draft",
        "favorite",
        "asset",
        "folder",
        "input",
        "raw_app",
        "resource",
        "resource_type",
        "schedule",
        "script",
        "variable",
        "job_logs",
        "job_stats",
        "v2_job_queue",
    ];

    for table in non_auth_tables {
        sqlx::query(&format!(
            r#"
                UPDATE 
                    {} 
                SET 
                    workspace_id = $1 
                WHERE 
                    workspace_id = $2
            "#,
            table
        ))
        .bind(target)
        .bind(source)
        .execute(&mut **tx)
        .await?;
    }

    sqlx::query!(
        "INSERT INTO flow
            (workspace_id, path, summary, description, archived, extra_perms, dependency_job, draft_only, tag, ws_error_handler_muted, dedicated_worker, timeout, visible_to_runner_only, on_behalf_of_email, concurrency_key, versions, value, schema, edited_by, edited_at)
        SELECT $1, path, summary, description, archived, extra_perms, dependency_job, draft_only, tag, ws_error_handler_muted, dedicated_worker, timeout, visible_to_runner_only, on_behalf_of_email, concurrency_key, versions, value, schema, edited_by, edited_at
            FROM flow WHERE workspace_id = $2",
        target,
        source
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!("DELETE FROM flow WHERE workspace_id = $1", source)
        .execute(&mut **tx)
        .await?;

    sqlx::query!(
        "UPDATE flow_version SET workspace_id = $1 WHERE workspace_id = $2",
        target,
        source
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        "UPDATE flow_node SET workspace_id = $1 WHERE workspace_id = $2",
        target,
        source
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        "UPDATE workspace_runnable_dependencies SET workspace_id = $1 WHERE workspace_id = $2",
        target,
        source
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        "INSERT INTO group_ SELECT $1, name, summary, extra_perms FROM group_ WHERE workspace_id = $2",
        target,
        source
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        "UPDATE usr_to_group SET workspace_id = $1 WHERE workspace_id = $2",
        target,
        source
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!("DELETE FROM group_ WHERE workspace_id = $1", source)
        .execute(&mut **tx)
        .await?;

    sqlx::query!(
        "UPDATE usage SET id = $1 WHERE id = $2 AND is_workspace = true",
        target,
        source
    )
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn migrate_auth_tables(
    tx: &mut Transaction<'_, Postgres>,
    source: &str,
    target: &str,
) -> Result<()> {
    let auth_tables = [
        "token",
        "usr",
        "workspace_env",
        "workspace_invite",
        "workspace_key",
        "workspace_settings",
    ];

    for table in auth_tables {
        sqlx::query(&format!(
            "UPDATE {} SET workspace_id = $1 WHERE workspace_id = $2",
            table
        ))
        .bind(target)
        .bind(source)
        .execute(&mut **tx)
        .await?;
    }

    Ok(())
}

#[derive(Deserialize)]
pub struct CompleteWorkspaceMigrationRequest {
    source_workspace_id: String,
    target_workspace_id: String,
}

pub async fn complete_workspace_migration(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Json(req): Json<CompleteWorkspaceMigrationRequest>,
) -> Result<String> {
    is_allowed_to_migrate(&db, &authed, *CREATE_WORKSPACE_REQUIRE_SUPERADMIN).await?;

    let mut tx = db.begin().await?;

    let source_workspace_id = sqlx::query_scalar!(
        "SELECT parent_workspace_id FROM workspace WHERE id = $1",
        &req.target_workspace_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .flatten();

    match source_workspace_id {
        Some(workspace) => {
            if workspace != req.source_workspace_id {
                return Err(Error::BadRequest(
                    "Invalid migration: target workspace was not created from source workspace"
                        .to_string(),
                ));
            }
        }
        None => {
            return Err(Error::BadRequest(
                "Target workspace does not exist".to_string(),
            ));
        }
    }

    migrate_auth_tables(&mut tx, &req.source_workspace_id, &req.target_workspace_id).await?;

    sqlx::query!(
        "UPDATE workspace SET parent_workspace_id = NULL WHERE id = $1",
        &req.target_workspace_id
    )
    .execute(&mut *tx)
    .await?;

    delete_workspace_tables(&mut tx, &req.source_workspace_id).await?;

    audit_log(
        &mut *tx,
        &authed,
        "workspace.complete_migration",
        ActionKind::Update,
        &req.target_workspace_id,
        Some(&authed.email),
        Some(
            [
                ("source", req.source_workspace_id.as_str()),
                ("target", req.target_workspace_id.as_str()),
            ]
            .into(),
        ),
    )
    .await?;

    tx.commit().await?;

    Ok(format!(
        "Completed migration from {} to {}",
        &req.source_workspace_id, &req.target_workspace_id
    ))
}

#[derive(Deserialize)]
pub struct RevertWorkspaceMigrationRequest {
    target_workspace_id: String,
}

pub async fn revert_workspace_migration(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(req): Json<RevertWorkspaceMigrationRequest>,
) -> Result<String> {
    is_allowed_to_migrate(&db, &authed, *CREATE_WORKSPACE_REQUIRE_SUPERADMIN).await?;

    let mut tx = db.begin().await?;

    let from_workspace_id = sqlx::query_scalar!(
        r#"
        SELECT 
            parent_workspace_id 
        FROM 
            workspace 
        WHERE 
            id = $1
        "#,
        &req.target_workspace_id,
    )
    .fetch_optional(&mut *tx)
    .await?
    .flatten();

    let source_workspace_id = match from_workspace_id {
        Some(source_workspace_id) => {
            if source_workspace_id != w_id {
                return Err(Error::BadRequest(
                    "No incomplete migration found for this workspace".to_string(),
                ));
            }
            source_workspace_id
        }
        None => {
            return Err(Error::BadRequest(
                "Target workspace does not exist".to_string(),
            ));
        }
    };

    migrate_workspace_data_tables(&mut tx, &req.target_workspace_id, &source_workspace_id).await?;

    sqlx::query!(
        r#"DELETE FROM workspace WHERE id = $1"#,
        req.target_workspace_id
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "workspace.revert_migration",
        ActionKind::Delete,
        &req.target_workspace_id,
        Some(&authed.email),
        Some(
            [
                ("source", source_workspace_id.as_str()),
                ("target", req.target_workspace_id.as_str()),
            ]
            .into(),
        ),
    )
    .await?;

    tx.commit().await?;

    Ok(format!(
        "Reverted migration for workspace {}",
        &req.target_workspace_id
    ))
}

pub async fn get_incomplete_migration(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> JsonResult<Option<String>> {
    require_admin(authed.is_admin, &authed.username)?;

    let target_workspace = sqlx::query_scalar!(
        r#"
        SELECT 
            w.id 
        FROM 
            workspace w 
        LEFT JOIN usr u ON u.workspace_id = w.id
        WHERE 
            w.parent_workspace_id = $1 
        AND u.workspace_id IS NULL
        "#,
        w_id
    )
    .fetch_optional(&db)
    .await?;

    Ok(Json(target_workspace))
}

const DEFAULT_BATCH_SIZE: i64 = 10000;

#[derive(Serialize)]
pub struct MigrateJobsBatchResponse {
    migrated_count: i64,
}

#[derive(Serialize)]
pub struct MigrationStatus {
    processed_jobs: i64,
}

pub async fn get_migration_status(
    authed: ApiAuthed,
    Query(params): Query<std::collections::HashMap<String, String>>,
    Extension(db): Extension<DB>,
) -> JsonResult<MigrationStatus> {
    require_admin(authed.is_admin, &authed.username)?;

    let source_workspace = params
        .get("source_workspace")
        .ok_or_else(|| Error::BadRequest("source_workspace parameter required".to_string()))?;

    let source_jobs = sqlx::query_scalar!(
        "SELECT 
            COUNT(*) 
        FROM (
            SELECT id FROM v2_job WHERE workspace_id = $1
            UNION ALL
            SELECT id FROM v2_job_completed WHERE workspace_id = $1
        ) as total_jobs",
        source_workspace
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(0);

    Ok(Json(MigrationStatus { processed_jobs: source_jobs }))
}

pub async fn migrate_jobs(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Json(req): Json<MigrateJobRequest>,
) -> JsonResult<MigrateJobsBatchResponse> {
    is_allowed_to_migrate(&db, &authed, *MIGRATE_JOBS_WORKSPACE_REQUIRE_SUPERADMIN).await?;

    let mut tx = user_db.begin(&authed).await?;

    let migrated_count = sqlx::query_scalar!(
        "WITH 
            batch_completed_jobs AS (
                SELECT id FROM v2_job_completed
                WHERE workspace_id = $1
                LIMIT $2
            ),
            batch_jobs AS (
                SELECT id FROM v2_job
                WHERE workspace_id = $1
                LIMIT $2
            ),
            updated_completed AS (
                UPDATE v2_job_completed
                SET workspace_id = $3
                WHERE id IN (SELECT id FROM batch_completed_jobs)
                RETURNING 1
            ),
            updated_jobs AS (
                UPDATE v2_job
                SET workspace_id = $3
                WHERE id IN (SELECT id FROM batch_jobs)
                RETURNING 1
            )
            SELECT 
                (SELECT COUNT(*) FROM updated_completed) + 
                (SELECT COUNT(*) FROM updated_jobs) as total_count",
        req.source_workspace_id,
        req.batch_size,
        req.target_workspace_id
    )
    .fetch_one(&mut *tx)
    .await?
    .unwrap_or(0);

    tx.commit().await?;

    Ok(Json(MigrateJobsBatchResponse { migrated_count }))
}
