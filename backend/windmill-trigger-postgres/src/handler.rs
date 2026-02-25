use std::collections::HashMap;

use axum::{
    async_trait,
    extract::Path,
    routing::{delete, get, post},
    Extension, Json, Router,
};
use itertools::Itertools;
use lazy_static::lazy_static;
use pg_escape::{quote_identifier, quote_literal};
use quick_cache::sync::Cache;
use rust_postgres::{types::Type, Client};
use sqlx::PgConnection;
use uuid;
use windmill_common::{
    db::UserDB,
    error::{self, to_anyhow, Error, Result},
    DB,
};
use windmill_git_sync::DeployedObject;

use windmill_api_auth::ApiAuthed;
use windmill_trigger::{Trigger, TriggerCrud, TriggerData};

use super::{
    check_if_valid_publication_for_postgres_version, create_logical_replication_slot,
    create_pg_publication, drop_publication, generate_random_string, get_default_pg_connection,
    mapper::{Mapper, MappingInfo},
    PostgresConfig, PostgresConfigRequest, PostgresPublicationReplication, PostgresTrigger,
    PublicationData, Relations, Slot, SlotList, TableToTrack, TemplateScript, TestPostgresConfig,
    ERROR_PUBLICATION_NAME_NOT_EXISTS,
};

// Lazy static template cache
lazy_static! {
    pub static ref TEMPLATE: Cache<String, String> = Cache::new(50);
}

#[async_trait]
impl TriggerCrud for PostgresTrigger {
    type TriggerConfig = PostgresConfig;
    type Trigger = Trigger<Self::TriggerConfig>;
    type TriggerConfigRequest = PostgresConfigRequest;
    type TestConnectionConfig = TestPostgresConfig;

    const TABLE_NAME: &'static str = "postgres_trigger";
    const TRIGGER_TYPE: &'static str = "postgres";
    const SUPPORTS_SERVER_STATE: bool = true;
    const SUPPORTS_TEST_CONNECTION: bool = true;
    const ROUTE_PREFIX: &'static str = "/postgres_triggers";
    const DEPLOYMENT_NAME: &'static str = "PostgreSQL trigger";
    const ADDITIONAL_SELECT_FIELDS: &[&'static str] = &[
        "postgres_resource_path",
        "replication_slot_name",
        "publication_name",
        "NULL::text AS basic_mode",
    ];
    const IS_ALLOWED_ON_CLOUD: bool = false;

    fn get_deployed_object(path: String, parent_path: Option<String>) -> DeployedObject {
        DeployedObject::PostgresTrigger { path, parent_path }
    }

    async fn create_trigger(
        &self,
        db: &DB,
        tx: &mut PgConnection,
        authed: &ApiAuthed,
        w_id: &str,
        trigger: TriggerData<Self::TriggerConfigRequest>,
    ) -> Result<()> {
        let Self::TriggerConfigRequest {
            postgres_resource_path,
            publication_name,
            replication_slot_name,
            publication,
        } = trigger.config;

        let (pub_name, slot_name) =
            if publication_name.is_empty() && replication_slot_name.is_empty() {
                if publication.is_none() {
                    return Err(Error::BadRequest("publication must be set".to_string()));
                }

                let PostgresPublicationReplication { publication_name, replication_slot_name } =
                    create_custom_slot_and_publication_inner(
                        authed.clone(),
                        UserDB::new(db.clone()),
                        &db,
                        &postgres_resource_path,
                        &w_id,
                        &publication.unwrap(),
                    )
                    .await?;

                (publication_name, replication_slot_name)
            } else {
                if publication_name.is_empty() {
                    return Err(Error::BadRequest(
                        "Publication name must not be empty".to_string(),
                    ));
                } else if replication_slot_name.is_empty() {
                    return Err(Error::BadRequest(
                        "Replication slot name must not be empty".to_string(),
                    ));
                }
                (publication_name, replication_slot_name)
            };

        sqlx::query!(
            r#"
            INSERT INTO postgres_trigger (
                workspace_id,
                path,
                postgres_resource_path,
                replication_slot_name,
                publication_name,
                script_path,
                is_flow,
                mode,
                edited_by,
                email,
                edited_at,
                error_handler_path,
                error_handler_args,
                retry
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, now(), $11, $12, $13
            )
            "#,
            w_id,
            trigger.base.path,
            postgres_resource_path,
            slot_name,
            pub_name,
            trigger.base.script_path,
            trigger.base.is_flow,
            trigger.base.mode() as _,
            authed.username,
            authed.email,
            trigger.error_handling.error_handler_path,
            trigger.error_handling.error_handler_args as _,
            trigger.error_handling.retry as _
        )
        .execute(tx)
        .await?;
        Ok(())
    }

    async fn update_trigger(
        &self,
        db: &DB,
        tx: &mut PgConnection,
        authed: &ApiAuthed,
        w_id: &str,
        path: &str,
        trigger: TriggerData<Self::TriggerConfigRequest>,
    ) -> Result<()> {
        let Self::TriggerConfigRequest {
            replication_slot_name,
            publication_name,
            postgres_resource_path,
            publication,
        } = trigger.config;

        let mut pg_connection = get_default_pg_connection(
            authed.clone(),
            Some(UserDB::new(db.clone())),
            db,
            &postgres_resource_path,
            &w_id,
        )
        .await
        .map_err(to_anyhow)?;

        let exists =
            check_if_logical_replication_slot_exist(&mut pg_connection, &replication_slot_name)
                .await?;

        let remote_db_tx = pg_connection.transaction().await.map_err(to_anyhow)?;

        if !exists {
            tracing::debug!(
                "Logical replication slot named: {} does not exists creating it...",
                &replication_slot_name
            );
            create_logical_replication_slot(remote_db_tx.client(), &replication_slot_name)
                .await
                .map_err(to_anyhow)?;
        }

        if let Some(publication) = publication {
            let publication_data =
                get_publication_scope_and_transaction(remote_db_tx.client(), &publication_name)
                    .await
                    .map_err(to_anyhow)?;

            update_pg_publication(
                remote_db_tx.client(),
                &publication_name,
                publication,
                publication_data.map(|publication| publication.0),
            )
            .await
            .map_err(to_anyhow)?;
        }

        remote_db_tx.commit().await.map_err(to_anyhow)?;

        sqlx::query!(
            r#"
            UPDATE postgres_trigger
            SET
                postgres_resource_path = $1,
                replication_slot_name = $2,
                publication_name = $3,
                script_path = $4,
                path = $5,
                is_flow = $6,
                edited_by = $7,
                email = $8,
                edited_at = now(),
                server_id = NULL,
                error = NULL,
                error_handler_path = $11,
                error_handler_args = $12,
                retry = $13
            WHERE
                workspace_id = $9 AND path = $10
            "#,
            postgres_resource_path,
            replication_slot_name,
            publication_name,
            trigger.base.script_path,
            trigger.base.path,
            trigger.base.is_flow,
            authed.username,
            authed.email,
            w_id,
            path,
            trigger.error_handling.error_handler_path,
            trigger.error_handling.error_handler_args as _,
            trigger.error_handling.retry as _
        )
        .execute(tx)
        .await?;
        Ok(())
    }

    async fn test_connection(
        &self,
        db: &DB,
        authed: &ApiAuthed,
        user_db: &UserDB,
        workspace_id: &str,
        config: Self::TestConnectionConfig,
    ) -> Result<()> {
        let connect_f = async {
            get_default_pg_connection(
                authed.clone(),
                Some(user_db.clone()),
                db,
                &config.postgres_resource_path,
                workspace_id,
            )
            .await
            .map_err(|err| {
                Error::BadConfig(format!("Error connecting to postgres: {}", err.to_string()))
            })?;

            Ok::<(), Error>(())
        };

        connect_f.await?;
        Ok(())
    }

    fn additional_routes(&self) -> Router {
        Router::new()
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

async fn check_if_logical_replication_slot_exist(
    pg_connection: &mut Client,
    replication_slot_name: &str,
) -> Result<bool> {
    let row = pg_connection
        .query_opt(
            "SELECT slot_name FROM pg_replication_slots WHERE slot_name = $1",
            &[&replication_slot_name],
        )
        .await
        .map_err(to_anyhow)?;
    Ok(row.is_some())
}

async fn create_custom_slot_and_publication_inner(
    authed: ApiAuthed,
    user_db: UserDB,
    db: &DB,
    postgres_resource_path: &str,
    w_id: &str,
    publication: &PublicationData,
) -> Result<PostgresPublicationReplication> {
    let mut pg_connection = get_default_pg_connection(
        authed.clone(),
        Some(user_db),
        &db,
        &postgres_resource_path,
        &w_id,
    )
    .await
    .map_err(to_anyhow)?;

    let tx = pg_connection.transaction().await.map_err(to_anyhow)?;

    let publication_name = format!("windmill_trigger_{}", generate_random_string());
    let replication_slot_name = publication_name.clone();

    create_logical_replication_slot(tx.client(), &replication_slot_name).await?;
    create_pg_publication(
        &tx.client(),
        &publication_name,
        publication.table_to_track.as_deref(),
        &publication.transaction_to_track,
    )
    .await?;

    tx.commit().await.map_err(to_anyhow)?;

    Ok(PostgresPublicationReplication::new(
        publication_name,
        replication_slot_name,
    ))
}

pub async fn get_postgres_version_internal(pg_connection: &Client) -> Result<String> {
    let row = pg_connection
        .query_one("SHOW server_version;", &[])
        .await
        .map_err(to_anyhow)?;

    let postgres_version: String = row.get(0);

    Ok(postgres_version)
}

pub async fn get_postgres_version(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, postgres_resource_path)): Path<(String, String)>,
) -> Result<String> {
    let pg_connection = get_default_pg_connection(
        authed.clone(),
        Some(user_db),
        &db,
        &postgres_resource_path,
        &w_id,
    )
    .await
    .map_err(to_anyhow)?;

    let postgres_version = get_postgres_version_internal(&pg_connection).await?;

    Ok(postgres_version)
}

pub async fn list_slot_name(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, postgres_resource_path)): Path<(String, String)>,
) -> Result<Json<Vec<SlotList>>> {
    let pg_connection: Client = get_default_pg_connection(
        authed.clone(),
        Some(user_db.clone()),
        &db,
        &postgres_resource_path,
        &w_id,
    )
    .await
    .map_err(to_anyhow)?;

    let rows = pg_connection
        .query(
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
            &[],
        )
        .await
        .map_err(to_anyhow)?;

    let slots = rows
        .into_iter()
        .map(|row| SlotList { slot_name: row.get("slot_name"), active: row.get("active") })
        .collect();

    Ok(Json(slots))
}

pub async fn create_slot(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, postgres_resource_path)): Path<(String, String)>,
    Json(Slot { name }): Json<Slot>,
) -> Result<String> {
    let pg_connection = get_default_pg_connection(
        authed.clone(),
        Some(user_db.clone()),
        &db,
        &postgres_resource_path,
        &w_id,
    )
    .await
    .map_err(to_anyhow)?;

    create_logical_replication_slot(&pg_connection, &name).await?;

    Ok(format!("Replication slot {} created!", name))
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

pub async fn drop_slot_name(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, postgres_resource_path)): Path<(String, String)>,
    Json(Slot { name }): Json<Slot>,
) -> Result<String> {
    let pg_connection =
        get_default_pg_connection(authed, Some(user_db), &db, &postgres_resource_path, &w_id)
            .await
            .map_err(to_anyhow)?;

    drop_logical_replication_slot(&pg_connection, &name)
        .await
        .map_err(to_anyhow)?;

    Ok(format!("Replication slot {} deleted!", name))
}

pub async fn list_database_publication(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, postgres_resource_path)): Path<(String, String)>,
) -> Result<Json<Vec<String>>> {
    let pg_connection = get_default_pg_connection(
        authed.clone(),
        Some(user_db.clone()),
        &db,
        &postgres_resource_path,
        &w_id,
    )
    .await
    .map_err(to_anyhow)?;

    let rows = pg_connection
        .query(
            "SELECT pubname AS publication_name FROM pg_publication;",
            &[],
        )
        .await
        .map_err(to_anyhow)?;

    let publications = rows
        .into_iter()
        .map(|row| row.get::<_, String>("publication_name"))
        .collect_vec();

    Ok(Json(publications))
}

pub async fn get_publication_info(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, publication_name, postgres_resource_path)): Path<(String, String, String)>,
) -> Result<Json<PublicationData>> {
    let mut pg_connection = get_default_pg_connection(
        authed.clone(),
        Some(user_db.clone()),
        &db,
        &postgres_resource_path,
        &w_id,
    )
    .await
    .map_err(to_anyhow)?;

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
    let mut pg_connection = get_default_pg_connection(
        authed.clone(),
        Some(user_db.clone()),
        &db,
        &postgres_resource_path,
        &w_id,
    )
    .await
    .map_err(to_anyhow)?;

    let PublicationData { table_to_track, transaction_to_track } = publication_data;

    let tx = pg_connection.transaction().await.map_err(to_anyhow)?;

    create_pg_publication(
        tx.client(),
        &publication_name,
        table_to_track.as_deref(),
        &transaction_to_track,
    )
    .await?;

    tx.commit().await.map_err(to_anyhow)?;

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
    let mut pg_connection = get_default_pg_connection(
        authed.clone(),
        Some(user_db.clone()),
        &db,
        &postgres_resource_path,
        &w_id,
    )
    .await
    .map_err(to_anyhow)?;

    drop_publication(&mut pg_connection, &publication_name).await?;

    Ok(format!(
        "Publication {} successfully deleted!",
        publication_name
    ))
}

pub async fn update_pg_publication(
    pg_connection: &Client,
    publication_name: &str,
    PublicationData { table_to_track, transaction_to_track }: PublicationData,
    all_table: Option<bool>,
) -> Result<()> {
    let quoted_publication_name = quote_identifier(publication_name);
    let transaction_to_track_as_str = transaction_to_track.iter().join(",");
    match table_to_track {
        Some(ref relations) if !relations.is_empty() => {
            // If all_table is None, the publication does not exist yet
            if all_table.unwrap_or(true) {
                if all_table.is_some_and(|all_table| all_table) {
                    drop_publication(pg_connection, publication_name)
                        .await
                        .map_err(to_anyhow)?;
                }
                create_pg_publication(
                    pg_connection,
                    publication_name,
                    table_to_track.as_deref(),
                    &transaction_to_track,
                )
                .await
                .map_err(to_anyhow)?;
            } else {
                let pg_14 = check_if_valid_publication_for_postgres_version(
                    pg_connection,
                    table_to_track.as_deref(),
                )
                .await
                .map_err(to_anyhow)?;

                let mut query = format!("ALTER PUBLICATION {} SET ", quoted_publication_name);
                let mut first = true;

                for (i, schema) in relations.iter().enumerate() {
                    if schema.table_to_track.is_empty() {
                        query.push_str("TABLES IN SCHEMA ");
                        query.push_str(&quote_identifier(&schema.schema_name));
                    } else {
                        if pg_14 && first {
                            query.push_str("TABLE ONLY ");
                            first = false;
                        } else if !pg_14 {
                            query.push_str("TABLE ONLY ");
                        }

                        for (j, table) in schema.table_to_track.iter().enumerate() {
                            let table_name = quote_identifier(&table.table_name);
                            let schema_name = quote_identifier(&schema.schema_name);
                            let full_name = format!("{}.{}", schema_name, table_name);
                            query.push_str(&full_name);

                            if let Some(columns) = table.columns_name.as_ref() {
                                let cols =
                                    columns.iter().map(|col| quote_identifier(col)).join(", ");
                                query.push_str(&format!(" ({})", cols));
                            }

                            if let Some(where_clause) = &table.where_clause {
                                query.push_str(&format!(" WHERE ({})", where_clause));
                            }

                            if j + 1 != schema.table_to_track.len() {
                                query.push_str(", ");
                            }
                        }
                    }

                    if i + 1 != relations.len() {
                        query.push_str(", ");
                    }
                }

                pg_connection
                    .execute(&query, &[])
                    .await
                    .map_err(to_anyhow)?;

                let publish_query = format!(
                    "ALTER PUBLICATION {} SET (publish = '{}');",
                    quoted_publication_name, transaction_to_track_as_str
                );
                pg_connection
                    .execute(&publish_query, &[])
                    .await
                    .map_err(to_anyhow)?;
            }
        }
        _ => {
            drop_publication(pg_connection, publication_name)
                .await
                .map_err(to_anyhow)?;
            let create_all_query = format!(
                "CREATE PUBLICATION {} FOR ALL TABLES WITH (publish = '{}');",
                quoted_publication_name, transaction_to_track_as_str
            );
            pg_connection
                .execute(&create_all_query, &[])
                .await
                .map_err(to_anyhow)?;
        }
    }

    Ok(())
}

pub async fn alter_publication(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, publication_name, postgres_resource_path)): Path<(String, String, String)>,
    Json(publication_data): Json<PublicationData>,
) -> Result<String> {
    let mut pg_connection = get_default_pg_connection(
        authed.clone(),
        Some(user_db.clone()),
        &db,
        &postgres_resource_path,
        &w_id,
    )
    .await
    .map_err(to_anyhow)?;

    let tx = pg_connection.transaction().await.map_err(to_anyhow)?;

    let publication = get_publication_scope_and_transaction(tx.client(), &publication_name)
        .await
        .map_err(to_anyhow)?;

    update_pg_publication(
        tx.client(),
        &publication_name,
        publication_data,
        publication.map(|publication| publication.0),
    )
    .await
    .map_err(to_anyhow)?;

    tx.commit().await.map_err(to_anyhow)?;

    Ok(format!(
        "Publication {} updated with success",
        publication_name
    ))
}

pub async fn get_publication_scope_and_transaction(
    pg_connection: &Client,
    publication_name: &str,
) -> Result<Option<(bool, Vec<String>)>> {
    let row_opt = pg_connection
        .query_opt(
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
            &[&publication_name],
        )
        .await
        .map_err(to_anyhow)?;

    let row = match row_opt {
        Some(r) => r,
        None => return Ok(None),
    };

    let all_table: bool = row.get("all_table");
    let pub_insert: bool = row.get("insert");
    let pub_update: bool = row.get("update");
    let pub_delete: bool = row.get("delete");

    let mut transaction_to_track = Vec::with_capacity(3);
    if pub_insert {
        transaction_to_track.push("insert".to_string());
    }
    if pub_update {
        transaction_to_track.push("update".to_string());
    }
    if pub_delete {
        transaction_to_track.push("delete".to_string());
    }

    Ok(Some((all_table, transaction_to_track)))
}

pub async fn get_tracked_relations(
    pg_connection: &Client,
    publication_name: &str,
) -> Result<Vec<Relations>> {
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

    let rows = pg_connection
        .query(query, &[&publication_name])
        .await
        .map_err(to_anyhow)?;

    let mut table_to_track: HashMap<String, Relations> = HashMap::new();

    for row in rows {
        let schema_name: Option<String> = row.get("schema_name");
        let table_name: Option<String> = row.get("table_name");
        let columns: Option<Vec<String>> = row.get("columns");
        let where_clause: Option<String> = row.get("where_clause");

        let schema_name = schema_name.ok_or_else::<Error, _>(|| {
            anyhow::anyhow!(
                "Unexpected NULL `schema_name` in publication entry (pubname: `{}`). This should never happen unless PostgreSQL internals are corrupted.",
                publication_name,
            )
            .into()
        })?;

        let table_name = table_name.ok_or_else::<Error, _>(|| {
            anyhow::anyhow!(
                "Unexpected NULL `table_name` for schema `{}` in publication `{}`. This should never happen unless PostgreSQL internals are corrupted.",
                schema_name,
                publication_name,
            )
            .into()
        })?;

        let entry = table_to_track.entry(schema_name.clone());
        let table_to_track_item = TableToTrack::new(table_name, where_clause, columns);

        match entry {
            std::collections::hash_map::Entry::Occupied(mut occupied) => {
                occupied.get_mut().add_new_table(table_to_track_item);
            }
            std::collections::hash_map::Entry::Vacant(vacant) => {
                vacant.insert(Relations::new(schema_name, vec![table_to_track_item]));
            }
        }
    }

    Ok(table_to_track.into_values().collect_vec())
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

    let relations = match relations {
        Some(r) => r,
        None => {
            return Err(
                anyhow::anyhow!("You must at least choose schema to fetch table from").into(),
            )
        }
    };

    let pg_connection: Client = get_default_pg_connection(
        authed.clone(),
        Some(user_db.clone()),
        &db,
        &postgres_resource_path,
        &w_id,
    )
    .await
    .map_err(to_anyhow)?;

    let mut schema_or_fully_qualified_name = Vec::with_capacity(relations.len());
    let mut columns_list = Vec::with_capacity(relations.len());

    for relation in relations {
        if !relation.table_to_track.is_empty() {
            for table in relation.table_to_track {
                let fully_qualified_name = format!("{}.{}", relation.schema_name, table.table_name);
                schema_or_fully_qualified_name.push(quote_literal(&fully_qualified_name));
                let columns = table
                    .columns_name
                    .map(|c| quote_literal(&c.join(",")))
                    .unwrap_or_else(|| "''".to_string());
                columns_list.push(columns);
            }
        } else {
            schema_or_fully_qualified_name.push(quote_literal(&relation.schema_name));
            columns_list.push("''".to_string());
        }
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
            FROM table_column_mapping tcm
        )
        SELECT
            ns.nspname AS table_schema,
            cls.relname AS table_name,
            attr.attname AS column_name,
            attr.atttypid AS oid,
            attr.attnotnull AS is_nullable
        FROM pg_attribute attr
        JOIN pg_class cls ON attr.attrelid = cls.oid
        JOIN pg_namespace ns ON cls.relnamespace = ns.oid
        JOIN parsed_columns pc
            ON ns.nspname || '.' || cls.relname = pc.table_name
            OR ns.nspname = pc.table_name
        WHERE
            attr.attnum > 0
            AND NOT attr.attisdropped
            AND cls.relkind = 'r'
            AND (
                pc.columns IS NULL
                OR attr.attname = ANY(pc.columns)
            );
        "#,
        tables_name, columns_list
    );

    let rows = pg_connection.query(&query, &[]).await.map_err(to_anyhow)?;

    let mut schema_map: HashMap<String, HashMap<String, Vec<MappingInfo>>> = HashMap::new();

    #[derive(Debug)]
    struct ColumnInfo {
        table_schema: String,
        table_name: String,
        column_name: String,
        oid: u32,
        is_nullable: bool,
    }

    for row in rows {
        let info = ColumnInfo {
            table_schema: row.get("table_schema"),
            table_name: row.get("table_name"),
            column_name: row.get("column_name"),
            oid: row.get::<_, u32>("oid"),
            is_nullable: row.get::<_, bool>("is_nullable"),
        };

        let mapped_info =
            MappingInfo::new(info.column_name, Type::from_oid(info.oid), info.is_nullable);

        match schema_map.entry(info.table_schema) {
            std::collections::hash_map::Entry::Occupied(mut schema_entry) => {
                match schema_entry.get_mut().entry(info.table_name) {
                    std::collections::hash_map::Entry::Occupied(mut table_entry) => {
                        table_entry.get_mut().push(mapped_info);
                    }
                    std::collections::hash_map::Entry::Vacant(v) => {
                        v.insert(vec![mapped_info]);
                    }
                }
            }
            std::collections::hash_map::Entry::Vacant(schema_vacant) => {
                let mut table_map = HashMap::new();
                table_map.insert(info.table_name, vec![mapped_info]);
                schema_vacant.insert(table_map);
            }
        }
    }

    let mapper = Mapper::new(schema_map, language);
    let template = mapper.get_template();

    let id = format!("{}-{}", w_id, uuid::Uuid::new_v4());

    TEMPLATE.insert(id.clone(), template);

    Ok(id)
}

pub async fn is_database_in_logical_level(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, postgres_resource_path)): Path<(String, String)>,
) -> error::JsonResult<bool> {
    let pg_connection = get_default_pg_connection(
        authed.clone(),
        Some(user_db.clone()),
        &db,
        &postgres_resource_path,
        &w_id,
    )
    .await
    .map_err(to_anyhow)?;

    let row_opt = pg_connection
        .query_opt("SHOW wal_level;", &[])
        .await
        .map_err(to_anyhow)?;

    let wal_level: Option<String> = row_opt.map(|row| row.get(0));

    let is_logical = matches!(wal_level.as_deref(), Some("logical"));

    Ok(Json(is_logical))
}
