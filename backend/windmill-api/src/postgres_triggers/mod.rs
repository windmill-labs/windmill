use crate::{
    db::{ApiAuthed, DB},
    resources::try_get_resource_from_db_as,
};
use native_tls::{Certificate, TlsConnector};
use pg_escape::quote_identifier;
use rust_postgres::{config::SslMode, Client, Config, NoTls};
use rust_postgres_native_tls::MakeTlsConnector;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use sqlx::FromRow;
use std::collections::HashMap;

use windmill_common::{
    db::UserDB,
    error::{to_anyhow, Error, Result},
    utils::empty_as_none,
};
mod bool;
mod converter;
mod hex;
mod relation;
mod replication_message;
mod trigger;

pub use trigger::start_database;

const ERROR_REPLICATION_SLOT_NOT_EXISTS: &str = r#"The replication slot associated with this trigger no longer exists. Recreate a new replication slot or select an existing one in the advanced tab, or delete and recreate a new trigger"#;

const ERROR_PUBLICATION_NAME_NOT_EXISTS: &str = r#"The publication associated with this trigger no longer exists. Recreate a new publication or select an existing one in the advanced tab, or delete and recreate a new trigger"#;

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
            if let Err(e) = connection.await {
                tracing::debug!("{:#?}", e);
            };
            tracing::info!("Successfully Connected into database");
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

pub async fn get_pg_connection(
    authed: ApiAuthed,
    user_db: Option<UserDB>,
    db: &DB,
    postgres_resource_path: &str,
    w_id: &str,
    logical_mode: bool,
) -> Result<Client> {
    let database =
        try_get_resource_from_db_as::<Postgres>(&authed, user_db, db, postgres_resource_path, w_id)
            .await?;

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_handler_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_handler_args: Option<sqlx::types::Json<HashMap<String, Box<RawValue>>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry: Option<sqlx::types::Json<windmill_common::flows::Retry>>,
}

pub async fn drop_logical_replication_slot(pg_connection: &Client, slot_name: &str) -> Result<()> {
    let row = pg_connection
        .query_opt(
            r#"
            SELECT 
                active_pid 
            FROM 
                pg_replication_slots 
            WHERE 
                slot_name = $1
            "#,
            &[&slot_name],
        )
        .await
        .map_err(to_anyhow)?;

    let active_pid = row.map(|r| r.get::<_, Option<i32>>(0)).flatten();

    if let Some(pid) = active_pid {
        pg_connection
            .execute("SELECT pg_terminate_backend($1)", &[&pid])
            .await
            .map_err(to_anyhow)?;
    }

    pg_connection
        .execute("SELECT pg_drop_replication_slot($1)", &[&slot_name])
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
