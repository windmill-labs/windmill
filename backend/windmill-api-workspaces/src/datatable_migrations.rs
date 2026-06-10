use axum::{
    extract::{Extension, Path},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use windmill_api_auth::ApiAuthed;
use windmill_common::datatable_migrations::{
    apply_pending_migrations, list_applied_migrations, MigrationToApply,
};
use windmill_common::db::UserDB;
use windmill_common::error::{Error, JsonResult, Result};
use windmill_common::utils::require_admin;
use windmill_common::DB;

use crate::workspaces::resolve_pg_source_checked;

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/{datatable}/list", get(list_migrations))
        .route("/{datatable}/create", post(create_migration))
        .route("/{datatable}/upsert", post(upsert_migration))
        .route("/{datatable}/delete", post(delete_migration))
        .route("/{datatable}/run", post(run_migrations))
}

fn checksum(content: &str) -> String {
    let mut hasher = Sha256::new();
    // Trailing whitespace is not significant (editors append final newlines);
    // ignoring it avoids spurious drift detection on round-trips through files.
    hasher.update(content.trim_end().as_bytes());
    hex::encode(hasher.finalize())
}

/// Generate a sortable version string from the current time (millisecond precision).
fn generate_version() -> String {
    Utc::now().format("%Y%m%d%H%M%S%3f").to_string()
}

#[derive(Serialize)]
struct MigrationInfo {
    version: String,
    name: String,
    checksum: String,
    content: String,
    created_at: chrono::DateTime<Utc>,
    created_by: String,
    /// Whether this migration has been applied to the datatable database.
    applied: bool,
    /// Applied but the stored content's checksum differs from what was applied
    /// (i.e. the migration was edited after being run).
    drifted: bool,
}

#[derive(Serialize)]
struct ListMigrationsResponse {
    migrations: Vec<MigrationInfo>,
    /// Set when the applied state could not be read from the datatable database.
    applied_state_error: Option<String>,
}

async fn list_migrations(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, datatable)): Path<(String, String)>,
) -> JsonResult<ListMigrationsResponse> {
    let rows = sqlx::query!(
        "SELECT version, name, checksum, content, created_at, created_by
         FROM datatable_migration
         WHERE workspace_id = $1 AND datatable = $2
         ORDER BY version",
        &w_id,
        &datatable,
    )
    .fetch_all(&db)
    .await?;

    // Best-effort: read which migrations have been applied to the datatable DB.
    let source = format!("datatable://{datatable}");
    let (applied_map, applied_state_error) =
        match resolve_pg_source_checked(&db, &user_db, &authed, &w_id, &source).await {
            Ok(pg) => match list_applied_migrations(&pg, Some(&db)).await {
                Ok(applied) => (
                    applied
                        .into_iter()
                        .map(|a| (a.version, a.checksum))
                        .collect::<HashMap<_, _>>(),
                    None,
                ),
                Err(e) => (HashMap::new(), Some(e.to_string())),
            },
            Err(e) => (HashMap::new(), Some(e.to_string())),
        };

    let migrations = rows
        .into_iter()
        .map(|r| {
            let applied_checksum = applied_map.get(&r.version);
            MigrationInfo {
                applied: applied_checksum.is_some(),
                drifted: applied_checksum
                    .map(|c| !c.is_empty() && c != &r.checksum)
                    .unwrap_or(false),
                version: r.version,
                name: r.name,
                checksum: r.checksum,
                content: r.content,
                created_at: r.created_at,
                created_by: r.created_by,
            }
        })
        .collect();

    Ok(Json(ListMigrationsResponse {
        migrations,
        applied_state_error,
    }))
}

#[derive(Deserialize)]
struct CreateMigrationRequest {
    name: Option<String>,
    content: String,
    /// Optional explicit version; generated from the current time if omitted.
    version: Option<String>,
}

#[derive(Serialize)]
struct CreateMigrationResponse {
    version: String,
}

async fn create_migration(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, datatable)): Path<(String, String)>,
    Json(req): Json<CreateMigrationRequest>,
) -> JsonResult<CreateMigrationResponse> {
    require_admin(authed.is_admin, &authed.username)?;
    let version = req.version.unwrap_or_else(generate_version);
    let name = req.name.unwrap_or_default();
    let checksum = checksum(&req.content);

    sqlx::query!(
        "INSERT INTO datatable_migration (workspace_id, datatable, version, name, content, checksum, created_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
        &w_id,
        &datatable,
        &version,
        &name,
        &req.content,
        &checksum,
        &authed.username,
    )
    .execute(&db)
    .await
    .map_err(|e| Error::internal_err(format!("Failed to create migration: {e}")))?;

    Ok(Json(CreateMigrationResponse { version }))
}

#[derive(Deserialize)]
struct UpsertMigrationRequest {
    version: String,
    name: Option<String>,
    content: String,
}

/// Create or update a migration by version. Used by the CLI when pushing
/// `_datatable_migrations/*.sql` files.
async fn upsert_migration(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, datatable)): Path<(String, String)>,
    Json(req): Json<UpsertMigrationRequest>,
) -> Result<String> {
    require_admin(authed.is_admin, &authed.username)?;
    let name = req.name.unwrap_or_default();
    let checksum = checksum(&req.content);

    sqlx::query!(
        "INSERT INTO datatable_migration (workspace_id, datatable, version, name, content, checksum, created_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         ON CONFLICT (workspace_id, datatable, version)
         DO UPDATE SET name = EXCLUDED.name, content = EXCLUDED.content, checksum = EXCLUDED.checksum",
        &w_id,
        &datatable,
        &req.version,
        &name,
        &req.content,
        &checksum,
        &authed.username,
    )
    .execute(&db)
    .await
    .map_err(|e| Error::internal_err(format!("Failed to upsert migration: {e}")))?;

    Ok(format!("Upserted migration {}", req.version))
}

#[derive(Deserialize)]
struct DeleteMigrationRequest {
    version: String,
}

async fn delete_migration(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, datatable)): Path<(String, String)>,
    Json(req): Json<DeleteMigrationRequest>,
) -> Result<String> {
    require_admin(authed.is_admin, &authed.username)?;
    sqlx::query!(
        "DELETE FROM datatable_migration WHERE workspace_id = $1 AND datatable = $2 AND version = $3",
        &w_id,
        &datatable,
        &req.version,
    )
    .execute(&db)
    .await?;
    Ok(format!("Deleted migration {}", req.version))
}

#[derive(Serialize)]
struct RunMigrationsResponse {
    applied: Vec<String>,
}

/// Apply all not-yet-applied migrations to the datatable database, in version order.
async fn run_migrations(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, datatable)): Path<(String, String)>,
) -> JsonResult<RunMigrationsResponse> {
    require_admin(authed.is_admin, &authed.username)?;

    let rows = sqlx::query!(
        "SELECT version, name, content, checksum
         FROM datatable_migration
         WHERE workspace_id = $1 AND datatable = $2
         ORDER BY version",
        &w_id,
        &datatable,
    )
    .fetch_all(&db)
    .await?;

    let migrations: Vec<MigrationToApply> = rows
        .into_iter()
        .map(|r| MigrationToApply {
            version: r.version,
            name: r.name,
            content: r.content,
            checksum: r.checksum,
        })
        .collect();

    let source = format!("datatable://{datatable}");
    let pg = resolve_pg_source_checked(&db, &user_db, &authed, &w_id, &source).await?;
    let applied = apply_pending_migrations(&pg, &migrations, Some(&db)).await?;

    Ok(Json(RunMigrationsResponse { applied }))
}
