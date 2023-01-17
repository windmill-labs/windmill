/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use hyper::StatusCode;
use reqwest::Client;
use sql_builder::prelude::*;

use axum::{
    extract::{Extension, Path, Query},
    routing::{get, post},
    Json, Router,
};
use sql_builder::SqlBuilder;
use sqlx::{Postgres, Transaction};
use windmill_audit::{audit_log, ActionKind};
use windmill_common::{
    error::{self, to_anyhow, Error, JsonResult, Result},
    flows::{Flow, ListFlowQuery, ListableFlow, NewFlow},
    schedule::Schedule,
    utils::{
        http_get_from_hub, list_elems_from_hub, not_found_if_none, paginate, Pagination, StripPath,
    },
};
use windmill_queue::{push, schedule::push_scheduled_job, JobPayload};

use crate::{
    db::{UserDB, DB},
    schedule::clear_schedule,
    users::{require_owner_of_path, Authed},
};

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list", get(list_flows))
        .route("/create", post(create_flow))
        .route("/update/*path", post(update_flow))
        .route("/archive/*path", post(archive_flow_by_path))
        .route("/get/*path", get(get_flow_by_path))
        .route("/exists/*path", get(exists_flow_by_path))
        .route("/list_paths", get(list_paths))
}

pub fn global_service() -> Router {
    Router::new()
        .route("/hub/list", get(list_hub_flows))
        .route("/hub/get/:id", get(get_hub_flow_by_id))
}

async fn list_flows(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(pagination): Query<Pagination>,
    Query(lq): Query<ListFlowQuery>,
) -> JsonResult<Vec<ListableFlow>> {
    let (per_page, offset) = paginate(pagination);

    let mut sqlb = SqlBuilder::select_from("flow as o")
        .fields(&[
            "o.workspace_id",
            "o.path",
            "summary",
            "description",
            "edited_by",
            "edited_at",
            "archived",
            "extra_perms",
            "favorite.path IS NOT NULL as starred",
        ])
        .left()
        .join("favorite")
        .on(
            "favorite.favorite_kind = 'flow' AND favorite.workspace_id = o.workspace_id AND favorite.path = o.path AND favorite.usr = ?"
                .bind(&authed.username),
        )
        .order_desc("favorite.path IS NOT NULL")
        .order_by("edited_at", lq.order_desc.unwrap_or(true))
        .and_where("o.workspace_id = ? OR o.workspace_id = 'starter'".bind(&w_id))
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
    if lq.starred_only.unwrap_or(false) {
        sqlb.and_where_is_not_null("favorite.path");
    }

    let sql = sqlb.sql().map_err(|e| Error::InternalErr(e.to_string()))?;
    let mut tx = user_db.begin(&authed).await?;
    let rows = sqlx::query_as::<_, ListableFlow>(&sql)
        .fetch_all(&mut tx)
        .await?;
    tx.commit().await?;
    Ok(Json(rows))
}

async fn list_hub_flows(
    Authed { email, .. }: Authed,
    Extension(http_client): Extension<Client>,
) -> JsonResult<serde_json::Value> {
    let flows = list_elems_from_hub(
        http_client,
        "https://hub.windmill.dev/searchFlowData?approved=true",
        &email,
    )
    .await?;
    Ok(Json(flows))
}

async fn list_paths(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
) -> JsonResult<Vec<String>> {
    let mut tx = user_db.begin(&authed).await?;

    let flows = sqlx::query_scalar!(
        "SELECT distinct(path) FROM flow WHERE  workspace_id = $1",
        w_id
    )
    .fetch_all(&mut tx)
    .await?;
    tx.commit().await?;

    Ok(Json(flows))
}

pub async fn get_hub_flow_by_id(
    Authed { email, .. }: Authed,
    Path(id): Path<i32>,
    Extension(http_client): Extension<Client>,
) -> JsonResult<serde_json::Value> {
    let value = http_get_from_hub(
        http_client,
        &format!("https://hub.windmill.dev/flows/{id}/json"),
        &email,
        false,
    )
    .await?
    .json()
    .await
    .map_err(to_anyhow)?;
    Ok(Json(value))
}

async fn check_path_conflict<'c>(
    tx: &mut Transaction<'c, Postgres>,
    w_id: &str,
    path: &str,
) -> Result<()> {
    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM flow WHERE path = $1 AND workspace_id = $2)",
        path,
        w_id
    )
    .fetch_one(tx)
    .await?
    .unwrap_or(false);
    if exists {
        return Err(Error::BadRequest(format!("Flow {} already exists", path)));
    }
    return Ok(());
}

async fn create_flow(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(nf): Json<NewFlow>,
) -> Result<(StatusCode, String)> {
    // cron::Schedule::from_str(&ns.schedule).map_err(|e| error::Error::BadRequest(e.to_string()))?;
    let mut tx = user_db.clone().begin(&authed).await?;

    check_path_conflict(&mut tx, &w_id, &nf.path).await?;
    check_schedule_conflict(&mut tx, &w_id, &nf.path).await?;

    sqlx::query!(
        "INSERT INTO flow (workspace_id, path, summary, description, value, edited_by, edited_at, \
         schema, dependency_job) VALUES ($1, $2, $3, $4, $5, $6, now(), $7::text::json, NULL)",
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

    let tx = user_db.begin(&authed).await?;
    let (dependency_job_uuid, mut tx) = push(
        tx,
        &w_id,
        JobPayload::FlowDependencies { path: nf.path.clone() },
        serde_json::Map::new(),
        &authed.username,
        &authed.email,
        windmill_common::users::username_to_permissioned_as(&authed.username),
        None,
        None,
        None,
        false,
        false,
        None,
        true,
    )
    .await?;
    sqlx::query!(
        "UPDATE flow SET dependency_job = $1 WHERE path = $2 AND workspace_id = $3",
        dependency_job_uuid,
        nf.path,
        w_id
    )
    .execute(&mut tx)
    .await?;
    tx.commit().await?;

    Ok((StatusCode::CREATED, nf.path.to_string()))
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
    Extension(db): Extension<DB>,
    Path((w_id, flow_path)): Path<(String, StripPath)>,
    Json(nf): Json<NewFlow>,
) -> Result<String> {
    let mut tx = user_db.clone().begin(&authed).await?;

    let flow_path = flow_path.to_path();
    check_schedule_conflict(&mut tx, &w_id, flow_path).await?;

    let schema = nf.schema.map(|x| x.0);
    let old_dep_job = sqlx::query_scalar!(
        "SELECT dependency_job FROM flow WHERE path = $1 AND workspace_id = $2",
        flow_path,
        w_id
    )
    .fetch_optional(&mut tx)
    .await?;
    let old_dep_job = not_found_if_none(old_dep_job, "Flow", flow_path)?;
    sqlx::query!(
        "UPDATE flow SET path = $1, summary = $2, description = $3, value = $4, edited_by = $5, \
         edited_at = now(), schema = $6::text::json, dependency_job = NULL WHERE path = $7 AND workspace_id = $8",
        nf.path,
        nf.summary,
        nf.description,
        nf.value,
        &authed.username,
        schema.and_then(|x| serde_json::to_string(&x).ok()),
        flow_path,
        w_id,
    )
    .execute(&mut tx)
    .await?;

    if nf.path != flow_path {
        check_schedule_conflict(&mut tx, &w_id, &nf.path).await?;

        if !authed.is_admin {
            require_owner_of_path(&w_id, &authed.username, &authed.groups, &flow_path, &db).await?;
        }

        let mut schedulables = sqlx::query_as!(
            Schedule,
                "UPDATE schedule SET script_path = $1 WHERE script_path = $2 AND path != $2 AND workspace_id = $3 AND is_flow IS true RETURNING *",
                nf.path,
                flow_path,
                w_id,
            )
            .fetch_all(&mut tx)
            .await?;

        let schedule = sqlx::query_as!(Schedule,
            "UPDATE schedule SET path = $1, script_path = $1 WHERE path = $2 AND workspace_id = $3 AND is_flow IS true RETURNING *",
            nf.path,
            flow_path,
            w_id,
        )
        .fetch_optional(&mut tx)
        .await?;

        if let Some(schedule) = schedule {
            schedulables.push(schedule);
        }

        for schedule in schedulables {
            clear_schedule(&mut tx, flow_path, true).await?;

            if schedule.enabled {
                tx = push_scheduled_job(tx, schedule).await?;
            }
        }
    }

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

    let tx = user_db.begin(&authed).await?;
    let (dependency_job_uuid, mut tx) = push(
        tx,
        &w_id,
        JobPayload::FlowDependencies { path: nf.path.clone() },
        serde_json::Map::new(),
        &authed.username,
        &authed.email,
        windmill_common::users::username_to_permissioned_as(&authed.username),
        None,
        None,
        None,
        false,
        false,
        None,
        true,
    )
    .await?;
    sqlx::query!(
        "UPDATE flow SET dependency_job = $1 WHERE path = $2 AND workspace_id = $3",
        dependency_job_uuid,
        nf.path,
        w_id
    )
    .execute(&mut tx)
    .await?;
    if let Some(old_dep_job) = old_dep_job {
        sqlx::query!(
            "UPDATE queue SET canceled = true WHERE id = $1",
            old_dep_job
        )
        .execute(&mut tx)
        .await?;
    }
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

    let flow = not_found_if_none(flow_o, "Flow", path)?;
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

    use std::{collections::HashMap, time::Duration};

    use windmill_common::{
        flows::{
            ConstantDelay, ExponentialDelay, FlowModule, FlowModuleValue, FlowValue,
            InputTransform, Retry, StopAfterIf,
        },
        scripts,
    };

    const SECOND: Duration = Duration::from_secs(1);

    #[test]
    fn flowmodule_serde() {
        let fv = FlowValue {
            modules: vec![
                FlowModule {
                    id: "a".to_string(),
                    input_transforms: [].into(),
                    value: FlowModuleValue::Script {
                        path: "test".to_string(),
                        input_transforms: [(
                            "test".to_string(),
                            InputTransform::Static { value: serde_json::json!("test2") },
                        )]
                        .into(),
                        hash: None,
                    },
                    stop_after_if: None,
                    summary: None,
                    suspend: Default::default(),
                    retry: None,
                    sleep: None,
                },
                FlowModule {
                    id: "b".to_string(),
                    input_transforms: HashMap::new(),
                    value: FlowModuleValue::RawScript {
                        input_transforms: HashMap::new(),
                        content: "test".to_string(),
                        language: scripts::ScriptLang::Deno,
                        path: None,
                        lock: None,
                    },
                    stop_after_if: Some(StopAfterIf {
                        expr: "foo = 'bar'".to_string(),
                        skip_if_stopped: false,
                    }),
                    summary: None,
                    suspend: Default::default(),
                    retry: None,
                    sleep: None,
                },
                FlowModule {
                    id: "c".to_string(),
                    input_transforms: HashMap::new(),
                    value: FlowModuleValue::ForloopFlow {
                        iterator: InputTransform::Static { value: serde_json::json!([1, 2, 3]) },
                        modules: vec![],
                        skip_failures: true,
                        parallel: false,
                    },
                    stop_after_if: Some(StopAfterIf {
                        expr: "previous.isEmpty()".to_string(),
                        skip_if_stopped: false,
                    }),
                    summary: None,
                    suspend: Default::default(),
                    retry: None,
                    sleep: None,
                },
            ],
            failure_module: Some(FlowModule {
                id: "d".to_string(),
                input_transforms: HashMap::new(),
                value: FlowModuleValue::Script {
                    path: "test".to_string(),
                    input_transforms: HashMap::new(),
                    hash: None,
                },
                stop_after_if: Some(StopAfterIf {
                    expr: "previous.isEmpty()".to_string(),
                    skip_if_stopped: false,
                }),
                summary: None,
                suspend: Default::default(),
                retry: None,
                sleep: None,
            }),
            same_worker: false,
        };
        let expect = serde_json::json!({
          "modules": [
            {
              "id": "a",
              "input_transforms": {},
              "value": {
                "input_transforms": {
                    "test": {
                      "type": "static",
                      "value": "test2"
                    }
                  },
                "type": "script",
                "path": "test"
              },
              "stop_after_if": null,
              "summary": null
            },
            {
              "id": "b",
              "input_transforms": {},
              "value": {
                "input_transforms": {},
                "type": "rawscript",
                "content": "test",
                "lock": null,
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
              "id": "c",
              "input_transforms": {},
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
                "parallel": false,
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
            "id": "d",
            "input_transforms": {},
            "value": {
              "input_transforms": {},
              "type": "script",
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
