use std::fmt;

use crate::db::{ApiAuthed, DB};
use axum::{
    extract::{Path, Query},
    routing::{delete, get, post},
    Extension, Json, Router,
};
use http::StatusCode;
use itertools::Itertools;
use pg_escape::{quote_identifier, quote_literal};
use rand::seq::SliceRandom;
use rust_postgres::{Client, Config, NoTls, SimpleQueryMessage};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::value::to_value;
use sql_builder::{bind::Bind, SqlBuilder};
use sqlx::FromRow;
use windmill_audit::{audit_ee::audit_log, ActionKind};
use windmill_common::{
    db::UserDB,
    error::{self, JsonResult},
    utils::{not_found_if_none, paginate, Pagination, StripPath},
    worker::CLOUD_HOSTED,
    INSTANCE_NAME,
};

#[derive(Clone, Debug, sqlx::Type, Deserialize, Serialize)]
#[sqlx(type_name = "transaction_type")]
enum TransactionType {
    Insert,
    Update,
    Delete,
}

pub enum Error {
    Sqlx(sqlx::Error),
    MissingTables(Vec<String>),
    Postgres(rust_postgres::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingTables(not_found_tables) => {
                write!(
                    f,
                    "The following tables do not exist in your database: {}",
                    not_found_tables.join(",")
                )
            }
            Self::Postgres(e) => write!(f, "{}", e),
            Self::Sqlx(e) => write!(f, "{}", e),
        }
    }
}

#[derive(FromRow, Serialize, Deserialize)]
struct Database {
    username: String,
    #[serde(skip_deserializing)]
    password: Option<String>,
    host: String,
    port: u16,
    db_name: String,
    schema: String,
}

struct PostgresClient {
    client: Client,
}

impl PostgresClient {
    pub async fn new(database: &Database) -> Result<PostgresClient, Error> {
        let mut config = Config::new();

        config
            .dbname(&database.db_name)
            .host(&database.host)
            .port(database.port)
            .user(&database.username)
            .replication_mode(rust_postgres::config::ReplicationMode::Logical);

        if let Some(password) = &database.password {
            config.password(password);
        }

        let (client, connection) = config.connect(NoTls).await.map_err(Error::Postgres)?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                println!("{:#?}", e);
            };
            println!("Connected");
        });

        Ok(PostgresClient { client })
    }

    pub async fn check_if_table_exists(
        &self,
        db_name: &str,
        table_to_track: &[&str],
    ) -> Result<(), Error> {
        if table_to_track.is_empty() {
            return Ok(());
        }
        let table_names = table_to_track
            .iter()
            .map(|table| quote_identifier(table))
            .join(",");

        let query = format!(
            r#"WITH target_tables AS (
                SELECT unnest(ARRAY[{}]) AS table_name
            )
            SELECT t.table_name
            FROM target_tables t
            LEFT JOIN information_schema.tables ist
            ON t.table_name = ist.table_name
            AND ist.table_type = 'BASE TABLE'
            AND ist.table_catalog = {}
            AND ist.table_schema NOT IN ('pg_catalog', 'information_schema')
            WHERE ist.table_name IS NULL;
            "#,
            table_names,
            quote_literal(db_name)
        );

        let rows = self
            .client
            .simple_query(&query)
            .await
            .map_err(Error::Postgres)?;

        if rows.is_empty() {
            return Ok(());
        }

        Err(Error::MissingTables(
            rows.into_iter()
                .filter_map(|row| {
                    if let SimpleQueryMessage::Row(row) = row {
                        return Some(row.get("table_name").unwrap().to_string());
                    }
                    None
                })
                .collect_vec(),
        ))
    }

    pub async fn alter_database_schema(&self, schemas: &[&str]) -> Result<(), Error> {
        let schemas = schemas
            .iter()
            .map(|table| quote_identifier(table))
            .join(",");
        let query = format!("SET search_path TO {}", schemas);
        self.client
            .execute(&query, &[])
            .await
            .map_err(Error::Postgres)?;
        Ok(())
    }
}

#[derive(FromRow, Serialize, Deserialize, Debug)]
struct TableToTrack {
    table_name: String,
    columns_name: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct EditDatabaseTrigger {
    path: String,
    script_path: String,
    is_flow: bool,
    database: String,
    table_to_track: Option<Vec<TableToTrack>>,
}

#[derive(Deserialize, Serialize, Debug)]

struct NewDatabaseTrigger {
    path: String,
    #[serde(deserialize_with = "check_if_valid_transaction_type")]
    transaction_type: String,
    script_path: String,
    is_flow: bool,
    enabled: bool,
    database: String,
    table_to_track: Option<Vec<TableToTrack>>,
}

fn check_if_valid_transaction_type<'de, D>(
    transaction_type: D,
) -> std::result::Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let transaction_type = String::deserialize(transaction_type)?;
    match transaction_type.as_str() {
        "Insert" | "Update" | "Delete" => Ok(transaction_type),
        _ => Err(serde::de::Error::custom(
            "Only the following transaction types are allowed: Insert, Update and Delete"
                .to_string(),
        )),
    }
}

#[derive(FromRow, Deserialize, Serialize)]
struct DatabaseTrigger {
    workspace_id: String,
    transaction_type: TransactionType,
    path: String,
    script_path: String,
    is_flow: bool,
    edited_by: String,
    email: String,
    edited_at: chrono::DateTime<chrono::Utc>,
    server_id: Option<String>,
    last_server_ping: Option<chrono::DateTime<chrono::Utc>>,
    extra_perms: serde_json::Value,
    error: Option<String>,
    enabled: bool,
    database: sqlx::types::Json<Database>,
    table_to_track: Vec<sqlx::types::Json<TableToTrack>>,
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

async fn create_database_trigger(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(new_database_trigger): Json<NewDatabaseTrigger>,
) -> error::Result<(StatusCode, String)> {
    let NewDatabaseTrigger {
        database,
        table_to_track,
        path,
        script_path,
        enabled,
        is_flow,
        transaction_type,
    } = new_database_trigger;
    if *CLOUD_HOSTED {
        return Err(error::Error::BadRequest(
            "Database triggers are not supported on multi-tenant cloud, use dedicated cloud or self-host".to_string(),
        ));
    }

    let table_to_track = table_to_track.map(|table_to_track| {
        table_to_track
            .into_iter()
            .map(sqlx::types::Json)
            .collect_vec()
    });

    let mut tx = user_db.begin(&authed).await?;
    sqlx::query_as::<_, DatabaseTrigger>("INSERT INTO database_trigger (workspace_id, path, script_path, transaction_type, is_flow, email, enabled, database, table_to_track, edited_by, edited_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, now()) RETURNING *")
    .bind(&w_id)
    .bind(&path)
    .bind(script_path)
    .bind(transaction_type)
    .bind(is_flow)
    .bind(&authed.email)
    .bind(enabled)
    .bind(database)
    .bind(table_to_track)
    .bind(&authed.username)
    .fetch_one(&mut *tx)
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

async fn list_database_triggers(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(lst): Query<ListDatabaseTriggerQuery>,
) -> error::JsonResult<Vec<DatabaseTrigger>> {
    let mut tx = user_db.begin(&authed).await?;
    let (per_page, offset) = paginate(Pagination { per_page: lst.per_page, page: lst.page });
    let mut sqlb = SqlBuilder::select_from("database_trigger")
        .field("*")
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
        .await?;
    tx.commit().await?;

    Ok(Json(rows))
}

async fn get_database_trigger(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> error::JsonResult<DatabaseTrigger> {
    let mut tx = user_db.begin(&authed).await?;
    let path = path.to_path();
    let trigger = sqlx::query_as::<_, DatabaseTrigger>(
        r#"SELECT *
          FROM database_trigger
          WHERE workspace_id = $1 AND path = $2"#,
    )
    .bind(w_id)
    .bind(path)
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;

    let trigger = not_found_if_none(trigger, "Trigger", path)?;

    Ok(Json(trigger))
}

async fn update_database_trigger(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Json(database_trigger): Json<EditDatabaseTrigger>,
) -> error::Result<String> {
    let workspace_path = path.to_path();
    let EditDatabaseTrigger { script_path, path, is_flow, database, table_to_track } =
        database_trigger;
    let mut tx = user_db.begin(&authed).await?;

    let table_to_track = table_to_track
        .into_iter()
        .map(|table| to_value(table).unwrap())
        .collect_vec();

    sqlx::query!(
        "UPDATE database_trigger SET script_path = $1, path = $2, is_flow = $3, edited_by = $4, email = $5, database = $6, table_to_track = $7, edited_at = now(), error = NULL
            WHERE workspace_id = $8 AND path = $9",
        script_path,
        path,
        is_flow,
        &authed.username,
        &authed.email,
        database,
        table_to_track.as_slice(),
        w_id,
        workspace_path,
    )
    .execute(&mut *tx).await?;

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

async fn delete_database_trigger(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> error::Result<String> {
    let path = path.to_path();
    let mut tx = user_db.begin(&authed).await?;
    sqlx::query!(
        "DELETE FROM database_trigger WHERE workspace_id = $1 AND path = $2",
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

async fn exists_database_trigger(
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<bool> {
    let path = path.to_path();
    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM database_trigger WHERE path = $1 AND workspace_id = $2)",
        path,
        w_id,
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(false);
    Ok(Json(exists))
}

async fn set_enabled(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Json(payload): Json<SetEnabled>,
) -> error::Result<String> {
    let mut tx = user_db.begin(&authed).await?;
    let path = path.to_path();

    // important to set server_id, last_server_ping and error to NULL to stop current websocket listener
    let one_o = sqlx::query_scalar!(
        "UPDATE database_trigger SET enabled = $1, email = $2, edited_by = $3, edited_at = now(), server_id = NULL, error = NULL
        WHERE path = $4 AND workspace_id = $5 RETURNING 1",
        payload.enabled,
        &authed.email,
        &authed.username,
        path,
        w_id,
    ).fetch_optional(&mut *tx).await?;

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

async fn update_ping(
    db: &DB,
    database_trigger: &DatabaseTrigger,
    error: Option<&str>,
) -> Option<()> {
    match sqlx::query_scalar!(
        "UPDATE database_trigger SET last_server_ping = now(), error = $1 WHERE workspace_id = $2 AND path = $3 AND server_id = $4 AND enabled IS TRUE RETURNING 1",
        error,
        database_trigger.workspace_id,
        database_trigger.path,
        *INSTANCE_NAME
    ).fetch_optional(db).await {
        Ok(updated) => {
            if updated.flatten().is_none() {
                tracing::info!("Database {} changed, disabled, or deleted, stopping...", database_trigger.path); 
                return None;
            }
        },
        Err(err) => {
            tracing::warn!("Error updating ping of database {}: {:?}", database_trigger.path, err);
        }
    };

    Some(())
}

async fn loop_ping(db: &DB, database_trigger: &DatabaseTrigger, error: Option<&str>) {
    loop {
        if update_ping(db, database_trigger, error).await.is_none() {
            return;
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}

async fn listen_to_transactions(
    database_trigger: DatabaseTrigger,
    db: DB,
    rsmq: Option<rsmq_async::MultiplexedRsmq>,
    mut killpill_rx: tokio::sync::broadcast::Receiver<()>,
) {
    let table_to_track = database_trigger
        .table_to_track
        .iter()
        .map(|table| table.table_name.as_str())
        .collect_vec();

    let client = match PostgresClient::new(&database_trigger.database).await {
        Ok(client) => client,
        Err(e) => {
            update_ping(&db, &database_trigger, Some(e.to_string().as_str())).await;
            return;
        }
    };

    if let Err(e) = client
        .check_if_table_exists(
            &database_trigger.database.db_name,
            table_to_track.as_slice(),
        )
        .await
    {
        update_ping(&db, &database_trigger, Some(e.to_string().as_str())).await;
        return;
    };

    tokio::select! {
        biased;
        _ = killpill_rx.recv() => {
            return;
        },
        _ = loop_ping(&db, &database_trigger, None) => {
            return ;
        }
    }
}

async fn try_to_listen_to_database_transactions(
    db_trigger: DatabaseTrigger,
    db: DB,
    rsmq: Option<rsmq_async::MultiplexedRsmq>,
    killpill_rx: tokio::sync::broadcast::Receiver<()>,
) {
    let database_trigger =  sqlx::query_scalar!(
        "UPDATE database_trigger SET server_id = $1, last_server_ping = now() WHERE enabled IS TRUE AND workspace_id = $2 AND path = $3 AND (server_id IS NULL OR last_server_ping IS NULL OR last_server_ping < now() - interval '15 seconds') RETURNING true",
        *INSTANCE_NAME,
        db_trigger.workspace_id,
        db_trigger.path,
    ).fetch_optional(&db).await;
    match database_trigger {
        Ok(has_lock) => {
            if has_lock.flatten().unwrap_or(false) {
                tokio::spawn(listen_to_transactions(db_trigger, db, rsmq, killpill_rx));
            } else {
                tracing::info!("Database {} already being listened to", db_trigger.path);
            }
        }
        Err(err) => {
            tracing::error!(
                "Error acquiring lock for database {}: {:?}",
                db_trigger.path,
                err
            );
        }
    };
}

async fn listen_to_unlistened_database_events(
    db: &DB,
    rsmq: &Option<rsmq_async::MultiplexedRsmq>,
    killpill_rx: &tokio::sync::broadcast::Receiver<()>,
) {
    let database_triggers =  sqlx::query_as::<_, DatabaseTrigger>(
        r#"SELECT *
            FROM database_trigger
            WHERE enabled IS TRUE AND (server_id IS NULL OR last_server_ping IS NULL OR last_server_ping < now() - interval '15 seconds')"#
    )
    .fetch_all(db)
    .await;

    match database_triggers {
        Ok(mut triggers) => {
            triggers.shuffle(&mut rand::thread_rng());
            for trigger in triggers {
                try_to_listen_to_database_transactions(
                    trigger,
                    db.clone(),
                    rsmq.clone(),
                    killpill_rx.resubscribe(),
                )
                .await;
            }
        }
        Err(err) => {
            tracing::error!("Error fetching database triggers: {:?}", err);
        }
    };
}

pub async fn start_database(
    db: DB,
    rsmq: Option<rsmq_async::MultiplexedRsmq>,
    mut killpill_rx: tokio::sync::broadcast::Receiver<()>,
) {
    tokio::spawn(async move {
        listen_to_unlistened_database_events(&db, &rsmq, &killpill_rx).await;
        loop {
            tokio::select! {
                biased;
                _ = killpill_rx.recv() => {
                    return;
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(15)) => {
                    listen_to_unlistened_database_events(&db, &rsmq, &killpill_rx).await
                }
            }
        }
    });
}

pub async fn can_be_listened(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
) -> JsonResult<bool> {
    let mut tx = user_db.begin(&authed).await?;

    Ok(Json(true))
}

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/create", post(create_database_trigger))
        .route("/list", get(list_database_triggers))
        .route("/get/*path", get(get_database_trigger))
        .route("/update/*path", post(update_database_trigger))
        .route("/delete/*path", delete(delete_database_trigger))
        .route("/exists/*path", get(exists_database_trigger))
        .route("/can_be_listened", get(can_be_listened))
        .route("/setenabled/*path", post(set_enabled))
}
