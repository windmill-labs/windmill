use std::collections::{
    hash_map::Entry::{Occupied, Vacant},
    HashMap,
};

use axum::{
    extract::{Path, Query},
    Extension, Json,
};
use http::StatusCode;
use itertools::Itertools;
use pg_escape::quote_literal;
use rust_postgres::types::Type;
use serde::{Deserialize, Deserializer, Serialize};
use sql_builder::{bind::Bind, SqlBuilder};
use sqlx::{
    postgres::{types::Oid, PgConnectOptions},
    Connection, FromRow, PgConnection,
};
use windmill_audit::{audit_ee::audit_log, ActionKind};
use windmill_common::{
    db::UserDB,
    error::{self, JsonResult},
    resource::get_resource,
    utils::{not_found_if_none, paginate, Pagination, StripPath},
    variables::get_variable_or_self,
    worker::CLOUD_HOSTED,
};

use crate::{
    database_triggers::mapper::{Mapper, MappingInfo},
    db::{ApiAuthed, DB},
};

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
    pub where_clause: Option<String>,
    pub columns_name: Vec<String>,
}

#[derive(FromRow, Serialize, Deserialize, Debug)]
pub struct Relations {
    pub schema_name: String,
    pub table_to_track: Vec<TableToTrack>,
}

#[derive(Deserialize)]
pub struct EditDatabaseTrigger {
    replication_slot_name: String,
    publication_name: String,
    path: String,
    script_path: String,
    is_flow: bool,
    database_resource_path: String,
    #[serde(deserialize_with = "check_if_not_duplication_relation")]
    table_to_track: Option<Vec<Relations>>,
    #[serde(deserialize_with = "check_if_valid_transaction_type")]
    transaction_to_track: Vec<TransactionType>,
}

#[derive(Deserialize, Serialize, Debug)]

pub struct NewDatabaseTrigger {
    path: String,
    #[serde(deserialize_with = "check_if_valid_transaction_type")]
    transaction_to_track: Vec<TransactionType>,
    script_path: String,
    is_flow: bool,
    enabled: bool,
    database_resource_path: String,
    #[serde(deserialize_with = "check_if_not_duplication_relation")]
    table_to_track: Option<Vec<Relations>>,
    replication_slot_name: String,
    publication_name: String,
}

#[derive(Debug)]
pub enum Language {
    Typescript,
}

#[derive(Debug, Deserialize)]
pub struct TemplateScript {
    database_resource_path: String,
    #[serde(deserialize_with = "check_if_not_duplication_relation")]
    relations: Option<Vec<Relations>>,
    #[serde(deserialize_with = "check_if_valid_language")]
    language: Language,
}

fn check_if_valid_language<'de, D>(language: D) -> std::result::Result<Language, D::Error>
where
    D: Deserializer<'de>,
{
    let language: String = String::deserialize(language)?;

    let language = match language.to_lowercase().as_str() {
        "typescript" => Language::Typescript,
        _ => {
            return Err(serde::de::Error::custom(
                "Language supported for custom script is only: Typescript",
            ))
        }
    };

    Ok(language)
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

fn check_if_valid_transaction_type<'de, D>(
    transaction_type: D,
) -> std::result::Result<Vec<TransactionType>, D::Error>
where
    D: Deserializer<'de>,
{
    let mut transaction_type: Vec<String> = Vec::deserialize(transaction_type)?;
    if transaction_type.len() > 3 {
        return Err(serde::de::Error::custom(
            "Only the 3 transaction types are at most allowed: Insert, Update and Delete"
                .to_string(),
        ));
    }
    transaction_type.sort_unstable();
    transaction_type.dedup();

    let mut result = Vec::with_capacity(transaction_type.len());

    for transaction in transaction_type {
        match transaction.as_str() {
            "Insert" => result.push(TransactionType::Insert),
            "Update" => result.push(TransactionType::Update),
            "Delete" => result.push(TransactionType::Delete),
            _ => {
                return Err(serde::de::Error::custom(
                    "Only the following transaction types are allowed: Insert, Update and Delete"
                        .to_string(),
                ))
            }
        }
    }

    Ok(result)
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
    pub transaction_to_track: Option<Vec<TransactionType>>,
    pub table_to_track: Option<SqlxJson<Vec<Relations>>>,
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
    let NewDatabaseTrigger {
        database_resource_path,
        table_to_track,
        path,
        script_path,
        enabled,
        is_flow,
        transaction_to_track,
        publication_name,
        replication_slot_name,
    } = new_database_trigger;
    if *CLOUD_HOSTED {
        return Err(error::Error::BadRequest(
            "Database triggers are not supported on multi-tenant cloud, use dedicated cloud or self-host".to_string(),
        ));
    }

    let mut tx = user_db.begin(&authed).await?;

    let table_to_track = serde_json::to_value(table_to_track).unwrap();

    sqlx::query!(
        r#"
        INSERT INTO database_trigger (
            publication_name,
            replication_slot_name,
            workspace_id, 
            path, 
            script_path, 
            transaction_to_track, 
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
        transaction_to_track as Vec<TransactionType>,
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
            "transaction_to_track",
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
            transaction_to_track AS "transaction_to_track: Vec<TransactionType>",
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
            table_to_track AS "table_to_track: SqlxJson<Vec<Relations>>"
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
    let EditDatabaseTrigger {
        replication_slot_name,
        publication_name,
        script_path,
        path,
        is_flow,
        database_resource_path,
        table_to_track,
        transaction_to_track,
    } = database_trigger;
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
                transaction_to_track = $8,
                replication_slot_name = $9,
                publication_name = $10,
                edited_at = now(), 
                error = NULL,
                server_id = NULL
            WHERE 
                workspace_id = $11 AND 
                path = $12
            "#,
        script_path,
        path,
        is_flow,
        &authed.username,
        &authed.email,
        database_resource_path,
        table_to_track,
        transaction_to_track as Vec<TransactionType>,
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

pub async fn get_template_script(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(template_script): Json<TemplateScript>,
) -> error::Result<String> {
    let TemplateScript { database_resource_path, relations, language } = template_script;
    if relations.is_none() {
        return Err(error::Error::BadRequest(
            "You must at least choose schema to fetch table from".to_string(),
        ));
    }

    let resource = get_resource::<Database>(&db, &database_resource_path, &w_id)
        .await
        .map_err(|_| {
            windmill_common::error::Error::NotFound("Database resource do not exist".to_string())
        })?;

    let Database { username, password, host, port, db_name } = resource.value;

    let options = {
        let options = PgConnectOptions::new()
            .port(port)
            .database(&db_name)
            .username(&username)
            .host(&host);
        if let Some(password_path) = password {
            let password = get_variable_or_self(password_path, &db, &w_id).await?;
            options.password(&password)
        } else {
            options
        }
    };

    let mut pg_connection = PgConnection::connect_with(&options)
        .await
        .map_err(|e| error::Error::ConnectingToDatabase(e.to_string()))?;

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
                columns_list.push(
                    table_to_track
                        .columns_name
                        .into_iter()
                        .map(|column_name| quote_literal(&column_name))
                        .join(","),
                );
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
        .fetch_all(&mut pg_connection)
        .await
        .map_err(error::Error::SqlErr)?;

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

    let template = mapper.get_template();

    println!("{}", template);

    Ok(template)
}
