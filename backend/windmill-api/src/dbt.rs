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
            let resource_type = warehouse_resource_type(&db, &w_id, &resource_path).await?;
            return Ok(Json(DbtWarehouseConnection { value, target, resource_type }));
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
    let resource_type = warehouse_resource_type(&db, &w_id, &resource_path).await?;
    Ok(Json(DbtWarehouseConnection { value, target, resource_type }))
}

/// A warehouse resource's type, which decides whether its value is translated
/// into a `profiles.yml` target or taken as one. Read separately from the value
/// because the interpolating loader returns the value alone.
async fn warehouse_resource_type(db: &DB, w_id: &str, path: &str) -> Result<String> {
    sqlx::query_scalar!(
        "SELECT resource_type FROM resource WHERE workspace_id = $1 AND path = $2",
        w_id,
        path
    )
    .fetch_optional(db)
    .await?
    .ok_or_else(|| Error::NotFound(format!("the dbt warehouse points at `{path}`, which does not exist")))
}

/// A settled node's state, for a worker that cannot write the database.
///
/// The job is taken from the TOKEN, never the body: a job may report its own
/// progress and no one else's.
///
/// NO no-auth branch, unlike its two siblings: they need only the warehouse
/// NAME, which the URL carries, while this needs the job — and the job lives in
/// the token's JWT claims, which the no-auth path never decodes. So on a
/// no-auth instance an agent worker's per-model rows do not appear. The run is
/// unaffected: the worker treats a failed post as a display problem and builds
/// the models regardless. Taking the job from the body instead would hand any
/// caller another job's run page.
async fn record_run_progress(
    OptJobAuthed { job_id, .. }: OptJobAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(rows): Json<Vec<windmill_common::dbt_manifest::DbtRunProgressRequest>>,
) -> Result<()> {
    let Some(job_id) = job_id else {
        return Err(Error::BadRequest(
            "this route records a running job's dbt progress and needs a job token".to_string(),
        ));
    };
    // A DBT job's, like its sibling above. Writing nothing secret, but a row
    // keyed to another language's job is a run-page entry for a run that has no
    // models, and the two routes disagreeing on who may call them is how one of
    // them ends up wrong later.
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
            "only a dbt job may record dbt run progress".to_string(),
        ));
    }
    // The agent's own sweep: these rows have no job foreign key, and the
    // worker-side prune runs only where the pool is reachable, so an
    // agent-only workspace would accumulate one row per model forever.
    windmill_common::dbt_manifest::prune_run_progress(&db, &w_id).await;
    // A run's nodes arrive together; the writes are local to this server, so the
    // loop that would have been a round trip each is a statement each.
    for req in &rows {
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
    }
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
    // A DBT job's token, like both siblings. No-auth mode carries no job at all,
    // and there the whole instance is unauthenticated.
    if !is_no_auth() {
        let Some(job_id) = job_id else {
            return Err(Error::BadRequest(
                "this route answers for a running job and needs a job token".to_string(),
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
                "only a dbt job may ask about a dbt warehouse".to_string(),
            ));
        }
    }
    windmill_common::workspaces::validate_dbt_warehouse_name(&name)?;
    // Mapped to a bare yes/no: the resolver's miss lists every warehouse the
    // workspace configures, which is a useful hint to an admin editing settings
    // and a needless disclosure to a job that only asked about one name.
    windmill_common::workspaces::dbt_warehouse_exists(&db, &w_id, &name)
        .await
        .map_err(|_| Error::NotFound(format!("no dbt warehouse named `{name}` in this workspace")))
}
