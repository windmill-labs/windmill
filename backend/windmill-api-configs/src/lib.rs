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

use windmill_api_auth::{is_instance_admin, require_devops_role, ApiAuthed};

pub fn global_service() -> Router {
    Router::new()
        .route("/list_worker_groups", get(list_worker_groups))
        .route("/update/{name}", post(update_config).delete(delete_config))
        .route("/get/{name}", get(get_config))
        .route("/list", get(list_configs))
        .route(
            "/list_autoscaling_events/{worker_group}",
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
        .route(
            "/list_all_workspace_dependencies",
            get(list_all_workspace_dependencies),
        )
        .route(
            "/list_all_dedicated_with_deps",
            get(list_all_dedicated_with_deps),
        )
}

#[derive(Serialize, Deserialize, FromRow)]
struct Config {
    name: Option<String>,
    config: serde_json::Value,
}

/// Credential-bearing fields across the `ObjectSettings` variants, which are flattened into a
/// single object by the `type` tag.
const OBJECT_STORE_SECRET_KEYS: &[&str] =
    &["access_key", "secret_key", "accessKey", "serviceAccountKey"];

/// What an obfuscated read shows a caller who may not see the real credential.
const OBJECT_STORE_SECRET_MASK: &str = "*****";

/// Blank the secrets in one worker-group config, in place.
///
/// Worker-group configs are instance-global and expose `env_vars_static` and the bucket
/// credentials of `object_store_cache_config`; a job token (capped at workspace admin) gets this
/// view even when its identity is a superadmin, as does a devops user who is not an instance
/// admin. See `is_instance_admin` (GHSA-hfh4-cx4h-3fcr). Every route that returns a worker-group
/// config must go through here — a single unobfuscated read hands over the whole bucket.
fn obfuscate_worker_config(config: &mut serde_json::Value) {
    let Some(config) = config.as_object_mut() else {
        return;
    };
    if let Some(env_vars) = config
        .get_mut("env_vars_static")
        .and_then(|v| v.as_object_mut())
    {
        for (_, value) in env_vars.iter_mut() {
            // the value is a string, so to_string() it and take -2 to drop the quotes
            *value = serde_json::json!("*".repeat(value.to_string().len().saturating_sub(2)));
        }
    }
    if let Some(store) = config
        .get_mut("object_store_cache_config")
        .and_then(|v| v.as_object_mut())
    {
        for key in OBJECT_STORE_SECRET_KEYS {
            if let Some(secret) = store.get_mut(*key) {
                *secret = serde_json::json!(OBJECT_STORE_SECRET_MASK);
            }
        }
    }
}

/// Put back the credentials behind [`OBJECT_STORE_SECRET_MASK`]. A devops user who is not an
/// instance admin edits the group from the obfuscated view, so a plain save would otherwise
/// store the mask as the secret and take the group's dependency cache offline — silently, since
/// a worker that cannot build its override just falls back to caching on local disk.
async fn restore_masked_object_store_secrets(
    db: &DB,
    name: &str,
    config: &mut serde_json::Value,
) -> error::Result<()> {
    let Some(store) = config
        .get_mut("object_store_cache_config")
        .and_then(|v| v.as_object_mut())
    else {
        return Ok(());
    };
    let masked = OBJECT_STORE_SECRET_KEYS
        .iter()
        .filter(|k| store.get(**k).and_then(|v| v.as_str()) == Some(OBJECT_STORE_SECRET_MASK))
        .collect::<Vec<_>>();
    if masked.is_empty() {
        return Ok(());
    }

    let stored = sqlx::query_as!(
        Config,
        "SELECT name, config FROM config WHERE name = $1",
        name
    )
    .fetch_optional(db)
    .await?
    .map(|c| c.config);
    let stored = stored
        .as_ref()
        .and_then(|c| c.get("object_store_cache_config"))
        .and_then(|v| v.as_object());

    for key in masked {
        match stored.and_then(|s| s.get(*key)) {
            Some(secret) => store.insert(key.to_string(), secret.clone()),
            // Nothing to put back: drop the mask rather than store it.
            None => store.remove(*key),
        };
    }
    Ok(())
}

async fn list_worker_groups(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
) -> error::JsonResult<Vec<Config>> {
    let mut configs_raw = sqlx::query_as!(
        Config,
        "SELECT name, config FROM config WHERE name LIKE 'worker__%'"
    )
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
    if !is_instance_admin(&authed) {
        for config in configs_raw.iter_mut() {
            obfuscate_worker_config(&mut config.config);
        }
    }
    Ok(Json(configs_raw))
}

async fn get_config(
    authed: ApiAuthed,
    Path(name): Path<String>,
    Extension(db): Extension<DB>,
) -> error::JsonResult<Option<serde_json::Value>> {
    require_devops_role(&db, &authed).await?;

    let mut config = sqlx::query_as!(
        Config,
        "SELECT name, config FROM config WHERE name = $1",
        name
    )
    .fetch_optional(&db)
    .await?
    .map(|c| c.config);

    if !is_instance_admin(&authed) {
        if let Some(config) = config.as_mut() {
            obfuscate_worker_config(config);
        }
    }

    Ok(Json(config))
}

async fn update_config(
    Path(name): Path<String>,
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Json(mut config): Json<serde_json::Value>,
) -> error::Result<String> {
    require_devops_role(&db, &authed).await?;

    if name.starts_with("worker__") {
        restore_masked_object_store_secrets(&db, &name, &mut config).await?;
    }

    #[cfg(not(feature = "enterprise"))]
    let config = if name.starts_with("worker__") {
        // In CE, only allow setting worker_tags, cache_clear, init_bash, and native_mode
        serde_json::json!({
            "worker_tags": config.get("worker_tags"),
            "cache_clear": config.get("cache_clear"),
            "init_bash": config.get("init_bash"),
            "native_mode": config.get("native_mode")
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
    require_devops_role(&db, &authed).await?;

    let mut tx = db.begin().await?;

    let deleted = sqlx::query!("DELETE FROM config WHERE name = $1 RETURNING name", name)
        .fetch_all(&mut *tx)
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
    applied_at: chrono::DateTime<chrono::Utc>,
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

    // applied_at is a naive TIMESTAMP; reinterpret it as UTC so the response
    // includes a timezone (otherwise the browser parses it as local time and
    // TimeAgo clamps future timestamps to "0s ago").
    let events = sqlx::query_as!(
        AutoscalingEvent,
        r#"SELECT id, worker_group, event_type::text, desired_workers, reason, (applied_at AT TIME ZONE 'UTC') AS "applied_at!: chrono::DateTime<chrono::Utc>" FROM autoscaling_event WHERE worker_group = $1 ORDER BY applied_at DESC LIMIT $2 OFFSET $3"#,
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
    require_devops_role(&db, &authed).await.map_err(|e| {
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
    #[cfg(not(all(feature = "python", feature = "run_inline")))]
    return Err(error::Error::BadRequest(
        "Python listing available only with 'python' feature enabled".to_string(),
    ));

    #[cfg(all(feature = "python", feature = "run_inline"))]
    use itertools::Itertools;
    #[cfg(all(feature = "python", feature = "run_inline"))]
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
    require_devops_role(&db, &authed).await?;
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

#[derive(Serialize)]
struct WorkspaceDependencySummary {
    workspace_id: String,
    name: Option<String>,
    language: windmill_common::scripts::ScriptLang,
}

async fn list_all_workspace_dependencies(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
) -> error::JsonResult<Vec<WorkspaceDependencySummary>> {
    require_devops_role(&db, &authed).await?;
    let deps = sqlx::query!(
        r#"SELECT workspace_id, name, language AS "language: windmill_common::scripts::ScriptLang"
           FROM workspace_dependencies
           WHERE archived = false
           ORDER BY workspace_id, name"#,
    )
    .fetch_all(&db)
    .await?;
    Ok(Json(
        deps.into_iter()
            .map(|r| WorkspaceDependencySummary {
                workspace_id: r.workspace_id,
                name: r.name,
                language: r.language,
            })
            .collect(),
    ))
}

#[derive(Serialize)]
struct DedicatedScriptDepsWithWorkspace {
    workspace_id: String,
    path: String,
    language: windmill_common::scripts::ScriptLang,
    workspace_dep_names: Vec<String>,
}

async fn list_all_dedicated_with_deps(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
) -> error::JsonResult<Vec<DedicatedScriptDepsWithWorkspace>> {
    require_devops_role(&db, &authed).await?;

    let rows = sqlx::query!(
        r#"SELECT DISTINCT ON (workspace_id, path)
                  workspace_id, path, language AS "language: windmill_common::scripts::ScriptLang", content
           FROM script
           WHERE archived = false
             AND dedicated_worker = true
             AND language = ANY($1::text[]::SCRIPT_LANG[])
           ORDER BY workspace_id, path, created_at DESC"#,
        &["python3", "bun", "bunnative", "deno"] as &[&str],
    )
    .fetch_all(&db)
    .await?;

    let result = rows
        .into_iter()
        .map(|row| {
            let dep_names =
                windmill_common::scripts::extract_workspace_dependencies_annotated_refs(
                    &row.language,
                    &row.content,
                    &row.path,
                )
                .map(|refs| refs.external)
                .unwrap_or_default();
            DedicatedScriptDepsWithWorkspace {
                workspace_id: row.workspace_id,
                path: row.path,
                language: row.language,
                workspace_dep_names: dep_names,
            }
        })
        .collect();

    Ok(Json(result))
}
