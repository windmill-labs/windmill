use axum::{
    extract::Path,
    routing::{get, post},
    Extension, Json, Router,
};
use windmill_common::{
    db::UserDB,
    error::{Error, Result},
    workspaces::{dbt_warehouse_resource, DbtWarehouseConnection},
    DB,
};

use crate::db::{ApiAuthed, OptJobAuthed};
use windmill_api_auth::{is_no_auth, Tokened};

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/warehouse/{name}", get(get_warehouse))
        .route("/warehouse_exists/{name}", get(warehouse_exists))
        .route("/run_progress", post(record_run_progress))
}

async fn get_warehouse(
    OptJobAuthed { job_id, authed }: OptJobAuthed,
    Tokened { token }: Tokened,
    Extension(db): Extension<DB>,
    Extension(_user_db): Extension<UserDB>,
    Path((w_id, name)): Path<(String, String)>,
) -> Result<Json<DbtWarehouseConnection>> {
    // Scoped to a running DBT job, and the reason it must stay that way: the
    // response carries the warehouse's credentials. A dbt job already holds them
    // in its rendered `profiles.yml`, so serving them changes nothing for it —
    // but every script job's token carries a job id too, and any other language
    // asking for them would be reading a credential it was never given.
    // In no-auth mode every request is the synthetic superadmin and carries no
    // job, so the scoping below has nothing to check. Refusing there would make
    // dbt unusable on an instance that has deliberately turned auth off, and
    // there is no credential boundary left to protect.
    let Some(job_id) = job_id else {
        if is_no_auth() {
            // Validated here too, so all three paths agree on what a name may be
            // rather than one of them accepting whatever the URL carried.
            windmill_common::workspaces::validate_dbt_warehouse_name(&name)?;
            let (resource_path, target) = dbt_warehouse_resource(&db, &w_id, &name).await?;
            let value = windmill_store::resources::get_resource_value_interpolated_internal(
                &windmill_common::db::DbWithOptAuthed::<ApiAuthed>::from_authed(
                    &authed,
                    db.clone(),
                    None,
                ),
                &w_id,
                &resource_path,
                None,
                Some(&token),
                false,
            )
            .await?
            .ok_or_else(|| {
                Error::NotFound(format!(
                    "the dbt warehouse `{name}` points at `{resource_path}`, which does not exist"
                ))
            })?;
            return Ok(Json(DbtWarehouseConnection { value, target }));
        }
        return Err(Error::BadRequest(
            "this route resolves a dbt warehouse for a running job and needs a job token"
                .to_string(),
        ));
    };
    let is_dbt = sqlx::query_scalar!(
        "SELECT script_lang = 'dbt' FROM v2_job WHERE id = $1 AND workspace_id = $2",
        job_id,
        &w_id
    )
    .fetch_optional(&db)
    .await?
    .flatten()
    .unwrap_or(false);
    if !is_dbt {
        return Err(Error::NotAuthorized(
            "only a dbt job may resolve a dbt warehouse".to_string(),
        ));
    }
    windmill_common::workspaces::validate_dbt_warehouse_name(&name)?;
    let (resource_path, target) = dbt_warehouse_resource(&db, &w_id, &name).await?;
    // Interpolated AGAINST THE JOB, so a warehouse whose resource carries
    // `$WM_TOKEN` or another `$WM_*` renders what the job would see rather than
    // the literal placeholder. Unchecked (no `user_db`) because dbt warehouses
    // are unpermissioned by design; uncached because a job-context value must
    // not be served to the next job.
    let value = windmill_store::resources::get_resource_value_interpolated_internal(
        &windmill_common::db::DbWithOptAuthed::<ApiAuthed>::from_authed(&authed, db.clone(), None),
        &w_id,
        &resource_path,
        Some(job_id),
        Some(&token),
        false,
    )
    .await?
    .ok_or_else(|| {
        Error::NotFound(format!(
            "the dbt warehouse `{name}` points at `{resource_path}`, which does not exist"
        ))
    })?;
    Ok(Json(DbtWarehouseConnection { value, target }))
}

/// A settled node's state, for a worker that cannot write the database.
///
/// The job is taken from the TOKEN, never the body: a job may report its own
/// progress and no one else's.
async fn record_run_progress(
    OptJobAuthed { job_id, .. }: OptJobAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(req): Json<windmill_common::dbt_manifest::DbtRunProgressRequest>,
) -> Result<()> {
    let Some(job_id) = job_id else {
        return Err(Error::BadRequest(
            "this route records a running job's dbt progress and needs a job token".to_string(),
        ));
    };
    windmill_common::dbt_manifest::record_run_progress(
        &db,
        &w_id,
        &job_id,
        &req.asset_path,
        req.status,
        req.row_count,
        req.error.as_deref(),
    )
    .await;
    Ok(())
}

/// Whether the workspace configures this warehouse. NOTHING is resolved.
///
/// A project that brings its own `profiles.yml` names a warehouse to say where
/// its assets belong and never opens it, so the answer it needs is a yes/no —
/// decrypting a connection for that would hand out a credential the run has no
/// use for.
async fn warehouse_exists(
    OptJobAuthed { job_id, .. }: OptJobAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, name)): Path<(String, String)>,
) -> Result<()> {
    // Same as above: no-auth mode carries no job, and this answers yes/no about
    // a workspace setting rather than handing anything over.
    if job_id.is_none() && !is_no_auth() {
        return Err(Error::BadRequest(
            "this route answers for a running job and needs a job token".to_string(),
        ));
    }
    windmill_common::workspaces::validate_dbt_warehouse_name(&name)?;
    windmill_common::workspaces::dbt_warehouse_exists(&db, &w_id, &name).await
}
