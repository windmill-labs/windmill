use axum::{
    extract::{Extension, Json, Path, Query},
    routing::{delete, get, post},
    Router,
};
use serde::Deserialize;
use windmill_audit::{audit_oss::audit_log, ActionKind};
use windmill_common::{
    db::UserDB,
    error::{Error, Result},
    trashbin::{self, TrashItem, TrashItemWithData},
    utils::require_admin,
};

use crate::db::{ApiAuthed, DB};

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list", get(list_trash))
        .route("/get/{id}", get(get_trash_item))
        .route("/restore/{id}", post(restore_trash_item))
        .route("/delete/{id}", delete(permanently_delete_item))
        .route("/empty", post(empty_trash))
}

#[derive(Deserialize)]
struct ListTrashQuery {
    item_kind: Option<String>,
    page: Option<i64>,
    per_page: Option<i64>,
}

async fn list_trash(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Query(query): Query<ListTrashQuery>,
) -> Result<Json<Vec<TrashItem>>> {
    require_admin(authed.is_admin, &authed.username)?;
    let items = trashbin::list_trash(
        &db,
        &w_id,
        query.item_kind.as_deref(),
        query.page,
        query.per_page,
    )
    .await?;
    Ok(Json(items))
}

async fn get_trash_item(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, id)): Path<(String, i64)>,
) -> Result<Json<TrashItemWithData>> {
    require_admin(authed.is_admin, &authed.username)?;
    let item = trashbin::get_trash_item(&db, &w_id, id).await?;
    Ok(Json(item))
}

async fn restore_trash_item(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, id)): Path<(String, i64)>,
) -> Result<String> {
    require_admin(authed.is_admin, &authed.username)?;
    let item = trashbin::get_trash_item(&db, &w_id, id).await?;
    let mut tx = user_db.begin(&authed).await?;

    match item.item_kind.as_str() {
        "script" => restore_script(&mut tx, &item).await?,
        "flow" => restore_flow(&mut tx, &item).await?,
        "app" => restore_app(&mut tx, &item).await?,
        "schedule" => restore_schedule(&mut tx, &item).await?,
        "variable" => restore_variable(&mut tx, &item).await?,
        "resource" => restore_resource(&mut tx, &item).await?,
        kind if kind.ends_with("_trigger") => restore_trigger(&mut tx, &item).await?,
        _ => {
            return Err(Error::BadRequest(format!(
                "Unknown item kind: {}",
                item.item_kind
            )))
        }
    }

    sqlx::query!("DELETE FROM trashbin WHERE id = $1", item.id)
        .execute(&mut *tx)
        .await?;

    audit_log(
        &mut *tx,
        &authed,
        "trash.restore",
        ActionKind::Create,
        &w_id,
        Some(&item.item_path),
        None,
    )
    .await?;

    tx.commit().await?;
    Ok(format!("{} '{}' restored", item.item_kind, item.item_path))
}

async fn permanently_delete_item(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, id)): Path<(String, i64)>,
) -> Result<String> {
    require_admin(authed.is_admin, &authed.username)?;
    trashbin::permanently_delete_item(&db, &w_id, id).await?;
    Ok("permanently deleted".to_string())
}

async fn empty_trash(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> Result<String> {
    require_admin(authed.is_admin, &authed.username)?;
    let count = trashbin::empty_trash(&db, &w_id).await?;
    Ok(format!("{} items permanently deleted", count))
}

// --- Restore functions per item kind ---

async fn restore_script(tx: &mut sqlx::PgConnection, item: &TrashItemWithData) -> Result<()> {
    let data = &item.item_data;

    // Check for path conflict
    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM script WHERE path = $1 AND workspace_id = $2)",
        &item.item_path,
        &item.workspace_id,
    )
    .fetch_one(&mut *tx)
    .await?
    .unwrap_or(false);

    if exists {
        return Err(Error::BadRequest(format!(
            "A script already exists at path '{}'",
            item.item_path
        )));
    }

    // Scripts are stored as an array (all versions for the path)
    let scripts = data
        .get("scripts")
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::internal_err("Invalid trash data for script"))?;

    for script in scripts {
        sqlx::query("INSERT INTO script SELECT * FROM jsonb_populate_record(null::script, $1)")
            .bind(script)
            .execute(&mut *tx)
            .await?;
    }

    // Restore drafts if present
    if let Some(drafts) = data.get("drafts").and_then(|v| v.as_array()) {
        for draft in drafts {
            sqlx::query(
                "INSERT INTO draft SELECT * FROM jsonb_populate_record(null::draft, $1)
                 ON CONFLICT DO NOTHING",
            )
            .bind(draft)
            .execute(&mut *tx)
            .await?;
        }
    }

    Ok(())
}

async fn restore_flow(tx: &mut sqlx::PgConnection, item: &TrashItemWithData) -> Result<()> {
    let data = &item.item_data;

    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM flow WHERE path = $1 AND workspace_id = $2)",
        &item.item_path,
        &item.workspace_id,
    )
    .fetch_one(&mut *tx)
    .await?
    .unwrap_or(false);

    if exists {
        return Err(Error::BadRequest(format!(
            "A flow already exists at path '{}'",
            item.item_path
        )));
    }

    let row = data
        .get("row")
        .ok_or_else(|| Error::internal_err("Invalid trash data for flow"))?;

    sqlx::query("INSERT INTO flow SELECT * FROM jsonb_populate_record(null::flow, $1)")
        .bind(row)
        .execute(&mut *tx)
        .await?;

    // Restore flow_versions
    if let Some(versions) = data.get("flow_versions").and_then(|v| v.as_array()) {
        for version in versions {
            sqlx::query(
                "INSERT INTO flow_version SELECT * FROM jsonb_populate_record(null::flow_version, $1)
                 ON CONFLICT DO NOTHING",
            )
            .bind(version)
            .execute(&mut *tx)
            .await?;
        }
    }

    // Restore flow_nodes
    if let Some(nodes) = data.get("flow_nodes").and_then(|v| v.as_array()) {
        for node in nodes {
            sqlx::query(
                "INSERT INTO flow_node SELECT * FROM jsonb_populate_record(null::flow_node, $1)
                 ON CONFLICT DO NOTHING",
            )
            .bind(node)
            .execute(&mut *tx)
            .await?;
        }
    }

    // Restore drafts
    if let Some(drafts) = data.get("drafts").and_then(|v| v.as_array()) {
        for draft in drafts {
            sqlx::query(
                "INSERT INTO draft SELECT * FROM jsonb_populate_record(null::draft, $1)
                 ON CONFLICT DO NOTHING",
            )
            .bind(draft)
            .execute(&mut *tx)
            .await?;
        }
    }

    Ok(())
}

async fn restore_app(tx: &mut sqlx::PgConnection, item: &TrashItemWithData) -> Result<()> {
    let data = &item.item_data;

    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM app WHERE path = $1 AND workspace_id = $2)",
        &item.item_path,
        &item.workspace_id,
    )
    .fetch_one(&mut *tx)
    .await?
    .unwrap_or(false);

    if exists {
        return Err(Error::BadRequest(format!(
            "An app already exists at path '{}'",
            item.item_path
        )));
    }

    let row = data
        .get("row")
        .ok_or_else(|| Error::internal_err("Invalid trash data for app"))?;

    // Restore app_versions first (app references them via FK)
    if let Some(versions) = data.get("app_versions").and_then(|v| v.as_array()) {
        for version in versions {
            sqlx::query(
                "INSERT INTO app_version SELECT * FROM jsonb_populate_record(null::app_version, $1)
                 ON CONFLICT DO NOTHING",
            )
            .bind(version)
            .execute(&mut *tx)
            .await?;
        }
    }

    sqlx::query("INSERT INTO app SELECT * FROM jsonb_populate_record(null::app, $1)")
        .bind(row)
        .execute(&mut *tx)
        .await?;

    // Restore drafts
    if let Some(drafts) = data.get("drafts").and_then(|v| v.as_array()) {
        for draft in drafts {
            sqlx::query(
                "INSERT INTO draft SELECT * FROM jsonb_populate_record(null::draft, $1)
                 ON CONFLICT DO NOTHING",
            )
            .bind(draft)
            .execute(&mut *tx)
            .await?;
        }
    }

    Ok(())
}

async fn restore_schedule(tx: &mut sqlx::PgConnection, item: &TrashItemWithData) -> Result<()> {
    let data = &item.item_data;

    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM schedule WHERE path = $1 AND workspace_id = $2)",
        &item.item_path,
        &item.workspace_id,
    )
    .fetch_one(&mut *tx)
    .await?
    .unwrap_or(false);

    if exists {
        return Err(Error::BadRequest(format!(
            "A schedule already exists at path '{}'",
            item.item_path
        )));
    }

    let row = data
        .get("row")
        .ok_or_else(|| Error::internal_err("Invalid trash data for schedule"))?;

    sqlx::query("INSERT INTO schedule SELECT * FROM jsonb_populate_record(null::schedule, $1)")
        .bind(row)
        .execute(&mut *tx)
        .await?;

    Ok(())
}

async fn restore_variable(tx: &mut sqlx::PgConnection, item: &TrashItemWithData) -> Result<()> {
    let data = &item.item_data;

    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM variable WHERE path = $1 AND workspace_id = $2)",
        &item.item_path,
        &item.workspace_id,
    )
    .fetch_one(&mut *tx)
    .await?
    .unwrap_or(false);

    if exists {
        return Err(Error::BadRequest(format!(
            "A variable already exists at path '{}'",
            item.item_path
        )));
    }

    let row = data
        .get("row")
        .ok_or_else(|| Error::internal_err("Invalid trash data for variable"))?;

    sqlx::query("INSERT INTO variable SELECT * FROM jsonb_populate_record(null::variable, $1)")
        .bind(row)
        .execute(&mut *tx)
        .await?;

    // Restore linked resource if present
    if let Some(linked_resource) = data.get("linked_resource") {
        if !linked_resource.is_null() {
            sqlx::query(
                "INSERT INTO resource SELECT * FROM jsonb_populate_record(null::resource, $1)
                 ON CONFLICT DO NOTHING",
            )
            .bind(linked_resource)
            .execute(&mut *tx)
            .await?;
        }
    }

    Ok(())
}

async fn restore_resource(tx: &mut sqlx::PgConnection, item: &TrashItemWithData) -> Result<()> {
    let data = &item.item_data;

    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM resource WHERE path = $1 AND workspace_id = $2)",
        &item.item_path,
        &item.workspace_id,
    )
    .fetch_one(&mut *tx)
    .await?
    .unwrap_or(false);

    if exists {
        return Err(Error::BadRequest(format!(
            "A resource already exists at path '{}'",
            item.item_path
        )));
    }

    let row = data
        .get("row")
        .ok_or_else(|| Error::internal_err("Invalid trash data for resource"))?;

    sqlx::query("INSERT INTO resource SELECT * FROM jsonb_populate_record(null::resource, $1)")
        .bind(row)
        .execute(&mut *tx)
        .await?;

    // Restore linked variables if present
    if let Some(linked_vars) = data.get("linked_variables").and_then(|v| v.as_array()) {
        for var in linked_vars {
            sqlx::query(
                "INSERT INTO variable SELECT * FROM jsonb_populate_record(null::variable, $1)
                 ON CONFLICT DO NOTHING",
            )
            .bind(var)
            .execute(&mut *tx)
            .await?;
        }
    }

    Ok(())
}

async fn restore_trigger(tx: &mut sqlx::PgConnection, item: &TrashItemWithData) -> Result<()> {
    let data = &item.item_data;

    let table_name = data
        .get("table_name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::internal_err("Invalid trash data for trigger: missing table_name"))?;

    // Validate table name to prevent SQL injection
    let valid_tables = [
        "http_trigger",
        "websocket_trigger",
        "kafka_trigger",
        "nats_trigger",
        "postgres_trigger",
        "mqtt_trigger",
        "sqs_trigger",
        "gcp_trigger",
        "email_trigger",
    ];

    if !valid_tables.contains(&table_name) {
        return Err(Error::BadRequest(format!(
            "Invalid trigger table: {}",
            table_name
        )));
    }

    let exists: bool = sqlx::query_scalar(&format!(
        "SELECT EXISTS(SELECT 1 FROM {} WHERE path = $1 AND workspace_id = $2)",
        table_name
    ))
    .bind(&item.item_path)
    .bind(&item.workspace_id)
    .fetch_one(&mut *tx)
    .await?;

    if exists {
        return Err(Error::BadRequest(format!(
            "A trigger already exists at path '{}'",
            item.item_path
        )));
    }

    let row = data
        .get("row")
        .ok_or_else(|| Error::internal_err("Invalid trash data for trigger"))?;

    sqlx::query(&format!(
        "INSERT INTO {} SELECT * FROM jsonb_populate_record(null::{}, $1)",
        table_name, table_name
    ))
    .bind(row)
    .execute(&mut *tx)
    .await?;

    Ok(())
}
