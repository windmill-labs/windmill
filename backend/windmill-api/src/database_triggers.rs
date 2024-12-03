use std::fmt;

use crate::db::{ApiAuthed, DB};
use axum::{
    extract::{Path, Query},
    routing::{delete, get, post},
    Extension, Json, Router,
};
use byteorder::{BigEndian, ReadBytesExt};
use bytes::{BufMut, Bytes, BytesMut};
use chrono::TimeZone;
use futures::{pin_mut, SinkExt, StreamExt};
use http::StatusCode;
use itertools::Itertools;
use pg_escape::{quote_identifier, quote_literal};
use rand::seq::SliceRandom;
use rust_postgres::{Client, Config, CopyBothDuplex, NoTls, SimpleQueryMessage};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::value::to_value;
use sql_builder::{bind::Bind, SqlBuilder};
use sqlx::FromRow;
use windmill_audit::{audit_ee::audit_log, ActionKind};
use windmill_common::{
    db::UserDB,
    error::{self, JsonResult},
    resource::get_resource,
    utils::{not_found_if_none, paginate, Pagination, StripPath},
    variables::get_variable_or_self,
    worker::CLOUD_HOSTED,
    INSTANCE_NAME,
};

use sqlx::types::Json as SqlxJson;

#[derive(Clone, Debug, sqlx::Type, Deserialize, Serialize)]
#[sqlx(type_name = "transaction")]
enum TransactionType {
    Insert,
    Update,
    Delete,
}

pub enum Error {
    Sqlx(sqlx::Error),
    MissingTables(Vec<String>),
    Postgres(rust_postgres::Error),
    Wal(&'static str),
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
            Self::Wal(e) => write!(f, "{}", e),
        }
    }
}

#[derive(FromRow, Serialize, Deserialize, Debug)]
struct Database {
    username: String,
    password: Option<String>,
    host: String,
    port: u16,
    db_name: String,
}

struct PostgresClient {
    client: Client,
}

impl PostgresClient {
    pub async fn new(database: &Database) -> core::result::Result<PostgresClient, Error> {
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
            tracing::info!("Successfully Connected into database");
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
            .map(|table| quote_literal(table))
            .join(",");

        let query = format!(
            r#"
                WITH target_tables AS (
                    SELECT unnest(ARRAY[{}]) AS table_name
                )
                SELECT t.table_name
                FROM 
                    target_tables t
                LEFT JOIN 
                    information_schema.tables ist
                ON 
                    t.table_name = ist.table_name
                    AND ist.table_type = 'BASE TABLE'
                    AND ist.table_catalog = {}
                    AND ist.table_schema NOT IN ('pg_catalog', 'information_schema')
                WHERE 
                    ist.table_name IS NULL;
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

    pub async fn get_logical_replication_stream(
        &self,
        publication_name: &str,
        logical_replication_slot_name: &str,
    ) -> Result<CopyBothDuplex<Bytes>, Error> {
        let options = format!(
            r#"("proto_version" '1', "publication_names" {}, "binary")"#,
            quote_literal(publication_name)
        );

        let query = format!(
            r#"START_REPLICATION SLOT {} LOGICAL 0/0 {}"#,
            quote_identifier(logical_replication_slot_name),
            options
        );

        self.client
            .copy_both_simple::<bytes::Bytes>(query.as_str())
            .await
            .map_err(Error::Postgres)
    }
}

#[derive(FromRow, Serialize, Deserialize, Debug)]
struct TableToTrack {
    table_name: String,
    columns_name: Vec<String>,
}

#[derive(Deserialize)]
struct EditDatabaseTrigger {
    path: String,
    script_path: String,
    is_flow: bool,
    database_resource_path: String,
    table_to_track: Option<Vec<TableToTrack>>,
}

#[derive(Deserialize, Serialize, Debug)]

struct NewDatabaseTrigger {
    path: String,
    #[serde(deserialize_with = "check_if_valid_transaction_type")]
    transaction_type: TransactionType,
    script_path: String,
    is_flow: bool,
    enabled: bool,
    database_resource_path: String,
    table_to_track: Option<Vec<TableToTrack>>,
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
struct DatabaseTrigger {
    path: String,
    script_path: String,
    is_flow: bool,
    workspace_id: String,
    edited_by: String,
    email: String,
    edited_at: chrono::DateTime<chrono::Utc>,
    extra_perms: Option<serde_json::Value>,
    database_resource_path: String,
    transaction_type: TransactionType,
    table_to_track: Option<SqlxJson<Vec<TableToTrack>>>,
    error: Option<String>,
    server_id: Option<String>,
    last_server_ping: Option<chrono::DateTime<chrono::Utc>>,
    enabled: bool,
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
    println!("{:#?}", &new_database_trigger);
    let NewDatabaseTrigger {
        database_resource_path,
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

    let table_to_track = to_value(table_to_track).unwrap();

    let mut tx = user_db.begin(&authed).await?;
    sqlx::query!(
        r#"
        INSERT INTO database_trigger (
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
            now()
        )"#,
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

async fn list_database_triggers(
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

async fn get_database_trigger(
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

async fn update_database_trigger(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Json(database_trigger): Json<EditDatabaseTrigger>,
) -> error::Result<String> {
    let workspace_path = path.to_path();
    let EditDatabaseTrigger { script_path, path, is_flow, database_resource_path, table_to_track } =
        database_trigger;
    let mut tx = user_db.begin(&authed).await?;

    let table_to_track = to_value(table_to_track).unwrap();

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

async fn delete_database_trigger(
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

async fn exists_database_trigger(
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

async fn set_enabled(
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

async fn update_ping(
    db: &DB,
    database_trigger: &DatabaseTrigger,
    error: Option<&str>,
) -> Option<()> {
    match sqlx::query_scalar!(
        r#"
        UPDATE 
            database_trigger
        SET 
            last_server_ping = now(),
            error = $1
        WHERE
            workspace_id = $2
            AND path = $3
            AND server_id = $4 
            AND enabled IS TRUE
        RETURNING 1
        "#,
        error,
        database_trigger.workspace_id,
        database_trigger.path,
        *INSTANCE_NAME
    )
    .fetch_optional(db)
    .await
    {
        Ok(updated) => {
            if updated.flatten().is_none() {
                tracing::info!(
                    "Database {} changed, disabled, or deleted, stopping...",
                    database_trigger.path
                );
                return None;
            }
        }
        Err(err) => {
            tracing::warn!(
                "Error updating ping of database {}: {:?}",
                database_trigger.path,
                err
            );
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
    let resource = get_resource::<Database>(
        &db,
        &database_trigger.database_resource_path,
        &database_trigger.workspace_id,
    )
    .await;

    let mut resource = match resource {
        Ok(resource) => resource,
        Err(e) => {
            tracing::debug!(
                "Error while trying to retrieve resource from database: {:#?}",
                e
            );
            update_ping(&db, &database_trigger, Some(e.to_string().as_str())).await;
            return;
        }
    };

    if resource.value.password.is_some() {
        let password = get_variable_or_self(
            resource.value.password.unwrap(),
            &db,
            &database_trigger.workspace_id,
        )
        .await;

        let password = match password {
            Ok(password) => password,
            Err(e) => {
                tracing::debug!("Error decoded variable: {:#?}", e);
                update_ping(&db, &database_trigger, Some("Internal server error")).await;
                return;
            }
        };
        resource.value.password = Some(password)
    }

    let client = match PostgresClient::new(&resource.value).await {
        Ok(client) => client,
        Err(e) => {
            tracing::debug!("Failed to connect to database: {}", e.to_string());
            update_ping(&db, &database_trigger, Some("Internal Server Error")).await;
            return;
        }
    };

    if let Some(table_to_track) = &database_trigger.table_to_track {
        let table_to_track = table_to_track
            .iter()
            .map(|table| table.table_name.as_str())
            .collect_vec();

        if let Err(e) = client
            .check_if_table_exists(&resource.value.db_name, table_to_track.as_slice())
            .await
        {
            let err = e.to_string();
            tracing::debug!("{}", &err);
            update_ping(&db, &database_trigger, Some(&err)).await;
            return;
        };
    }

    tracing::info!("Starting tokio select futures");

    let logical_replication_stream = client
        .get_logical_replication_stream("publication_name", "logical_replication_slot_name")
        .await;

    let logical_replication_stream = match logical_replication_stream {
        Ok(logical_replication_stream) => logical_replication_stream,
        Err(e) => {
            let err = e.to_string();
            tracing::debug!("{}", &err);
            update_ping(&db, &database_trigger, Some(&err)).await;
            return;
        }
    };

    pin_mut!(logical_replication_stream);

    tokio::select! {
        biased;
        _ = killpill_rx.recv() => {
            return;
        }
        _ = loop_ping(&db, &database_trigger, None) => {
            return ;
        }
        _ = async {
            while let Ok(message) = logical_replication_stream.next().await.unwrap() {
                if let Some((first_byte, mut message)) = message.split_first() {
                    let code = char::from_u32(*first_byte as u32).unwrap();

                    match code {
                        'k' => {
                            let end_wal_server: i64 = message.read_i64::<BigEndian>().unwrap();
                            message.read_i64::<BigEndian>().unwrap();
                            let reply = message.read_u8().unwrap();
                            if reply == 1 {
                                let mut buf = BytesMut::new();
                                let ts = chrono::Utc.with_ymd_and_hms(2000, 1, 1, 0, 0, 0).unwrap();
                                let ts = chrono::Utc::now()
                                    .signed_duration_since(ts)
                                    .num_microseconds()
                                    .unwrap_or(0);

                                buf.put_u8(b'r');
                                buf.put_i64(end_wal_server);
                                buf.put_i64(end_wal_server);
                                buf.put_i64(end_wal_server);
                                buf.put_i64(ts);
                                buf.put_u8(0);
                                logical_replication_stream.send(buf.freeze()).await.unwrap();
                            }
                        }
                        'w' => {}
                        _ => {}
                    }
                }
            }
        } => {
            return;
        }
    }
}

async fn try_to_listen_to_database_transactions(
    db_trigger: DatabaseTrigger,
    db: DB,
    rsmq: Option<rsmq_async::MultiplexedRsmq>,
    killpill_rx: tokio::sync::broadcast::Receiver<()>,
) {
    let database_trigger = sqlx::query_scalar!(
        r#"
        UPDATE database_trigger 
        SET 
            server_id = $1, 
            last_server_ping = now() 
        WHERE 
            enabled IS TRUE 
            AND workspace_id = $2 
            AND path = $3 
            AND (
                server_id IS NULL 
                OR last_server_ping IS NULL 
                OR last_server_ping < now() - INTERVAL '15 seconds'
            ) 
        RETURNING true
        "#,
        *INSTANCE_NAME,
        db_trigger.workspace_id,
        db_trigger.path,
    )
    .fetch_optional(&db)
    .await;
    match database_trigger {
        Ok(has_lock) => {
            if has_lock.flatten().unwrap_or(false) {
                tracing::info!("Spawning new task to listen_to_database_transaction");
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
    let database_triggers = sqlx::query_as!(
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
                database_resource_path,
                table_to_track AS "table_to_track: SqlxJson<Vec<TableToTrack>>"
            FROM
                database_trigger
            WHERE
                enabled IS TRUE
                AND (
                    server_id IS NULL OR
                    last_server_ping IS NULL OR
                    last_server_ping < now() - interval '15 seconds'
                )
            "#
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
