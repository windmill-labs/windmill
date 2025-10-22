use crate::db::ApiAuthed;

use crate::workspaces::{check_w_id_conflict, CREATE_WORKSPACE_REQUIRE_SUPERADMIN, WM_FORK_PREFIX};
use crate::{db::DB, utils::require_super_admin};

use axum::extract::Query;
use axum::{
    extract::{Extension, Path},
    Json,
};

use sqlx::{Postgres, Transaction};
use windmill_audit::audit_oss::audit_log;
use windmill_audit::ActionKind;

use windmill_common::error::JsonResult;
use windmill_common::worker::CLOUD_HOSTED;

use windmill_common::{
    auth::is_super_admin_email,
    error::{Error, Result},
    utils::require_admin,
};

use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct MigrateWorkspaceRequest {
    source_workspace_id: String,
    target_workspace_name: String,
    target_workspace_id: String,
    migration_type: MigrationType,
    #[serde(default = "default_disable_workspace")]
    disable_workspace: bool,
}

fn default_disable_workspace() -> bool {
    true
}

#[derive(Default, Deserialize, Serialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum MigrationType {
    #[default]
    All,
    Metadata,
    Jobs,
}

#[derive(Deserialize)]
pub(crate) struct DeleteWorkspaceQuery {
    pub(crate) only_delete_forks: Option<bool>,
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

    sqlx::query!("DELETE FROM dependency_map WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query!("DELETE FROM v2_job_queue WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query!("DELETE FROM v2_job WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query!("DELETE FROM capture WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;

    // capture_config has on delete cascade

    sqlx::query!("DELETE FROM draft WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query!("DELETE FROM script WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query!("DELETE FROM flow WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query!("DELETE FROM app WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query!("DELETE FROM raw_app WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query!("DELETE FROM input WHERE workspace_id = $1", &w_id)
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

    sqlx::query!(
        "DELETE FROM v2_job_completed WHERE workspace_id = $1",
        &w_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!("DELETE FROM job_stats WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query!(
        "DELETE FROM deployment_metadata WHERE workspace_id = $1",
        &w_id
    )
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

    sqlx::query!("DELETE FROM account WHERE workspace_id = $1", &w_id)
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

    sqlx::query!("DELETE FROM token WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query!("DELETE FROM http_trigger WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query!(
        "DELETE FROM websocket_trigger WHERE workspace_id = $1",
        &w_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!("DELETE FROM kafka_trigger WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;

    // NATS triggers have on delete cascade

    sqlx::query!("DELETE FROM workspace WHERE id = $1", &w_id)
        .execute(&mut *tx)
        .await?;

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

pub async fn migrate_workspace(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Json(req): Json<MigrateWorkspaceRequest>,
) -> Result<String> {
    if *CLOUD_HOSTED && !is_super_admin_email(&db, &authed.email).await? {
        return Err(Error::BadRequest(
            "This feature is not available on the cloud".to_string(),
        ));
    }

    if *CREATE_WORKSPACE_REQUIRE_SUPERADMIN {
        require_super_admin(&db, &authed.email).await?;
    } else {
        require_admin(authed.is_admin, &authed.username)?;
    }

    require_super_admin(&db, &authed.email).await?;

    let mut tx = db.begin().await?;

    if req.migration_type != MigrationType::Jobs {
        check_w_id_conflict(&mut tx, &req.target_workspace_id).await?;

        sqlx::query!(
            "INSERT INTO
                workspace SELECT $1, $2, owner, deleted, premium FROM workspace WHERE id = $3",
            &req.target_workspace_id,
            &req.target_workspace_name,
            &req.source_workspace_id
        )
        .execute(&mut *tx)
        .await?;
    }

    match req.migration_type {
        MigrationType::Metadata | MigrationType::All => {
            migrate_metadata_tables(&mut tx, &req.source_workspace_id, &req.target_workspace_id)
                .await?;
        }
        _ => {}
    }

    match req.migration_type {
        MigrationType::Jobs | MigrationType::All => {
            let result = migrate_jobs_batch(
                &mut tx,
                &req.source_workspace_id,
                &req.target_workspace_id,
                DEFAULT_BATCH_SIZE,
            )
            .await?;
        }
        _ => {}
    }

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
                (
                    "type",
                    match req.migration_type {
                        MigrationType::All => "all",
                        MigrationType::Metadata => "metadata",
                        MigrationType::Jobs => "jobs",
                    },
                ),
            ]
            .into(),
        ),
    )
    .await?;

    tx.commit().await?;

    Ok(format!(
        "Migrated {} from {} to {}",
        match req.migration_type {
            MigrationType::All => "all data",
            MigrationType::Metadata => "metadata",
            MigrationType::Jobs => "jobs",
        },
        &req.source_workspace_id,
        &req.target_workspace_id
    ))
}

async fn migrate_metadata_tables(
    tx: &mut Transaction<'_, Postgres>,
    source: &str,
    target: &str,
) -> Result<()> {
    let simple_tables = vec![
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
        "flow_version",
        "workspace_runnable_dependencies",
        "asset",
        "flow_node",
        "folder",
        "input",
        "raw_app",
        "resource",
        "resource_type",
        "schedule",
        "script",
        "token",
        "usr",
        "variable",
        "workspace_env",
        "workspace_invite",
        "workspace_key",
        "workspace_settings",
    ];

    for table in simple_tables {
        sqlx::query(&format!(
            "UPDATE {} SET workspace_id = $1 WHERE workspace_id = $2",
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

async fn migrate_job_tables(
    tx: &mut Transaction<'_, Postgres>,
    source: &str,
    target: &str,
) -> Result<()> {
    sqlx::query!(
        "UPDATE job_logs SET workspace_id = $1 WHERE workspace_id = $2",
        target,
        source
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        "UPDATE job_stats SET workspace_id = $1 WHERE workspace_id = $2",
        target,
        source
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        "UPDATE v2_job_queue SET workspace_id = $1 WHERE workspace_id = $2",
        target,
        source
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        "UPDATE v2_job SET workspace_id = $1 WHERE workspace_id = $2",
        target,
        source
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query!(
        "UPDATE v2_job_completed SET workspace_id = $1 WHERE workspace_id = $2",
        target,
        source
    )
    .execute(&mut **tx)
    .await?;

    Ok(())
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
        "SELECT COUNT(*) FROM v2_job_completed WHERE workspace_id = $1",
        source_workspace
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(0);

    Ok(Json(MigrationStatus { processed_jobs: source_jobs }))
}

async fn migrate_jobs_batch(
    tx: &mut Transaction<'_, Postgres>,
    source_workspace_id: &str,
    target_workspace_id: &str,
    batch_size: i64,
) -> Result<MigrateJobsBatchResponse> {
    let migrated_count = sqlx::query_scalar!(
        "WITH batch AS (
                SELECT id FROM v2_job_completed
                WHERE workspace_id = $1
                LIMIT $2
            )
            UPDATE v2_job_completed
            SET workspace_id = $3
            WHERE id IN (SELECT id FROM batch)
            RETURNING 1",
        source_workspace_id,
        batch_size,
        target_workspace_id
    )
    .fetch_all(&mut **tx)
    .await?
    .len() as i64;

    Ok(MigrateJobsBatchResponse { migrated_count })
}
