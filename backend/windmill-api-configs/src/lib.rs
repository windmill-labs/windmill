/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use axum::{
    extract::{Extension, Path, Query},
    routing::{get, post},
    Json, Router,
};

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use windmill_audit::audit_oss::audit_log;
use windmill_audit::ActionKind;
use windmill_common::{
    error::{self},
    utils::Pagination,
    worker::MIN_PERIODIC_SCRIPT_INTERVAL_SECONDS,
    DB,
};

use windmill_api_auth::{require_devops_role, ApiAuthed};

pub fn global_service() -> Router {
    Router::new()
        .route("/list_worker_groups", get(list_worker_groups))
        .route("/update/:name", post(update_config).delete(delete_config))
        .route("/get/:name", get(get_config))
        .route("/list", get(list_configs))
        .route(
            "/list_autoscaling_events/:worker_group",
            get(list_autoscaling_events),
        )
        .route(
            "/native_kubernetes_autoscaling_healthcheck",
            get(native_kubernetes_autoscaling_healthcheck),
        )
        .route(
            "/list_available_python_versions",
            get(list_available_python_versions),
        )
}

#[derive(Serialize, Deserialize, FromRow)]
struct Config {
    name: Option<String>,
    config: serde_json::Value,
}

async fn list_worker_groups(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
) -> error::JsonResult<Vec<Config>> {
    let mut configs_raw =
        sqlx::query_as!(Config, "SELECT * FROM config WHERE name LIKE 'worker__%'")
            .fetch_all(&db)
            .await?;
    // Remove the 'worker__' prefix from all config names
    for config in configs_raw.iter_mut() {
        if let Some(name) = &config.name {
            if name.starts_with("worker__") {
                config.name = Some(name.strip_prefix("worker__").unwrap().to_string());
            }
        }
    }
    let configs = if !authed.is_admin {
        let mut obfuscated_configs: Vec<Config> = vec![];
        for config in configs_raw {
            let config_value_opt = config.config.as_object().map(|obj| obj.to_owned());
            if let Some(mut config_value) = config_value_opt {
                if let Some(env_var_map) = config_value
                    .get("env_vars_static")
                    .map(|obj| obj.as_object())
                    .flatten()
                {
                    let mut new_env_var_map: serde_json::Map<String, serde_json::Value> =
                        serde_json::Map::new();
                    for (key, value) in env_var_map {
                        new_env_var_map.insert(
                            key.to_owned(),
                            // we know the value is a string here, so we to_string() it and take -2 to remove the quotes
                            serde_json::json!("*".repeat(value.to_string().len() - 2)),
                        );
                    }
                    config_value.insert(
                        "env_vars_static".to_string(),
                        serde_json::Value::Object(new_env_var_map),
                    );
                }
                obfuscated_configs.push(Config {
                    name: config.name,
                    config: serde_json::Value::Object(config_value),
                })
            }
        }
        obfuscated_configs
    } else {
        configs_raw
    };
    Ok(Json(configs))
}

async fn get_config(
    authed: ApiAuthed,
    Path(name): Path<String>,
    Extension(db): Extension<DB>,
) -> error::JsonResult<Option<serde_json::Value>> {
    require_devops_role(&db, &authed.email).await?;

    let config = sqlx::query_as!(Config, "SELECT * FROM config WHERE name = $1", name)
        .fetch_optional(&db)
        .await?
        .map(|c| c.config);

    Ok(Json(config))
}

async fn update_config(
    Path(name): Path<String>,
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Json(config): Json<serde_json::Value>,
) -> error::Result<String> {
    require_devops_role(&db, &authed.email).await?;

    #[cfg(not(feature = "enterprise"))]
    let config = if name.starts_with("worker__") {
        // In CE, only allow setting worker_tags, cache_clear, and init_bash
        serde_json::json!({
            "worker_tags": config.get("worker_tags"),
            "cache_clear": config.get("cache_clear"),
            "init_bash": config.get("init_bash")
        })
    } else {
        config
    };

    if name.starts_with("worker__") {
        let periodic_script_bash = config
            .get("periodic_script_bash")
            .filter(|v| !v.is_null())
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty());

        let periodic_script_interval = config
            .get("periodic_script_interval_seconds")
            .filter(|v| !v.is_null());

        match (periodic_script_bash, periodic_script_interval) {
            (Some(_), Some(interval_value)) => {
                if let Some(interval) = interval_value.as_u64() {
                    if interval < MIN_PERIODIC_SCRIPT_INTERVAL_SECONDS {
                        return Err(error::Error::BadRequest(format!(
                            "Periodic script interval must be at least {} seconds, got {} seconds",
                            MIN_PERIODIC_SCRIPT_INTERVAL_SECONDS, interval
                        )));
                    }
                } else {
                    return Err(error::Error::BadRequest(
                        "Periodic script interval must be a valid number".to_string(),
                    ));
                }
            }
            (Some(_), None) => {
                return Err(error::Error::BadRequest(
                    "Periodic script interval must be specified when periodic script is configured"
                        .to_string(),
                ));
            }
            _ => {}
        }
    }

    let mut tx = db.begin().await?;
    sqlx::query!(
        "INSERT INTO config (name, config) VALUES ($1, $2) ON CONFLICT (name) DO UPDATE SET config = EXCLUDED.config",
        &name,
        config
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "worker_config.update",
        ActionKind::Update,
        "global",
        Some(&name),
        None,
    )
    .await?;
    tx.commit().await?;
    Ok(format!("Updated config {name}"))
}

async fn delete_config(
    Path(name): Path<String>,
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
) -> error::Result<String> {
    require_devops_role(&db, &authed.email).await?;

    let mut tx = db.begin().await?;

    let deleted = sqlx::query!("DELETE FROM config WHERE name = $1 RETURNING name", name)
        .fetch_all(&db)
        .await?;

    audit_log(
        &mut *tx,
        &authed,
        "worker_config.delete",
        ActionKind::Delete,
        "global",
        Some(&name),
        None,
    )
    .await?;
    tx.commit().await?;
    if deleted.len() == 0 {
        return Err(error::Error::NotFound(format!(
            "Config {name} not found",
            name = name
        )));
    }
    Ok(format!("Deleted config {name}"))
}

#[derive(Serialize, Deserialize, FromRow)]
struct AutoscalingEvent {
    id: i64,
    worker_group: String,
    event_type: Option<String>,
    desired_workers: i32,
    reason: Option<String>,
    applied_at: chrono::NaiveDateTime,
}

async fn list_autoscaling_events(
    Extension(db): Extension<DB>,
    Path(worker_group): Path<String>,
    Query(mut pagination): Query<Pagination>,
) -> error::JsonResult<Vec<AutoscalingEvent>> {
    if pagination.per_page.is_none() {
        pagination.per_page = Some(5);
    }
    let (per_page, offset) = windmill_common::utils::paginate(pagination);

    let events = sqlx::query_as!(
        AutoscalingEvent,
        "SELECT id, worker_group, event_type::text, desired_workers, reason, applied_at FROM autoscaling_event WHERE worker_group = $1 ORDER BY applied_at DESC LIMIT $2 OFFSET $3",
        worker_group,
        per_page as i64,
        offset as i64
    )
    .fetch_all(&db)
    .await?;
    Ok(Json(events))
}

#[cfg(all(feature = "enterprise", feature = "private"))]
async fn native_kubernetes_autoscaling_healthcheck(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
) -> Result<(), windmill_autoscaling::kubernetes_integration_ee::KubeError> {
    require_devops_role(&db, &authed.email).await.map_err(|e| {
        windmill_autoscaling::kubernetes_integration_ee::KubeError::Other(e.to_string())
    })?;

    windmill_autoscaling::kubernetes_integration_ee::kubernetes_healthcheck().await
}

#[cfg(not(all(feature = "enterprise", feature = "private")))]
async fn native_kubernetes_autoscaling_healthcheck() -> Result<(), error::Error> {
    Err(error::Error::BadRequest(
        "Native Kubernetes autoscaling available only in the enterprise version".to_string(),
    ))
}

async fn list_available_python_versions() -> error::JsonResult<Vec<String>> {
    #[cfg(not(all(feature = "python", feature = "inline_preview")))]
    return Err(error::Error::BadRequest(
        "Python listing available only with 'python' feature enabled".to_string(),
    ));

    #[cfg(all(feature = "python", feature = "inline_preview"))]
    use itertools::Itertools;
    #[cfg(all(feature = "python", feature = "inline_preview"))]
    return Ok(Json(
        windmill_worker::PyV::list_available_python_versions()
            .await
            .iter()
            .map(|v| v.to_string())
            .collect_vec(),
    ));
}

#[cfg(feature = "enterprise")]
async fn list_configs(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
) -> error::JsonResult<Vec<Config>> {
    require_devops_role(&db, &authed.email).await?;
    let configs = sqlx::query_as!(Config, "SELECT name, config FROM config")
        .fetch_all(&db)
        .await?;
    Ok(Json(configs))
}

#[cfg(not(feature = "enterprise"))]
async fn list_configs() -> error::JsonResult<String> {
    Err(error::Error::BadRequest(
        "Config listing available only in the enterprise version".to_string(),
    ))
}
