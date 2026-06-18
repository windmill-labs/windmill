use std::collections::HashMap;

use chrono::Utc;
use itertools::Itertools;
use native_tls::{Certificate, TlsConnector};
use pg_escape::quote_identifier;
use rand::Rng;
use rust_postgres::{config::SslMode, Client, Config, NoTls};
use rust_postgres_native_tls::MakeTlsConnector;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::value::RawValue;
use sqlx::FromRow;
use windmill_api_auth::ApiAuthed;
use windmill_common::workspaces::get_datatable_resource_from_db_unchecked;
use windmill_common::{
    db::UserDB,
    error::{to_anyhow, Error, Result},
    triggers::TriggerKind,
    utils::empty_as_none,
    DB,
};
use windmill_store::resources::try_get_resource_from_db_as;
use windmill_trigger::trigger_helpers::TriggerJobArgs;

mod bool;
mod converter;
pub mod handler;
mod hex;
pub mod listener;
mod mapper;
mod relation;
mod replication_message;

#[derive(Clone, Copy)]
pub struct PostgresTrigger;

impl TriggerJobArgs for PostgresTrigger {
    type Payload = HashMap<String, Box<RawValue>>;
    const TRIGGER_KIND: TriggerKind = TriggerKind::Postgres;
    fn v1_payload_fn(payload: &HashMap<String, Box<RawValue>>) -> HashMap<String, Box<RawValue>> {
        payload.to_owned()
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct PostgresConfig {
    pub postgres_resource_path: String,
    pub replication_slot_name: String,
    pub publication_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub basic_mode: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgresConfigRequest {
    postgres_resource_path: String,
    #[serde(default)]
    replication_slot_name: String,
    #[serde(default)]
    publication_name: String,
    publication: Option<PublicationData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestPostgresConfig {
    pub postgres_resource_path: String,
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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PublicationData {
    #[serde(default, deserialize_with = "check_if_valid_relation")]
    pub table_to_track: Option<Vec<Relations>>,
    #[serde(deserialize_with = "check_if_valid_transaction_type")]
    pub transaction_to_track: Vec<String>,
}

impl PublicationData {
    pub fn new(
        table_to_track: Option<Vec<Relations>>,
        transaction_to_track: Vec<String>,
    ) -> PublicationData {
        PublicationData { table_to_track, transaction_to_track }
    }
}

// Slot list struct
#[derive(FromRow, Debug, Serialize)]
pub struct SlotList {
    pub slot_name: Option<String>,
    pub active: Option<bool>,
}

// Slot struct
#[derive(Debug, Serialize, Deserialize)]
pub struct Slot {
    pub name: String,
}

// Template script struct
#[derive(Debug, Deserialize)]
pub struct TemplateScript {
    pub postgres_resource_path: String,
    #[serde(deserialize_with = "check_if_valid_relation")]
    pub relations: Option<Vec<Relations>>,
    pub language: Language,
}

// Language enum
#[derive(Deserialize, Debug)]
pub enum Language {
    #[serde(rename = "typescript", alias = "Typescript")]
    Typescript,
}

// Test postgres struct
#[derive(Serialize, Deserialize)]
pub struct TestPostgres {
    pub postgres_resource_path: String,
}

// PostgreSQL publication replication struct
#[derive(Serialize, Deserialize)]
pub struct PostgresPublicationReplication {
    pub publication_name: String,
    pub replication_slot_name: String,
}

impl PostgresPublicationReplication {
    pub fn new(
        publication_name: String,
        replication_slot_name: String,
    ) -> PostgresPublicationReplication {
        PostgresPublicationReplication { publication_name, replication_slot_name }
    }
}

pub const ERROR_PUBLICATION_NAME_NOT_EXISTS: &str = r#"The publication associated with this trigger no longer exists. Recreate a new publication or select an existing one in the advanced tab, or delete and recreate a new trigger"#;

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
    pub fn new(
        table_name: String,
        where_clause: Option<String>,
        columns_name: Option<Vec<String>>,
    ) -> TableToTrack {
        TableToTrack { table_name, where_clause, columns_name }
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Relations {
    pub schema_name: String,
    pub table_to_track: Vec<TableToTrack>,
}

impl Relations {
    pub fn new(schema_name: String, table_to_track: Vec<TableToTrack>) -> Relations {
        Relations { schema_name, table_to_track }
    }

    pub fn add_new_table(&mut self, table_to_track: TableToTrack) {
        self.table_to_track.push(table_to_track);
    }
}

fn build_tls_connector(
    ssl_mode: SslMode,
    root_certificate_pem: Option<&String>,
) -> Result<Option<MakeTlsConnector>> {
    let get_tls_builder_for_verify = |root_certificate: Option<&String>| {
        let mut builder = TlsConnector::builder();
        if let Some(root_certificate) = root_certificate {
            let root_certificate_pem =
                Certificate::from_pem(root_certificate.as_bytes()).map_err(to_anyhow)?;
            builder.add_root_certificate(root_certificate_pem);
        }
        Ok::<_, Error>(builder)
    };
    let connector = match ssl_mode {
        SslMode::Disable => return Ok(None),
        SslMode::Require | SslMode::Prefer => {
            let mut builder = TlsConnector::builder();
            builder.danger_accept_invalid_certs(true);
            builder.danger_accept_invalid_hostnames(true);
            builder
        }

        SslMode::VerifyCa => {
            let mut builder = get_tls_builder_for_verify(root_certificate_pem)?;
            builder.danger_accept_invalid_hostnames(true);
            builder
        }

        SslMode::VerifyFull => {
            let builder = get_tls_builder_for_verify(root_certificate_pem)?;
            builder
        }
        _ => unreachable!(),
    };

    Ok(Some(MakeTlsConnector::new(
        connector.build().map_err(to_anyhow)?,
    )))
}

pub async fn get_raw_postgres_connection(
    database: &Postgres,
    logical_mode: bool,
) -> Result<Client> {
    let ssl_mode = match database.sslmode.as_ref() {
            "disable" => SslMode::Disable,
            "" | "prefer" | "allow" => SslMode::Prefer,
            "require" => SslMode::Require,
            "verify-ca" => SslMode::VerifyCa,
            "verify-full" => SslMode::VerifyFull,
            ssl_mode => {
                return Err(Error::BadRequest(
                    format!("Invalid ssl mode for postgres: {}, please put a valid ssl_mode among the following available ssl mode: ['disable', 'allow', 'prefer', 'verify-ca', 'verify-full']", ssl_mode),
                ))
            }
        };

    let mut config = Config::new();
    config
        .dbname(&database.dbname)
        .host(&database.host)
        .user(&database.user)
        .ssl_mode(ssl_mode);

    if logical_mode {
        config.replication_mode(rust_postgres::config::ReplicationMode::Logical);
    }

    if let Some(port) = database.port {
        config.port(port);
    };

    if !database.password.is_empty() {
        config.password(&database.password);
    }

    let connector = build_tls_connector(ssl_mode, database.root_certificate_pem.as_ref())?;
    let client = if let Some(connector) = connector {
        let (client, connection) = config.connect(connector).await.map_err(to_anyhow)?;
        tokio::spawn(async move {
            tracing::info!("Successfully connected to PostgreSQL database for trigger execution");
            if let Err(e) = connection.await {
                tracing::debug!("Error during PostgreSQL trigger connection: {:#?}", e);
            };
            tracing::info!("PostgreSQL trigger connection closed");
        });
        client
    } else {
        let (client, connection) = config.connect(NoTls).await.map_err(to_anyhow)?;
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                tracing::debug!("{:#?}", e);
            };
            tracing::info!("Successfully Connected into database");
        });
        client
    };

    Ok(client)
}

pub async fn resolve_postgres_resource(
    authed: &ApiAuthed,
    user_db: Option<UserDB>,
    db: &DB,
    postgres_resource_path: &str,
    w_id: &str,
) -> Result<Postgres> {
    if let Some(datatable_name) = postgres_resource_path.strip_prefix("datatable://") {
        let resource_value =
            get_datatable_resource_from_db_unchecked(db, w_id, datatable_name).await?;
        serde_json::from_value::<Postgres>(resource_value).map_err(|e| Error::SerdeJson {
            error: e,
            location: "resolve_postgres_resource".to_string(),
        })
    } else {
        try_get_resource_from_db_as::<Postgres>(authed, user_db, db, postgres_resource_path, w_id)
            .await
    }
}

pub async fn get_pg_connection(
    authed: ApiAuthed,
    user_db: Option<UserDB>,
    db: &DB,
    postgres_resource_path: &str,
    w_id: &str,
    logical_mode: bool,
) -> Result<Client> {
    let database =
        resolve_postgres_resource(&authed, user_db, db, postgres_resource_path, w_id).await?;

    Ok(get_raw_postgres_connection(&database, logical_mode).await?)
}

pub async fn get_default_pg_connection(
    authed: ApiAuthed,
    user_db: Option<UserDB>,
    db: &DB,
    postgres_resource_path: &str,
    w_id: &str,
) -> Result<Client> {
    get_pg_connection(authed, user_db, db, postgres_resource_path, w_id, false).await
}

pub async fn create_logical_replication_slot(tx: &Client, slot_name: &str) -> Result<()> {
    tx.execute(
        &format!("SELECT * FROM pg_create_logical_replication_slot($1, 'pgoutput')"),
        &[&slot_name],
    )
    .await
    .map_err(to_anyhow)?;
    Ok(())
}

pub async fn check_if_valid_publication_for_postgres_version(
    pg_connection: &Client,
    table_to_track: Option<&[Relations]>,
) -> Result<bool> {
    use crate::handler::get_postgres_version_internal;

    let postgres_version = get_postgres_version_internal(pg_connection).await?;

    let pg_14 = postgres_version.starts_with("14");
    if pg_14 {
        let unsupported_publication = table_to_track
            .and_then(|relations| {
                relations.iter().find(|relation| {
                    let invalid_relation = relation.table_to_track.iter().find(|table_to_track| {
                        table_to_track.where_clause.is_some()
                            || table_to_track.columns_name.is_some()
                    });

                    relation.table_to_track.is_empty() || invalid_relation.is_some()
                })
            })
            .is_some();

        if unsupported_publication {
            return Err(Error::BadRequest(
                    "Your PostgreSQL database is running version 14, which does not support the following publication features: \
                    - WHERE clause filtering, \
                    - selective column tracking, and \
                    - tracking all tables within a schema.\n\
                    These features are only available in PostgreSQL 15 and above.".to_string(),
                ));
        }
    }
    Ok(pg_14)
}

pub async fn create_pg_publication(
    pg_connection: &Client,
    publication_name: &str,
    table_to_track: Option<&[Relations]>,
    transaction_to_track: &[String],
) -> Result<()> {
    let pg_14 =
        check_if_valid_publication_for_postgres_version(pg_connection, table_to_track).await?;
    let mut query = String::from("CREATE PUBLICATION ");

    query.push_str(&quote_identifier(publication_name));

    match table_to_track {
        Some(database_component) if !database_component.is_empty() => {
            query.push_str(" FOR");
            let mut first = true;
            for (i, schema) in database_component.iter().enumerate() {
                if schema.table_to_track.is_empty() {
                    query.push_str(" TABLES IN SCHEMA ");
                    query.push_str(&quote_identifier(&schema.schema_name));
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
                            query.push_str(")");
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
                if i < database_component.len() - 1 {
                    query.push_str(", ");
                }
            }
        }
        _ => {
            query.push_str(" FOR ALL TABLES ");
        }
    };

    if !transaction_to_track.is_empty() {
        let transactions = || transaction_to_track.iter().join(", ");
        query.push_str(" WITH (publish = '");
        query.push_str(&transactions());
        query.push_str("');");
    }

    pg_connection
        .execute(&query, &[])
        .await
        .map_err(to_anyhow)?;
    Ok(())
}

pub async fn drop_publication(pg_connection: &Client, publication_name: &str) -> Result<()> {
    let mut query = String::from("DROP PUBLICATION IF EXISTS ");
    let quoted_publication_name = quote_identifier(publication_name);
    query.push_str(&quoted_publication_name);

    pg_connection
        .execute(&query, &[])
        .await
        .map_err(to_anyhow)?;

    Ok(())
}

pub fn generate_random_string() -> String {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_publication_data() {
        let json = r#"{
            "table_to_track": [
                {
                    "schema_name": "public",
                    "table_to_track": [
                        {"table_name": "users"}
                    ]
                }
            ],
            "transaction_to_track": ["insert", "update"]
        }"#;
        let result: std::result::Result<PublicationData, _> = serde_json::from_str(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_publication_data_empty_schema_name() {
        let json = r#"{
            "table_to_track": [
                {
                    "schema_name": "",
                    "table_to_track": []
                }
            ],
            "transaction_to_track": ["insert"]
        }"#;
        let result: std::result::Result<PublicationData, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_publication_data_empty_table_name() {
        let json = r#"{
            "table_to_track": [
                {
                    "schema_name": "public",
                    "table_to_track": [
                        {"table_name": "  "}
                    ]
                }
            ],
            "transaction_to_track": ["insert"]
        }"#;
        let result: std::result::Result<PublicationData, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_publication_data_invalid_transaction_type() {
        let json = r#"{
            "transaction_to_track": ["insert", "truncate"]
        }"#;
        let result: std::result::Result<PublicationData, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_publication_data_too_many_transaction_types() {
        let json = r#"{
            "transaction_to_track": ["insert", "update", "delete", "insert"]
        }"#;
        let result: std::result::Result<PublicationData, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_publication_data_duplicate_schema_names() {
        let json = r#"{
            "table_to_track": [
                {"schema_name": "public", "table_to_track": [{"table_name": "a"}]},
                {"schema_name": "public", "table_to_track": [{"table_name": "b"}]}
            ],
            "transaction_to_track": ["insert"]
        }"#;
        let result: std::result::Result<PublicationData, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_publication_data_all_tables_in_schema() {
        let json = r#"{
            "table_to_track": [
                {"schema_name": "public", "table_to_track": []}
            ],
            "transaction_to_track": ["insert"]
        }"#;
        let result: std::result::Result<PublicationData, _> = serde_json::from_str(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_publication_data_incompatible_tracking() {
        let json = r#"{
            "table_to_track": [
                {"schema_name": "schema1", "table_to_track": []},
                {"schema_name": "schema2", "table_to_track": [{"table_name": "t1", "columns_name": ["col1"]}]}
            ],
            "transaction_to_track": ["insert"]
        }"#;
        let result: std::result::Result<PublicationData, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_postgres_config_serialization() {
        let config = PostgresConfig {
            postgres_resource_path: "f/db/postgres".to_string(),
            replication_slot_name: "slot_1".to_string(),
            publication_name: "pub_1".to_string(),
            basic_mode: Some(false),
        };
        let json = serde_json::to_value(&config).unwrap();
        assert_eq!(json["postgres_resource_path"], "f/db/postgres");
        assert_eq!(json["replication_slot_name"], "slot_1");
    }

    #[test]
    fn test_generate_random_string_format() {
        let s = generate_random_string();
        assert!(s.contains('_'));
        let parts: Vec<&str> = s.split('_').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[1].len(), 10);
    }

    #[test]
    fn test_relations_add_table() {
        let mut rel = Relations::new("public".to_string(), vec![]);
        rel.add_new_table(TableToTrack::new("users".to_string(), None, None));
        assert_eq!(rel.table_to_track.len(), 1);
        assert_eq!(rel.table_to_track[0].table_name, "users");
    }

    #[test]
    fn test_table_to_track_with_where_clause() {
        let tt = TableToTrack::new(
            "orders".to_string(),
            Some("status = 'active'".to_string()),
            None,
        );
        assert_eq!(tt.where_clause, Some("status = 'active'".to_string()));
    }

    #[test]
    fn test_table_to_track_with_columns() {
        let tt = TableToTrack::new(
            "users".to_string(),
            None,
            Some(vec!["id".to_string(), "email".to_string()]),
        );
        assert_eq!(tt.columns_name.as_ref().unwrap().len(), 2);
    }
}
