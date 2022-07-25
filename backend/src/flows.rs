/*
 * Author & Copyright: Ruben Fiszel 2021
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::collections::HashMap;

use reqwest::Client;
use sql_builder::prelude::*;

use axum::{
    extract::{Extension, Host, Path, Query},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sql_builder::SqlBuilder;
use sqlx::{FromRow, Postgres, Transaction};

use crate::{
    audit::{audit_log, ActionKind},
    db::{UserDB, DB},
    error::{self, to_anyhow, Error, JsonResult, Result},
    jobs::RawCode,
    scripts::Schema,
    users::Authed,
    utils::{http_get_from_hub, list_elems_from_hub, Pagination, StripPath},
};

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list", get(list_flows))
        .route("/create", post(create_flow))
        .route("/update/*path", post(update_flow))
        .route("/archive/*path", post(archive_flow_by_path))
        .route("/get/*path", get(get_flow_by_path))
        .route("/exists/*path", get(exists_flow_by_path))
}

pub fn global_service() -> Router {
    Router::new()
        .route("/hub/list", get(list_hub_flows))
        .route("/hub/get/:id", get(get_hub_flow_by_id))
}

#[derive(FromRow, Serialize)]
pub struct Flow {
    pub workspace_id: String,
    pub path: String,
    pub summary: String,
    pub description: String,
    pub value: serde_json::Value,
    pub edited_by: String,
    pub edited_at: chrono::DateTime<chrono::Utc>,
    pub archived: bool,
    pub schema: Option<Schema>,
    pub extra_perms: serde_json::Value,
}

#[derive(FromRow, Deserialize)]
pub struct NewFlow {
    pub path: String,
    pub summary: String,
    pub description: String,
    pub value: serde_json::Value,
    pub schema: Option<Schema>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct FlowValue {
    pub modules: Vec<FlowModule>,
    pub failure_module: Option<FlowModule>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct FlowModule {
    pub input_transform: HashMap<String, InputTransform>,
    pub value: FlowModuleValue,
    pub stop_after_if_expr: Option<String>,
    pub skip_if_stopped: Option<bool>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(
    tag = "type",
    rename_all(serialize = "lowercase", deserialize = "lowercase")
)]
pub enum InputTransform {
    Static { value: serde_json::Value },
    Javascript { expr: String },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(
    tag = "type",
    rename_all(serialize = "lowercase", deserialize = "lowercase")
)]
pub enum FlowModuleValue {
    Script {
        path: String,
    },
    ForloopFlow {
        iterator: InputTransform,
        value: Box<FlowValue>,
    },
    Flow {
        path: String,
    },
    RawScript(RawCode),
}

#[derive(Deserialize)]
pub struct ListFlowQuery {
    pub path_start: Option<String>,
    pub path_exact: Option<String>,
    pub edited_by: Option<String>,
    pub show_archived: Option<bool>,
    pub order_by: Option<String>,
    pub order_desc: Option<bool>,
}

async fn list_flows(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(pagination): Query<Pagination>,
    Query(lq): Query<ListFlowQuery>,
) -> JsonResult<Vec<Flow>> {
    let (per_page, offset) = crate::utils::paginate(pagination);

    let mut sqlb = SqlBuilder::select_from("flow as o")
        .fields(&[
            "workspace_id",
            "path",
            "summary",
            "description",
            "'{}'::jsonb as value",
            "edited_by",
            "edited_at",
            "archived",
            "null schema",
            "extra_perms",
        ])
        .order_by("edited_at", lq.order_desc.unwrap_or(true))
        .and_where("workspace_id = ? OR workspace_id = 'starter'".bind(&w_id))
        .offset(offset)
        .limit(per_page)
        .clone();

    if !lq.show_archived.unwrap_or(false) {
        sqlb.and_where_eq("archived", false);
    }
    if let Some(ps) = &lq.path_start {
        sqlb.and_where_like_left("path", "?".bind(ps));
    }
    if let Some(p) = &lq.path_exact {
        sqlb.and_where_eq("path", "?".bind(p));
    }
    if let Some(cb) = &lq.edited_by {
        sqlb.and_where_eq("edited_by", "?".bind(cb));
    }

    let sql = sqlb.sql().map_err(|e| Error::InternalErr(e.to_string()))?;
    let mut tx = user_db.begin(&authed).await?;
    let rows = sqlx::query_as::<_, Flow>(&sql).fetch_all(&mut tx).await?;
    tx.commit().await?;
    Ok(Json(rows))
}

async fn list_hub_flows(
    Authed {
        email, username, ..
    }: Authed,
    Extension(http_client): Extension<Client>,
    Host(host): Host,
) -> JsonResult<serde_json::Value> {
    let flows = list_elems_from_hub(
        http_client,
        "https://hub.windmill.dev/searchFlowData?approved=true",
        email,
        username,
        host,
    )
    .await?;
    Ok(Json(flows))
}

pub async fn get_hub_flow_by_id(
    Authed {
        email, username, ..
    }: Authed,
    Path(id): Path<i32>,
    Extension(http_client): Extension<Client>,
    Host(host): Host,
) -> JsonResult<serde_json::Value> {
    let value = http_get_from_hub(
        http_client,
        &format!("https://hub.windmill.dev/flows/{id}/json"),
        email,
        username,
        host,
        false,
    )
    .await?
    .json()
    .await
    .map_err(to_anyhow)?;
    Ok(Json(value))
}

async fn create_flow(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(nf): Json<NewFlow>,
) -> Result<String> {
    // cron::Schedule::from_str(&ns.schedule).map_err(|e| error::Error::BadRequest(e.to_string()))?;
    let mut tx = user_db.begin(&authed).await?;

    check_schedule_conflict(&mut tx, &w_id, &nf.path).await?;

    sqlx::query!(
        "INSERT INTO flow (workspace_id, path, summary, description, value, edited_by, edited_at, schema) VALUES ($1, $2, $3, $4, $5, $6, $7, $8::text::json)",
        w_id,
        nf.path,
        nf.summary,
        nf.description,
        nf.value,
        &authed.username,
        &chrono::Utc::now(),
        nf.schema.and_then(|x| serde_json::to_string(&x.0).ok()),
    )
    .execute(&mut tx)
    .await?;

    audit_log(
        &mut tx,
        &authed.username,
        "flows.create",
        ActionKind::Create,
        &w_id,
        Some(&nf.path.to_string()),
        Some(
            [Some(("flow", nf.path.as_str()))]
                .into_iter()
                .flatten()
                .collect(),
        ),
    )
    .await?;

    tx.commit().await?;
    Ok(nf.path.to_string())
}

async fn check_schedule_conflict<'c>(
    tx: &mut Transaction<'c, Postgres>,
    w_id: &str,
    path: &str,
) -> error::Result<()> {
    let exists_flow = sqlx::query_scalar!(
        "SELECT EXISTS (SELECT 1 FROM schedule WHERE path = $1 AND workspace_id = $2 AND path != script_path)",
        path,
        w_id
    )
    .fetch_one(tx)
    .await?
    .unwrap_or(false);
    if exists_flow {
        return Err(error::Error::BadConfig(format!(
            "A flow cannot have the same path as a schedule if the schedule does not trigger that same flow: {path}",
        )));
    };
    Ok(())
}

async fn update_flow(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, flow_path)): Path<(String, StripPath)>,
    Json(nf): Json<NewFlow>,
) -> Result<String> {
    let mut tx = user_db.begin(&authed).await?;

    let flow_path = flow_path.to_path();
    check_schedule_conflict(&mut tx, &w_id, flow_path).await?;

    let schema = nf.schema.map(|x| x.0);
    let flow = sqlx::query_scalar!(
        "UPDATE flow SET path = $1, summary = $2, description = $3, value = $4, edited_by = $5, edited_at = $6, schema = $7 WHERE path = $8 AND workspace_id = $9 RETURNING path",
        nf.path,
        nf.summary,
        nf.description,
        nf.value,
        &authed.username,
        &chrono::Utc::now(),
        schema,
        flow_path,
        w_id,
    )
    .fetch_optional(&mut tx)
    .await?;
    crate::utils::not_found_if_none(flow, "Flow", flow_path)?;

    audit_log(
        &mut tx,
        &authed.username,
        "flows.update",
        ActionKind::Create,
        &w_id,
        Some(&nf.path.to_string()),
        Some(
            [Some(("flow", nf.path.as_str()))]
                .into_iter()
                .flatten()
                .collect(),
        ),
    )
    .await?;

    tx.commit().await?;
    Ok(nf.path.to_string())
}

async fn get_flow_by_path(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<Flow> {
    let path = path.to_path();
    let mut tx = user_db.begin(&authed).await?;

    let flow_o = sqlx::query_as::<_, Flow>(
        "SELECT * FROM flow WHERE path = $1 AND (workspace_id = $2 OR workspace_id = 'starter')",
    )
    .bind(path)
    .bind(w_id)
    .fetch_optional(&mut tx)
    .await?;
    tx.commit().await?;

    let flow = crate::utils::not_found_if_none(flow_o, "Flow", path)?;
    Ok(Json(flow))
}

async fn exists_flow_by_path(
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<bool> {
    let path = path.to_path();

    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM flow WHERE path = $1 AND (workspace_id = $2 OR workspace_id = 'starter'))",
        path,
        w_id
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(false);

    Ok(Json(exists))
}

async fn archive_flow_by_path(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> Result<String> {
    let path = path.to_path();
    let mut tx = user_db.begin(&authed).await?;

    sqlx::query!(
        "UPDATE flow SET archived = true WHERE path = $1 AND workspace_id = $2",
        path,
        &w_id
    )
    .execute(&mut tx)
    .await?;

    audit_log(
        &mut tx,
        &authed.username,
        "flows.archive",
        ActionKind::Delete,
        &w_id,
        Some(path),
        Some([("workspace", w_id.as_str())].into()),
    )
    .await?;
    tx.commit().await?;

    Ok(format!("Flow {path} archived"))
}

#[cfg(test)]
mod tests {

    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_serialize() -> anyhow::Result<()> {
        let mut hm = HashMap::new();
        hm.insert(
            "test".to_owned(),
            InputTransform::Static {
                value: serde_json::json!("test2"),
            },
        );
        let fv = FlowValue {
            modules: vec![
                FlowModule {
                    input_transform: hm,
                    value: FlowModuleValue::Script {
                        path: "test".to_string(),
                    },
                    stop_after_if_expr: None,
                    skip_if_stopped: Some(false),
                },
                FlowModule {
                    input_transform: HashMap::new(),
                    value: FlowModuleValue::RawScript(RawCode {
                        content: "test".to_string(),
                        language: crate::scripts::ScriptLang::Deno,
                        path: None,
                    }),
                    stop_after_if_expr: Some("foo = 'bar'".to_string()),
                    skip_if_stopped: None,
                },
                FlowModule {
                    input_transform: [(
                        "iterand".to_string(),
                        InputTransform::Static {
                            value: serde_json::json!(vec![1, 2, 3]),
                        },
                    )]
                    .into(),
                    value: FlowModuleValue::ForloopFlow {
                        iterator: InputTransform::Static {
                            value: serde_json::json!([1, 2, 3]),
                        },
                        value: Box::new(FlowValue {
                            modules: vec![],
                            failure_module: None,
                        }),
                    },
                    stop_after_if_expr: Some("previous.res1.isEmpty()".to_string()),
                    skip_if_stopped: None,
                },
            ],
            failure_module: Some(FlowModule {
                input_transform: HashMap::new(),
                value: FlowModuleValue::Flow {
                    path: "test".to_string(),
                },
                stop_after_if_expr: Some("previous.res1.isEmpty()".to_string()),
                skip_if_stopped: None,
            }),
        };
        println!("{}", serde_json::json!(fv).to_string());
        Ok(())
    }
}
