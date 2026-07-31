use axum::{extract::Path, routing::get, Extension, Json, Router};
use windmill_common::{
    db::UserDB,
    error::{Error, Result},
    workspaces::{dbt_warehouse_resource, DbtWarehouseRef},
    DB,
};

use crate::db::OptJobAuthed;

pub fn workspaced_service() -> Router {
    Router::new().route("/warehouse/{name}", get(get_warehouse))
}

async fn get_warehouse(
    OptJobAuthed { job_id, .. }: OptJobAuthed,
    Extension(db): Extension<DB>,
    Extension(_user_db): Extension<UserDB>,
    Path((w_id, name)): Path<(String, String)>,
) -> Result<Json<DbtWarehouseRef>> {
    // Job-scoped: a workspace's warehouse names are a running job's business,
    // not a browsable list.
    if job_id.is_none() {
        return Err(Error::BadRequest(
            "this route resolves a dbt warehouse for a running job and needs a job token"
                .to_string(),
        ));
    }
    windmill_common::workspaces::validate_dbt_warehouse_name(&name)?;
    let (resource_path, target) = dbt_warehouse_resource(&db, &w_id, &name).await?;
    Ok(Json(DbtWarehouseRef { resource_path, target }))
}
