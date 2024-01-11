use crate::{db::ApiAuthed, utils::require_super_admin};
use axum::{
    extract::Path,
    routing::{delete, get, post},
    Extension, Json, Router,
};

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use windmill_common::{error, utils::not_found_if_none, DB};

pub fn global_service() -> Router {
    Router::new()
        .route("/get/:name", get(get_group))
        .route("/create", post(create_group))
        .route("/update/:name", post(update_group))
        .route("/delete/:name", delete(delete_group))
}

#[derive(Serialize, Deserialize, FromRow)]
struct InstanceGroup {
    name: String,
    summary: Option<String>,
    external_id: Option<String>,
}

#[derive(Deserialize)]
struct InstanceGroupUpdate {
    summary_update: Option<String>,
    external_id_update: Option<String>,
}

async fn get_group(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(name): Path<String>,
) -> error::JsonResult<InstanceGroup> {
    require_super_admin(&db, &authed.email).await?;

    let group_opt = sqlx::query_as!(
        InstanceGroup,
        "SELECT * FROM instance_group WHERE name = $1",
        name
    )
    .fetch_optional(&db)
    .await?;

    let group = not_found_if_none(group_opt, "instance_group", name)?;

    Ok(Json(group))
}

async fn create_group(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Json(group): Json<InstanceGroup>,
) -> error::JsonResult<()> {
    require_super_admin(&db, &authed.email).await?;

    let name = group.name.clone();

    let exists = sqlx::query_scalar!("SELECT true FROM instance_group WHERE name = $1", name)
        .fetch_optional(&db)
        .await?
        .flatten();

    if exists.is_some() {
        return Err(error::Error::BadRequest(
            format!("instance_group {} already exists", name).to_string(),
        ));
    }

    sqlx::query!(
        "INSERT INTO instance_group (name, summary, external_id) VALUES ($1, $2, $3)",
        name,
        group.summary,
        group
            .external_id
            .unwrap_or("created_via_rest_api".to_string())
    )
    .execute(&db)
    .await?;

    Ok(Json(()))
}

async fn update_group(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(name): Path<String>,
    Json(group_update): Json<InstanceGroupUpdate>,
) -> error::JsonResult<InstanceGroup> {
    require_super_admin(&db, &authed.email).await?;

    let existing_group_opt =
        sqlx::query_as::<_, InstanceGroup>("SELECT * FROM instance_group WHERE name = $1")
            .bind(name.clone())
            .fetch_optional(&db)
            .await?;
    let existing_group = not_found_if_none(existing_group_opt, "instance_group", name.clone())?;

    let updated_group = sqlx::query_as::<_, InstanceGroup>(
        "UPDATE instance_group SET summary = $1, external_id = $2 WHERE name = $3 RETURNING *",
    )
    .bind(group_update.summary_update.and(existing_group.summary))
    .bind(
        group_update
            .external_id_update
            .and(existing_group.external_id),
    )
    .bind(name)
    .fetch_one(&db)
    .await?;

    Ok(Json(updated_group))
}

async fn delete_group(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(name): Path<String>,
) -> error::JsonResult<Option<InstanceGroup>> {
    require_super_admin(&db, &authed.email).await?;

    let deleted_group = sqlx::query_as::<_, InstanceGroup>(
        "DELETE FROM instance_group WHERE name = $1 RETURNING *",
    )
    .bind(name)
    .fetch_optional(&db)
    .await?;

    Ok(Json(deleted_group))
}
