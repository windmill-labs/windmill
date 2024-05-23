/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use axum::{
    extract::{Extension, Path},
    routing::{get, post},
    Json, Router,
};

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use windmill_audit::audit_ee::audit_log;
use windmill_audit::ActionKind;
use windmill_common::{
    error::{self},
    DB,
};

use crate::{db::ApiAuthed, utils::require_super_admin};

pub fn global_service() -> Router {
    Router::new()
        .route("/list_worker_groups", get(list_worker_groups))
        .route("/update/:name", post(update_config).delete(delete_config))
        .route("/get/:name", get(get_config))
}

#[derive(Serialize, Deserialize, FromRow)]
struct Config {
    name: String,
    config: serde_json::Value,
}

async fn list_worker_groups(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
) -> error::JsonResult<Vec<Config>> {
    let configs_raw = sqlx::query_as!(Config, "SELECT * FROM config WHERE name LIKE 'worker__%'")
        .fetch_all(&db)
        .await?;
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
    require_super_admin(&db, &authed.email).await?;

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
    require_super_admin(&db, &authed.email).await?;

    #[cfg(not(feature = "enterprise"))]
    if name.starts_with("worker__") {
        return Err(error::Error::BadRequest(
            "Worker groups configurable from UI available only in the enterprise version"
                .to_string(),
        ));
    }

    let mut tx = db.begin().await?;
    sqlx::query!(
        "INSERT INTO config (name, config) VALUES ($1, $2) ON CONFLICT (name) DO UPDATE SET config = $2",
        &name,
        config
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed.username,
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
    require_super_admin(&db, &authed.email).await?;

    let mut tx = db.begin().await?;

    let deleted = sqlx::query!("DELETE FROM config WHERE name = $1 RETURNING name", name)
        .fetch_all(&db)
        .await?;

    audit_log(
        &mut *tx,
        &authed.username,
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
