use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::fs::{create_dir_all, remove_dir_all};
use std::path::{Component, Path, PathBuf};

#[cfg(feature = "python")]
use crate::ansible_executor::{get_git_repos_lock, AnsibleDependencyLocks};
use async_recursion::async_recursion;
use itertools::Itertools;
use serde_json::value::RawValue;
use serde_json::{from_value, json, Value};
use sha2::Digest;
use sqlx::types::Json;
use uuid::Uuid;
use windmill_common::assets::{clear_asset_usage, insert_asset_usage, AssetUsageKind};
use windmill_common::error::Error;
use windmill_common::error::Result;
use windmill_common::flows::{FlowModule, FlowModuleValue, FlowNodeId};
use windmill_common::get_latest_deployed_hash_for_path;
use windmill_common::jobs::JobPayload;
use windmill_common::scripts::{hash_script, NewScript, ScriptHash};
#[cfg(feature = "python")]
use windmill_common::worker::PythonAnnotations;
use windmill_common::worker::{to_raw_value, to_raw_value_owned, write_file, Connection};
#[cfg(feature = "python")]
use windmill_parser_yaml::AnsibleRequirements;

use windmill_common::{
    apps::AppScriptId,
    cache::{self, RawData},
    error::{self, to_anyhow},
    flows::{add_virtual_items_if_necessary, FlowValue},
    scripts::ScriptLang,
    DB,
};
use windmill_git_sync::{handle_deployment_metadata, DeployedObject};
#[cfg(feature = "python")]
use windmill_parser_py_imports::parse_relative_imports;
use windmill_parser_ts::parse_expr_for_imports;
use windmill_queue::{append_logs, CanceledBy, MiniPulledJob, PushIsolationLevel};

// TODO: To be removed in future versions
lazy_static::lazy_static! {
    static ref WMDEBUG_NO_HASH_CHANGE_ON_DJ: bool = std::env::var("WMDEBUG_NO_HASH_CHANGE_ON_DJ").is_ok();
    static ref WMDEBUG_NO_NEW_FLOW_VERSION_ON_DJ: bool = std::env::var("WMDEBUG_NO_NEW_FLOW_VERSION_ON_DJ").is_ok();
    static ref WMDEBUG_NO_NEW_APP_VERSION_ON_DJ: bool = std::env::var("WMDEBUG_NO_NEW_APP_VERSION_ON_DJ").is_ok();
    static ref WMDEBUG_NO_COMPONENTS_TO_RELOCK: bool = std::env::var("WMDEBUG_NO_COMPONENTS_TO_RELOCK").is_ok();
    static ref WMDEBUG_NO_DMAP_DISSOLVE: bool = std::env::var("WMDEBUG_NO_DMAP_DISSOLVE").is_ok();
}

use crate::common::OccupancyMetrics;
use crate::csharp_executor::generate_nuget_lockfile;

#[cfg(feature = "java")]
use crate::java_executor;

#[cfg(feature = "ruby")]
use crate::ruby_executor;

#[cfg(feature = "php")]
use crate::php_executor::{composer_install, parse_php_imports};
#[cfg(feature = "python")]
use crate::python_executor::{
    create_dependencies_dir, handle_python_reqs, split_requirements, uv_pip_compile,
};
#[cfg(feature = "rust")]
use crate::rust_executor::generate_cargo_lockfile;
use crate::{
    bun_executor::gen_bun_lockfile, deno_executor::generate_deno_lock,
    go_executor::install_go_dependencies,
};

// TODO: make sure these are ok
pub async fn update_script_dependency_map(
    job_id: &Uuid,
    db: &DB,
    w_id: &str,
    parent_path: &Option<String>,
    script_path: &str,
    relative_imports: Vec<String>,
) -> error::Result<()> {
    let importer_kind = "script";

    let mut tx = db.begin().await?;
    tx = clear_dependency_parent_path(parent_path, script_path, w_id, importer_kind, tx).await?;

    tx = clear_dependency_map_for_item(script_path, w_id, importer_kind, tx, &None, false).await?;

    if !relative_imports.is_empty() {
        let mut logs = "".to_string();
        logs.push_str("\n--- RELATIVE IMPORTS ---\n\n");
        logs.push_str(&relative_imports.join("\n"));

        tx = add_relative_imports_to_dependency_map(
            script_path,
            w_id,
            relative_imports,
            importer_kind,
            tx,
            &mut logs,
            None,
        )
        .await?;
        append_logs(job_id, w_id, logs, &db.into()).await;
    }
    tx.commit().await?;

    Ok(())
}

async fn add_relative_imports_to_dependency_map<'c>(
    script_path: &str,
    w_id: &str,
    relative_imports: Vec<String>,
    importer_kind: &str,
    mut tx: sqlx::Transaction<'c, sqlx::Postgres>,
    logs: &mut String,
    node_id: Option<String>,
) -> error::Result<sqlx::Transaction<'c, sqlx::Postgres>> {
    for import in relative_imports {
        sqlx::query!(
            "INSERT INTO dependency_map (workspace_id, importer_path, importer_kind, imported_path, importer_node_id)
                 VALUES ($1, $2, $4::text::IMPORTER_KIND, $3, $5) ON CONFLICT DO NOTHING",
            w_id,
            script_path,
            import,
            importer_kind,
            node_id.clone().unwrap_or_default()
        )
        .execute(&mut *tx)
        .await?;
        logs.push_str(&format!("{}\n", import));
    }
    Ok(tx)
}

async fn clear_dependency_map_for_item<'c>(
    item_path: &str,
    w_id: &str,
    importer_kind: &str,
    mut tx: sqlx::Transaction<'c, sqlx::Postgres>,
    importer_node_id: &Option<String>,
    // If true, function will remove all rows regardless of node_id
    ignore_node_id: bool,
) -> Result<sqlx::Transaction<'c, sqlx::Postgres>> {
    sqlx::query!(
        "DELETE FROM dependency_map
                 WHERE importer_path = $1 AND importer_kind = $3::text::IMPORTER_KIND
                 AND workspace_id = $2 AND ($4::text IS NULL OR importer_node_id = $4::text OR $5)",
        item_path,
        w_id,
        importer_kind,
        importer_node_id.clone(),
        ignore_node_id
    )
    .execute(&mut *tx)
    .await?;
    Ok(tx)
}

async fn clear_dependency_parent_path<'c>(
    parent_path: &Option<String>,
    item_path: &str,
    w_id: &str,
    importer_kind: &str,
    mut tx: sqlx::Transaction<'c, sqlx::Postgres>,
) -> Result<sqlx::Transaction<'c, sqlx::Postgres>> {
    if parent_path
        .as_ref()
        .is_some_and(|x| !x.is_empty() && x != item_path)
    {
        sqlx::query!(
            "DELETE FROM dependency_map
                 WHERE importer_path = $1 AND importer_kind = $3::text::IMPORTER_KIND
                 AND workspace_id = $2",
            parent_path.clone().unwrap(),
            w_id,
            importer_kind
        )
        .execute(&mut *tx)
        .await?;
    }
    Ok(tx)
}

fn try_normalize(path: &Path) -> Option<PathBuf> {
    let mut ret = PathBuf::new();

    for component in path.components() {
        match component {
            Component::Prefix(..) | Component::RootDir => return None,
            Component::CurDir => {}
            Component::ParentDir => {
                if !ret.pop() {
                    return None;
                }
            }
            Component::Normal(c) => {
                ret.push(c);
            }
        }
    }

    Some(ret)
}

fn parse_ts_relative_imports(raw_code: &str, script_path: &str) -> error::Result<Vec<String>> {
    let mut relative_imports = vec![];
    let r = parse_expr_for_imports(raw_code, true)?;
    for import in r {
        let import = import.trim_end_matches(".ts");
        if import.starts_with("/") {
            relative_imports.push(import.trim_start_matches("/").to_string());
        } else if import.starts_with(".") {
            let normalized = try_normalize(std::path::Path::new(&format!(
                "{}/../{}",
                script_path, import
            )));
            if let Some(normalized) = normalized {
                let normalized = normalized.to_str().unwrap().to_string();
                relative_imports.push(normalized);
            } else {
                tracing::error!("error canonicalizing path: {script_path} with import {import}");
            }
        }
    }

    Ok(relative_imports)
}

pub fn extract_relative_imports(
    raw_code: &str,
    script_path: &str,
    language: &Option<ScriptLang>,
) -> Option<Vec<String>> {
    match language {
        #[cfg(feature = "python")]
        Some(ScriptLang::Python3) => parse_relative_imports(&raw_code, script_path).ok(),
        Some(ScriptLang::Bun) | Some(ScriptLang::Bunnative) | Some(ScriptLang::Deno) => {
            parse_ts_relative_imports(&raw_code, script_path).ok()
        }
        _ => None,
    }
}

#[tracing::instrument(level = "trace", skip_all)]
pub async fn handle_dependency_job(
    job: &MiniPulledJob,
    preview_data: Option<&RawData>,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job_dir: &str,
    db: &DB,
    worker_name: &str,
    worker_dir: &str,
    base_internal_url: &str,
    token: &str,
    occupancy_metrics: &mut OccupancyMetrics,
) -> error::Result<Box<RawValue>> {
    dbg!("DEPENDENCY_JOB!");
    dbg!("DEPENDENCY_JOB!");
    dbg!("DEPENDENCY_JOB!");
    dbg!("DEPENDENCY_JOB!");
    dbg!("DEPENDENCY_JOB!");
    let script_path = job.runnable_path();
    let raw_deps = job
        .args
        .as_ref()
        .map(|x| {
            x.get("raw_deps")
                .is_some_and(|y| y.to_string().as_str() == "true")
        })
        .unwrap_or(false);

    let npm_mode = if job
        .script_lang
        .as_ref()
        .map(|v| v == &ScriptLang::Bun)
        .unwrap_or(false)
    {
        Some(
            job.args
                .as_ref()
                .map(|x| {
                    x.get("npm_mode")
                        .is_some_and(|y| y.to_string().as_str() == "true")
                })
                .unwrap_or(false),
        )
    } else {
        None
    };

    // `JobKind::Dependencies` job store either:
    // - A saved script `hash` in the `script_hash` column.
    // - Preview raw lock and code in the `queue` or `job` table.
    let script_data = &match job.runnable_id {
        Some(hash) => match cache::script::fetch(&Connection::from(db.clone()), hash).await {
            Ok(d) => Cow::Owned(d.0),
            Err(e) => {
                let logs2 = sqlx::query_scalar!(
                    "SELECT logs FROM job_logs WHERE job_id = $1 AND workspace_id = $2",
                    &job.id,
                    &job.workspace_id
                )
                .fetch_optional(db)
                .await?
                .flatten()
                .unwrap_or_else(|| "no logs".to_string());
                sqlx::query!(
                    "UPDATE script SET lock_error_logs = $1 WHERE hash = $2 AND workspace_id = $3",
                    &format!("{logs2}\n{e}"),
                    &job.runnable_id.unwrap_or(ScriptHash(0)).0,
                    &job.workspace_id
                )
                .execute(db)
                .await?;
                return Err(Error::ExecutionErr(format!(
                    "Error creating schema validator: {e}"
                )));
            }
        },
        _ => match preview_data {
            Some(RawData::Script(data)) => Cow::Borrowed(data),
            _ => return Err(Error::internal_err("expected script hash")),
        },
    };

    let content = capture_dependency_job(
        &job.id,
        job.script_lang.as_ref().map(|v| Ok(v)).unwrap_or_else(|| {
            Err(Error::internal_err(
                "Job Language required for dependency jobs".to_owned(),
            ))
        })?,
        &script_data.code,
        mem_peak,
        canceled_by,
        job_dir,
        db,
        worker_name,
        &job.workspace_id,
        worker_dir,
        base_internal_url,
        token,
        script_path,
        raw_deps,
        npm_mode,
        occupancy_metrics,
    )
    .await;

    match content {
        Ok(content) => {
            if job.runnable_id.is_none() {
                // it a one-off raw script dependency job, no need to update the db
                return Ok(to_raw_value_owned(
                    json!({ "status": "Successful lock file generation", "lock": content }),
                ));
            }

            let current_hash = job.runnable_id.unwrap_or(ScriptHash(0));
            let w_id = &job.workspace_id;
            let (deployment_message, parent_path) =
                get_deployment_msg_and_parent_path_from_args(job.args.clone());

            let script_info = sqlx::query_as::<_, windmill_common::scripts::Script>(
                "SELECT * FROM script WHERE hash = $1 AND workspace_id = $2",
            )
            .bind(&current_hash.0)
            .bind(w_id)
            .fetch_one(db)
            .await?;

            // DependencyJob can be triggered only from 2 places:
            // 1. create_script function in windmill-api/src/scripts.rs
            // 2. trigger_dependents_to_recompute_dependencies (in this file)
            //
            // First will **always** produce script with null in `lock`
            // where Second will **always** do with lock being not null
            let deployed_hash = if script_info.lock.is_some() && !*WMDEBUG_NO_HASH_CHANGE_ON_DJ {
                let path = script_info.path.clone();

                let mut tx = db.begin().await?;
                // This entire section exists to solve following problem:
                //
                // 2 workers, one script that depend on another in python
                // run the original script on both workers
                // you update the dependenecy of a relative import,
                // run it again until you ran it on both, normally it should fail on one of those
                //
                // It happens because every worker has cached their own script versions.
                // However usual dependency job does not update hash of the script (and cache is keyed by the hash).
                // This logical branch will create new script which will update the hash and automatically invalidate cache.
                //
                // IMPORTANT: This will **only** be triggered by another DependencyJob. It will never be triggered by script (re)deployement

                let ns = NewScript {
                    path: script_info.path,
                    parent_hash: Some(current_hash),
                    summary: script_info.summary,
                    description: script_info.description,
                    content: script_info.content,
                    schema: script_info.schema,
                    is_template: Some(script_info.is_template),
                    // TODO: Make it either None everywhere (particularely when raw reqs are calculated)
                    // Or handle this case and conditionally make Some (only with raw reqs)
                    lock: None,
                    language: script_info.language,
                    kind: Some(script_info.kind),
                    tag: script_info.tag,
                    draft_only: script_info.draft_only,
                    envs: script_info.envs,
                    concurrent_limit: script_info.concurrent_limit,
                    concurrency_time_window_s: script_info.concurrency_time_window_s,
                    cache_ttl: script_info.cache_ttl,
                    dedicated_worker: script_info.dedicated_worker,
                    ws_error_handler_muted: script_info.ws_error_handler_muted,
                    priority: script_info.priority,
                    timeout: script_info.timeout,
                    delete_after_use: script_info.delete_after_use,
                    restart_unless_cancelled: script_info.restart_unless_cancelled,
                    deployment_message: deployment_message.clone(),
                    concurrency_key: script_info.concurrency_key,
                    visible_to_runner_only: script_info.visible_to_runner_only,
                    no_main_func: script_info.no_main_func,
                    codebase: script_info.codebase,
                    has_preprocessor: script_info.has_preprocessor,
                    on_behalf_of_email: script_info.on_behalf_of_email,
                    assets: script_info.assets,
                };

                let new_hash = hash_script(&ns);

                sqlx::query!("
    INSERT INTO script
    (workspace_id, hash, path, parent_hashes, summary, description, content, \
    created_by, schema, is_template, extra_perms, lock, language, kind, tag, \
    draft_only, envs, concurrent_limit, concurrency_time_window_s, cache_ttl, \
    dedicated_worker, ws_error_handler_muted, priority, restart_unless_cancelled, \
    delete_after_use, timeout, concurrency_key, visible_to_runner_only, no_main_func, \
    codebase, has_preprocessor, on_behalf_of_email, schema_validation, assets)

    SELECT  workspace_id, $1, path, array_prepend($2::bigint, COALESCE(parent_hashes, '{}'::bigint[])), summary, description, \
            content, created_by, schema, is_template, extra_perms, $4, language, kind, tag, \
            draft_only, envs, concurrent_limit, concurrency_time_window_s, cache_ttl, \
            dedicated_worker, ws_error_handler_muted, priority, restart_unless_cancelled, \
            delete_after_use, timeout, concurrency_key, visible_to_runner_only, no_main_func, \
            codebase, has_preprocessor, on_behalf_of_email, schema_validation, assets

    FROM script WHERE hash = $2 AND workspace_id = $3;
            ",
                new_hash, current_hash.0, w_id, &content).execute(db).await?;
                tracing::info!("Updated script at path {} with hash {} to new hash {}", path, current_hash.0, new_hash);
                // Archive current
                sqlx::query!(
                    "UPDATE script SET archived = true WHERE hash = $1 AND workspace_id = $2",
                    current_hash.0,
                    w_id
                )
                .execute(&mut *tx)
                .await?;
                tracing::info!("Archived script at path {} from dependency job {}", path, current_hash.0);
                tx.commit().await?;

                ScriptHash(new_hash)
            } else {
                dbg!("OUT");
                dbg!("OUT");
                dbg!("OUT");
                dbg!("OUT");
                dbg!("OUT");
                // We do not create new row for this update
                // That means we can keep current hash and just update lock
                sqlx::query!(
                    "UPDATE script SET lock = $1 WHERE hash = $2 AND workspace_id = $3",
                    &content,
                    &current_hash.0,
                    w_id
                )
                .execute(db)
                .await?;

                // `lock` has been updated; invalidate the cache.
                // Since only worker that ran this Dependency Job has the cache
                // we do not need to think about invalidating cache for other workers.
                cache::script::invalidate(current_hash);

                if *WMDEBUG_NO_HASH_CHANGE_ON_DJ {
                    tracing::warn!("WMDEBUG_NO_HASH_CHANGE_ON_DJ usually should not be used. Behavior might be unstable. Please contact Windmill Team for support.")
                }

                current_hash
            };

            if let Err(e) = handle_deployment_metadata(
                &job.permissioned_as_email,
                &job.created_by,
                &db,
                &w_id,
                DeployedObject::Script {
                    hash: deployed_hash,
                    path: script_path.to_string(),
                    parent_path: parent_path.clone(),
                },
                deployment_message.clone(),
                false,
            )
            .await
            {
                tracing::error!(%e, "error handling deployment metadata");
            }

            process_relative_imports(
                db,
                Some(job.id),
                job.args.as_ref(),
                &job.workspace_id,
                script_path,
                parent_path,
                deployment_message,
                &script_data.code,
                &job.script_lang,
                &job.permissioned_as_email,
                &job.created_by,
                &job.permissioned_as,
                None,
            )
            .await?;

            Ok(to_raw_value_owned(
                json!({ "status": "Successful lock file generation", "lock": content }),
            ))
        }
        Err(error) => {
            let logs2 = sqlx::query_scalar!(
                "SELECT logs FROM job_logs WHERE job_id = $1 AND workspace_id = $2",
                &job.id,
                &job.workspace_id
            )
            .fetch_optional(db)
            .await?
            .flatten()
            .unwrap_or_else(|| "no logs".to_string());
            sqlx::query!(
                "UPDATE script SET lock_error_logs = $1 WHERE hash = $2 AND workspace_id = $3",
                &format!("{logs2}\n{error}"),
                &job.runnable_id.unwrap_or(ScriptHash(0)).0,
                &job.workspace_id
            )
            .execute(db)
            .await?;
            Err(Error::ExecutionErr(format!(
                "Error locking file: {error}\n\nlogs:\n{}",
                remove_ansi_codes(&logs2)
            )))?
        }
    }
}
fn remove_ansi_codes(s: &str) -> String {
    lazy_static::lazy_static! {
        static ref ANSI_REGEX: regex::Regex = regex::Regex::new(r"\x1b\[[0-9;]*[a-zA-Z]").unwrap();
    }
    ANSI_REGEX.replace_all(s, "").to_string()
}

pub async fn process_relative_imports(
    db: &sqlx::Pool<sqlx::Postgres>,
    job_id: Option<Uuid>,
    args: Option<&Json<HashMap<String, Box<RawValue>>>>,
    w_id: &str,
    script_path: &str,
    parent_path: Option<String>,
    deployment_message: Option<String>,
    code: &str,
    script_lang: &Option<ScriptLang>,
    permissioned_as_email: &str,
    created_by: &str,
    permissioned_as: &str,
    lock: Option<String>,
) -> error::Result<()> {
    let relative_imports = extract_relative_imports(&code, script_path, script_lang);
    if let Some(relative_imports) = relative_imports {
        if (script_lang.is_some_and(|v| v == ScriptLang::Bun)
            && lock
                .as_ref()
                .is_some_and(|v| v.contains("generatedFromPackageJson")))
            || (script_lang.is_some_and(|v| v == ScriptLang::Python3)
                && lock
                    .as_ref()
                    .is_some_and(|v| v.starts_with(LOCKFILE_GENERATED_FROM_REQUIREMENTS_TXT)))
        {
            // if the lock file is generated from a package.json/requirements.txt, we need to clear the dependency map
            // because we do not want to have dependencies be recomputed automatically. Empty relative imports passed
            // to update_script_dependency_map will clear the dependency map.
            update_script_dependency_map(
                &job_id.unwrap_or_else(|| Uuid::nil()),
                db,
                w_id,
                &parent_path,
                script_path,
                vec![],
            )
            .await?;
        } else {
            update_script_dependency_map(
                &job_id.unwrap_or_else(|| Uuid::nil()),
                db,
                w_id,
                &parent_path,
                script_path,
                relative_imports,
            )
            .await?;
        }
        let already_visited = args
            .map(|x| {
                x.get("already_visited")
                    .map(|v| serde_json::from_str::<Vec<String>>(v.get()).ok())
                    .flatten()
            })
            .flatten()
            .unwrap_or_default();
        if let Err(e) = trigger_dependents_to_recompute_dependencies(
            w_id,
            script_path,
            deployment_message,
            parent_path,
            permissioned_as_email,
            created_by,
            permissioned_as,
            db,
            already_visited,
        )
        .await
        {
            tracing::error!(%e, "error triggering dependents to recompute dependencies");
        }
    }
    Ok(())
}

pub async fn trigger_dependents_to_recompute_dependencies(
    w_id: &str,
    script_path: &str,
    deployment_message: Option<String>,
    parent_path: Option<String>,
    email: &str,
    created_by: &str,
    permissioned_as: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    mut already_visited: Vec<String>,
) -> error::Result<()> {
    let script_importers = sqlx::query!(
        "SELECT importer_path, importer_kind::text, array_agg(importer_node_id) as importer_node_ids FROM dependency_map
         WHERE imported_path = $1
         AND workspace_id = $2
         GROUP BY importer_path, importer_kind",
        script_path,
        w_id
    )
    .fetch_all(db)
    .await?;

    already_visited.push(script_path.to_string());
    for s in script_importers.iter() {
        if already_visited.contains(&s.importer_path) {
            continue;
        }

        let tx = PushIsolationLevel::IsolatedRoot(db.clone());
        let mut args: HashMap<String, Box<RawValue>> = HashMap::new();
        if let Some(ref dm) = deployment_message {
            args.insert("deployment_message".to_string(), to_raw_value(&dm));
        }
        if let Some(ref p_path) = parent_path {
            // NOTE:
            // it's not used but maybe one day it will be useful. allows more back-compatibility for the workers when we need it
            // also very useful for debugging/observability
            // it adds that information to the job args so you can see from the runs page
            args.insert("common_dependency_path".to_string(), to_raw_value(&p_path));
        }

        args.insert(
            "already_visited".to_string(),
            to_raw_value(&already_visited),
        );

        let kind = s.importer_kind.clone().unwrap_or_default();
        let job_payload = if kind == "script" {
            let r =
                // TODO: Not sure if this is safe:
                // might have race conditions in edge-cases
                get_latest_deployed_hash_for_path(None, db.clone(), w_id, s.importer_path.as_str())
                    .await;
            match r {
                // We will create Dependency job as is. But the Dep Job Handler will detect that the job originates
                // from [[trigger_dependents_to_recompute_dependencies]] and will create new script with new hash instead
                Ok(r) => JobPayload::Dependencies {
                    path: s.importer_path.clone(),
                    hash: ScriptHash(r.hash),
                    language: r.language,
                    dedicated_worker: r.dedicated_worker,
                },
                Err(err) => {
                    tracing::error!(
                        "error getting latest deployed hash for path {path}: {err}",
                        path = s.importer_path,
                        err = err
                    );
                    continue;
                }
            }
        } else if kind == "flow" {
            // Unlike 'script', 'flow' will not delegate redeployment of new flow to the Dep Job Handler.
            // We will create new flow in-place.
            // It would be harder to do otherwise.

            // Create transaction to make operation atomic.
            let mut flow_tx = db.begin().await?;
            args.insert(
                "nodes_to_relock".to_string(),
                to_raw_value(&s.importer_node_ids),
            );

            let r = sqlx::query_scalar!(
                "SELECT versions[array_upper(versions, 1)] FROM flow WHERE path = $1 AND workspace_id = $2",
                s.importer_path,
                w_id,
            ).fetch_one(&mut *flow_tx)
            .await
            .map_err(to_anyhow);

            match r {
                // TODO: Fallback - remove eventually.
                Ok(Some(version)) if *WMDEBUG_NO_NEW_FLOW_VERSION_ON_DJ => {
                    tracing::warn!("WMDEBUG_NO_NEW_FLOW_VERSION_ON_DJ usually should not be used. Behavior might be unstable. Please contact Windmill Team for support.");
                    JobPayload::FlowDependencies {
                        path: s.importer_path.clone(),
                        dedicated_worker: None,
                        version,
                    }
                }
                // Get current version of current flow.
                Ok(Some(cur_version)) => {
                    // NOTE: Temporary solution. See the usage for more details.
                    args.insert(
                        "triggered_by_relative_import".to_string(),
                        to_raw_value(&()),
                    );
                    // Find out what would be the next version.
                    // Also clone current flow_version to get new_version (which is usually c_v + 1).
                    // NOTE: It is fine if something goes wrong downstream and `flow` is not being appended with this new version.
                    // This version will just remain in db and cause no trouble.
                    let new_version = sqlx::query_scalar!(
                        "INSERT INTO flow_version
                (workspace_id, path, value, schema, created_by)

                SELECT workspace_id, path, value, schema, created_by
                FROM flow_version WHERE path = $1 AND workspace_id = $2 AND id = $3

                RETURNING id",
                        &s.importer_path,
                        w_id,
                        cur_version
                    )
                    .fetch_one(&mut *flow_tx)
                    .await
                    .map_err(|e| {
                        error::Error::internal_err(format!(
                            "Error updating flow due to flow history insert: {e:#}"
                        ))
                    })?;

                    // Commit the transaction.
                    // NOTE:
                    // We do not append flow.versions with new version.
                    // We will do this in the end of the dependency job handler.
                    // Otherwise it might become a source of race-conditions.
                    flow_tx.commit().await?;
                    JobPayload::FlowDependencies {
                        path: s.importer_path.clone(),
                        dedicated_worker: None,
                        // Point Dep Job to the new version.
                        // We do this since we want to assume old ones are immutable.
                        version: new_version,
                    }
                }
                Ok(None) => {
                    tracing::error!(
                        "no flow version found for path {path}",
                        path = s.importer_path
                    );
                    clear_dependency_map_for_item(
                        &s.importer_path,
                        w_id,
                        "flow",
                        flow_tx,
                        &None,
                        true,
                    )
                    .await?
                    .commit()
                    .await?;
                    continue;
                }
                Err(err) => {
                    tracing::error!(
                        "error getting latest deployed flow version for path {path}: {err}",
                        path = s.importer_path,
                    );
                    // Do not commit the transaction. It will be dropped and rollbacked
                    continue;
                }
            }
        } else if kind == "app" && !*WMDEBUG_NO_NEW_APP_VERSION_ON_DJ {
            // Create transaction to make operation atomic.
            let mut tx = db.begin().await?;

            args.insert(
                "components_to_relock".to_string(),
                to_raw_value(&s.importer_node_ids),
            );

            let r = sqlx::query_scalar!(
                "SELECT versions[array_upper(versions, 1)] FROM app WHERE path = $1 AND workspace_id = $2",
                s.importer_path,
                w_id,
            ).fetch_one(&mut *tx)
            .await
            .map_err(to_anyhow);

            match r {
                // Get current version of current flow.
                Ok(Some(cur_version)) => {
                    // NOTE: Temporary solution. See the usage for more details.
                    args.insert(
                        "triggered_by_relative_import".to_string(),
                        to_raw_value(&()),
                    );

                    let new_version = sqlx::query_scalar!(
                        "INSERT INTO app_version
                            (app_id, value, created_by, raw_app)
                        SELECT app_id, value, created_by, raw_app
                        FROM app_version WHERE id = $1
                        RETURNING id",
                        cur_version
                    )
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(|e| {
                        error::Error::internal_err(format!(
                            "Error updating App due to App history insert: {e:#}"
                        ))
                    })?;

                    // Commit the transaction.
                    // NOTE:
                    // We do not append app.versions with new version.
                    // We will do this in the end of the dependency job handler.
                    // Otherwise it might become a source of race-conditions.
                    tx.commit().await?;
                    JobPayload::AppDependencies {
                        path: s.importer_path.clone(),
                        // Point Dep Job to the new version.
                        // We do this since we want to assume old ones are immutable.
                        version: new_version,
                    }
                }
                Ok(None) => {
                    tracing::error!(
                        "no app version found for path {path}",
                        path = s.importer_path
                    );
                    clear_dependency_map_for_item(&s.importer_path, w_id, "app", tx, &None, true)
                        .await?
                        .commit()
                        .await?;
                    continue;
                }
                Err(err) => {
                    // TODO: Clean malformed dependency_map row
                    // TODO: If flow or app also take into account nodes.
                    tracing::error!(
                        "error getting latest deployed app version for path {path}: {err}",
                        path = s.importer_path,
                    );
                    // Do not commit the transaction. It will be dropped and rollbacked
                    continue;
                }
            }
        } else {
            tracing::error!(
                "unexpected importer kind: {kind} for path {path}",
                kind = kind,
                path = s.importer_path
            );
            continue;
        };

        let (job_uuid, new_tx) = windmill_queue::push(
            db,
            tx,
            &w_id,
            job_payload,
            windmill_queue::PushArgs { args: &args, extra: None },
            &created_by,
            email,
            permissioned_as.to_string(),
            Some("trigger.dependents.to.recompute.dependencies"),
            None,
            None,
            None,
            None,
            None,
            None,
            false,
            false,
            None,
            true,
            None,
            None,
            None,
            None,
            None,
            false,
        )
        .await?;
        tracing::info!(
            "pushed dependency job due to common python path: {job_uuid} for path {path}",
            path = s.importer_path,
        );
        new_tx.commit().await?;
    }
    Ok(())
}

pub async fn handle_flow_dependency_job(
    job: &MiniPulledJob,
    preview_data: Option<&RawData>,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job_dir: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    worker_name: &str,
    worker_dir: &str,
    base_internal_url: &str,
    token: &str,
    occupancy_metrics: &mut OccupancyMetrics,
) -> error::Result<Box<serde_json::value::RawValue>> {
    dbg!("HANDLE_FLOW_DEPENDENCY_JOB");
    let job_path = job.runnable_path.clone().ok_or_else(|| {
        error::Error::internal_err(
            "Cannot resolve flow dependencies for flow without path".to_string(),
        )
    })?;

    let skip_flow_update = job
        .args
        .as_ref()
        .map(|x| {
            x.get("skip_flow_update")
                .map(|v| serde_json::from_str::<bool>(v.get()).ok())
                .flatten()
        })
        .flatten()
        .unwrap_or(false);

    let version = if skip_flow_update {
        None
    } else {
        Some(
            job.runnable_id
                .clone()
                .ok_or_else(|| {
                    Error::internal_err(
                        "Flow Dependency requires script hash (flow version)".to_owned(),
                    )
                })?
                .0,
        )
    };

    let (deployment_message, parent_path) =
        get_deployment_msg_and_parent_path_from_args(job.args.clone());

    let nodes_to_relock = job
        .args
        .as_ref()
        .map(|x| {
            x.get("nodes_to_relock")
                .map(|v| serde_json::from_str::<Vec<String>>(v.get()).ok())
                .flatten()
        })
        .flatten();

    let raw_deps = job
        .args
        .as_ref()
        .map(|x| {
            x.get("raw_deps")
                .map(|v| serde_json::from_str::<HashMap<String, String>>(v.get()).ok())
                .flatten()
        })
        .flatten();

    let triggered_by_relative_import = job
        .args
        .as_ref()
        .map(|x| x.get("triggered_by_relative_import").is_some())
        .unwrap_or_default();

    // `JobKind::FlowDependencies` job store either:
    // - A saved flow version `id` in the `script_hash` column.
    // - Preview raw flow in the `queue` or `job` table.
    let mut flow = match job.runnable_id {
        Some(ScriptHash(id)) => cache::flow::fetch_version(db, id).await?,
        _ => match preview_data {
            Some(RawData::Flow(data)) => data.clone(),
            _ => return Err(Error::internal_err("expected script hash")),
        },
    }
    .value()
    .clone();

    let mut tx = db.begin().await?;

    let mut dependency_map =
        ScopedDependencyMap::new(&job.workspace_id, &job_path, "flow", &mut *tx).await?;

    // TODO: is parent_path only used for this edge-case?
    dependency_map
        .rearrange_top_level(&parent_path, &mut *tx)
        .await?;

    if !skip_flow_update {
        sqlx::query!(
        "DELETE FROM workspace_runnable_dependencies WHERE flow_path = $1 AND workspace_id = $2",
        job_path,
        job.workspace_id
    )
        .execute(&mut *tx)
        .await?;
    }

    clear_asset_usage(&mut *tx, &job.workspace_id, &job_path, AssetUsageKind::Flow).await?;

    let modified_ids;
    let errors;
    (flow.modules, tx, modified_ids, errors) = lock_modules(
        flow.modules,
        job,
        mem_peak,
        canceled_by,
        job_dir,
        db,
        tx,
        worker_name,
        worker_dir,
        &job_path,
        base_internal_url,
        token,
        &nodes_to_relock,
        occupancy_metrics,
        skip_flow_update,
        raw_deps,
        &mut dependency_map,
    )
    .await?;

    tx = dependency_map.dissolve(tx).await;

    if !errors.is_empty() {
        let error_message = errors
            .iter()
            .map(|e| format!("{}: {}", e.id, e.error))
            .collect::<Vec<String>>()
            .join("\n");
        let logs2 = sqlx::query_scalar!(
            "SELECT logs FROM job_logs WHERE job_id = $1 AND workspace_id = $2",
            &job.id,
            &job.workspace_id
        )
        .fetch_optional(db)
        .await?
        .flatten()
        .unwrap_or_else(|| "no logs".to_string());

        sqlx::query!(
            "UPDATE flow SET lock_error_logs = $1 WHERE path = $2 AND workspace_id = $3",
            &format!("{logs2}\n{error_message}"),
            &job.runnable_path(),
            &job.workspace_id
        )
        .execute(db)
        .await?;

        return Err(Error::ExecutionErr(format!(
            "Error locking flow modules:\n{}\n\nlogs:\n{}",
            error_message,
            remove_ansi_codes(&logs2)
        )));
    } else {
        sqlx::query!(
            "UPDATE flow SET lock_error_logs = NULL WHERE path = $1 AND workspace_id = $2",
            &job.runnable_path(),
            &job.workspace_id
        )
        .execute(db)
        .await?;
    }

    let new_flow_value = Json(serde_json::value::to_raw_value(&flow).map_err(to_anyhow)?);

    // Re-check cancellation to ensure we don't accidentally override a flow.
    if sqlx::query_scalar!(
        "SELECT canceled_by IS NOT NULL AS \"canceled!\" FROM v2_job_queue WHERE id = $1",
        job.id
    )
    .fetch_optional(db)
    .await
    .map(|v| Some(true) == v)
    .unwrap_or_else(|err| {
        tracing::error!(%job.id, %err, "error checking cancellation for job {0}: {err}", job.id);
        false
    }) {
        return Ok(to_raw_value_owned(json!({
            "status": "Flow lock generation was canceled",
        })));
    }

    if !skip_flow_update {
        let version = version.ok_or_else(|| {
            Error::internal_err("Flow Dependency requires script hash (flow version)".to_owned())
        })?;

        sqlx::query!(
            "UPDATE flow SET value = $1 WHERE path = $2 AND workspace_id = $3",
            &new_flow_value as &Json<Box<RawValue>>,
            job_path,
            job.workspace_id
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query!(
            "UPDATE flow_version SET value = $1 WHERE id = $2",
            &new_flow_value as &Json<Box<RawValue>>,
            version
        )
        .execute(&mut *tx)
        .await?;

        // Compute a lite version of the flow value (`RawScript` => `FlowScript`).
        let (mut value_lite, mut step_to_id_hm) = (flow.clone(), HashMap::new());

        tx = reduce_flow(
            tx,
            &mut value_lite.modules,
            &mut step_to_id_hm,
            &job_path,
            &job.workspace_id,
            flow.failure_module.as_ref(),
            flow.same_worker,
        )
        .await?;

        sqlx::query!(
            "INSERT INTO flow_version_lite (id, value) VALUES ($1, $2)
             ON CONFLICT (id) DO UPDATE SET value = EXCLUDED.value",
            version,
            Json(value_lite) as Json<FlowValue>,
        )
        .execute(&mut *tx)
        .await?;

        // NOTE: Temporary solution.
        // Ideally we do this for every job regardless whether it was triggered by relative import or by creation/update of the flow.
        if triggered_by_relative_import {
            // Making new version viewable as the current one.
            // This will also trigger `flow_versions_append_trigger` (check _flow_versions_update_notify.up.sql)
            // which will invalidate cache for the latest flow versions for all workers.
            sqlx::query!("UPDATE flow SET versions = array_append(versions, $1) WHERE path = $2 AND workspace_id = $3",
                version,
                &job_path,
                &job.workspace_id,
            ).execute(&mut *tx).await?;
        }

        tx.commit().await?;

        if let Err(e) = handle_deployment_metadata(
            &job.permissioned_as_email,
            &job.created_by,
            &db,
            &job.workspace_id,
            DeployedObject::Flow { path: job_path, parent_path, version },
            deployment_message,
            false,
        )
        .await
        {
            tracing::error!(%e, "error handling deployment metadata");
        }
    }

    Ok(to_raw_value_owned(json!({
        "status": "Successful lock file generation",
        "modified_ids": modified_ids,
        "updated_flow_value": new_flow_value,
    })))
}

// TODO: Clean dependency_map table
// TODO: Completely messup the thing
// - Change referenced node.
/// self-heals even if dependency_map is deleted
struct ScopedDependencyMap {
    dm: HashSet<(String, String)>,
    w_id: String,
    importer_path: String,
    importer_kind: String,
}

impl ScopedDependencyMap {
    async fn new<'a>(
        w_id: &str,
        importer_path: &str,
        importer_kind: &str,
        executor: impl sqlx::Executor<'a, Database = sqlx::Postgres>,
    ) -> Result<Self> {
        let dm = sqlx::query_as::<_, (String, String)>(
"SELECT importer_node_id, imported_path FROM dependency_map WHERE workspace_id = $1 AND importer_path = $2 AND importer_kind = $3::text::IMPORTER_KIND",
        )
        .bind(w_id)
        .bind(importer_path)
        .bind(importer_kind)
        .fetch_all(executor)
        .await?;

        Ok(Self {
            dm: HashSet::from_iter(dm.into_iter()),
            w_id: w_id.to_owned(),
            importer_path: importer_path.to_owned(),
            importer_kind: importer_kind.to_owned(),
        })
    }

    /// Add missing entries to `dependency_map`
    /// Remove matching entries
    async fn reduce<'c>(
        &mut self,
        relative_imports: Option<Vec<String>>,
        node_id: &str, // Flow Step/Node ID
        mut tx: sqlx::Transaction<'c, sqlx::Postgres>,
    ) -> error::Result<sqlx::Transaction<'c, sqlx::Postgres>> {
        let Some(mut relative_imports) = relative_imports else {
            return Ok(tx);
        };

        if node_id.trim().is_empty() {
            tracing::error!("internal: node_id passed to ScopedDependencyMap::reduce is not supposed to be empty");
            return Ok(tx);
        }

        dbg!(&relative_imports);
        dbg!(node_id);
        dbg!(&self.importer_path);

        // This does:
        // 1. remove all relative imports from relative_imports that ARE tracked in dependency_map
        // 2. remove corresponding trackers from dependency_map
        //
        // After this operation `relative_imports` variable has only untracked imports.
        // We will handle those in the next expression.
        //
        // After all `reduce`'s called ScopedDependencyMap has only extra/orphan imports
        // these are going to be clean up by calling [dissolve]
        relative_imports.retain(|imported_path| {
            !self
                .dm
                // As dm is HashSet, removing is O(1) operation
                // thus making entire process very efficient
                .remove(&(node_id.to_owned(), imported_path.to_owned()))
        });

        // As mentioned above, usually this will always be empty.
        if !relative_imports.is_empty() {
            let mut logs = "".to_string();
            logs.push_str(format!("\n\n--- RELATIVE IMPORTS of {} ---\n\n", node_id).as_str());

            tracing::info!("adding missing entries to dependency_map: importer_node_id - {}, importer_kind - {}, new_imported_paths - {:?}",
                &node_id,
                &self.importer_kind,
                &relative_imports,
            );

            tx = add_relative_imports_to_dependency_map(
                &self.importer_path,
                &self.w_id,
                relative_imports,
                &self.importer_kind,
                tx,
                &mut logs,
                Some(node_id.to_owned()),
            )
            .await?;
            // TODO:
            // append_logs(&job.id, &job.workspace_id, logs, &db.into()).await;
        }
        Ok(tx)
    }

    /// clean orphan entries from `dependency_map`
    async fn dissolve<'a>(
        self,
        mut tx: sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> sqlx::Transaction<'a, sqlx::Postgres> {
        if *WMDEBUG_NO_DMAP_DISSOLVE {
            tracing::warn!(
                "WMDEBUG_NO_DMAP_DISSOLVE usually should not be used. Behavior might be unstable."
            );
            return tx;
        }

        // We _could_ shove it into single query, but this query is rarely called AND let's keep it simple for redability.
        for (importer_node_id, imported_path) in self.dm.into_iter() {
            tracing::info!("cleaning orphan entry from dependency_map: importer_kind - {}, imported_path - {}, importer_node_id - {}",
                "flow",
                &imported_path,
                &importer_node_id,
            );

            // Dissolve MUST succeed. Error in dissolve MUST not block the execution.
            if let Err(err) = sqlx::query!(
                "
        DELETE FROM dependency_map
        WHERE workspace_id = $1
            AND importer_path = $2
            AND importer_kind = $3::text::IMPORTER_KIND
            AND importer_node_id = $4
            AND imported_path = $5
            ",
                &self.w_id,
                &self.importer_path,
                &self.importer_kind,
                &importer_node_id,
                &imported_path,
            )
            .execute(&mut *tx)
            .await
            {
                tracing::error!(
                    "error while cleaning dependency_map for: importer_node_id - {}, imported_path - {}, importer_path - {}: {err}",
                    importer_node_id,
                    imported_path,
                    self.importer_path,
                );
            }
        }
        tx
    }

    /// Used to move top level apps/flows if they were renamed/moved
    async fn rearrange_top_level<'a>(
        &mut self,
        parent_path: &Option<String>,
        executor: impl sqlx::Executor<'a, Database = sqlx::Postgres>,
    ) -> error::Result<()> {
        if parent_path
            .as_ref()
            .is_some_and(|x| !x.is_empty() && x != &self.importer_path)
        {
            sqlx::query(
                "
UPDATE dependency_map
    SET importer_path = $1
    WHERE importer_path = $2
        AND importer_kind = $3::text::IMPORTER_KIND
        AND workspace_id = $4
        ",
            )
            .bind(&self.importer_path)
            .bind(parent_path.clone().unwrap())
            .bind(&self.importer_kind)
            .bind(&self.w_id)
            .execute(executor)
            .await?;
        }
        Ok(())
    }

    /// Remove invalid entry
    fn remove_by_importer_path(importer_path: &str, importer_kind: &str, w_id: &str) {
        tracing::warn!("self-healed, but is the bug");
    }
}

fn get_deployment_msg_and_parent_path_from_args(
    args: Option<Json<HashMap<String, Box<RawValue>>>>,
) -> (Option<String>, Option<String>) {
    let args_map = args.map(|json_hashmap| json_hashmap.0);
    let deployment_message = args_map
        .clone()
        .map(|hashmap| {
            hashmap
                .get("deployment_message")
                .map(|map_value| serde_json::from_str::<String>(map_value.get()).ok())
                .flatten()
        })
        .flatten();
    let parent_path = args_map
        .clone()
        .map(|hashmap| {
            hashmap
                .get("parent_path")
                .map(|map_value| serde_json::from_str::<String>(map_value.get()).ok())
                .flatten()
        })
        .flatten();
    (deployment_message, parent_path)
}

struct LockModuleError {
    id: String,
    error: Error,
}

async fn lock_modules<'c>(
    modules: Vec<FlowModule>,
    job: &MiniPulledJob,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job_dir: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    mut tx: sqlx::Transaction<'c, sqlx::Postgres>,
    worker_name: &str,
    worker_dir: &str,
    job_path: &str,
    base_internal_url: &str,
    token: &str,
    locks_to_reload: &Option<Vec<String>>,
    occupancy_metrics: &mut OccupancyMetrics,
    skip_flow_update: bool,
    raw_deps: Option<HashMap<String, String>>,
    dependency_map: &mut ScopedDependencyMap, // (modules to replace old seq (even unmmodified ones), new transaction, modified ids) )
) -> Result<(
    Vec<FlowModule>,
    sqlx::Transaction<'c, sqlx::Postgres>,
    Vec<String>,
    Vec<LockModuleError>,
)> {
    let mut new_flow_modules = Vec::new();
    let mut modified_ids = Vec::new();
    let mut errors = Vec::new();
    for mut e in modules.into_iter() {
        let id = e.id.clone();
        dbg!(&id);
        let mut nmodified_ids = Vec::new();
        let FlowModuleValue::RawScript {
            lock,
            path,
            content,
            mut language,
            input_transforms,
            tag,
            custom_concurrency_key,
            concurrent_limit,
            concurrency_time_window_s,
            is_trigger,
            assets,
        } = e.get_value()?
        else {
            match e.get_value()? {
                FlowModuleValue::ForloopFlow {
                    iterator,
                    modules,
                    modules_node,
                    skip_failures,
                    parallel,
                    parallelism,
                } => {
                    let nmodules;
                    (nmodules, tx, modified_ids, errors) = Box::pin(lock_modules(
                        modules,
                        job,
                        mem_peak,
                        canceled_by,
                        job_dir,
                        db,
                        tx,
                        worker_name,
                        worker_dir,
                        job_path,
                        base_internal_url,
                        token,
                        locks_to_reload,
                        occupancy_metrics,
                        skip_flow_update,
                        raw_deps.clone(),
                        dependency_map,
                    ))
                    .await?;
                    e.value = FlowModuleValue::ForloopFlow {
                        iterator,
                        modules: nmodules,
                        modules_node,
                        skip_failures,
                        parallel,
                        parallelism,
                    }
                    .into()
                }
                FlowModuleValue::BranchAll { branches, parallel } => {
                    let mut nbranches = vec![];
                    nmodified_ids = vec![];
                    for mut b in branches {
                        let nmodules;
                        let inner_modified_ids;
                        let inner_errors;
                        (nmodules, tx, inner_modified_ids, inner_errors) = Box::pin(lock_modules(
                            b.modules,
                            job,
                            mem_peak,
                            canceled_by,
                            job_dir,
                            db,
                            tx,
                            worker_name,
                            worker_dir,
                            job_path,
                            base_internal_url,
                            token,
                            locks_to_reload,
                            occupancy_metrics,
                            skip_flow_update,
                            raw_deps.clone(),
                            dependency_map,
                        ))
                        .await?;
                        nmodified_ids.extend(inner_modified_ids);
                        errors.extend(inner_errors);
                        b.modules = nmodules;
                        nbranches.push(b)
                    }
                    e.value = FlowModuleValue::BranchAll { branches: nbranches, parallel }.into()
                }
                FlowModuleValue::WhileloopFlow { modules, modules_node, skip_failures } => {
                    let nmodules;
                    (nmodules, tx, nmodified_ids, errors) = Box::pin(lock_modules(
                        modules,
                        job,
                        mem_peak,
                        canceled_by,
                        job_dir,
                        db,
                        tx,
                        worker_name,
                        worker_dir,
                        job_path,
                        base_internal_url,
                        token,
                        locks_to_reload,
                        occupancy_metrics,
                        skip_flow_update,
                        raw_deps.clone(),
                        dependency_map,
                    ))
                    .await?;
                    e.value = FlowModuleValue::WhileloopFlow {
                        modules: nmodules,
                        modules_node,
                        skip_failures,
                    }
                    .into()
                }
                FlowModuleValue::BranchOne { branches, default, default_node } => {
                    let mut nbranches = vec![];
                    nmodified_ids = vec![];
                    for mut b in branches {
                        let nmodules;
                        let inner_modified_ids;
                        let inner_errors;
                        (nmodules, tx, inner_modified_ids, inner_errors) = Box::pin(lock_modules(
                            b.modules,
                            job,
                            mem_peak,
                            canceled_by,
                            job_dir,
                            db,
                            tx,
                            worker_name,
                            worker_dir,
                            job_path,
                            base_internal_url,
                            token,
                            locks_to_reload,
                            occupancy_metrics,
                            skip_flow_update,
                            raw_deps.clone(),
                            dependency_map,
                        ))
                        .await?;
                        nmodified_ids.extend(inner_modified_ids);
                        errors.extend(inner_errors);
                        b.modules = nmodules;
                        nbranches.push(b)
                    }
                    let ndefault;
                    let ninner_errors;
                    (ndefault, tx, nmodified_ids, ninner_errors) = Box::pin(lock_modules(
                        default,
                        job,
                        mem_peak,
                        canceled_by,
                        job_dir,
                        db,
                        tx,
                        worker_name,
                        worker_dir,
                        job_path,
                        base_internal_url,
                        token,
                        locks_to_reload,
                        occupancy_metrics,
                        skip_flow_update,
                        raw_deps.clone(),
                        dependency_map,
                    ))
                    .await?;
                    errors.extend(ninner_errors);
                    e.value = FlowModuleValue::BranchOne {
                        branches: nbranches,
                        default: ndefault,
                        default_node,
                    }
                    .into();
                }
                FlowModuleValue::Script { path, hash, .. }
                    if !path.starts_with("hub/") && !skip_flow_update =>
                {
                    sqlx::query!(
                        "INSERT INTO workspace_runnable_dependencies (flow_path, runnable_path, script_hash, runnable_is_flow, workspace_id) VALUES ($1, $2, $3, FALSE, $4) ON CONFLICT DO NOTHING",
                        job_path,
                        path,
                        hash.map(|h| h.0),
                        job.workspace_id
                    )
                    .execute(&mut *tx)
                    .await?;
                }
                FlowModuleValue::Flow { path, .. } if !skip_flow_update => {
                    sqlx::query!(
                        "INSERT INTO workspace_runnable_dependencies (flow_path, runnable_path, runnable_is_flow, workspace_id) VALUES ($1, $2, TRUE, $3) ON CONFLICT DO NOTHING",
                        job_path,
                        path,
                        job.workspace_id,
                    )
                    .execute(&mut *tx)
                    .await?;
                }
                _ => (),
            };
            modified_ids.extend(nmodified_ids);
            new_flow_modules.push(e);
            continue;
        };

        for asset in assets.iter().flatten() {
            insert_asset_usage(
                &mut *tx,
                &job.workspace_id,
                asset,
                job_path,
                AssetUsageKind::Flow,
            )
            .await?;
        }

        let dep_path = path.clone().unwrap_or_else(|| job_path.to_string());
        let relative_imports = extract_relative_imports(
            &content,
            &format!("{dep_path}/flow"),
            &Some(language.clone()),
        );

        if let Some(locks_to_reload) = locks_to_reload {
            if !locks_to_reload.contains(&e.id) {
                // TODO: Is it safe to override semi-global tx here?
                tx = dependency_map
                    .reduce(relative_imports.clone(), &e.id, tx)
                    .await?;

                new_flow_modules.push(e);
                continue;
            }
        } else {
            if lock.as_ref().is_some_and(|x| !x.trim().is_empty()) {
                let skip_creating_new_lock = skip_creating_new_lock(&language, &content);
                if skip_creating_new_lock {
                    // TODO: Is it safe to override semi-global tx here?
                    tx = dependency_map
                        .reduce(relative_imports.clone(), &e.id, tx)
                        .await?;

                    new_flow_modules.push(e);
                    continue;
                }
            }
        }

        modified_ids.push(e.id.clone());

        remove_dir_all(job_dir).map_err(|e| {
            Error::ExecutionErr(format!("Error removing job dir for flow step lock: {e}"))
        })?;
        create_dir_all(job_dir).map_err(|e| {
            Error::ExecutionErr(format!("Error creating job dir for flow step lock: {e}"))
        })?;

        // If we have local lockfiles (and they are enabled) we will replace script content with lockfile and tell hander that it is raw_deps job
        let (content, raw_deps) = raw_deps
            .as_ref()
            .and_then(|llfs| llfs.get(language.as_str()))
            .map(|lock| (lock.to_owned(), true))
            .unwrap_or((content, false));

        let new_lock = capture_dependency_job(
            &job.id,
            &language,
            &content,
            mem_peak,
            canceled_by,
            job_dir,
            db,
            worker_name,
            &job.workspace_id,
            worker_dir,
            base_internal_url,
            token,
            &format!(
                "{}/flow",
                &path.clone().unwrap_or_else(|| job_path.to_string())
            ),
            raw_deps,
            None,
            occupancy_metrics,
        )
        .await;
        //
        let lock = match new_lock {
            Ok(new_lock) => {
                tx = dependency_map
                    .reduce(relative_imports.clone(), &e.id, tx)
                    .await?;

                if language == ScriptLang::Bun || language == ScriptLang::Bunnative {
                    let anns = windmill_common::worker::TypeScriptAnnotations::parse(&content);
                    if anns.native && language == ScriptLang::Bun {
                        language = ScriptLang::Bunnative;
                    } else if !anns.native && language == ScriptLang::Bunnative {
                        language = ScriptLang::Bun;
                    };
                }
                Some(new_lock)
            }
            Err(error) => {
                // TODO: Record flow raw script error lock logs
                errors.push(LockModuleError { id, error });
                None
            }
        };

        e.value = windmill_common::worker::to_raw_value(&FlowModuleValue::RawScript {
            lock,
            path,
            input_transforms,
            content,
            language,
            tag,
            custom_concurrency_key,
            concurrent_limit,
            concurrency_time_window_s,
            is_trigger,
            assets,
        });
        new_flow_modules.push(e);

        continue;
    }

    Ok((new_flow_modules, tx, modified_ids, errors))
}

/// Parse relative imports in script and call db to get each scripts' hash.
async fn relative_imports_bytes<'a>(
    e: impl sqlx::Executor<'a, Database = sqlx::Postgres>,
    code: Option<&String>,
    path: &str,
    language: Option<ScriptLang>,
) -> Result<Vec<u8>> {
    Ok(
        if let Some(imports) = extract_relative_imports(
            code.map(|s| s.as_str()).unwrap_or_default(),
            path,
            &language,
        ) {
            sqlx::query_scalar::<_, i64>(
                "SELECT hash FROM script WHERE path = ANY($1) AND archived = false",
            )
            .bind(imports)
            .fetch_all(e)
            .await?
            .iter()
            .flat_map(|h| h.to_le_bytes())
            .collect_vec()
        } else {
            vec![]
        },
    )
}

// TODO: Clean up dependency map when moved/renamed?
async fn insert_flow_node<'c>(
    mut tx: sqlx::Transaction<'c, sqlx::Postgres>,
    path: &str,
    workspace_id: &str,
    code: Option<&String>,
    lock: Option<&String>,
    flow: Option<&Json<Box<RawValue>>>,
    language: Option<ScriptLang>,
) -> Result<(sqlx::Transaction<'c, sqlx::Postgres>, FlowNodeId)> {
    let hash = {
        let mut hasher = sha2::Sha256::new();
        hasher.update(code.unwrap_or(&Default::default()));
        hasher.update(lock.unwrap_or(&Default::default()));
        hasher.update(flow.unwrap_or(&Default::default()).get());
        if !*WMDEBUG_NO_NEW_FLOW_VERSION_ON_DJ {
            // We also want to take into account hashes of relative imports.
            hasher.update(
                relative_imports_bytes(&mut *tx, code, &format!("{path}/flow"), language).await?,
            );
        }
        format!("{:x}", hasher.finalize())
    };

    // Insert the flow node if it doesn't exist.
    let id = sqlx::query_scalar!(
        r#"
        INSERT INTO flow_node (path, workspace_id, hash_v2, lock, code, flow)
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (path, workspace_id, hash_v2) DO UPDATE SET path = EXCLUDED.path -- trivial update to return the id
        RETURNING id
        "#,
        path,
        workspace_id,
        hash,
        lock,
        code,
        flow as Option<&Json<Box<RawValue>>>
    )
    .fetch_one(&mut *tx)
    .await?;
    Ok((tx, FlowNodeId(id)))
}

// TODO: Clean up dependency map when moved/renamed?
async fn insert_app_script(
    db: &sqlx::Pool<sqlx::Postgres>,
    path: &str,
    app: i64,
    code: String,
    lock: Option<String>,
    language: Option<ScriptLang>,
) -> Result<AppScriptId> {
    let code_sha256 = format!("{:x}", sha2::Sha256::digest(&code));
    let hash = {
        let mut hasher = sha2::Sha256::new();
        hasher.update(app.to_le_bytes());
        hasher.update(&code_sha256);
        hasher.update(lock.as_ref().unwrap_or(&Default::default()));
        // We also want to take into account hashes of relative imports.
        if !*WMDEBUG_NO_NEW_APP_VERSION_ON_DJ {
            hasher.update(
                relative_imports_bytes(db, Some(&code), &format!("{path}/app"), language).await?,
            );
        }
        format!("{:x}", hasher.finalize())
    };

    // Insert the app script if it doesn't exist.
    sqlx::query_scalar!(
        r#"
        INSERT INTO app_script (app, hash, lock, code, code_sha256)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (hash) DO UPDATE SET app = EXCLUDED.app -- trivial update to return the id
        RETURNING id
        "#,
        app,
        hash,
        lock,
        code,
        code_sha256
    )
    .fetch_one(db)
    .await
    .map(AppScriptId)
    .map_err(Into::into)
}

async fn insert_flow_modules<'c>(
    mut tx: sqlx::Transaction<'c, sqlx::Postgres>,
    path: &str,
    workspace_id: &str,
    failure_module: Option<&Box<FlowModule>>,
    same_worker: bool,
    modules: &mut Vec<FlowModule>,
    modules_node: &mut Option<FlowNodeId>,
    step_to_id_hm: &mut HashMap<String, FlowNodeId>,
) -> Result<sqlx::Transaction<'c, sqlx::Postgres>> {
    tx = Box::pin(reduce_flow(
        tx,
        modules,
        step_to_id_hm,
        path,
        workspace_id,
        failure_module,
        same_worker,
    ))
    .await?;
    if modules.is_empty() || crate::worker_flow::is_simple_modules(modules, failure_module) {
        return Ok(tx);
    }
    let id;
    (tx, id) = insert_flow_node(
        tx,
        path,
        workspace_id,
        None,
        None,
        Some(&Json(to_raw_value(&FlowValue {
            modules: std::mem::take(modules),
            failure_module: failure_module.cloned(),
            same_worker,
            ..Default::default()
        }))),
        None,
    )
    .await?;
    *modules_node = Some(id);
    Ok(tx)
}

async fn reduce_flow<'c>(
    mut tx: sqlx::Transaction<'c, sqlx::Postgres>,
    modules: &mut Vec<FlowModule>,
    step_to_id_hm: &mut HashMap<String, FlowNodeId>,
    path: &str,
    workspace_id: &str,
    failure_module: Option<&Box<FlowModule>>,
    same_worker: bool,
) -> Result<sqlx::Transaction<'c, sqlx::Postgres>> {
    use FlowModuleValue::*;
    for module in &mut *modules {
        let step = module.id.clone();
        let mut val =
            serde_json::from_str::<FlowModuleValue>(module.value.get()).map_err(|err| {
                Error::internal_err(format!(
                    "reduce_flow: Failed to parse flow module value: {}",
                    err
                ))
            })?;
        match &mut val {
            RawScript { .. } => {
                // In order to avoid an unnecessary `.clone()` of `val`, take ownership of it's content
                // using `std::mem::replace`.
                let RawScript {
                    lock,
                    content,
                    language,
                    input_transforms,
                    tag,
                    custom_concurrency_key,
                    concurrent_limit,
                    concurrency_time_window_s,
                    is_trigger,
                    assets,
                    ..
                } = std::mem::replace(&mut val, Identity)
                else {
                    unreachable!()
                };
                let id;
                (tx, id) = insert_flow_node(
                    tx,
                    path,
                    workspace_id,
                    Some(&content),
                    lock.as_ref(),
                    None,
                    Some(language),
                )
                .await?;

                step_to_id_hm.insert(step, id);

                val = FlowScript {
                    input_transforms,
                    id,
                    tag,
                    language,
                    custom_concurrency_key,
                    concurrent_limit,
                    concurrency_time_window_s,
                    is_trigger,
                    assets,
                };
            }
            ForloopFlow { modules, modules_node, .. }
            | WhileloopFlow { modules, modules_node, .. } => {
                tx = insert_flow_modules(
                    tx,
                    path,
                    workspace_id,
                    failure_module,
                    same_worker,
                    modules,
                    modules_node,
                    step_to_id_hm,
                )
                .await?;
            }
            BranchOne { branches, default, default_node, .. } => {
                for branch in branches.iter_mut() {
                    tx = insert_flow_modules(
                        tx,
                        path,
                        workspace_id,
                        failure_module,
                        same_worker,
                        &mut branch.modules,
                        &mut branch.modules_node,
                        step_to_id_hm,
                    )
                    .await?;
                }
                tx = insert_flow_modules(
                    tx,
                    path,
                    workspace_id,
                    failure_module,
                    same_worker,
                    default,
                    default_node,
                    step_to_id_hm,
                )
                .await?;
            }
            BranchAll { branches, .. } => {
                for branch in branches.iter_mut() {
                    tx = insert_flow_modules(
                        tx,
                        path,
                        workspace_id,
                        failure_module,
                        same_worker,
                        &mut branch.modules,
                        &mut branch.modules_node,
                        step_to_id_hm,
                    )
                    .await?;
                }
            }
            _ => {}
        }
        module.value = to_raw_value(&val);
    }
    add_virtual_items_if_necessary(&mut *modules);
    Ok(tx)
}

async fn reduce_app(
    db: &sqlx::Pool<sqlx::Postgres>,
    path: &str,
    value: &mut Value,
    app: i64,
) -> Result<()> {
    match value {
        Value::Object(object) => {
            if let Some(Value::Object(script)) = object.get_mut("inlineScript") {
                let language = script.get("language").cloned();
                if language == Some(Value::String("frontend".to_owned())) {
                    return Ok(());
                }
                // replace `content` with an empty string:
                let Some(Value::String(code)) = script.get_mut("content").map(std::mem::take)
                else {
                    return Err(error::Error::internal_err(
                        "Missing `content` in inlineScript".to_string(),
                    ));
                };
                // remove `lock`:
                let lock = script.remove("lock").and_then(|x| match x {
                    Value::String(s) => Some(s),
                    _ => None,
                });
                let id = insert_app_script(
                    db,
                    path,
                    app,
                    code,
                    lock,
                    language.map(|v| from_value(v).ok()).flatten(),
                )
                .await?;
                // insert the `id` into the `script` object:
                script.insert("id".to_string(), json!(id.0));
            } else {
                for (_, value) in object {
                    Box::pin(reduce_app(db, path, value, app)).await?;
                }
            }
        }
        Value::Array(array) => {
            for value in array {
                Box::pin(reduce_app(db, path, value, app)).await?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn skip_creating_new_lock(language: &ScriptLang, content: &str) -> bool {
    if language == &ScriptLang::Bun || language == &ScriptLang::Bunnative {
        let anns = windmill_common::worker::TypeScriptAnnotations::parse(&content);
        if anns.native && language == &ScriptLang::Bun {
            return false;
        } else if !anns.native && language == &ScriptLang::Bunnative {
            return false;
        };
    }
    true
}

// TODO: Use transaction?
#[async_recursion]
async fn lock_modules_app(
    value: Value,
    job: &MiniPulledJob,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job_dir: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    worker_name: &str,
    worker_dir: &str,
    job_path: &str,
    base_internal_url: &str,
    token: &str,
    occupancy_metrics: &mut OccupancyMetrics,
    locks_to_reload: &Option<Vec<String>>,
    // Represents the closest container id
    container_id: Option<String>,
    dependency_map: &mut ScopedDependencyMap,
) -> Result<Value> {
    match value {
        Value::Object(mut m) => {
            if let (Some(Value::String(ref run_type)), Some(path), Some("runnableByPath")) = (
                m.get("runType"),
                m.get("path").and_then(|s| s.as_str()),
                m.get("type").and_then(|s| s.as_str()),
            ) {
                // No script_hash because apps don't supports script version locks yet
                sqlx::query!(
                    "INSERT INTO workspace_runnable_dependencies (app_path, runnable_path, runnable_is_flow, workspace_id) VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING",
                    job_path,
                    path,
                    run_type == "flow",
                    job.workspace_id
                )
                .execute(db)
                .await?;
            }
            if m.contains_key("inlineScript") {
                let v = m.get_mut("inlineScript").unwrap();
                if let Some(v) = v.as_object_mut() {
                    if v.contains_key("content") && v.contains_key("language") {
                        if let Ok(language) =
                            serde_json::from_value::<ScriptLang>(v.get("language").unwrap().clone())
                        {
                            let content = v
                                .get("content")
                                .unwrap()
                                .as_str()
                                .unwrap_or_default()
                                .to_string();
                            let mut logs = "".to_string();

                            let relative_imports = extract_relative_imports(
                                &content,
                                &format!("{job_path}/app"),
                                &Some(language.clone()),
                            );

                            if let Some((l, id)) = locks_to_reload
                                .as_ref()
                                .zip(container_id.as_ref())
                                // TODO: Remove fallback
                                .and_then(|e| {
                                    if *WMDEBUG_NO_COMPONENTS_TO_RELOCK {
                                        None
                                    } else {
                                        Some(e)
                                    }
                                })
                            {
                                if !l.contains(id) {
                                    dependency_map
                                        .reduce(
                                            relative_imports.clone(),
                                            &container_id.unwrap_or_default(),
                                            db.begin().await?,
                                        )
                                        .await?
                                        .commit()
                                        .await?;
                                    return Ok(Value::Object(m.clone()));
                                }
                            } else if v
                                .get("lock")
                                .is_some_and(|x| !x.as_str().unwrap().trim().is_empty())
                            {
                                if skip_creating_new_lock(&language, &content) {
                                    dependency_map
                                        .reduce(
                                            relative_imports.clone(),
                                            &container_id.unwrap_or_default(),
                                            db.begin().await?,
                                        )
                                        .await?
                                        .commit()
                                        .await?;

                                    logs.push_str(
                                        "Found already locked inline script. Skipping lock...\n",
                                    );
                                    return Ok(Value::Object(m.clone()));
                                }
                            }
                            logs.push_str("Found lockable inline script. Generating lock...\n");
                            let new_lock = capture_dependency_job(
                                &job.id,
                                &language,
                                &content,
                                mem_peak,
                                canceled_by,
                                job_dir,
                                db,
                                worker_name,
                                &job.workspace_id,
                                worker_dir,
                                base_internal_url,
                                token,
                                &format!("{}/app", job.runnable_path()),
                                false,
                                None,
                                occupancy_metrics,
                            )
                            .await;
                            match new_lock {
                                Ok(new_lock) => {
                                    append_logs(&job.id, &job.workspace_id, logs, &db.into()).await;

                                    dependency_map
                                        .reduce(
                                            relative_imports.clone(),
                                            &container_id.unwrap_or_default(),
                                            db.begin().await?,
                                        )
                                        .await?
                                        .commit()
                                        .await?;

                                    let anns =
                                        windmill_common::worker::TypeScriptAnnotations::parse(
                                            &content,
                                        );
                                    let nlang = if anns.native && language == ScriptLang::Bun {
                                        Some(ScriptLang::Bunnative)
                                    } else if !anns.native && language == ScriptLang::Bunnative {
                                        Some(ScriptLang::Bun)
                                    } else {
                                        None
                                    };
                                    if let Some(nlang) = nlang {
                                        v.insert(
                                            "language".to_string(),
                                            serde_json::Value::String(nlang.as_str().to_string()),
                                        );
                                    }
                                    v.insert(
                                        "lock".to_string(),
                                        serde_json::Value::String(new_lock),
                                    );
                                    return Ok(Value::Object(m.clone()));
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        language = ?language,
                                        error = ?e,
                                        logs = ?logs,
                                        "Failed to generate flow lock for inline script"
                                    );
                                    ()
                                }
                            }
                        }
                    }
                }
            }
            for (a, b) in m.clone().into_iter() {
                m.insert(
                    a.clone(),
                    lock_modules_app(
                        b,
                        job,
                        mem_peak,
                        canceled_by,
                        job_dir,
                        db,
                        worker_name,
                        worker_dir,
                        job_path,
                        base_internal_url,
                        token,
                        occupancy_metrics,
                        locks_to_reload,
                        m.get("id")
                            .and_then(Value::as_str)
                            .map(str::to_owned)
                            .or(container_id.clone()),
                        dependency_map,
                    )
                    .await?,
                );
            }
            Ok(Value::Object(m))
        }
        Value::Array(a) => {
            let mut nv = vec![];
            for b in a.clone().into_iter() {
                nv.push(
                    lock_modules_app(
                        b,
                        job,
                        mem_peak,
                        canceled_by,
                        job_dir,
                        db,
                        worker_name,
                        worker_dir,
                        job_path,
                        base_internal_url,
                        token,
                        occupancy_metrics,
                        locks_to_reload,
                        container_id.clone(),
                        dependency_map,
                    )
                    .await?,
                );
            }
            Ok(Value::Array(nv))
        }
        a @ _ => Ok(a),
    }
}

pub async fn handle_app_dependency_job(
    job: &MiniPulledJob,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job_dir: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    worker_name: &str,
    worker_dir: &str,
    base_internal_url: &str,
    token: &str,
    occupancy_metrics: &mut OccupancyMetrics,
) -> error::Result<()> {
    let job_path = job.runnable_path.clone().ok_or_else(|| {
        error::Error::internal_err(
            "Cannot resolve app dependencies for app without path".to_string(),
        )
    })?;

    let id = job
        .runnable_id
        .clone()
        .ok_or_else(|| Error::internal_err("App Dependency requires script hash".to_owned()))?
        .0;

    let components_to_relock = job
        .args
        .as_ref()
        .map(|x| {
            x.get("components_to_relock")
                .map(|v| serde_json::from_str::<Vec<String>>(v.get()).ok())
                .flatten()
        })
        .flatten();

    let triggered_by_relative_import = job
        .args
        .as_ref()
        .map(|x| x.get("triggered_by_relative_import").is_some())
        .unwrap_or_default();

    sqlx::query!(
        "DELETE FROM workspace_runnable_dependencies WHERE app_path = $1 AND workspace_id = $2",
        job_path,
        job.workspace_id
    )
    .execute(db)
    .await?;

    let record = sqlx::query!("SELECT app_id, value FROM app_version WHERE id = $1", id)
        .fetch_optional(db)
        .await?
        .map(|record| (record.app_id, record.value));

    let mut dependency_map =
        ScopedDependencyMap::new(&job.workspace_id, &job_path, "flow", db).await?;

    // // TODO: is parent_path only used for this edge-case?
    // dependency_map
    //     .rearrange_top_level(&parent_path, &mut *tx)
    //     .await?;

    // TODO: Use transaction for entire segment?
    if let Some((app_id, value)) = record {
        let value = lock_modules_app(
            value,
            job,
            mem_peak,
            canceled_by,
            job_dir,
            db,
            worker_name,
            worker_dir,
            &job_path,
            base_internal_url,
            token,
            occupancy_metrics,
            &components_to_relock,
            None,
            &mut dependency_map,
        )
        .await?;

        // Compute a lite version of the app value (w/ `inlineScript.{lock,code}`).
        let mut value_lite = value.clone();
        reduce_app(db, &job_path, &mut value_lite, app_id).await?;
        if let Value::Object(object) = &mut value_lite {
            object.insert("version".to_string(), json!(id));
        }
        sqlx::query!(
            "INSERT INTO app_version_lite (id, value) VALUES ($1, $2)
             ON CONFLICT (id) DO UPDATE SET value = EXCLUDED.value",
            id,
            sqlx::types::Json(to_raw_value(&value_lite)) as sqlx::types::Json<Box<RawValue>>,
        )
        .execute(db)
        .await?;

        // Re-check cancelation to ensure we don't accidentially override an app.
        if sqlx::query_scalar!(
            "SELECT canceled_by IS NOT NULL AS \"canceled!\" FROM v2_job_queue WHERE id = $1",
            job.id
        )
        .fetch_optional(db)
        .await
        .map(|v| Some(true) == v)
        .unwrap_or_else(|err| {
            tracing::error!(%job.id, %err, "error checking cancelation for job {0}: {err}", job.id);
            false
        }) {
            return Ok(());
        }

        sqlx::query!("UPDATE app_version SET value = $1 WHERE id = $2", value, id,)
            .execute(db)
            .await?;

        // NOTE: Temporary solution.
        // Ideally we do this for every job regardless whether it was triggered by relative import or by creation/update of the app.
        // NOTE: For now is not solving any problem but at some point we will introduce latest version caching
        // and when we do this will be last operation that will make new version appear as the latest and will trigger cache invalidation for all worker.
        if triggered_by_relative_import {
            sqlx::query!(
                "UPDATE app SET versions = array_append(versions, $1::bigint) WHERE path = $2 AND workspace_id = $3",
                id,
                &job_path,
                &job.workspace_id
            )
            .execute(db)
            .await?;
        }
        let (deployment_message, parent_path) =
            get_deployment_msg_and_parent_path_from_args(job.args.clone());

        if let Err(e) = handle_deployment_metadata(
            &job.permissioned_as_email,
            &job.created_by,
            &db,
            &job.workspace_id,
            DeployedObject::App { path: job_path, version: id, parent_path },
            deployment_message,
            false,
        )
        .await
        {
            tracing::error!(%e, "error handling deployment metadata");
        }

        // tx = PushIsolationLevel::Transaction(new_tx);
        // tx = handle_deployment_metadata(
        //     tx,
        //     &authed,
        //     &db,
        //     &w_id,
        //     DeployedObject::App { path: app.path.clone(), version: v_id },
        //     app.deployment_message,
        // )
        // .await?;

        // match tx {
        //     PushIsolationLevel::Transaction(tx) => tx.commit().await?,
        //     _ => {
        //         return Err(Error::internal_err(
        //             "Expected a transaction here".to_string(),
        //         ));
        //     }
        // }

        Ok(())
    } else {
        Ok(())
    }
}

// async fn upload_raw_app(
//     app_value: &RawAppValue,
//     job: &QueuedJob,
//     mem_peak: &mut i32,
//     canceled_by: &mut Option<CanceledBy>,
//     job_dir: &str,
//     db: &sqlx::Pool<sqlx::Postgres>,
//     worker_name: &str,
//     occupancy_metrics: &mut Option<&mut OccupancyMetrics>,
//     version: i64,
// ) -> Result<()> {
//     let mut entrypoint = "index.ts";
//     for file in app_value.files.iter() {
//         if file.0 == "/index.tsx" {
//             entrypoint = "index.tsx";
//         } else if file.0 == "/index.js" {
//             entrypoint = "index.js";
//         }
//         write_file(&job_dir, file.0, &file.1)?;
//     }
//     let common_bun_proc_envs: HashMap<String, String> = get_common_bun_proc_envs(None).await;

//     install_bun_lockfile(
//         mem_peak,
//         canceled_by,
//         &job.id,
//         &job.workspace_id,
//         Some(db),
//         job_dir,
//         worker_name,
//         common_bun_proc_envs,
//         false,
//         occupancy_metrics,
//     )
//     .await?;
//     let mut cmd = tokio::process::Command::new("esbuild");
//     let mut args = "--bundle --minify --outdir=dist/"
//         .split(' ')
//         .collect::<Vec<_>>();
//     args.push(entrypoint);
//     cmd.current_dir(job_dir)
//         .env_clear()
//         .args(args)
//         .stdout(Stdio::piped())
//         .stderr(Stdio::piped());
//     let child = start_child_process(cmd, "esbuild", false).await?;

//     crate::handle_child::handle_child(
//         &job.id,
//         db,
//         mem_peak,
//         canceled_by,
//         child,
//         false,
//         worker_name,
//         &job.workspace_id,
//         "esbuild",
//         Some(30),
//         false,
//         occupancy_metrics,
//     )
//     .await?;
//     let output_dir = format!("{}/dist", job_dir);
//     let target_dir = format!("/home/rfiszel/wmill/{}/{}", job.workspace_id, version);

//     tokio::fs::create_dir_all(&target_dir).await?;

//     tracing::info!("Copying files from {} to {}", output_dir, target_dir);

//     let index_ts = format!("{}/index.js", output_dir);
//     let index_css = format!("{}/index.css", output_dir);

//     if tokio::fs::metadata(&index_ts).await.is_ok() {
//         tokio::fs::copy(&index_ts, format!("{}/index.js", target_dir)).await?;
//     }

//     if tokio::fs::metadata(&index_css).await.is_ok() {
//         tokio::fs::copy(&index_css, format!("{}/index.css", target_dir)).await?;
//     }
//     // let file_path = format!("/home/rfiszel/wmill/{}/{}", job.workspace_id, version);

//     Ok(())
// }

#[cfg(feature = "python")]
async fn python_dep(
    reqs: String,
    job_id: &Uuid,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job_dir: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    worker_name: &str,
    w_id: &str,
    worker_dir: &str,
    occupancy_metrics: &mut Option<&mut OccupancyMetrics>,
    py_version: crate::PyV,
    annotations: PythonAnnotations,
) -> std::result::Result<String, Error> {
    use crate::python_executor::split_requirements;

    create_dependencies_dir(job_dir).await;

    let req: std::result::Result<String, Error> = uv_pip_compile(
        job_id,
        &reqs,
        mem_peak,
        canceled_by,
        job_dir,
        &db.into(),
        worker_name,
        w_id,
        occupancy_metrics,
        py_version,
        annotations.no_cache,
    )
    .await;
    // install the dependencies to pre-fill the cache
    if let Ok(req) = req.as_ref() {
        let r = handle_python_reqs(
            split_requirements(req),
            // req.split("\n").filter(|x| !x.starts_with("--")).collect(),
            job_id,
            w_id,
            mem_peak,
            canceled_by,
            &Connection::Sql(db.clone()),
            worker_name,
            job_dir,
            worker_dir,
            occupancy_metrics,
            // final_version,
            crate::PyVAlias::default().into(),
        )
        .await;

        if let Err(e) = r {
            tracing::error!(
                "Failed to install python dependencies to prefill the cache: {:?} \n",
                e
            );
        }
    }
    req
}

#[cfg(feature = "python")]
async fn ansible_dep(
    reqs: AnsibleRequirements,
    job_id: &Uuid,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job_dir: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    worker_name: &str,
    w_id: &str,
    worker_dir: &str,
    occupancy_metrics: &mut OccupancyMetrics,
    token: &str,
    base_internal_url: &str,
) -> std::result::Result<String, Error> {
    use windmill_parser_yaml::add_versions_to_requirements_yaml;

    use crate::ansible_executor::{
        create_ansible_cfg, get_collection_locks, get_git_ssh_cmd, get_role_locks,
        install_galaxy_collections,
    };
    use windmill_common::client::AuthedClient;

    let python_lockfile = python_dep(
        reqs.python_reqs.join("\n").to_string(),
        job_id,
        mem_peak,
        canceled_by,
        job_dir,
        db,
        worker_name,
        w_id,
        worker_dir,
        &mut Some(occupancy_metrics),
        crate::PyV::gravitational_version(job_id, w_id, Some(db.clone().into())).await,
        PythonAnnotations::default(),
    )
    .await?;

    let conn = &Connection::Sql(db.clone());

    let authed_client = AuthedClient::new(
        base_internal_url.to_string(),
        w_id.to_string(),
        token.to_string(),
        None,
    );

    let git_ssh_cmd = get_git_ssh_cmd(&reqs, job_dir, &authed_client).await?;

    let git_repos = get_git_repos_lock(
        &reqs.git_repos,
        job_dir,
        job_id,
        worker_name,
        conn,
        mem_peak,
        canceled_by,
        w_id,
        occupancy_metrics,
        &git_ssh_cmd,
    )
    .await?;

    let ansible_lockfile;

    create_ansible_cfg(Some(&reqs), job_dir, false)?;

    if let Some(collections) = reqs.roles_and_collections.as_ref() {
        install_galaxy_collections(
            collections,
            job_dir,
            job_id,
            worker_name,
            w_id,
            mem_peak,
            canceled_by,
            conn,
            occupancy_metrics,
            &git_ssh_cmd,
        )
        .await?;

        let (collection_versions, logs1) = get_collection_locks(job_dir).await?;

        let (role_versions, logs2) = if collections.contains("roles:") {
            get_role_locks(job_dir).await?
        } else {
            (HashMap::new(), String::new())
        };

        let (reqs_yaml, logs3) =
            add_versions_to_requirements_yaml(&collections, &role_versions, &collection_versions)?;

        let logs = format!("\n{logs1}\n{logs2}\n{logs3}\n");

        append_logs(job_id, w_id, &logs, conn).await;

        ansible_lockfile = AnsibleDependencyLocks {
            python_lockfile,
            git_repos,
            collections_and_roles: reqs_yaml,
            collections_and_roles_logs: logs,
        };
    } else {
        ansible_lockfile = AnsibleDependencyLocks {
            python_lockfile,
            git_repos,
            collections_and_roles: String::new(),
            collections_and_roles_logs: String::new(),
        };
    }

    serde_json::to_string(&ansible_lockfile).map_err(|e| e.into())
}

pub const LOCKFILE_GENERATED_FROM_REQUIREMENTS_TXT: &str = "# from requirements.txt";

async fn capture_dependency_job(
    job_id: &Uuid,
    job_language: &ScriptLang,
    job_raw_code: &str,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job_dir: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    worker_name: &str,
    w_id: &str,
    #[allow(unused_variables)] worker_dir: &str,
    base_internal_url: &str,
    token: &str,
    script_path: &str,
    raw_deps: bool,
    npm_mode: Option<bool>,
    occupancy_metrics: &mut OccupancyMetrics,
) -> error::Result<String> {
    match job_language {
        ScriptLang::Python3 => {
            #[cfg(not(feature = "python"))]
            return Err(Error::internal_err(
                "Python requires the python feature to be enabled".to_string(),
            ));
            #[cfg(feature = "python")]
            {
                // Manually assigned version from requirements.txt
                // let assigned_py_version;
                let (reqs, py_version) = if raw_deps {
                    // `wmill script generate-metadata`
                    // should also respect annotated pyversion
                    // can be annotated in script itself
                    // or in requirements.txt if present

                    (
                        job_raw_code.to_owned(),
                        match crate::PyV::try_parse_from_requirements(&split_requirements(
                            job_raw_code,
                        )) {
                            Some(pyv) => pyv,
                            None => crate::PyV::gravitational_version(job_id, w_id, None).await,
                        },
                    )
                } else {
                    let mut version_specifiers = vec![];
                    let PythonAnnotations { py_select_latest, .. } =
                        PythonAnnotations::parse(job_raw_code);
                    (
                        windmill_parser_py_imports::parse_python_imports(
                            job_raw_code,
                            &w_id,
                            script_path,
                            &db,
                            &mut version_specifiers,
                        )
                        .await?
                        .0
                        .join("\n"),
                        crate::PyV::resolve(
                            version_specifiers,
                            job_id,
                            w_id,
                            py_select_latest,
                            Some(db.clone().into()),
                            None,
                            None,
                        )
                        .await?,
                    )
                };

                python_dep(
                    reqs,
                    job_id,
                    mem_peak,
                    canceled_by,
                    job_dir,
                    db,
                    worker_name,
                    w_id,
                    worker_dir,
                    &mut Some(occupancy_metrics),
                    py_version,
                    PythonAnnotations::parse(job_raw_code),
                )
                .await
                .map(|res| {
                    if raw_deps {
                        format!("{}\n{}", LOCKFILE_GENERATED_FROM_REQUIREMENTS_TXT, res)
                    } else {
                        res
                    }
                })
            }
        }
        ScriptLang::Ansible => {
            #[cfg(not(feature = "python"))]
            return Err(Error::internal_err(
                "Ansible requires the python feature to be enabled".to_string(),
            ));

            #[cfg(feature = "python")]
            {
                if raw_deps {
                    return Err(Error::ExecutionErr(
                        "Raw dependencies not supported for ansible".to_string(),
                    ));
                }
                let (_logs, reqs, _) = windmill_parser_yaml::parse_ansible_reqs(job_raw_code)?;

                ansible_dep(
                    reqs.unwrap_or_default(),
                    job_id,
                    mem_peak,
                    canceled_by,
                    job_dir,
                    db,
                    worker_name,
                    w_id,
                    worker_dir,
                    occupancy_metrics,
                    token,
                    base_internal_url,
                )
                .await
            }
        }
        ScriptLang::Go => {
            install_go_dependencies(
                job_id,
                job_raw_code,
                mem_peak,
                canceled_by,
                job_dir,
                &db.into(),
                false,
                false,
                false,
                raw_deps,
                worker_name,
                w_id,
                occupancy_metrics,
            )
            .await
        }
        ScriptLang::Deno => {
            if raw_deps {
                return Err(Error::ExecutionErr(
                    "Raw dependencies not supported for deno".to_string(),
                ));
            }
            generate_deno_lock(
                job_id,
                job_raw_code,
                mem_peak,
                canceled_by,
                job_dir,
                Some(&db.into()),
                w_id,
                worker_name,
                base_internal_url,
                &mut Some(occupancy_metrics),
            )
            .await
        }
        ScriptLang::Bun | ScriptLang::Bunnative => {
            let npm_mode = npm_mode.unwrap_or_else(|| {
                windmill_common::worker::TypeScriptAnnotations::parse(job_raw_code).npm
            });
            if !raw_deps {
                let _ = write_file(job_dir, "main.ts", job_raw_code)?;
            }
            let req = gen_bun_lockfile(
                mem_peak,
                canceled_by,
                job_id,
                w_id,
                Some(&db.into()),
                token,
                script_path,
                job_dir,
                base_internal_url,
                worker_name,
                true,
                if raw_deps {
                    Some(job_raw_code.to_string())
                } else {
                    None
                },
                npm_mode,
                &mut Some(occupancy_metrics),
            )
            .await?;
            if req.is_some() && !raw_deps {
                crate::bun_executor::prebundle_bun_script(
                    job_raw_code,
                    req.as_ref(),
                    script_path,
                    job_id,
                    w_id,
                    Some(&db),
                    &job_dir,
                    base_internal_url,
                    worker_name,
                    &token,
                    &mut Some(occupancy_metrics),
                )
                .await?;
            }
            Ok(req.unwrap_or_else(String::new))
        }
        ScriptLang::Php => {
            #[cfg(not(feature = "php"))]
            return Err(Error::internal_err(
                "PHP requires the php feature to be enabled".to_string(),
            ));

            #[cfg(feature = "php")]
            {
                let reqs = if raw_deps {
                    if job_raw_code.is_empty() {
                        return Ok("".to_string());
                    }
                    job_raw_code.to_string()
                } else {
                    match parse_php_imports(job_raw_code)? {
                        Some(reqs) => reqs,
                        None => {
                            return Ok("".to_string());
                        }
                    }
                };
                composer_install(
                    mem_peak,
                    canceled_by,
                    job_id,
                    w_id,
                    &Connection::Sql(db.clone()),
                    job_dir,
                    worker_name,
                    reqs,
                    None,
                    occupancy_metrics,
                )
                .await
            }
        }
        ScriptLang::Rust => {
            if raw_deps {
                return Err(Error::ExecutionErr(
                    "Raw dependencies not supported for rust".to_string(),
                ));
            }

            #[cfg(not(feature = "rust"))]
            return Err(Error::internal_err(
                "Rust requires the rust feature to be enabled".to_string(),
            ));

            #[cfg(feature = "rust")]
            let lockfile = generate_cargo_lockfile(
                job_id,
                job_raw_code,
                mem_peak,
                canceled_by,
                job_dir,
                &Connection::Sql(db.clone()),
                worker_name,
                w_id,
                occupancy_metrics,
            )
            .await?;

            #[cfg(feature = "rust")]
            Ok(lockfile)
        }
        ScriptLang::CSharp => {
            if raw_deps {
                return Err(Error::ExecutionErr(
                    "Raw dependencies not supported for C#".to_string(),
                ));
            }

            generate_nuget_lockfile(
                job_id,
                job_raw_code,
                mem_peak,
                canceled_by,
                job_dir,
                &Connection::Sql(db.clone()),
                worker_name,
                w_id,
                occupancy_metrics,
            )
            .await
        }
        #[cfg(feature = "java")]
        ScriptLang::Java => {
            if raw_deps {
                return Err(Error::ExecutionErr(
                    "Raw dependencies not supported for Java".to_string(),
                ));
            }

            java_executor::resolve(
                job_id,
                job_raw_code,
                job_dir,
                &Connection::Sql(db.clone()),
                w_id,
            )
            .await
        }
        #[cfg(feature = "ruby")]
        ScriptLang::Ruby => {
            if raw_deps {
                return Err(Error::ExecutionErr(
                    "Raw dependencies not supported for Ruby".to_string(),
                ));
            }

            ruby_executor::resolve(
                job_id,
                job_raw_code,
                mem_peak,
                canceled_by,
                job_dir,
                &Connection::Sql(db.clone()),
                worker_name,
                w_id,
            )
            .await
        }
        // for related places search: ADD_NEW_LANG
        _ => Ok("".to_owned()),
    }
}
