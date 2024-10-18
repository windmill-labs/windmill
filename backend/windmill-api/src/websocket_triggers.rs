use axum::{
    extract::{Path, Query},
    routing::{delete, get, post},
    Extension, Json, Router,
};
use futures::StreamExt;
use http::StatusCode;
use itertools::Itertools;
use rand::seq::SliceRandom;
use serde::{
    de::{self, MapAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};
use serde_json::Value;
use sql_builder::{bind::Bind, SqlBuilder};
use sqlx::prelude::FromRow;
use std::{collections::HashMap, fmt};
use tokio_tungstenite::connect_async;
use windmill_audit::{audit_ee::audit_log, ActionKind};
use windmill_common::{
    db::UserDB,
    error::{self, JsonResult},
    utils::{not_found_if_none, paginate, require_admin, Pagination, StripPath},
    worker::{to_raw_value, CLOUD_HOSTED},
    INSTANCE_NAME,
};
use windmill_queue::PushArgsOwned;

use crate::{
    db::{ApiAuthed, DB},
    jobs::{
        run_wait_result_flow_by_path_internal, run_wait_result_script_by_path_internal, RunJobQuery,
    },
    users::fetch_api_authed,
};

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/create", post(create_websocket_trigger))
        .route("/list", get(list_websocket_triggers))
        .route("/get/*path", get(get_websocket_trigger))
        .route("/update/*path", post(update_websocket_trigger))
        .route("/delete/*path", delete(delete_websocket_trigger))
        .route("/exists/*path", get(exists_websocket_trigger))
        .route("/setenabled/*path", post(set_enabled))
}

#[derive(Deserialize)]
struct NewWebsocketTrigger {
    path: String,
    url: String,
    script_path: String,
    is_flow: bool,
    enabled: Option<bool>,
    filters: Vec<serde_json::Value>,
}

#[derive(FromRow, Serialize, Clone)]
pub struct WebsocketTrigger {
    workspace_id: String,
    path: String,
    url: String,
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
    filters: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
struct EditWebsocketTrigger {
    path: String,
    url: String,
    script_path: String,
    is_flow: bool,
    filters: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
pub struct ListWebsocketTriggerQuery {
    pub page: Option<usize>,
    pub per_page: Option<usize>,
    pub path: Option<String>,
    pub is_flow: Option<bool>,
    pub path_start: Option<String>,
}

async fn list_websocket_triggers(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(lst): Query<ListWebsocketTriggerQuery>,
) -> error::JsonResult<Vec<WebsocketTrigger>> {
    let mut tx = user_db.begin(&authed).await?;
    let (per_page, offset) = paginate(Pagination { per_page: lst.per_page, page: lst.page });
    let mut sqlb = SqlBuilder::select_from("websocket_trigger")
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
    let rows = sqlx::query_as::<_, WebsocketTrigger>(&sql)
        .fetch_all(&mut *tx)
        .await?;
    tx.commit().await?;

    Ok(Json(rows))
}

async fn get_websocket_trigger(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> error::JsonResult<WebsocketTrigger> {
    let mut tx = user_db.begin(&authed).await?;
    let path = path.to_path();
    let trigger = sqlx::query_as!(
        WebsocketTrigger,
        r#"SELECT *
          FROM websocket_trigger
          WHERE workspace_id = $1 AND path = $2"#,
        w_id,
        path,
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;

    let trigger = not_found_if_none(trigger, "Trigger", path)?;

    Ok(Json(trigger))
}

async fn create_websocket_trigger(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(ct): Json<NewWebsocketTrigger>,
) -> error::Result<(StatusCode, String)> {
    if *CLOUD_HOSTED {
        return Err(error::Error::BadRequest(
            "Websocket triggers are not supported on multi-tenant cloud, use dedicated cloud or self-host".to_string(),
        ));
    }
    require_admin(authed.is_admin, &authed.username)?;

    let mut tx = user_db.begin(&authed).await?;
    sqlx::query_as!(
        WebsocketTrigger,
      "INSERT INTO websocket_trigger (workspace_id, path, url, script_path, is_flow, enabled, filters, edited_by, email, edited_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, now()) RETURNING *",
      w_id,
      ct.path,
      ct.url,
      ct.script_path,
      ct.is_flow,
      ct.enabled.unwrap_or(true),
      &ct.filters,
      &authed.username,
      &authed.email
    )
    .fetch_one(&mut *tx).await?;

    audit_log(
        &mut *tx,
        &authed,
        "websocket_triggers.create",
        ActionKind::Create,
        &w_id,
        Some(ct.path.as_str()),
        None,
    )
    .await?;

    tx.commit().await?;

    Ok((StatusCode::CREATED, format!("{}", ct.path)))
}

async fn update_websocket_trigger(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Json(ct): Json<EditWebsocketTrigger>,
) -> error::Result<String> {
    let path = path.to_path();
    let mut tx = user_db.begin(&authed).await?;

    // important to update server_id, last_server_ping and error to NULL to stop current websocket listener
    sqlx::query!(
        "UPDATE websocket_trigger SET url = $1, script_path = $2, path = $3, is_flow = $4, filters = $5, edited_by = $6, email = $7, edited_at = now(), server_id = NULL, last_server_ping = NULL, error = NULL
            WHERE workspace_id = $8 AND path = $9",
        ct.url,
        ct.script_path,
        ct.path,
        ct.is_flow,
        &ct.filters,
        &authed.username,
        &authed.email,
        w_id,
        path,
    )
    .execute(&mut *tx).await?;

    audit_log(
        &mut *tx,
        &authed,
        "websocket_triggers.update",
        ActionKind::Create,
        &w_id,
        Some(path),
        None,
    )
    .await?;

    tx.commit().await?;

    Ok(path.to_string())
}

#[derive(Deserialize)]
pub struct SetEnabled {
    pub enabled: bool,
}

pub async fn set_enabled(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Json(payload): Json<SetEnabled>,
) -> error::Result<String> {
    let mut tx = user_db.begin(&authed).await?;
    let path = path.to_path();

    // important to set server_id, last_server_ping and error to NULL to stop current websocket listener
    let one_o = sqlx::query_scalar!(
        "UPDATE websocket_trigger SET enabled = $1, email = $2, edited_by = $3, edited_at = now(), server_id = NULL, last_server_ping = NULL, error = NULL
        WHERE path = $4 AND workspace_id = $5 RETURNING 1",
        payload.enabled,
        &authed.email,
        &authed.username,
        path,
        w_id,
    ).fetch_optional(&mut *tx).await?;

    not_found_if_none(one_o.flatten(), "Websocket trigger", path)?;

    audit_log(
        &mut *tx,
        &authed,
        "websocket_triggers.setenabled",
        ActionKind::Update,
        &w_id,
        Some(path),
        Some([("enabled", payload.enabled.to_string().as_ref())].into()),
    )
    .await?;

    tx.commit().await?;

    Ok(format!(
        "succesfully updated websocket trigger at path {} to status {}",
        path, payload.enabled
    ))
}

async fn delete_websocket_trigger(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> error::Result<String> {
    require_admin(authed.is_admin, &authed.username)?;
    let path = path.to_path();
    let mut tx = user_db.begin(&authed).await?;
    sqlx::query!(
        "DELETE FROM websocket_trigger WHERE workspace_id = $1 AND path = $2",
        w_id,
        path,
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "websocket_triggers.delete",
        ActionKind::Delete,
        &w_id,
        Some(path),
        None,
    )
    .await?;

    tx.commit().await?;

    Ok(format!("Websocket trigger {path} deleted"))
}

async fn exists_websocket_trigger(
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<bool> {
    let path = path.to_path();
    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM websocket_trigger WHERE path = $1 AND workspace_id = $2)",
        path,
        w_id,
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(false);
    Ok(Json(exists))
}

async fn listen_to_unlistened_websockets(
    db: &DB,
    rsmq: &Option<rsmq_async::MultiplexedRsmq>,
    killpill_rx: &tokio::sync::broadcast::Receiver<()>,
) -> () {
    match sqlx::query_as!(
        WebsocketTrigger,
        r#"SELECT *
            FROM websocket_trigger
            WHERE enabled IS TRUE AND (server_id IS NULL OR last_server_ping IS NULL OR last_server_ping < now() - interval '15 seconds')"#
    )
    .fetch_all(db)
    .await
    {
        Ok(mut triggers) => {
            triggers.shuffle(&mut rand::thread_rng());
            for trigger in triggers {
                maybe_listen_to_websocket(trigger, db.clone(), rsmq.clone(), killpill_rx.resubscribe()).await;
            }
        }
        Err(err) => {
            tracing::error!("Error fetching websocket triggers: {:?}", err);
        }
    };
}

pub async fn start_websockets(
    db: DB,
    rsmq: Option<rsmq_async::MultiplexedRsmq>,
    mut killpill_rx: tokio::sync::broadcast::Receiver<()>,
) -> () {
    tokio::spawn(async move {
        listen_to_unlistened_websockets(&db, &rsmq, &killpill_rx).await;
        loop {
            tokio::select! {
                biased;
                _ = killpill_rx.recv() => {
                    return;
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(15)) => {
                    listen_to_unlistened_websockets(&db, &rsmq, &killpill_rx).await;
                }
            }
        }
    });
}

async fn maybe_listen_to_websocket(
    ws_trigger: WebsocketTrigger,
    db: DB,
    rsmq: Option<rsmq_async::MultiplexedRsmq>,
    killpill_rx: tokio::sync::broadcast::Receiver<()>,
) -> () {
    match sqlx::query_scalar!(
        "UPDATE websocket_trigger SET server_id = $1, last_server_ping = now() WHERE enabled IS TRUE AND workspace_id = $2 AND path = $3 AND (server_id IS NULL OR last_server_ping IS NULL OR last_server_ping < now() - interval '15 seconds') RETURNING true",
        *INSTANCE_NAME,
        ws_trigger.workspace_id,
        ws_trigger.path,
    ).fetch_optional(&db).await {
        Ok(has_lock) => {
            if has_lock.flatten().unwrap_or(false) {
                tokio::spawn(listen_to_websocket(ws_trigger, db, rsmq, killpill_rx));
            } else {
                tracing::info!("Websocket {} already being listened to", ws_trigger.url);
            }
        },
        Err(err) => {
            tracing::error!("Error acquiring lock for websocket {}: {:?}", ws_trigger.path, err);
        }
    };
}

struct SupersetVisitor<'a> {
    key: &'a str,
    value_to_check: &'a Value,
}

impl<'de, 'a> Visitor<'de> for SupersetVisitor<'a> {
    type Value = bool;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a JSON object with a specific key at the top level")
    }

    fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
    where
        V: MapAccess<'de>,
    {
        while let Some(key) = map.next_key::<String>()? {
            if key == self.key {
                // Deserialize the value for the key and check if it's a superset
                let json_value: Value = map.next_value()?;
                tracing::info!("json_value: {:?}", json_value);
                tracing::info!("value_to_check: {:?}", self.value_to_check);
                return Ok(is_superset(&json_value, self.value_to_check));
            } else {
                // Skip the value if it's not the one we're interested in
                let _ = map.next_value::<de::IgnoredAny>()?;
            }
        }
        // If the key was not found, return false
        Ok(false)
    }
}

// Function to check if json_value is a superset of value_to_check
fn is_superset(json_value: &Value, value_to_check: &Value) -> bool {
    match (json_value, value_to_check) {
        (Value::Object(json_map), Value::Object(check_map)) => {
            // Check that all keys and values in check_map exist and match in json_map
            check_map.iter().all(|(k, v)| {
                json_map
                    .get(k)
                    .map_or(false, |json_val| is_superset(json_val, v))
            })
        }
        (Value::Array(json_array), Value::Array(check_array)) => {
            // Check that all elements in check_array exist in json_array
            check_array.iter().all(|check_item| {
                json_array
                    .iter()
                    .any(|json_item| is_superset(json_item, check_item))
            })
        }
        _ => json_value == value_to_check,
    }
}

// A function to deserialize and check if the value at the given key is a superset of a passed value
fn is_value_superset<'a, 'de, D>(
    deserializer: D,
    key: &'a str,
    value_to_check: &'a Value,
) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_map(SupersetVisitor { key, value_to_check })
}

async fn listen_to_websocket(
    ws_trigger: WebsocketTrigger,
    db: DB,
    rsmq: Option<rsmq_async::MultiplexedRsmq>,
    mut killpill_rx: tokio::sync::broadcast::Receiver<()>,
) -> () {
    async fn update_ping(db: DB, ws_trigger: &WebsocketTrigger, error: Option<&str>) -> Option<()> {
        match sqlx::query_scalar!(
            "UPDATE websocket_trigger SET last_server_ping = now(), error = $1 WHERE workspace_id = $2 AND path = $3 AND server_id = $4 AND enabled IS TRUE RETURNING 1",
            error,
            ws_trigger.workspace_id,
            ws_trigger.path,
            *INSTANCE_NAME
        ).fetch_optional(&db).await {
            Ok(updated) => {
                if updated.flatten().is_none() {
                    tracing::info!("Websocket {} changed, disabled, or deleted, stopping...", ws_trigger.url); 
                    return None;
                }
            },
            Err(err) => {
                tracing::warn!("Error updating ping of websocket {}: {:?}", ws_trigger.url, err);
            }
        };

        Some(())
    }

    let url = ws_trigger.url.as_str();

    #[derive(Deserialize)]
    struct JsonFilter {
        key: String,
        value: serde_json::Value,
    }

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Filter {
        JsonFilter(JsonFilter),
    }
    let filters: Vec<Filter> = ws_trigger
        .filters
        .iter()
        .filter_map(|m| serde_json::from_value(m.clone()).ok())
        .collect_vec();

    loop {
        tokio::select! {
            biased;
            _ = killpill_rx.recv() => {
                return;
            },
            connection = connect_async(url) => {
                match connection {
                    Ok((ws_stream, _)) => {
                        tracing::info!("Listening to websocket {}", url);
                        if let None = update_ping(db.clone(), &ws_trigger, None).await {
                            return;
                        }
                        let mut last_ping = tokio::time::Instant::now();
                        let (_, mut read) = ws_stream.split();
                        loop {
                            tokio::select! {
                                biased;
                                _ = killpill_rx.recv() => {
                                    return;
                                }
                                msg = read.next() => {
                                    if let Some(msg) = msg {
                                        if last_ping.elapsed() > tokio::time::Duration::from_secs(5) {
                                            if let None = update_ping(db.clone(), &ws_trigger, None).await {
                                                return;
                                            }
                                            last_ping = tokio::time::Instant::now();
                                        }
                                        match msg {
                                            Ok(msg) => {
                                                match msg {
                                                    tokio_tungstenite::tungstenite::Message::Text(text) => {
                                                        let mut should_handle = true;
                                                        for filter in &filters {
                                                            match filter {
                                                                Filter::JsonFilter(JsonFilter { key, value }) => {
                                                                    let mut deserializer = serde_json::Deserializer::from_str(text.as_str());
                                                                    should_handle = match is_value_superset(&mut deserializer, key, &value) {
                                                                        Ok(filter_match) => {
                                                                            filter_match
                                                                        },
                                                                        Err(err) => {
                                                                            tracing::warn!("Error deserializing filter for websocket {}: {:?}", url, err);
                                                                            false
                                                                        }
                                                                    };
                                                                }
                                                            }
                                                            if !should_handle {
                                                                break;
                                                            }
                                                        }
                                                        if should_handle {
                                                            let db_ = db.clone();
                                                            let rsmq_ = rsmq.clone();
                                                            let ws_trigger_ = ws_trigger.clone();
                                                            tokio::spawn(async move {
                                                                let url = ws_trigger_.url.clone();
                                                                if let Err(err) = run_job(db_, rsmq_, ws_trigger_, text).await {
                                                                    tracing::error!("Error running job on websocket {}: {:?}", url, err);
                                                                };
                                                            });
                                                        }
                                                    },
                                                    _ => {}
                                                }
                                            },
                                            Err(err) => {
                                                tracing::error!("Error reading from websocket {}: {:?}", url, err);
                                            }
                                        }
                                    } else {
                                        tracing::error!("Websocket {} closed", url);
                                        if let None =
                                            update_ping(db.clone(), &ws_trigger, Some("Websocket closed")).await
                                        {
                                            return;
                                        }
                                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                                        break;
                                    }
                                },
                                _ = tokio::time::sleep(tokio::time::Duration::from_secs(5)) => {
                                    if let None = update_ping(db.clone(), &ws_trigger, None).await {
                                        return;
                                    }
                                    last_ping = tokio::time::Instant::now();
                                },
                            }
                        }
                    }
                    Err(err) => {
                        tracing::error!("Error connecting to websocket {}: {:?}", url, err);
                        if let None =
                            update_ping(db.clone(), &ws_trigger, Some(err.to_string().as_str())).await
                        {
                            return;
                        }
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    }
                }
            }
        }
    }
}

async fn run_job(
    db: DB,
    rsmq: Option<rsmq_async::MultiplexedRsmq>,
    trigger: WebsocketTrigger,
    msg: String,
) -> anyhow::Result<()> {
    let args = PushArgsOwned {
        args: HashMap::from([("msg".to_string(), to_raw_value(&msg))]),
        extra: Some(HashMap::from([(
            "wm_trigger".to_string(),
            to_raw_value(&serde_json::json!({"kind": "websocket"})),
        )])),
    };
    let label_prefix = Some(format!("ws-{}-", trigger.path));

    let authed = fetch_api_authed(
        trigger.edited_by.clone(),
        trigger.email.clone(),
        &trigger.workspace_id,
        &db,
        "anonymous".to_string(),
    )
    .await?;

    let user_db = UserDB::new(db.clone());

    let run_query = RunJobQuery::default();

    if trigger.is_flow {
        run_wait_result_flow_by_path_internal(
            db,
            run_query,
            StripPath(trigger.script_path.to_owned()),
            authed,
            rsmq,
            user_db,
            args,
            trigger.workspace_id.clone(),
            label_prefix,
        )
        .await?;
    } else {
        run_wait_result_script_by_path_internal(
            db,
            run_query,
            StripPath(trigger.script_path.to_owned()),
            authed,
            rsmq,
            user_db,
            trigger.workspace_id.clone(),
            args,
            label_prefix,
        )
        .await?;
    }

    Ok(())
}
