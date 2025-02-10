use std::{
    collections::{
        hash_map::Entry::{Occupied, Vacant},
        HashMap,
    },
    str::FromStr,
};

use crate::{
    db::{ApiAuthed, DB},
    postgres_triggers::mapper::{Mapper, MappingInfo},
};
use axum::{
    extract::{Path, Query},
    Extension, Json,
};
use chrono::Utc;
use http::StatusCode;
use itertools::Itertools;
use pg_escape::{quote_identifier, quote_literal};
use quick_cache::sync::Cache;
use rand::Rng;
use rust_postgres::types::Type;
use serde::{Deserialize, Deserializer, Serialize};
use sql_builder::{bind::Bind, SqlBuilder};
use sqlx::{
    postgres::{types::Oid, PgConnectOptions, PgSslMode},
    Connection, FromRow, PgConnection, QueryBuilder,
};
use windmill_audit::{audit_ee::audit_log, ActionKind};
use windmill_common::error::Error;
use windmill_common::{
    db::UserDB,
    error::{self, JsonResult},
    utils::{not_found_if_none, paginate, Pagination, StripPath},
    worker::CLOUD_HOSTED,
};

use super::get_database_resource;
use lazy_static::lazy_static;

#[derive(FromRow, Serialize, Deserialize, Debug)]
pub struct Database {
    pub user: String,
    pub password: String,
    pub host: String,
    pub port: u16,
    pub dbname: String,
    #[serde(default)]
    pub sslmode: String,
    pub root_certificate_pem: String,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct TableToTrack {
    pub table_name: String,
    pub where_clause: Option<String>,
    pub columns_name: Vec<String>,
}

impl TableToTrack {
    fn new(
        table_name: String,
        where_clause: Option<String>,
        columns_name: Vec<String>,
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

#[derive(Deserialize)]
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

pub async fn get_database_connection(
    authed: ApiAuthed,
    user_db: Option<UserDB>,
    db: &DB,
    postgres_resource_path: &str,
    w_id: &str,
) -> Result<PgConnection, windmill_common::error::Error> {
    let database = get_database_resource(authed, user_db, db, postgres_resource_path, w_id).await?;

    Ok(get_raw_postgres_connection(&database).await?)
}

pub async fn get_raw_postgres_connection(db: &Database) -> Result<PgConnection, Error> {
    let options = {
        let sslmode = if !db.sslmode.is_empty() {
            PgSslMode::from_str(&db.sslmode)?
        } else {
            PgSslMode::Prefer
        };
        let options = PgConnectOptions::new()
            .host(&db.host)
            .database(&db.dbname)
            .port(db.port)
            .ssl_mode(sslmode)
            .username(&db.user);

        let options = if !db.root_certificate_pem.is_empty() {
            options.ssl_root_cert_from_pem(db.root_certificate_pem.as_bytes().to_vec())
        } else {
            options
        };

        if !db.password.is_empty() {
            options.password(&db.password)
        } else {
            options
        }
    };

    PgConnection::connect_with(&options)
        .await
        .map_err(|e| e.into())
}

#[derive(Deserialize, Debug)]
pub enum Language {
    #[serde(rename = "typescript", alias = "Typescript")]
    Typescript,
}

#[derive(Debug, Deserialize)]
pub struct TemplateScript {
    postgres_resource_path: String,
    #[serde(deserialize_with = "check_if_not_duplication_relation")]
    relations: Option<Vec<Relations>>,
    language: Language,
}

fn check_if_not_duplication_relation<'de, D>(
    relations: D,
) -> std::result::Result<Option<Vec<Relations>>, D::Error>
where
    D: Deserializer<'de>,
{
    let relations: Option<Vec<Relations>> = Option::deserialize(relations)?;

    match relations {
        Some(relations) => {
            for relation in relations.iter() {
                if relation.schema_name.is_empty() {
                    return Err(serde::de::Error::custom(
                        "Schema Name must not be empty".to_string(),
                    ));
                }

                for table_to_track in relation.table_to_track.iter() {
                    if table_to_track.table_name.trim().is_empty() {
                        return Err(serde::de::Error::custom(
                            "Table name must not be empty".to_string(),
                        ));
                    }
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

pub async fn create_postgres_trigger(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(new_postgres_trigger): Json<NewPostgresTrigger>,
) -> error::Result<(StatusCode, String)> {
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

    let create_slot = replication_slot_name.is_none();
    let create_publication = publication_name.is_none();

    let name;
    let mut pub_name = publication_name.as_deref().unwrap_or_default();
    let mut slot_name = replication_slot_name.as_deref().unwrap_or_default();
    if create_publication || create_slot {
        let generate_random_string = move || {
            let timestamp = Utc::now().timestamp_millis().to_string();
            let mut rng = rand::rng();
            let charset = "abcdefghijklmnopqrstuvwxyz0123456789";

            let random_part = (0..10)
                .map(|_| {
                    charset
                        .chars()
                        .nth(rng.random_range(0..charset.len()))
                        .unwrap()
                })
                .collect::<String>();

            format!("{}_{}", timestamp, random_part)
        };

        name = format!("windmill_{}", generate_random_string());
        pub_name = &name;
        slot_name = &name;
        let publication = publication.unwrap();

        let mut connection = get_database_connection(
            authed.clone(),
            Some(user_db.clone()),
            &db,
            &postgres_resource_path,
            &w_id,
        )
        .await?;

        new_publication(
            &mut connection,
            pub_name,
            publication.table_to_track.as_deref(),
            &publication
                .transaction_to_track
                .iter()
                .map(AsRef::as_ref)
                .collect_vec(),
        )
        .await?;

        new_slot(&mut connection, slot_name).await?;
    }

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
    #[serde(default, deserialize_with = "check_if_not_duplication_relation")]
    table_to_track: Option<Vec<Relations>>,
    #[serde(deserialize_with = "check_if_valid_transaction_type")]
    transaction_to_track: Vec<String>,
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

#[derive(Debug, Serialize)]
pub struct SlotList {
    slot_name: Option<String>,
    active: Option<bool>,
}

pub async fn list_slot_name(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, postgres_resource_path)): Path<(String, String)>,
) -> error::Result<Json<Vec<SlotList>>> {
    let mut connection = get_database_connection(
        authed.clone(),
        Some(user_db.clone()),
        &db,
        &postgres_resource_path,
        &w_id,
    )
    .await?;

    let slots = sqlx::query_as!(
        SlotList,
        r#"
        SELECT 
            slot_name,
            active
        FROM
            pg_replication_slots 
        WHERE 
            plugin = 'pgoutput' AND
            slot_type = 'logical';
        "#
    )
    .fetch_all(&mut connection)
    .await?;

    Ok(Json(slots))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Slot {
    name: String,
}

async fn new_slot(connection: &mut PgConnection, name: &str) -> error::Result<()> {
    let query = format!(
        r#"
        SELECT 
                *
        FROM
            pg_create_logical_replication_slot({}, 'pgoutput');"#,
        quote_literal(&name)
    );

    sqlx::query(&query).execute(connection).await?;

    Ok(())
}

pub async fn create_slot(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, postgres_resource_path)): Path<(String, String)>,
    Json(Slot { name }): Json<Slot>,
) -> error::Result<String> {
    let mut connection = get_database_connection(
        authed.clone(),
        Some(user_db.clone()),
        &db,
        &postgres_resource_path,
        &w_id,
    )
    .await?;

    new_slot(&mut connection, &name).await?;

    Ok(format!("Slot {} created!", name))
}

pub async fn drop_slot_name(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, postgres_resource_path)): Path<(String, String)>,
    Json(Slot { name }): Json<Slot>,
) -> error::Result<String> {
    let mut connection = get_database_connection(
        authed.clone(),
        Some(user_db.clone()),
        &db,
        &postgres_resource_path,
        &w_id,
    )
    .await?;

    let query = format!("SELECT pg_drop_replication_slot({});", quote_literal(&name));
    sqlx::query(&query).execute(&mut connection).await?;

    Ok(format!("Slot name {} deleted!", name))
}
#[derive(Debug, Serialize)]
struct PublicationName {
    publication_name: String,
}

pub async fn list_database_publication(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, postgres_resource_path)): Path<(String, String)>,
) -> error::Result<Json<Vec<String>>> {
    let mut connection = get_database_connection(
        authed.clone(),
        Some(user_db.clone()),
        &db,
        &postgres_resource_path,
        &w_id,
    )
    .await?;

    let publication_names = sqlx::query_as!(
        PublicationName,
        "SELECT pubname AS publication_name FROM pg_publication;"
    )
    .fetch_all(&mut connection)
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
) -> error::Result<Json<PublicationData>> {
    let mut connection = get_database_connection(
        authed.clone(),
        Some(user_db.clone()),
        &db,
        &postgres_resource_path,
        &w_id,
    )
    .await?;

    let publication_data =
        get_publication_scope_and_transaction(&publication_name, &mut connection).await;

    let (all_table, transaction_to_track) = match publication_data {
        Ok(pub_data) => pub_data,
        Err(Error::SqlErr { error: sqlx::Error::RowNotFound, .. }) => {
            return Err(Error::NotFound(
                "Publication was not found, please create a new publication".to_string(),
            ))
        }
        Err(e) => return Err(e),
    };

    let table_to_track = if !all_table {
        Some(get_tracked_relations(&mut connection, &publication_name).await?)
    } else {
        None
    };
    Ok(Json(PublicationData::new(
        table_to_track,
        transaction_to_track,
    )))
}

async fn new_publication(
    connection: &mut PgConnection,
    publication_name: &str,
    table_to_track: Option<&[Relations]>,
    transaction_to_track: &[&str],
) -> Result<(), Error> {
    let mut query = QueryBuilder::new("CREATE PUBLICATION ");

    query.push(quote_identifier(publication_name));

    match table_to_track {
        Some(database_component) if !database_component.is_empty() => {
            query.push(" FOR");
            for (i, schema) in database_component.iter().enumerate() {
                if schema.table_to_track.is_empty() {
                    query.push(" TABLES IN SCHEMA ");
                    query.push(quote_identifier(&schema.schema_name));
                } else {
                    query.push(" TABLE ONLY ");
                    for (j, table) in schema.table_to_track.iter().enumerate() {
                        let table_name = quote_identifier(&table.table_name);
                        let schema_name = quote_identifier(&schema.schema_name);
                        let full_name = format!("{}.{}", &schema_name, &table_name);
                        query.push(full_name);
                        if !table.columns_name.is_empty() {
                            query.push(" (");
                            let columns = table
                                .columns_name
                                .iter()
                                .map(|column| quote_identifier(column))
                                .join(", ");
                            query.push(&columns);
                            query.push(")");
                        }

                        if let Some(where_clause) = &table.where_clause {
                            query.push(" WHERE (");
                            query.push(where_clause);
                            query.push(')');
                        }

                        if j + 1 != schema.table_to_track.len() {
                            query.push(", ");
                        }
                    }
                }
                if i < database_component.len() - 1 {
                    query.push(", ");
                }
            }
        }
        _ => {
            query.push(" FOR ALL TABLES ");
        }
    };

    if !transaction_to_track.is_empty() {
        let transactions = || transaction_to_track.iter().join(", ");
        query.push(" WITH (publish = '");
        query.push(transactions());
        query.push("');");
    }

    let query = query.build();
    query.execute(&mut *connection).await?;

    Ok(())
}

pub async fn create_publication(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, publication_name, postgres_resource_path)): Path<(String, String, String)>,
    Json(publication_data): Json<PublicationData>,
) -> error::Result<String> {
    let PublicationData { table_to_track, transaction_to_track } = publication_data;

    let mut connection = get_database_connection(
        authed.clone(),
        Some(user_db.clone()),
        &db,
        &postgres_resource_path,
        &w_id,
    )
    .await?;

    new_publication(
        &mut connection,
        &publication_name,
        table_to_track.as_deref(),
        &transaction_to_track.iter().map(AsRef::as_ref).collect_vec(),
    )
    .await?;

    Ok(format!(
        "Publication {} successfully created!",
        publication_name
    ))
}

async fn drop_publication(
    publication_name: &str,
    connection: &mut PgConnection,
) -> Result<(), Error> {
    let mut query = QueryBuilder::new("DROP PUBLICATION IF EXISTS ");
    let quoted_publication_name = quote_identifier(publication_name);
    query.push(quoted_publication_name);
    query.push(";");
    query.build().execute(&mut *connection).await?;
    Ok(())
}

pub async fn delete_publication(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, publication_name, postgres_resource_path)): Path<(String, String, String)>,
) -> error::Result<String> {
    let mut connection = get_database_connection(
        authed.clone(),
        Some(user_db.clone()),
        &db,
        &postgres_resource_path,
        &w_id,
    )
    .await?;

    drop_publication(&publication_name, &mut connection).await?;

    Ok(format!(
        "Publication {} successfully deleted!",
        publication_name
    ))
}

async fn update_publication(
    connection: &mut PgConnection,
    publication_name: &str,
    PublicationData { table_to_track, transaction_to_track }: PublicationData,
) -> error::Result<String> {
    let (all_table, _) =
        get_publication_scope_and_transaction(&publication_name, connection).await?;

    let mut query = QueryBuilder::new("");
    let quoted_publication_name = quote_identifier(&publication_name);

    let transaction_to_track_as_str = transaction_to_track.iter().join(",");

    match table_to_track {
        Some(ref relations) if !relations.is_empty() => {
            if all_table {
                drop_publication(&publication_name, connection).await?;
                new_publication(
                    connection,
                    &publication_name,
                    table_to_track.as_deref(),
                    &transaction_to_track.iter().map(AsRef::as_ref).collect_vec(),
                )
                .await?;
            } else {
                query.push("ALTER PUBLICATION ");
                query.push(&quoted_publication_name);
                query.push(" SET");
                for (i, schema) in relations.iter().enumerate() {
                    if schema.table_to_track.is_empty() {
                        query.push(" TABLES IN SCHEMA ");
                        let quoted_schema = quote_identifier(&schema.schema_name);
                        query.push(&quoted_schema);
                    } else {
                        query.push(" TABLE ONLY ");
                        for (j, table) in schema.table_to_track.iter().enumerate() {
                            let table_name = quote_identifier(&table.table_name);
                            let schema_name = quote_identifier(&schema.schema_name);
                            let full_name = format!("{}.{}", &schema_name, &table_name);
                            query.push(&full_name);
                            if !table.columns_name.is_empty() {
                                query.push(" (");
                                let columns = table
                                    .columns_name
                                    .iter()
                                    .map(|column| quote_identifier(column))
                                    .join(", ");
                                query.push(&columns);
                                query.push(") ");
                            }

                            if let Some(where_clause) = &table.where_clause {
                                query.push(" WHERE (");
                                query.push(where_clause);
                                query.push(')');
                            }

                            if j + 1 != schema.table_to_track.len() {
                                query.push(", ");
                            }
                        }
                    }
                    if i < relations.len() - 1 {
                        query.push(',');
                    }
                }
                query.push(";");
                query.build().execute(&mut *connection).await?;
                query.reset();
                query.push("ALTER PUBLICATION ");
                query.push(&quoted_publication_name);
                query.push(format!(
                    " SET (publish = '{}');",
                    transaction_to_track_as_str
                ));
            }
        }
        _ => {
            drop_publication(&publication_name, connection).await?;
            let to_execute = format!(
                r#"
                CREATE
                    PUBLICATION {} FOR ALL TABLES WITH (publish = '{}')
                "#,
                quoted_publication_name, transaction_to_track_as_str
            );
            query.push(&to_execute);
        }
    };

    query.build().execute(&mut *connection).await?;

    Ok(format!(
        "Publication {} successfully updated!",
        publication_name
    ))
}

pub async fn alter_publication(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, publication_name, postgres_resource_path)): Path<(String, String, String)>,
    Json(publication_data): Json<PublicationData>,
) -> error::Result<String> {
    let mut connection = get_database_connection(
        authed.clone(),
        Some(user_db.clone()),
        &db,
        &postgres_resource_path,
        &w_id,
    )
    .await?;
    let message = update_publication(&mut connection, &publication_name, publication_data).await?;

    Ok(message)
}

async fn get_publication_scope_and_transaction(
    publication_name: &str,
    connection: &mut PgConnection,
) -> Result<(bool, Vec<String>), Error> {
    #[derive(Debug, Deserialize, FromRow)]
    struct PublicationTransaction {
        all_table: bool,
        insert: bool,
        update: bool,
        delete: bool,
    }

    let transaction = sqlx::query_as!(
        PublicationTransaction,
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
        publication_name
    )
    .fetch_one(&mut *connection)
    .await?;

    let mut transaction_to_track = Vec::with_capacity(3);

    if transaction.insert {
        transaction_to_track.push("insert".to_string());
    }
    if transaction.update {
        transaction_to_track.push("update".to_string());
    }
    if transaction.delete {
        transaction_to_track.push("delete".to_string());
    }

    Ok((transaction.all_table, transaction_to_track))
}

async fn get_tracked_relations(
    connection: &mut PgConnection,
    publication_name: &str,
) -> error::Result<Vec<Relations>> {
    #[derive(Debug, Deserialize, FromRow)]
    struct PublicationData {
        schema_name: Option<String>,
        table_name: Option<String>,
        columns: Option<Vec<String>>,
        where_clause: Option<String>,
    }

    let publications = sqlx::query_as!(
        PublicationData,
        r#"
            SELECT
                schemaname AS schema_name,
                tablename AS table_name,
                attnames AS columns,
                rowfilter AS where_clause
            FROM
                pg_publication_tables
            WHERE
                pubname = $1
            "#,
        publication_name
    )
    .fetch_all(&mut *connection)
    .await?;

    let mut table_to_track: HashMap<String, Relations> = HashMap::new();

    for publication in publications {
        let schema_name = publication.schema_name.unwrap();
        let entry = table_to_track.entry(schema_name.clone());
        let table_to_track = TableToTrack::new(
            publication.table_name.unwrap(),
            publication.where_clause,
            publication.columns.unwrap(),
        );
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
) -> error::Result<String> {
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

    if let Some(publication) = publication {
        let mut connection = get_database_connection(
            authed.clone(),
            Some(user_db.clone()),
            &db,
            &postgres_resource_path,
            &w_id,
        )
        .await?;
        update_publication(&mut connection, &publication_name, publication).await?;
    }
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
        ActionKind::Create,
        &w_id,
        Some(&path),
        None,
    )
    .await?;

    tx.commit().await?;

    Ok(workspace_path.to_string())
}

pub async fn delete_postgres_trigger(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> error::Result<String> {
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
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Json(payload): Json<SetEnabled>,
) -> error::Result<String> {
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

    Ok(format!(
        "succesfully updated postgres trigger at path {} to status {}",
        path, payload.enabled
    ))
}

pub async fn get_template_script(Path((_, id)): Path<(String, String)>) -> error::Result<String> {
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
) -> error::Result<String> {
    let TemplateScript { postgres_resource_path, relations, language } = template_script;
    if relations.is_none() {
        return Err(Error::BadRequest(
            "You must at least choose schema to fetch table from".to_string(),
        ));
    }

    let mut connection = get_database_connection(
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

                let columns = if !table_to_track.columns_name.is_empty() {
                    quote_literal(&table_to_track.columns_name.join(","))
                } else {
                    "''".to_string()
                };
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

    let rows: Vec<ColumnInfo> = sqlx::query_as(&query)
        .fetch_all(&mut connection)
        .await
        .map_err(|e| error::Error::SqlErr { error: e, location: "pg_trigger".to_string() })?;

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
    let mut connection = get_database_connection(
        authed.clone(),
        Some(user_db.clone()),
        &db,
        &postgres_resource_path,
        &w_id,
    )
    .await?;

    let wal_level = sqlx::query_scalar!("SHOW WAL_LEVEL;")
        .fetch_optional(&mut connection)
        .await?
        .flatten();

    let is_logical = match wal_level.as_deref() {
        Some("logical") => true,
        _ => false,
    };

    Ok(Json(is_logical))
}
