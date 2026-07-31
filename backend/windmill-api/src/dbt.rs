use axum::{extract::Path, routing::{get, post}, Extension, Json, Router};
use windmill_common::{
    db::UserDB,
    error::{Error, Result},
    workspaces::{dbt_warehouse_connection, DbtWarehouseConnection},
    DB,
};

use crate::db::OptJobAuthed;

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/warehouse/{name}", get(get_warehouse))
        .route("/run_progress", post(record_run_progress))
}

async fn get_warehouse(
    OptJobAuthed { job_id, .. }: OptJobAuthed,
    Extension(db): Extension<DB>,
    Extension(_user_db): Extension<UserDB>,
    Path((w_id, name)): Path<(String, String)>,
) -> Result<Json<DbtWarehouseConnection>> {
    // Job-scoped, and the reason it must stay that way: the response carries the
    // warehouse's credentials, which a running job already holds in its rendered
    // `profiles.yml`. A browsable route would hand them to anyone.
    if job_id.is_none() {
        return Err(Error::BadRequest(
            "this route resolves a dbt warehouse for a running job and needs a job token"
                .to_string(),
        ));
    }
    windmill_common::workspaces::validate_dbt_warehouse_name(&name)?;
    Ok(Json(dbt_warehouse_connection(&db, &w_id, &name).await?))
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
