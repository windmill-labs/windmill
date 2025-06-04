use std::collections::{
    hash_map::Entry::{Occupied, Vacant},
    HashMap,
};

use crate::{
    db::{ApiAuthed, DB},
    postgres_triggers::mapper::{Mapper, MappingInfo},
};
use axum::{
    extract::{Path, Query},
    Extension, Json,
};
use http::StatusCode;
use itertools::Itertools;
use pg_escape::{quote_identifier, quote_literal};
use quick_cache::sync::Cache;
use rust_postgres::types::Type;
use serde::{Deserialize, Deserializer, Serialize};
use sql_builder::{bind::Bind, SqlBuilder};
use sqlx::{postgres::types::Oid, Connection, FromRow, PgConnection};
use windmill_audit::{audit_oss::audit_log, ActionKind};
use windmill_common::{
    db::UserDB,
    error::{self, Error, JsonResult, Result},
    utils::{empty_as_none, not_found_if_none, paginate, Pagination, StripPath},
    worker::CLOUD_HOSTED,
};
use windmill_git_sync::{handle_deployment_metadata, DeployedObject};

use super::{
    check_if_valid_publication_for_postgres_version, create_logical_replication_slot,
    create_pg_publication, drop_publication, generate_random_string, get_pg_connection,
    ERROR_PUBLICATION_NAME_NOT_EXISTS,
};
use lazy_static::lazy_static;

#[derive(FromRow, Serialize, Deserialize, Debug)]
pub struct Postgres {
    pub user: String,
    pub password: String,
    pub host: String,
    pub port: Option<u16>,
    pub dbname: String,
    #[serde(default)]
    pub sslmode: String,
    #[serde(default, deserialize_with = "empty_as_none")]
    pub root_certificate_pem: Option<String>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct TableToTrack {
    pub table_name: String,
    #[serde(default, deserialize_with = "empty_as_none")]
    pub where_clause: Option<String>,
    #[serde(default, deserialize_with = "empty_as_none")]
    pub columns_name: Option<Vec<String>>,
}

impl TableToTrack {
    fn new(
        table_name: String,
        where_clause: Option<String>,
        columns_name: Option<Vec<String>>,
    ) -> TableToTrack {
        TableToTrack { table_name, where_clause, columns_name }
    }
}

lazy_static! {
    pub static ref TEMPLATE: Cache<String, String> = Cache::new(50);
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Relations {
    pub schema_name: String,
    pub table_to_track: Vec<TableToTrack>,
}

impl Relations {
    fn new(schema_name: String, table_to_track: Vec<TableToTrack>) -> Relations {
        Relations { schema_name, table_to_track }
    }

    fn add_new_table(&mut self, table_to_track: TableToTrack) {
        self.table_to_track.push(table_to_track);
    }
}

#[derive(Debug, Deserialize)]
pub struct EditPostgresTrigger {
    replication_slot_name: String,
    publication_name: String,
    path: String,
    script_path: String,
    is_flow: bool,
    postgres_resource_path: String,
    publication: Option<PublicationData>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct NewPostgresTrigger {
    path: String,
    script_path: String,
    is_flow: bool,
    enabled: bool,
    postgres_resource_path: String,
    replication_slot_name: Option<String>,
    publication_name: Option<String>,
    publication: Option<PublicationData>,
}

#[derive(Serialize, Deserialize)]
pub struct TestPostgres {
    pub postgres_resource_path: String,
}

pub async fn test_postgres_connection(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(workspace_id): Path<String>,
    Json(test_postgres): Json<TestPostgres>,
) -> Result<()> {
    let connect_f = async {
        get_pg_connection(
            authed,
            Some(user_db),
            &db,
            &test_postgres.postgres_resource_path,
            &workspace_id,
        )
        .await
        .map_err(|err| {
            error::Error::BadConfig(format!("Error connecting to postgres: {}", err.to_string()))
        })
    };
    tokio::time::timeout(tokio::time::Duration::from_secs(30), connect_f)
        .await
        .map_err(|_| {
            error::Error::BadConfig(format!("Timeout connecting to postgres after 30 seconds"))
        })??;

    Ok(())
}

#[derive(Deserialize, Debug)]
pub enum Language {
    #[serde(rename = "typescript", alias = "Typescript")]
    Typescript,
}

#[derive(Debug, Deserialize)]
pub struct TemplateScript {
    postgres_resource_path: String,
    #[serde(deserialize_with = "check_if_valid_relation")]
    relations: Option<Vec<Relations>>,
    language: Language,
}

fn check_if_valid_relation<'de, D>(
    relations: D,
) -> std::result::Result<Option<Vec<Relations>>, D::Error>
where
    D: Deserializer<'de>,
{
    let relations: Option<Vec<Relations>> = Option::deserialize(relations)?;
    let mut track_all_table_in_schema = false;
    let mut track_specific_columns_in_table = false;
    match relations {
        Some(relations) => {
            for relation in relations.iter() {
                if relation.schema_name.is_empty() {
                    return Err(serde::de::Error::custom(
                        "Schema Name must not be empty".to_string(),
                    ));
                }

                if !track_all_table_in_schema && relation.table_to_track.is_empty() {
                    track_all_table_in_schema = true;
                    continue;
                }

                for table_to_track in relation.table_to_track.iter() {
                    if table_to_track.table_name.trim().is_empty() {
                        return Err(serde::de::Error::custom(
                            "Table name must not be empty".to_string(),
                        ));
                    }

                    if !track_specific_columns_in_table && table_to_track.columns_name.is_some() {
                        track_specific_columns_in_table = true;
                    }
                }

                if track_all_table_in_schema && track_specific_columns_in_table {
                    return Err(serde::de::Error::custom("Incompatible tracking options. Schema-level tracking and specific table tracking with column selection cannot be used together. Refer to the documentation for valid configurations."));
                }
            }

            if !relations
                .iter()
                .map(|relation| relation.schema_name.as_str())
                .all_unique()
            {
                return Err(serde::de::Error::custom(
                    "You cannot choose a schema more than one time".to_string(),
                ));
            }

            Ok(Some(relations))
        }
        None => Ok(None),
    }
}

#[derive(FromRow, Deserialize, Serialize, Debug)]
pub struct PostgresTrigger {
    pub path: String,
    pub script_path: String,
    pub is_flow: bool,
    pub workspace_id: String,
    pub edited_by: String,
    pub email: String,
    pub edited_at: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_perms: Option<serde_json::Value>,
    pub postgres_resource_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_id: Option<String>,
    pub replication_slot_name: String,
    pub publication_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_server_ping: Option<chrono::DateTime<chrono::Utc>>,
    pub enabled: bool,
}

#[derive(Deserialize, Serialize)]
pub struct ListPostgresTriggerQuery {
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

#[derive(Serialize, Deserialize)]
pub struct PostgresPublicationReplication {
    publication_name: String,
    replication_slot_name: String,
}

impl PostgresPublicationReplication {
    pub fn new(
        publication_name: String,
        replication_slot_name: String,
    ) -> PostgresPublicationReplication {
        PostgresPublicationReplication { publication_name, replication_slot_name }
    }
}

async fn check_if_logical_replication_slot_exist(
    pg_connection: &mut PgConnection,
    replication_slot_name: &str,
) -> Result<bool> {
    let exists = sqlx::query("SELECT slot_name FROM pg_replication_slots where slot_name = $1")
        .bind(&replication_slot_name)
        .fetch_optional(pg_connection)
        .await?
        .is_some();
    Ok(exists)
}

async fn create_custom_slot_and_publication_inner(
    authed: ApiAuthed,
    user_db: UserDB,
    db: &DB,
    postgres_resource_path: &str,
    w_id: &str,
    publication: &PublicationData,
) -> Result<PostgresPublicationReplication> {
    let mut pg_connection = get_pg_connection(
        authed.clone(),
        Some(user_db),
        &db,
        &postgres_resource_path,
        &w_id,
    )
    .await?;

    let mut tx = pg_connection.begin().await?;
    let publication_name = format!("windmill_trigger_{}", generate_random_string());
    let replication_slot_name = publication_name.clone();

    create_logical_replication_slot(&mut tx, &replication_slot_name).await?;

    create_pg_publication(
        &mut tx,
        &publication_name,
        publication.table_to_track.as_deref(),
        &publication.transaction_to_track,
    )
    .await?;

    tx.commit().await?;

    Ok(PostgresPublicationReplication::new(
        publication_name,
        replication_slot_name,
    ))
}

pub async fn get_postgres_version_internal(pg_connection: &mut PgConnection) -> Result<String> {
    let postgres_version: String = sqlx::query_scalar("SHOW server_version;")
        .fetch_one(&mut *pg_connection)
        .await
        .map_err(|e| Error::Anyhow {
            error: anyhow::anyhow!("Failed to retrieve PostgreSQL version: {}", e),
            location: "postgres_triggers/handler.rs@379".to_string(),
        })?;

    Ok(postgres_version)
}

pub async fn get_postgres_version(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, postgres_resource_path)): Path<(String, String)>,
) -> Result<String> {
    let mut pg_connection = get_pg_connection(
        authed.clone(),
        Some(user_db),
        &db,
        &postgres_resource_path,
        &w_id,
    )
    .await?;

    let postgres_version = get_postgres_version_internal(&mut pg_connection).await?;

    Ok(postgres_version)
}

pub async fn create_postgres_trigger(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(new_postgres_trigger): Json<NewPostgresTrigger>,
) -> Result<(StatusCode, String)> {
    if *CLOUD_HOSTED {
        return Err(error::Error::BadRequest(
            "Postgres triggers are not supported on multi-tenant cloud, use dedicated cloud or self-host".to_string(),
        ));
    }

    let NewPostgresTrigger {
        postgres_resource_path,
        path,
        script_path,
        enabled,
        is_flow,
        publication_name,
        replication_slot_name,
        publication,
    } = new_postgres_trigger;

    if publication_name.is_none() && publication.is_none() {
        return Err(error::Error::BadRequest(
            "Publication data is missing".to_string(),
        ));
    }
    let (pub_name, slot_name) = if publication_name.is_none() && replication_slot_name.is_none() {
        if publication.is_none() {
            return Err(Error::BadRequest("publication must be set".to_string()));
        }

        let PostgresPublicationReplication { publication_name, replication_slot_name } =
            create_custom_slot_and_publication_inner(
                authed.clone(),
                user_db.clone(),
                &db,
                &postgres_resource_path,
                &w_id,
                &publication.unwrap(),
            )
            .await?;

        (publication_name, replication_slot_name)
    } else {
        if publication_name.is_none() {
            return Err(Error::BadRequest("Missing publication name".to_string()));
        } else if replication_slot_name.is_none() {
            return Err(Error::BadRequest(
                "Missing replication slot name".to_string(),
            ));
        }
        (publication_name.unwrap(), replication_slot_name.unwrap())
    };

    let mut tx = user_db.begin(&authed).await?;

    sqlx::query!(
        r#"
        INSERT INTO postgres_trigger (
            publication_name,
            replication_slot_name,
            workspace_id, 
            path, 
            script_path, 
            is_flow, 
            email, 
            enabled, 
            postgres_resource_path, 
            edited_by
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
            $10
        )"#,
        pub_name,
        slot_name,
        &w_id,
        &path,
        script_path,
        is_flow,
        &authed.email,
        enabled,
        postgres_resource_path,
        &authed.username
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "postgres_triggers.create",
        ActionKind::Create,
        &w_id,
        Some(path.as_str()),
        None,
    )
    .await?;

    tx.commit().await?;

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        DeployedObject::PostgresTrigger { path: path.to_string() },
        Some(format!("Postgres trigger '{}' created", path)),
        true,
    )
    .await?;

    Ok((StatusCode::CREATED, path.to_string()))
}

pub async fn list_postgres_triggers(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(lst): Query<ListPostgresTriggerQuery>,
) -> error::JsonResult<Vec<PostgresTrigger>> {
    let mut tx = user_db.begin(&authed).await?;
    let (per_page, offset) = paginate(Pagination { per_page: lst.per_page, page: lst.page });
    let mut sqlb = SqlBuilder::select_from("postgres_trigger")
        .fields(&[
            "workspace_id",
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
            "postgres_resource_path",
            "replication_slot_name",
            "publication_name",
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
    let rows = sqlx::query_as::<_, PostgresTrigger>(&sql)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| {
            tracing::debug!("Error fetching postgres_trigger: {:#?}", e);
            windmill_common::error::Error::InternalErr("server error".to_string())
        })?;
    tx.commit().await.map_err(|e| {
        tracing::debug!("Error commiting postgres_trigger: {:#?}", e);
        windmill_common::error::Error::InternalErr("server error".to_string())
    })?;

    Ok(Json(rows))
}

#[derive(Deserialize, Serialize, Debug)]
pub struct PublicationData {
    #[serde(default, deserialize_with = "check_if_valid_relation")]
    pub table_to_track: Option<Vec<Relations>>,
    #[serde(deserialize_with = "check_if_valid_transaction_type")]
    pub transaction_to_track: Vec<String>,
}

fn check_if_valid_transaction_type<'de, D>(
    transaction_type: D,
) -> std::result::Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let mut transaction_type: Vec<String> = Vec::deserialize(transaction_type)?;
    if transaction_type.len() > 3 {
        return Err(serde::de::Error::custom(
            "More than 3 transaction type which is not authorized, you are only allowed to those 3 transaction types: Insert, Update and Delete"
                .to_string(),
        ));
    }
    transaction_type.sort_unstable();
    transaction_type.dedup();

    for transaction in transaction_type.iter() {
        match transaction.to_lowercase().as_ref() {
            "insert" => {},
            "update" => {},
            "delete" => {},
            _ => {
                return Err(serde::de::Error::custom(
                    "Only the following transaction types are allowed: Insert, Update and Delete (case insensitive)"
                        .to_string(),
                ))
            }
        }
    }

    Ok(transaction_type)
}

impl PublicationData {
    fn new(
        table_to_track: Option<Vec<Relations>>,
        transaction_to_track: Vec<String>,
    ) -> PublicationData {
        PublicationData { table_to_track, transaction_to_track }
    }
}

#[derive(FromRow, Debug, Serialize)]
pub struct SlotList {
    slot_name: Option<String>,
    active: Option<bool>,
}

pub async fn list_slot_name(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, postgres_resource_path)): Path<(String, String)>,
) -> Result<Json<Vec<SlotList>>> {
    let mut pg_connection = get_pg_connection(
        authed.clone(),
        Some(user_db.clone()),
        &db,
        &postgres_resource_path,
        &w_id,
    )
    .await?;

    let slots: Vec<SlotList> = sqlx::query_as(
        r#"
        SELECT 
            slot_name,
            active
        FROM
            pg_replication_slots 
        WHERE 
            plugin = 'pgoutput' AND
            slot_type = 'logical';
        "#,
    )
    .fetch_all(&mut pg_connection)
    .await?;

    Ok(Json(slots))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Slot {
    name: String,
}

pub async fn create_slot(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, postgres_resource_path)): Path<(String, String)>,
    Json(Slot { name }): Json<Slot>,
) -> Result<String> {
    let mut pg_connection = get_pg_connection(
        authed.clone(),
        Some(user_db.clone()),
        &db,
        &postgres_resource_path,
        &w_id,
    )
    .await?;

    create_logical_replication_slot(&mut pg_connection, &name).await?;

    Ok(format!("Replication slot {} created!", name))
}

pub async fn drop_logical_replication_slot(
    pg_connection: &mut PgConnection,
    slot_name: &str,
) -> Result<()> {
    let active_pid: Option<i32> = sqlx::query_scalar(
        r#"SELECT 
            active_pid 
        FROM 
            pg_replication_slots 
        WHERE 
            slot_name = $1
        "#,
    )
    .bind(&slot_name)
    .fetch_optional(&mut *pg_connection)
    .await?
    .flatten();

    if let Some(pid) = active_pid {
        sqlx::query("SELECT pg_terminate_backend($1)")
            .bind(pid)
            .execute(&mut *pg_connection)
            .await?;
    }
    sqlx::query("SELECT pg_drop_replication_slot($1)")
        .bind(&slot_name)
        .execute(pg_connection)
        .await?;
    Ok(())
}

pub async fn drop_slot_name(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, postgres_resource_path)): Path<(String, String)>,
    Json(Slot { name }): Json<Slot>,
) -> Result<String> {
    let mut pg_connection =
        get_pg_connection(authed, Some(user_db), &db, &postgres_resource_path, &w_id).await?;

    drop_logical_replication_slot(&mut pg_connection, &name).await?;

    Ok(format!("Replication slot {} deleted!", name))
}
#[derive(FromRow, Debug, Serialize)]
struct PublicationName {
    publication_name: String,
}

pub async fn list_database_publication(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, postgres_resource_path)): Path<(String, String)>,
) -> Result<Json<Vec<String>>> {
    let mut pg_connection = get_pg_connection(
        authed.clone(),
        Some(user_db.clone()),
        &db,
        &postgres_resource_path,
        &w_id,
    )
    .await?;

    let publication_names: Vec<PublicationName> =
        sqlx::query_as("SELECT pubname AS publication_name FROM pg_publication;")
            .fetch_all(&mut pg_connection)
            .await?;

    let publications = publication_names
        .iter()
        .map(|publication| publication.publication_name.to_owned())
        .collect_vec();

    Ok(Json(publications))
}

pub async fn get_publication_info(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, publication_name, postgres_resource_path)): Path<(String, String, String)>,
) -> Result<Json<PublicationData>> {
    let mut pg_connection = get_pg_connection(
        authed.clone(),
        Some(user_db.clone()),
        &db,
        &postgres_resource_path,
        &w_id,
    )
    .await?;

    let publication_data =
        get_publication_scope_and_transaction(&mut pg_connection, &publication_name).await;

    let (all_table, transaction_to_track) = match publication_data {
        Ok(Some(pub_data)) => pub_data,
        Ok(None) => {
            return Err(Error::NotFound(
                ERROR_PUBLICATION_NAME_NOT_EXISTS.to_string(),
            ))
        }
        Err(e) => return Err(e),
    };

    let table_to_track = if !all_table {
        Some(get_tracked_relations(&mut pg_connection, &publication_name).await?)
    } else {
        None
    };
    Ok(Json(PublicationData::new(
        table_to_track,
        transaction_to_track,
    )))
}

pub async fn create_publication(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, publication_name, postgres_resource_path)): Path<(String, String, String)>,
    Json(publication_data): Json<PublicationData>,
) -> Result<String> {
    let mut pg_connection = get_pg_connection(
        authed.clone(),
        Some(user_db.clone()),
        &db,
        &postgres_resource_path,
        &w_id,
    )
    .await?;

    let PublicationData { table_to_track, transaction_to_track } = publication_data;

    let mut tx = pg_connection.begin().await?;

    create_pg_publication(
        &mut tx,
        &publication_name,
        table_to_track.as_deref(),
        &transaction_to_track,
    )
    .await?;

    tx.commit().await?;

    Ok(format!(
        "Publication {} successfully created!",
        publication_name
    ))
}

pub async fn delete_publication(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, publication_name, postgres_resource_path)): Path<(String, String, String)>,
) -> Result<String> {
    let mut pg_connection = get_pg_connection(
        authed.clone(),
        Some(user_db.clone()),
        &db,
        &postgres_resource_path,
        &w_id,
    )
    .await?;

    drop_publication(&mut pg_connection, &publication_name).await?;

    Ok(format!(
        "Publication {} successfully deleted!",
        publication_name
    ))
}

pub async fn update_pg_publication(
    pg_connection: &mut PgConnection,
    publication_name: &str,
    PublicationData { table_to_track, transaction_to_track }: PublicationData,
    all_table: Option<bool>,
) -> Result<()> {
    let quoted_publication_name = quote_identifier(&publication_name);
    let transaction_to_track_as_str = transaction_to_track.iter().join(",");
    match table_to_track {
        Some(ref relations) if !relations.is_empty() => {
            //if all table is none it means that the publication do not exist in the database
            if all_table.unwrap_or(true) {
                if all_table.is_some() {
                    drop_publication(pg_connection, &publication_name).await?;
                }
                create_pg_publication(
                    pg_connection,
                    &publication_name,
                    table_to_track.as_deref(),
                    &transaction_to_track,
                )
                .await?;
            } else {
                let pg_14 = check_if_valid_publication_for_postgres_version(
                    pg_connection,
                    table_to_track.as_deref(),
                )
                .await?;

                let mut query = String::from("");
                let mut first = true;
                query.push_str("ALTER PUBLICATION ");
                query.push_str(&quoted_publication_name);
                query.push_str(" SET");
                for (i, schema) in relations.iter().enumerate() {
                    if schema.table_to_track.is_empty() {
                        query.push_str(" TABLES IN SCHEMA ");
                        let quoted_schema = quote_identifier(&schema.schema_name);
                        query.push_str(&quoted_schema);
                    } else {
                        if pg_14 && first {
                            query.push_str(" TABLE ONLY ");
                            first = false
                        } else if !pg_14 {
                            query.push_str(" TABLE ONLY ");
                        }
                        for (j, table) in schema.table_to_track.iter().enumerate() {
                            let table_name = quote_identifier(&table.table_name);
                            let schema_name = quote_identifier(&schema.schema_name);
                            let full_name = format!("{}.{}", &schema_name, &table_name);
                            query.push_str(&full_name);
                            if let Some(columns) = table.columns_name.as_ref() {
                                query.push_str(" (");
                                let columns = columns
                                    .iter()
                                    .map(|column| quote_identifier(column))
                                    .join(", ");
                                query.push_str(&columns);
                                query.push_str(") ");
                            }

                            if let Some(where_clause) = &table.where_clause {
                                query.push_str(" WHERE (");
                                query.push_str(where_clause);
                                query.push(')');
                            }

                            if j + 1 != schema.table_to_track.len() {
                                query.push_str(", ");
                            }
                        }
                    }
                    if i < relations.len() - 1 {
                        query.push(',');
                    }
                }

                sqlx::query(&query).execute(&mut *pg_connection).await?;

                let mut query = String::new();

                query.push_str("ALTER PUBLICATION ");
                query.push_str(&quoted_publication_name);
                query.push_str(&format!(
                    " SET (publish = '{}');",
                    transaction_to_track_as_str
                ));

                sqlx::query(&query).execute(pg_connection).await?;
            }
        }
        _ => {
            drop_publication(pg_connection, &publication_name).await?;
            let query_to_execute = format!(
                r#"
                CREATE
                    PUBLICATION {} FOR ALL TABLES WITH (publish = '{}');
                "#,
                quoted_publication_name, transaction_to_track_as_str
            );
            sqlx::query(&query_to_execute)
                .execute(pg_connection)
                .await?;
        }
    };
    Ok(())
}

pub async fn alter_publication(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, publication_name, postgres_resource_path)): Path<(String, String, String)>,
    Json(publication_data): Json<PublicationData>,
) -> Result<String> {
    let mut pg_connection = get_pg_connection(
        authed.clone(),
        Some(user_db.clone()),
        &db,
        &postgres_resource_path,
        &w_id,
    )
    .await?;

    let mut tx = pg_connection.begin().await?;

    let publication = get_publication_scope_and_transaction(&mut tx, &publication_name).await?;

    update_pg_publication(
        &mut tx,
        &publication_name,
        publication_data,
        publication.map(|publication| publication.0),
    )
    .await?;

    tx.commit().await?;

    Ok(format!(
        "Publication {} updated with success",
        publication_name
    ))
}

async fn get_publication_scope_and_transaction(
    pg_connection: &mut PgConnection,
    publication_name: &str,
) -> Result<Option<(bool, Vec<String>)>> {
    #[derive(Debug, Deserialize, FromRow)]
    struct PublicationTransaction {
        all_table: bool,
        insert: bool,
        update: bool,
        delete: bool,
    }

    let publication: Option<PublicationTransaction> = sqlx::query_as(
        r#"
        SELECT
            puballtables AS all_table,
            pubinsert AS insert,
            pubupdate AS update,
            pubdelete AS delete
        FROM
            pg_publication
        WHERE
            pubname = $1
        "#,
    )
    .bind(publication_name)
    .fetch_optional(&mut *pg_connection)
    .await?;

    if publication.is_none() {
        return Ok(None);
    }

    let mut transaction_to_track = Vec::with_capacity(3);

    let publication = publication.unwrap();
    if publication.insert {
        transaction_to_track.push("insert".to_string());
    }
    if publication.update {
        transaction_to_track.push("update".to_string());
    }
    if publication.delete {
        transaction_to_track.push("delete".to_string());
    }

    Ok(Some((publication.all_table, transaction_to_track)))
}

async fn get_tracked_relations(
    pg_connection: &mut PgConnection,
    publication_name: &str,
) -> Result<Vec<Relations>> {
    #[derive(Debug, Deserialize, FromRow)]
    struct PublicationData {
        schema_name: Option<String>,
        table_name: Option<String>,
        #[serde(default)]
        columns: Option<Vec<String>>,
        #[serde(default)]
        where_clause: Option<String>,
    }

    let pg_version = get_postgres_version_internal(pg_connection).await?;
    let query = if pg_version.starts_with("14") {
        r#"
            SELECT
            schemaname AS schema_name,
            tablename AS table_name,
            NULL::text[] AS columns,
            NULL::text AS where_clause
            FROM
                pg_publication_tables
            WHERE
                pubname = $1;
            "#
    } else {
        r#"
            SELECT
            schemaname AS schema_name,
            tablename AS table_name,
            attnames AS columns,
            rowfilter AS where_clause
            FROM
                pg_publication_tables
            WHERE
                pubname = $1;
            "#
    };

    let publications: Vec<PublicationData> = sqlx::query_as(query)
        .bind(publication_name)
        .fetch_all(&mut *pg_connection)
        .await?;

    let mut table_to_track: HashMap<String, Relations> = HashMap::new();

    for publication in publications {
        let schema_name = publication.schema_name.ok_or_else(|| Error::Anyhow {
            error: anyhow::anyhow!(
                "Unexpected NULL `schema_name` in publication entry (pubname: `{}`). This should never happen unless PostgreSQL internals are corrupted.",
                publication_name,
            ),
            location: "postgres_triggers/handler.rs@1093".to_string(),
        })?;

        let table_name = publication.table_name.ok_or_else(|| Error::Anyhow {
            error: anyhow::anyhow!(
                "Unexpected NULL `table_name` for schema `{}` in publication `{}`. This should never happen unless PostgreSQL internals are corrupted.",
                schema_name,
                publication_name,
            ),
            location: "postgres_triggers/handler.rs@1102".to_string(),
        })?;

        let entry = table_to_track.entry(schema_name.clone());
        let table_to_track =
            TableToTrack::new(table_name, publication.where_clause, publication.columns);
        match entry {
            Occupied(mut occuped) => {
                occuped.get_mut().add_new_table(table_to_track);
            }
            Vacant(vacant) => {
                vacant.insert(Relations::new(schema_name, vec![table_to_track]));
            }
        }
    }
    Ok(table_to_track.into_values().collect_vec())
}

pub async fn get_postgres_trigger(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<PostgresTrigger> {
    let mut tx = user_db.begin(&authed).await?;
    let path = path.to_path();
    let trigger = sqlx::query_as!(
        PostgresTrigger,
        r#"
        SELECT
            workspace_id,
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
            postgres_resource_path
        FROM 
            postgres_trigger
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

pub async fn update_postgres_trigger(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Json(postgres_trigger): Json<EditPostgresTrigger>,
) -> Result<String> {
    let workspace_path = path.to_path();

    let EditPostgresTrigger {
        replication_slot_name,
        publication_name,
        script_path,
        path,
        is_flow,
        postgres_resource_path,
        publication,
    } = postgres_trigger;

    let mut pg_connection = get_pg_connection(
        authed.clone(),
        Some(user_db.clone()),
        &db,
        &postgres_resource_path,
        &w_id,
    )
    .await?;

    let exists =
        check_if_logical_replication_slot_exist(&mut pg_connection, &replication_slot_name).await?;

    let mut tx = pg_connection.begin().await?;

    if !exists {
        tracing::debug!(
            "Logical replication slot named: {} does not exists creating it...",
            &replication_slot_name
        );
        create_logical_replication_slot(&mut tx, &replication_slot_name).await?;
    }

    if let Some(publication) = publication {
        let publication_data =
            get_publication_scope_and_transaction(&mut tx, &publication_name).await?;

        update_pg_publication(
            &mut tx,
            &publication_name,
            publication,
            publication_data.map(|publication| publication.0),
        )
        .await?;
    }
    tx.commit().await?;

    let mut tx = user_db.begin(&authed).await?;

    sqlx::query!(
        r#"
            UPDATE postgres_trigger 
            SET 
                script_path = $1, 
                path = $2, 
                is_flow = $3, 
                edited_by = $4, 
                email = $5, 
                postgres_resource_path = $6, 
                replication_slot_name = $7,
                publication_name = $8,
                edited_at = now(), 
                error = NULL,
                server_id = NULL
            WHERE 
                workspace_id = $9 AND 
                path = $10
            "#,
        script_path,
        path,
        is_flow,
        &authed.username,
        &authed.email,
        postgres_resource_path,
        replication_slot_name,
        publication_name,
        w_id,
        workspace_path,
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "postgres_triggers.update",
        ActionKind::Update,
        &w_id,
        Some(&path),
        None,
    )
    .await?;

    tx.commit().await?;

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        DeployedObject::PostgresTrigger { path: path.to_string() },
        Some(format!("Postgres trigger '{}' updated", path)),
        true,
    )
    .await?;

    Ok(workspace_path.to_string())
}

pub async fn delete_postgres_trigger(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> Result<String> {
    let path = path.to_path();
    let mut tx = user_db.begin(&authed).await?;
    sqlx::query!(
        r#"
        DELETE FROM postgres_trigger 
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
        "postgres_triggers.delete",
        ActionKind::Delete,
        &w_id,
        Some(path),
        None,
    )
    .await?;

    tx.commit().await?;

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        DeployedObject::PostgresTrigger { path: path.to_string() },
        Some(format!("Postgres trigger '{}' deleted", path)),
        true,
    )
    .await?;

    Ok(format!("Postgres trigger {path} deleted"))
}

pub async fn exists_postgres_trigger(
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<bool> {
    let path = path.to_path();
    let exists = sqlx::query_scalar!(
        r#"
        SELECT EXISTS(
            SELECT 1 
            FROM postgres_trigger 
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
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Json(payload): Json<SetEnabled>,
) -> Result<String> {
    let mut tx = user_db.begin(&authed).await?;
    let path = path.to_path();

    // important to set server_id, last_server_ping and error to NULL to stop current postgres listener
    let one_o = sqlx::query_scalar!(
        r#"
        UPDATE postgres_trigger 
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
    .await?
    .flatten();

    not_found_if_none(one_o, "Postgres trigger", path)?;

    audit_log(
        &mut *tx,
        &authed,
        "postgres_triggers.setenabled",
        ActionKind::Update,
        &w_id,
        Some(path),
        Some([("enabled", payload.enabled.to_string().as_ref())].into()),
    )
    .await?;

    tx.commit().await?;

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        DeployedObject::PostgresTrigger { path: path.to_string() },
        Some(format!("Postgres trigger '{}' updated", path)),
        true,
    )
    .await?;

    Ok(format!(
        "succesfully updated postgres trigger at path {} to status {}",
        path, payload.enabled
    ))
}

pub async fn get_template_script(Path((_, id)): Path<(String, String)>) -> Result<String> {
    let template = if let Some((_, template)) = TEMPLATE.remove(&id) {
        template
    } else {
        "".to_string()
    };
    Ok(template)
}

pub async fn create_template_script(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(template_script): Json<TemplateScript>,
) -> Result<String> {
    let TemplateScript { postgres_resource_path, relations, language } = template_script;
    if relations.is_none() {
        return Err(Error::BadRequest(
            "You must at least choose schema to fetch table from".to_string(),
        ));
    }

    let mut pg_connection = get_pg_connection(
        authed.clone(),
        Some(user_db.clone()),
        &db,
        &postgres_resource_path,
        &w_id,
    )
    .await?;

    #[derive(Debug, FromRow, Deserialize)]
    struct ColumnInfo {
        table_schema: Option<String>,
        table_name: Option<String>,
        column_name: Option<String>,
        oid: Oid,
        is_nullable: bool,
    }

    let relations = relations.unwrap();
    let mut schema_or_fully_qualified_name = Vec::with_capacity(relations.len());
    let mut columns_list = Vec::new();
    for relation in relations {
        if !relation.table_to_track.is_empty() {
            for table_to_track in relation.table_to_track {
                let fully_qualified_name =
                    format!("{}.{}", &relation.schema_name, table_to_track.table_name);
                schema_or_fully_qualified_name.push(quote_literal(&fully_qualified_name));

                let columns = table_to_track
                    .columns_name
                    .map(|columns| quote_literal(&columns.join(",")))
                    .or_else(|| Some("''".to_string()))
                    .unwrap();

                columns_list.push(columns);
            }
            continue;
        }

        schema_or_fully_qualified_name.push(quote_literal(&relation.schema_name));
        columns_list.push(String::from("''"));
    }

    let tables_name = schema_or_fully_qualified_name.join(",");
    let columns_list = columns_list.join(",");

    let query = format!(
        r#"
        WITH table_column_mapping AS (
            SELECT
                unnest(ARRAY[{}]) AS table_name,
                unnest(ARRAY[{}]) AS column_list
        ),
        parsed_columns AS (
            SELECT
                tcm.table_name,
                CASE
                    WHEN tcm.column_list = '' THEN NULL
                    ELSE string_to_array(tcm.column_list, ',')
                END AS columns
            FROM
                table_column_mapping tcm
        )
        SELECT
            ns.nspname AS table_schema,
            cls.relname AS table_name,
            attr.attname AS column_name,
            attr.atttypid AS oid,
            attr.attnotnull AS is_nullable
        FROM
            pg_attribute attr
        JOIN
            pg_class cls
            ON attr.attrelid = cls.oid
        JOIN
            pg_namespace ns
            ON cls.relnamespace = ns.oid
        JOIN
            parsed_columns pc
            ON ns.nspname || '.' || cls.relname = pc.table_name
            OR ns.nspname = pc.table_name
        WHERE
            attr.attnum > 0 -- Exclude system columns
            AND NOT attr.attisdropped -- Exclude dropped columns
            AND cls.relkind = 'r' -- Restrict to base tables
            AND (
                pc.columns IS NULL
                OR attr.attname = ANY(pc.columns)
            );
        "#,
        tables_name, columns_list
    );

    let rows: Vec<ColumnInfo> = sqlx::query_as(&query).fetch_all(&mut pg_connection).await?;
    let mut mapper: HashMap<String, HashMap<String, Vec<MappingInfo>>> = HashMap::new();

    for row in rows {
        let ColumnInfo { table_schema, table_name, column_name, oid, is_nullable } = row;

        let entry = mapper.entry(table_schema.unwrap());

        let mapped_info =
            MappingInfo::new(column_name.unwrap(), Type::from_oid(oid.0), is_nullable);

        match entry {
            Occupied(mut occupied) => {
                let entry = occupied.get_mut().entry(table_name.unwrap());
                match entry {
                    Occupied(mut occuped) => {
                        let mapping_info = occuped.get_mut();
                        mapping_info.push(mapped_info);
                    }
                    Vacant(vacant) => {
                        let mut mapping_info = Vec::with_capacity(10);
                        mapping_info.push(mapped_info);
                        vacant.insert(mapping_info);
                    }
                }
            }
            Vacant(vacant) => {
                let mut mapping_info = Vec::with_capacity(10);
                mapping_info.push(mapped_info);
                vacant.insert(HashMap::from([(table_name.unwrap(), mapping_info)]));
            }
        }
    }

    let mapper = Mapper::new(mapper, language);

    let create_template_id = |w_id: &str| -> String {
        let uuid = uuid::Uuid::new_v4().to_string();
        let id = format!("{}-{}", &w_id, &uuid);

        id
    };

    let template = mapper.get_template();
    let id = create_template_id(&w_id);

    TEMPLATE.insert(id.clone(), template);

    Ok(id)
}

pub async fn is_database_in_logical_level(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, postgres_resource_path)): Path<(String, String)>,
) -> error::JsonResult<bool> {
    let mut pg_connection = get_pg_connection(
        authed.clone(),
        Some(user_db.clone()),
        &db,
        &postgres_resource_path,
        &w_id,
    )
    .await?;

    let wal_level: Option<String> = sqlx::query_scalar("SHOW WAL_LEVEL;")
        .fetch_optional(&mut pg_connection)
        .await?
        .flatten();

    let is_logical = match wal_level.as_deref() {
        Some("logical") => true,
        _ => false,
    };

    Ok(Json(is_logical))
}
