use axum::{extract::Path, routing::get, Extension, Json, Router};
use windmill_common::{
    db::UserDB,
    error::{Error, Result},
    workspaces::{dbt_warehouse_connection, DbtWarehouseConnection},
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
