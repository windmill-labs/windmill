use crate::{
    db::{ApiAuthed, DB},
    jobs::{run_flow_by_path_inner, run_script_by_path_inner, RunJobQuery},
    resources::try_get_resource_from_db_as,
    trigger_helpers::TriggerJobArgs,
    users::fetch_api_authed,
};
use chrono::Utc;
use itertools::Itertools;
use pg_escape::{quote_identifier, quote_literal};
use rand::Rng;
use serde_json::value::RawValue;
use sqlx::{
    postgres::{PgConnectOptions, PgSslMode},
    Connection, PgConnection,
};
use std::collections::HashMap;
use std::str::FromStr;

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
    error::{Error, Result},
    utils::StripPath,
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

pub async fn get_pg_connection(
    authed: ApiAuthed,
    user_db: Option<UserDB>,
    db: &DB,
    postgres_resource_path: &str,
    w_id: &str,
) -> Result<PgConnection> {
    let database =
        try_get_resource_from_db_as::<Postgres>(authed, user_db, db, postgres_resource_path, w_id)
            .await?;

    Ok(get_raw_postgres_connection(&database).await?)
}

pub async fn get_raw_postgres_connection(db: &Postgres) -> Result<PgConnection> {
    let options = {
        let sslmode = if !db.sslmode.is_empty() {
            PgSslMode::from_str(&db.sslmode)?
        } else {
            PgSslMode::Prefer
        };
        let options = {
            let inner_options = PgConnectOptions::new()
                .host(&db.host)
                .database(&db.dbname)
                .ssl_mode(sslmode)
                .username(&db.user);

            if let Some(port) = db.port {
                inner_options.port(port)
            } else {
                inner_options
            }
        };

        let options = if let Some(root_certificate_pem) = &db.root_certificate_pem {
            options.ssl_root_cert_from_pem(root_certificate_pem.as_bytes().to_vec())
        } else {
            options
        };

        if !db.password.is_empty() {
            options.password(&db.password)
        } else {
            options
        }
    };
    Ok(PgConnection::connect_with(&options).await?)
}

pub async fn create_logical_replication_slot(
    pg_connection: &mut PgConnection,
    name: &str,
) -> Result<()> {
    let query = format!(
        r#"
        SELECT 
                *
        FROM
            pg_create_logical_replication_slot({}, 'pgoutput');"#,
        quote_literal(&name)
    );

    sqlx::query(&query).execute(pg_connection).await?;

    Ok(())
}

async fn check_if_valid_publication_for_postgres_version(
    pg_connection: &mut PgConnection,
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
    pg_connection: &mut PgConnection,
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

    sqlx::query(&query).execute(pg_connection).await?;

    Ok(())
}

pub async fn drop_publication(
    pg_connection: &mut PgConnection,
    publication_name: &str,
) -> Result<()> {
    let mut query = String::from("DROP PUBLICATION IF EXISTS ");
    let quoted_publication_name = quote_identifier(publication_name);
    query.push_str(&quoted_publication_name);

    sqlx::query(&query).execute(pg_connection).await?;

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

    let user_db = UserDB::new(db.clone());

    let run_query = RunJobQuery::default();

    if trigger.is_flow {
        run_flow_by_path_inner(
            authed,
            db.clone(),
            user_db,
            trigger.workspace_id.clone(),
            StripPath(trigger.script_path.to_owned()),
            run_query,
            args,
        )
        .await?;
    } else {
        run_script_by_path_inner(
            authed,
            db.clone(),
            user_db,
            trigger.workspace_id.clone(),
            StripPath(trigger.script_path.to_owned()),
            run_query,
            args,
        )
        .await?;
    }

    Ok(())
}
