use axum::{
    extract::{Path, Query},
    Extension, Json,
};
use http::StatusCode;
use serde::{Deserialize, Deserializer, Serialize};
use sql_builder::{bind::Bind, SqlBuilder};
use sqlx::FromRow;
use windmill_audit::{audit_ee::audit_log, ActionKind};
use windmill_common::{
    db::UserDB,
    error::{self, JsonResult},
    utils::{not_found_if_none, paginate, Pagination, StripPath},
    worker::CLOUD_HOSTED,
};

use crate::db::{ApiAuthed, DB};

use super::SqlxJson;

#[derive(Clone, Debug, sqlx::Type, Deserialize, Serialize)]
#[sqlx(type_name = "transaction")]
pub enum TransactionType {
    Insert,
    Update,
    Delete,
}

#[derive(FromRow, Serialize, Deserialize, Debug)]
pub struct Database {
    pub username: String,
    pub password: Option<String>,
    pub host: String,
    pub port: u16,
    pub db_name: String,
}

#[derive(FromRow, Serialize, Deserialize, Debug)]
pub struct TableToTrack {
    pub table_name: String,
    pub columns_name: Vec<String>,
}

#[derive(Deserialize)]
pub struct EditDatabaseTrigger {
    path: String,
    script_path: String,
    is_flow: bool,
    database_resource_path: String,
    table_to_track: Option<Vec<TableToTrack>>,
}

#[derive(Deserialize, Serialize, Debug)]

pub struct NewDatabaseTrigger {
    path: String,
    #[serde(deserialize_with = "check_if_valid_transaction_type")]
    transaction_type: TransactionType,
    script_path: String,
    is_flow: bool,
    enabled: bool,
    database_resource_path: String,
    table_to_track: Option<Vec<TableToTrack>>,
    replication_slot_name: String,
    publication_name: String,
}

fn check_if_valid_transaction_type<'de, D>(
    transaction_type: D,
) -> std::result::Result<TransactionType, D::Error>
where
    D: Deserializer<'de>,
{
    let transaction_type = String::deserialize(transaction_type)?;
    match transaction_type.as_str() {
        "Insert" => Ok(TransactionType::Insert),
        "Update" => Ok(TransactionType::Update),
        "Delete" => Ok(TransactionType::Delete),
        _ => Err(serde::de::Error::custom(
            "Only the following transaction types are allowed: Insert, Update and Delete"
                .to_string(),
        )),
    }
}

#[derive(FromRow, Deserialize, Serialize, Debug)]
pub struct DatabaseTrigger {
    pub path: String,
    pub script_path: String,
    pub is_flow: bool,
    pub workspace_id: String,
    pub edited_by: String,
    pub email: String,
    pub edited_at: chrono::DateTime<chrono::Utc>,
    pub extra_perms: Option<serde_json::Value>,
    pub database_resource_path: String,
    pub transaction_type: TransactionType,
    pub table_to_track: Option<SqlxJson<Vec<TableToTrack>>>,
    pub error: Option<String>,
    pub server_id: Option<String>,
    pub replication_slot_name: String,
    pub publication_name: String,
    pub last_server_ping: Option<chrono::DateTime<chrono::Utc>>,
    pub enabled: bool,
}

#[derive(Deserialize, Serialize)]
pub struct ListDatabaseTriggerQuery {
    pub page: Option<usize>,
    pub per_page: Option<usize>,
    pub path: Option<String>,
    pub is_flow: Option<bool>,
    pub path_start: Option<String>,
}

#[derive(Deserialize)]
pub struct SetEnabled {
    pub enabled: bool,
}

pub async fn create_database_trigger(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(new_database_trigger): Json<NewDatabaseTrigger>,
) -> error::Result<(StatusCode, String)> {
    println!("{:#?}", &new_database_trigger);
    let NewDatabaseTrigger {
        database_resource_path,
        table_to_track,
        path,
        script_path,
        enabled,
        is_flow,
        transaction_type,
        publication_name,
        replication_slot_name,
    } = new_database_trigger;
    if *CLOUD_HOSTED {
        return Err(error::Error::BadRequest(
            "Database triggers are not supported on multi-tenant cloud, use dedicated cloud or self-host".to_string(),
        ));
    }

    let table_to_track = serde_json::to_value(table_to_track).unwrap();

    let mut tx = user_db.begin(&authed).await?;
    sqlx::query!(
        r#"
        INSERT INTO database_trigger (
            publication_name,
            replication_slot_name,
            workspace_id, 
            path, 
            script_path, 
            transaction_type, 
            is_flow, 
            email, 
            enabled, 
            database_resource_path, 
            table_to_track, 
            edited_by,
            edited_at
        ) 
        VALUES (
            $1, 
            $2, 
            $3, 
            $4, 
            $5, 
            $6, 
            $7, 
            $8, 
            $9, 
            $10, 
            $11,
            $12, 
            now()
        )"#,
        &publication_name,
        &replication_slot_name,
        &w_id,
        &path,
        script_path,
        transaction_type as TransactionType,
        is_flow,
        &authed.email,
        enabled,
        database_resource_path,
        table_to_track,
        &authed.username
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "database_triggers.create",
        ActionKind::Create,
        &w_id,
        Some(path.as_str()),
        None,
    )
    .await?;

    tx.commit().await?;

    Ok((StatusCode::CREATED, path.to_string()))
}

pub async fn list_database_triggers(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(lst): Query<ListDatabaseTriggerQuery>,
) -> error::JsonResult<Vec<DatabaseTrigger>> {
    let mut tx = user_db.begin(&authed).await?;
    let (per_page, offset) = paginate(Pagination { per_page: lst.per_page, page: lst.page });
    let mut sqlb = SqlBuilder::select_from("database_trigger")
        .fields(&[
            "workspace_id",
            "transaction_type",
            "path",
            "script_path",
            "is_flow",
            "edited_by",
            "email",
            "edited_at",
            "server_id",
            "last_server_ping",
            "extra_perms",
            "error",
            "enabled",
            "database_resource_path",
            "replication_slot_name",
            "publication_name",
            "table_to_track",
        ])
        .order_by("edited_at", true)
        .and_where("workspace_id = ?".bind(&w_id))
        .offset(offset)
        .limit(per_page)
        .clone();
    if let Some(path) = lst.path {
        sqlb.and_where_eq("script_path", "?".bind(&path));
    }
    if let Some(is_flow) = lst.is_flow {
        sqlb.and_where_eq("is_flow", "?".bind(&is_flow));
    }
    if let Some(path_start) = &lst.path_start {
        sqlb.and_where_like_left("path", path_start);
    }
    let sql = sqlb
        .sql()
        .map_err(|e| error::Error::InternalErr(e.to_string()))?;
    let rows = sqlx::query_as::<_, DatabaseTrigger>(&sql)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| {
            tracing::debug!("Error fetching database_trigger: {:#?}", e);
            windmill_common::error::Error::InternalErr("server error".to_string())
        })?;
    tx.commit().await.map_err(|e| {
        tracing::debug!("Error commiting database_trigger: {:#?}", e);
        windmill_common::error::Error::InternalErr("server error".to_string())
    })?;

    Ok(Json(rows))
}

pub async fn get_database_trigger(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> error::JsonResult<DatabaseTrigger> {
    let mut tx = user_db.begin(&authed).await?;
    let path = path.to_path();
    let trigger = sqlx::query_as!(
        DatabaseTrigger,
        r#"
        SELECT
            workspace_id,
            transaction_type AS "transaction_type: TransactionType",
            path,
            script_path,
            is_flow,
            edited_by,
            email,
            edited_at,
            server_id,
            last_server_ping,
            extra_perms,
            error,
            enabled,
            replication_slot_name,
            publication_name,
            database_resource_path,
            table_to_track AS "table_to_track: SqlxJson<Vec<TableToTrack>>"
        FROM 
            database_trigger
        WHERE 
            workspace_id = $1 AND 
            path = $2
        "#,
        &w_id,
        &path
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;

    let trigger = not_found_if_none(trigger, "Trigger", path)?;

    Ok(Json(trigger))
}

pub async fn update_database_trigger(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Json(database_trigger): Json<EditDatabaseTrigger>,
) -> error::Result<String> {
    let workspace_path = path.to_path();
    let EditDatabaseTrigger { script_path, path, is_flow, database_resource_path, table_to_track } =
        database_trigger;
    let mut tx = user_db.begin(&authed).await?;

    let table_to_track = serde_json::to_value(table_to_track).unwrap();

    sqlx::query!(
        r#"
            UPDATE database_trigger 
            SET 
                script_path = $1, 
                path = $2, 
                is_flow = $3, 
                edited_by = $4, 
                email = $5, 
                database_resource_path = $6, 
                table_to_track = $7, 
                edited_at = now(), 
                error = NULL
            WHERE 
                workspace_id = $8 AND 
                path = $9
            "#,
        script_path,
        path,
        is_flow,
        &authed.username,
        &authed.email,
        database_resource_path,
        table_to_track,
        w_id,
        workspace_path,
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "database_triggers.update",
        ActionKind::Create,
        &w_id,
        Some(&path),
        None,
    )
    .await?;

    tx.commit().await?;

    Ok(workspace_path.to_string())
}

pub async fn delete_database_trigger(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> error::Result<String> {
    let path = path.to_path();
    let mut tx = user_db.begin(&authed).await?;
    sqlx::query!(
        r#"
        DELETE FROM database_trigger 
        WHERE 
            workspace_id = $1 AND 
            path = $2
        "#,
        w_id,
        path,
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "database_triggers.delete",
        ActionKind::Delete,
        &w_id,
        Some(path),
        None,
    )
    .await?;

    tx.commit().await?;

    Ok(format!("Database trigger {path} deleted"))
}

pub async fn exists_database_trigger(
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<bool> {
    let path = path.to_path();
    let exists = sqlx::query_scalar!(
        r#"
        SELECT EXISTS(
            SELECT 1 
            FROM database_trigger 
            WHERE 
                path = $1 AND 
                workspace_id = $2
        )"#,
        path,
        w_id,
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(false);
    Ok(Json(exists))
}

pub async fn set_enabled(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Json(payload): Json<SetEnabled>,
) -> error::Result<String> {
    let mut tx = user_db.begin(&authed).await?;
    let path = path.to_path();

    // important to set server_id, last_server_ping and error to NULL to stop current database listener
    let one_o = sqlx::query_scalar!(
        r#"
        UPDATE database_trigger 
        SET 
            enabled = $1, 
            email = $2, 
            edited_by = $3, 
            edited_at = now(), 
            server_id = NULL, 
            error = NULL
        WHERE 
            path = $4 AND 
            workspace_id = $5 
        RETURNING 1
        "#,
        payload.enabled,
        &authed.email,
        &authed.username,
        path,
        w_id,
    )
    .fetch_optional(&mut *tx)
    .await?;

    not_found_if_none(one_o.flatten(), "Database trigger", path)?;

    audit_log(
        &mut *tx,
        &authed,
        "database_triggers.setenabled",
        ActionKind::Update,
        &w_id,
        Some(path),
        Some([("enabled", payload.enabled.to_string().as_ref())].into()),
    )
    .await?;

    tx.commit().await?;

    Ok(format!(
        "succesfully updated database trigger at path {} to status {}",
        path, payload.enabled
    ))
}
