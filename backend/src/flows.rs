/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::collections::HashMap;
use std::time::Duration;

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
    more_serde::{default_true, is_default},
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

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct FlowValue {
    pub modules: Vec<FlowModule>,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_default")]
    pub retry: Retry,
    #[serde(default)]
    pub failure_module: Option<FlowModule>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct StopAfterIf {
    pub expr: String,
    pub skip_if_stopped: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default, PartialEq)]
#[serde(default)]
pub struct Retry {
    constant: ConstantDelay,
    exponential: ExponentialDelay,
}

impl Retry {
    /// Takes the number of previous retries and returns the interval until the next retry if any.
    ///
    /// May return [`Duration::ZERO`] to retry immediately.
    pub fn interval(&self, previous_attempts: u16) -> Option<Duration> {
        let Self { constant, exponential } = self;

        if previous_attempts < constant.attempts {
            Some(Duration::from_secs(constant.seconds as u64))
        } else if previous_attempts - constant.attempts < exponential.attempts {
            let exp = previous_attempts.saturating_add(1) as u32;
            let secs = exponential.multiplier * exponential.seconds.saturating_pow(exp);
            Some(Duration::from_secs(secs as u64))
        } else {
            None
        }
    }

    pub fn has_attempts(&self) -> bool {
        self.constant.attempts != 0 || self.exponential.attempts != 0
    }

    pub fn max_attempts(&self) -> u16 {
        self.constant
            .attempts
            .saturating_add(self.exponential.attempts)
    }

    pub fn max_interval(&self) -> Option<Duration> {
        self.max_attempts()
            .checked_sub(1)
            .and_then(|p| self.interval(p))
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Default, PartialEq)]
#[serde(default)]
pub struct ConstantDelay {
    pub attempts: u16,
    pub seconds: u16,
}

/// multiplier * seconds ^ failures
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct ExponentialDelay {
    pub attempts: u16,
    pub multiplier: u16,
    pub seconds: u16,
}

impl Default for ExponentialDelay {
    fn default() -> Self {
        Self { attempts: 0, multiplier: 1, seconds: 0 }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct FlowModule {
    #[serde(default)]
    #[serde(alias = "input_transform")]
    pub input_transforms: HashMap<String, InputTransform>,
    pub value: FlowModuleValue,
    pub stop_after_if: Option<StopAfterIf>,
    pub summary: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_default")]
    pub suspend: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry: Option<Retry>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
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
        modules: Vec<FlowModule>,
        #[serde(default = "default_true")]
        skip_failures: bool,
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
    Authed { email, username, .. }: Authed,
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
    Authed { email, username, .. }: Authed,
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
        "INSERT INTO flow (workspace_id, path, summary, description, value, edited_by, edited_at, \
         schema) VALUES ($1, $2, $3, $4, $5, $6, now(), $7::text::json)",
        w_id,
        nf.path,
        nf.summary,
        nf.description,
        nf.value,
        &authed.username,
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
        "SELECT EXISTS (SELECT 1 FROM schedule WHERE path = $1 AND workspace_id = $2 AND path != \
         script_path)",
        path,
        w_id
    )
    .fetch_one(tx)
    .await?
    .unwrap_or(false);
    if exists_flow {
        return Err(error::Error::BadConfig(format!(
            "A flow cannot have the same path as a schedule if the schedule does not trigger that \
             same flow: {path}",
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
        "UPDATE flow SET path = $1, summary = $2, description = $3, value = $4, edited_by = $5, \
         edited_at = now(), schema = $6 WHERE path = $7 AND workspace_id = $8 RETURNING path",
        nf.path,
        nf.summary,
        nf.description,
        nf.value,
        &authed.username,
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
        "SELECT EXISTS(SELECT 1 FROM flow WHERE path = $1 AND (workspace_id = $2 OR workspace_id \
         = 'starter'))",
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

    const SECOND: Duration = Duration::from_secs(1);

    #[test]
    fn flowmodule_serde() {
        let fv = FlowValue {
            modules: vec![
                FlowModule {
                    input_transforms: [(
                        "test".to_string(),
                        InputTransform::Static { value: serde_json::json!("test2") },
                    )]
                    .into(),
                    value: FlowModuleValue::Script { path: "test".to_string() },
                    stop_after_if: None,
                    summary: None,
                    suspend: Default::default(),
                    retry: None,
                },
                FlowModule {
                    input_transforms: HashMap::new(),
                    value: FlowModuleValue::RawScript(RawCode {
                        content: "test".to_string(),
                        language: crate::scripts::ScriptLang::Deno,
                        path: None,
                    }),
                    stop_after_if: Some(StopAfterIf {
                        expr: "foo = 'bar'".to_string(),
                        skip_if_stopped: false,
                    }),
                    summary: None,
                    suspend: Default::default(),
                    retry: None,
                },
                FlowModule {
                    input_transforms: [(
                        "iterand".to_string(),
                        InputTransform::Static { value: serde_json::json!(vec![1, 2, 3]) },
                    )]
                    .into(),
                    value: FlowModuleValue::ForloopFlow {
                        iterator: InputTransform::Static { value: serde_json::json!([1, 2, 3]) },
                        modules: vec![],
                        skip_failures: true,
                    },
                    stop_after_if: Some(StopAfterIf {
                        expr: "previous.isEmpty()".to_string(),
                        skip_if_stopped: false,
                    }),
                    summary: None,
                    suspend: Default::default(),
                    retry: None,
                },
            ],
            failure_module: Some(FlowModule {
                input_transforms: HashMap::new(),
                value: FlowModuleValue::Flow { path: "test".to_string() },
                stop_after_if: Some(StopAfterIf {
                    expr: "previous.isEmpty()".to_string(),
                    skip_if_stopped: false,
                }),
                summary: None,
                suspend: Default::default(),
                retry: None,
            }),
            retry: Default::default(),
        };
        let expect = serde_json::json!({
          "modules": [
            {
              "input_transforms": {
                "test": {
                  "type": "static",
                  "value": "test2"
                }
              },
              "value": {
                "type": "script",
                "path": "test"
              },
              "stop_after_if": null,
              "summary": null
            },
            {
              "input_transforms": {},
              "value": {
                "type": "rawscript",
                "content": "test",
                "path": null,
                "language": "deno"
              },
              "stop_after_if": {
                  "expr": "foo = 'bar'",
                  "skip_if_stopped": false
              },
              "summary": null
            },
            {
              "input_transforms": {
                "iterand": {
                  "type": "static",
                  "value": [
                    1,
                    2,
                    3
                  ]
                }
              },
              "value": {
                "type": "forloopflow",
                "iterator": {
                  "type": "static",
                  "value": [
                    1,
                    2,
                    3
                  ]
                },
                "skip_failures": true,
                "modules": []
              },
              "stop_after_if": {
                  "expr": "previous.isEmpty()",
                  "skip_if_stopped": false,
              },
              "summary": null
            }
          ],
          "failure_module": {
            "input_transforms": {},
            "value": {
              "type": "flow",
              "path": "test"
            },
            "stop_after_if": {
                "expr": "previous.isEmpty()",
                "skip_if_stopped": false
            },
            "summary": null
          }
        });
        assert_eq!(dbg!(serde_json::json!(fv)), dbg!(expect));
    }

    #[test]
    fn test_back_compat() {
        /* renamed input_transform -> input_transforms but should deserialize old name */
        let s = r#"
        {
            "value": {
                "type": "rawscript",
                "content": "def main(n): return",
                "language": "python3"
            },
            "input_transform": {
                "n": {
                    "expr": "flow_input.iter.value",
                    "type": "javascript"
                }
            }
        }
        "#;
        let module: FlowModule = serde_json::from_str(s).unwrap();
        assert_eq!(
            module.input_transforms["n"],
            InputTransform::Javascript { expr: "flow_input.iter.value".to_string() }
        );
    }

    #[test]
    fn retry_serde() {
        assert_eq!(Retry::default(), serde_json::from_str(r#"{}"#).unwrap());

        assert_eq!(
            Retry::default(),
            serde_json::from_str(
                r#"
                {
                  "constant": {
                    "seconds": 0
                  },
                  "exponential": {
                    "multiplier": 1,
                    "seconds": 0
                  }
                }
                "#
            )
            .unwrap()
        );

        assert_eq!(
            Retry {
                constant: Default::default(),
                exponential: ExponentialDelay { attempts: 0, multiplier: 1, seconds: 123 }
            },
            serde_json::from_str(
                r#"
                {
                  "constant": {},
                  "exponential": { "seconds": 123 }
                }
                "#
            )
            .unwrap()
        );
    }

    #[test]
    fn retry_exponential() {
        let retry = Retry {
            constant: ConstantDelay::default(),
            exponential: ExponentialDelay { attempts: 3, multiplier: 4, seconds: 3 },
        };
        assert_eq!(
            vec![
                Some(12 * SECOND),
                Some(36 * SECOND),
                Some(108 * SECOND),
                None
            ],
            (0..4)
                .map(|previous_attempts| retry.interval(previous_attempts))
                .collect::<Vec<_>>()
        );

        assert_eq!(Some(108 * SECOND), retry.max_interval());
    }

    #[test]
    fn retry_both() {
        let retry = Retry {
            constant: ConstantDelay { attempts: 2, seconds: 4 },
            exponential: ExponentialDelay { attempts: 2, multiplier: 1, seconds: 3 },
        };
        assert_eq!(
            vec![
                Some(4 * SECOND),
                Some(4 * SECOND),
                Some(27 * SECOND),
                Some(81 * SECOND),
                None,
            ],
            (0..5)
                .map(|previous_attempts| retry.interval(previous_attempts))
                .collect::<Vec<_>>()
        );

        assert_eq!(Some(81 * SECOND), retry.max_interval());
    }
}
