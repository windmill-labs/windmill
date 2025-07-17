use crate::{
    db::{ApiAuthed, DB},
    resources::try_get_resource_from_db_as,
    trigger_helpers::{trigger_runnable, TriggerJobArgs},
    users::fetch_api_authed,
};
use chrono::Utc;
use itertools::Itertools;
use native_tls::{Certificate, TlsConnector};
use pg_escape::quote_identifier;
use rand::Rng;
use rust_postgres::{config::SslMode, Client, Config, NoTls};
use rust_postgres_native_tls::MakeTlsConnector;
use serde_json::value::RawValue;
use std::collections::HashMap;

use axum::{
    routing::{delete, get, post},
    Router,
};
pub use handler::PostgresTrigger;
use handler::{
    alter_publication, create_postgres_trigger, create_publication, create_slot,
    create_template_script, delete_postgres_trigger, delete_publication, drop_slot_name,
    exists_postgres_trigger, get_postgres_trigger, get_postgres_version,
    get_postgres_version_internal, get_publication_info, get_template_script,
    is_database_in_logical_level, list_database_publication, list_postgres_triggers,
    list_slot_name, set_enabled, test_postgres_connection, update_postgres_trigger, Postgres,
    Relations,
};
use windmill_common::{
    db::UserDB,
    error::{to_anyhow, Error, Result},
};
mod bool;
mod converter;
mod handler;
mod hex;
mod mapper;
mod relation;
mod replication_message;
mod trigger;

pub use handler::PublicationData;
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

pub async fn create_logical_replication_slot(tx: &Client, slot_name: &str) -> Result<()> {
    tx.execute(
        &format!("SELECT * FROM pg_create_logical_replication_slot($1, 'pgoutput')"),
        &[&slot_name],
    )
    .await
    .map_err(to_anyhow)?;
    Ok(())
}

async fn check_if_valid_publication_for_postgres_version(
    pg_connection: &Client,
    table_to_track: Option<&[Relations]>,
) -> Result<bool> {
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

fn publication_service() -> Router {
    Router::new()
        .route("/get/:publication_name/*path", get(get_publication_info))
        .route("/create/:publication_name/*path", post(create_publication))
        .route("/update/:publication_name/*path", post(alter_publication))
        .route(
            "/delete/:publication_name/*path",
            delete(delete_publication),
        )
        .route("/list/*path", get(list_database_publication))
}

fn slot_service() -> Router {
    Router::new()
        .route("/list/*path", get(list_slot_name))
        .route("/create/*path", post(create_slot))
        .route("/delete/*path", delete(drop_slot_name))
}

fn postgres_service() -> Router {
    Router::new().route("/version/*path", get(get_postgres_version))
}

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/test", post(test_postgres_connection))
        .route("/create", post(create_postgres_trigger))
        .route("/list", get(list_postgres_triggers))
        .route("/get/*path", get(get_postgres_trigger))
        .route("/update/*path", post(update_postgres_trigger))
        .route("/delete/*path", delete(delete_postgres_trigger))
        .route("/exists/*path", get(exists_postgres_trigger))
        .route("/setenabled/*path", post(set_enabled))
        .route("/get_template_script/:id", get(get_template_script))
        .route("/create_template_script", post(create_template_script))
        .route(
            "/is_valid_postgres_configuration/*path",
            get(is_database_in_logical_level),
        )
        .nest("/publication", publication_service())
        .nest("/slot", slot_service())
        .nest("/postgres", postgres_service())
}

async fn run_job(
    payload: HashMap<String, Box<RawValue>>,
    db: &DB,
    trigger: &PostgresTrigger,
) -> anyhow::Result<()> {
    let args = PostgresTrigger::build_job_args(
        &trigger.script_path,
        trigger.is_flow,
        &trigger.workspace_id,
        db,
        payload,
        HashMap::new(),
    )
    .await?;

    let authed = fetch_api_authed(
        trigger.edited_by.clone(),
        trigger.email.clone(),
        &trigger.workspace_id,
        db,
        Some(format!("postgres-{}", trigger.path)),
    )
    .await?;

    trigger_runnable(
        db,
        None,
        authed,
        &trigger.workspace_id,
        &trigger.script_path,
        trigger.is_flow,
        args,
        trigger.retry.as_ref(),
        trigger.error_handler_path.as_deref(),
        trigger.error_handler_args.as_ref(),
        format!("postgres_trigger/{}", trigger.path),
    )
    .await?;

    Ok(())
}
