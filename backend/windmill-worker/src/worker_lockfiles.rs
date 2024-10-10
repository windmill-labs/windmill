use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

use async_recursion::async_recursion;
use serde_json::value::RawValue;
use serde_json::{json, Value};
use sqlx::types::Json;
use uuid::Uuid;
use windmill_common::error::Error;
use windmill_common::error::Result;
use windmill_common::flows::{FlowModule, FlowModuleValue};
use windmill_common::get_latest_deployed_hash_for_path;
use windmill_common::jobs::JobPayload;
use windmill_common::scripts::ScriptHash;
use windmill_common::worker::{get_annotation_ts, to_raw_value, to_raw_value_owned, write_file};
use windmill_common::{
    error::{self, to_anyhow},
    flows::FlowValue,
    jobs::QueuedJob,
    scripts::ScriptLang,
    DB,
};
use windmill_git_sync::{handle_deployment_metadata, DeployedObject};
use windmill_parser_py_imports::parse_relative_imports;
use windmill_parser_ts::parse_expr_for_imports;
use windmill_queue::{append_logs, CanceledBy, PushIsolationLevel};

use crate::common::OccupancyMetrics;
use crate::python_executor::{create_dependencies_dir, handle_python_reqs, uv_pip_compile};
use crate::rust_executor::{build_rust_crate, compute_rust_hash, generate_cargo_lockfile};
use crate::{
    bun_executor::gen_bun_lockfile,
    deno_executor::generate_deno_lock,
    go_executor::install_go_dependencies,
    php_executor::{composer_install, parse_php_imports},
};

pub async fn update_script_dependency_map(
    job_id: &Uuid,
    db: &DB,
    w_id: &str,
    parent_path: &Option<String>,
    script_path: &str,
    relative_imports: Vec<String>,
) -> error::Result<()> {
    let importer_kind = "script";
    if !relative_imports.is_empty() {
        let mut logs = "".to_string();
        logs.push_str("\n--- RELATIVE IMPORTS ---\n\n");
        logs.push_str(&relative_imports.join("\n"));

        let mut tx = db.begin().await?;
        tx =
            clear_dependency_parent_path(parent_path, script_path, w_id, importer_kind, tx).await?;

        tx = clear_dependency_map_for_item(script_path, w_id, importer_kind, tx, &None).await?;

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
        tx.commit().await?;
        append_logs(job_id, w_id, logs, db).await;
    }
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
) -> Result<sqlx::Transaction<'c, sqlx::Postgres>> {
    sqlx::query!(
        "DELETE FROM dependency_map
                 WHERE importer_path = $1 AND importer_kind = $3::text::IMPORTER_KIND
                 AND workspace_id = $2 AND ($4::text IS NULL OR importer_node_id = $4::text)",
        item_path,
        w_id,
        importer_kind,
        importer_node_id.clone()
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

fn parse_bun_relative_imports(raw_code: &str, script_path: &str) -> error::Result<Vec<String>> {
    let mut relative_imports = vec![];
    let r = parse_expr_for_imports(raw_code)?;
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
                tracing::error!("error canonicalizing path: {:?}", normalized);
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
        Some(ScriptLang::Python3) => parse_relative_imports(&raw_code, script_path).ok(),
        Some(ScriptLang::Bun) | Some(ScriptLang::Bunnative) => {
            parse_bun_relative_imports(&raw_code, script_path).ok()
        }
        _ => None,
    }
}
#[tracing::instrument(level = "trace", skip_all)]
pub async fn handle_dependency_job<R: rsmq_async::RsmqConnection + Send + Sync + Clone>(
    job: &QueuedJob,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job_dir: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    worker_name: &str,
    worker_dir: &str,
    base_internal_url: &str,
    token: &str,
    rsmq: Option<R>,
    occupancy_metrics: &mut OccupancyMetrics,
) -> error::Result<Box<RawValue>> {
    let raw_code = match job.raw_code {
        Some(ref code) => code.to_owned(),
        None => sqlx::query_scalar!(
            "SELECT content FROM script WHERE hash = $1 AND workspace_id = $2",
            &job.script_hash.unwrap_or(ScriptHash(0)).0,
            &job.workspace_id
        )
        .fetch_optional(db)
        .await?
        .unwrap_or_else(|| "No script found at this hash".to_string()),
    };

    let script_path = job.script_path();
    let raw_deps = job
        .args
        .as_ref()
        .map(|x| {
            x.get("raw_deps")
                .is_some_and(|y| y.to_string().as_str() == "true")
        })
        .unwrap_or(false);
    let npm_mode = if job
        .language
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

    let content = capture_dependency_job(
        &job.id,
        job.language.as_ref().map(|v| Ok(v)).unwrap_or_else(|| {
            Err(Error::InternalErr(
                "Job Language required for dependency jobs".to_owned(),
            ))
        })?,
        &raw_code,
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
            if job.script_hash.is_none() {
                // it a one-off raw script dependency job, no need to update the db
                return Ok(to_raw_value_owned(
                    json!({ "status": "Successful lock file generation", "lock": content }),
                ));
            }

            let hash = job.script_hash.unwrap_or(ScriptHash(0));
            let w_id = &job.workspace_id;
            sqlx::query!(
                "UPDATE script SET lock = $1, created_at = now() WHERE hash = $2 AND workspace_id = $3",
                &content,
                &hash.0,
                w_id
            )
            .execute(db)
            .await?;

            let (deployment_message, parent_path) =
                get_deployment_msg_and_parent_path_from_args(job.args.clone());

            if let Err(e) = handle_deployment_metadata(
                &job.email,
                &job.created_by,
                &db,
                &w_id,
                DeployedObject::Script {
                    hash,
                    path: script_path.to_string(),
                    parent_path: parent_path.clone(),
                },
                deployment_message.clone(),
                rsmq.clone(),
                false,
            )
            .await
            {
                tracing::error!(%e, "error handling deployment metadata");
            }

            let relative_imports = extract_relative_imports(&raw_code, script_path, &job.language);
            if let Some(relative_imports) = relative_imports {
                update_script_dependency_map(
                    &job.id,
                    db,
                    w_id,
                    &parent_path,
                    script_path,
                    relative_imports,
                )
                .await?;
                let already_visited = job
                    .args
                    .as_ref()
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
                    &job.email,
                    &job.created_by,
                    &job.permissioned_as,
                    db,
                    rsmq,
                    already_visited,
                )
                .await
                {
                    tracing::error!(%e, "error triggering dependents to recompute dependencies");
                }
            }

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
                &job.script_hash.unwrap_or(ScriptHash(0)).0,
                &job.workspace_id
            )
            .execute(db)
            .await?;
            Err(Error::ExecutionErr(format!("Error locking file: {error}")))?
        }
    }
}

async fn trigger_dependents_to_recompute_dependencies<
    R: rsmq_async::RsmqConnection + Send + Sync + Clone,
>(
    w_id: &str,
    script_path: &str,
    deployment_message: Option<String>,
    parent_path: Option<String>,
    email: &str,
    created_by: &str,
    permissioned_as: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    rsmq: Option<R>,
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
        let tx: PushIsolationLevel<'_, R> =
            PushIsolationLevel::IsolatedRoot(db.clone(), rsmq.clone());
        let mut args: HashMap<String, Box<RawValue>> = HashMap::new();
        if let Some(ref dm) = deployment_message {
            args.insert("deployment_message".to_string(), to_raw_value(&dm));
        }
        if let Some(ref p_path) = parent_path {
            args.insert("common_dependency_path".to_string(), to_raw_value(&p_path));
        }

        args.insert(
            "already_visited".to_string(),
            to_raw_value(&already_visited),
        );
        let kind = s.importer_kind.clone().unwrap_or_default();
        let job_payload = if kind == "script" {
            let r = get_latest_deployed_hash_for_path(db, w_id, s.importer_path.as_str()).await;
            if let Ok(r) = r {
                JobPayload::Dependencies {
                    path: s.importer_path.clone(),
                    hash: r.0,
                    language: r.6,
                    dedicated_worker: r.7,
                }
            } else {
                tracing::error!(
                    "error getting latest deployed hash for path {path}: {err}",
                    path = s.importer_path,
                    err = r.unwrap_err()
                );
                continue;
            }
        } else if kind == "flow" {
            args.insert(
                "nodes_to_relock".to_string(),
                to_raw_value(&s.importer_node_ids),
            );
            let r = sqlx::query_scalar!(
                "SELECT versions[array_upper(versions, 1)] FROM flow WHERE path = $1 AND workspace_id = $2",
                s.importer_path,
                w_id,
            ).fetch_one(db)
            .await
            .map_err(to_anyhow);
            match r {
                Ok(Some(version)) => JobPayload::FlowDependencies {
                    path: s.importer_path.clone(),
                    dedicated_worker: None,
                    version: version,
                },
                Ok(None) => {
                    tracing::error!(
                        "no flow version found for path {path}",
                        path = s.importer_path
                    );
                    continue;
                }
                Err(err) => {
                    tracing::error!(
                        "error getting latest deployed flow version for path {path}: {err}",
                        path = s.importer_path,
                    );
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

pub async fn handle_flow_dependency_job<R: rsmq_async::RsmqConnection + Send + Sync + Clone>(
    job: &QueuedJob,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job_dir: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    worker_name: &str,
    worker_dir: &str,
    base_internal_url: &str,
    token: &str,
    rsmq: Option<R>,
    occupancy_metrics: &mut OccupancyMetrics,
) -> error::Result<Box<serde_json::value::RawValue>> {
    let job_path = job.script_path.clone().ok_or_else(|| {
        error::Error::InternalErr(
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
            job.script_hash
                .clone()
                .ok_or_else(|| {
                    Error::InternalErr(
                        "Flow Dependency requires script hash (flow version)".to_owned(),
                    )
                })?
                .0,
        )
    };

    let raw_flow = job.raw_flow.clone().map(|v| Ok(v)).unwrap_or_else(|| {
        Err(Error::InternalErr(
            "Flow Dependency requires raw flow".to_owned(),
        ))
    })?;
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

    let mut flow = serde_json::from_str::<FlowValue>((*raw_flow.0).get()).map_err(to_anyhow)?;

    let mut tx = db.begin().await?;

    tx = clear_dependency_parent_path(&parent_path, &job_path, &job.workspace_id, "flow", tx)
        .await?;
    let modified_ids;
    (flow.modules, tx, modified_ids) = lock_modules(
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
    )
    .await?;
    let new_flow_value = serde_json::to_value(flow).map_err(to_anyhow)?;

    // Re-check cancelation to ensure we don't accidentially override a flow.
    if sqlx::query_scalar!("SELECT canceled FROM queue WHERE id = $1", job.id)
        .fetch_optional(db)
        .await
        .map(|v| Some(true) == v)
        .unwrap_or_else(|err| {
            tracing::error!(%job.id, %err, "error checking cancelation for job {0}: {err}", job.id);
            false
        })
    {
        return Ok(to_raw_value_owned(json!({
            "status": "Flow lock generation was canceled",
        })));
    }

    if !skip_flow_update {
        let version = version.ok_or_else(|| {
            Error::InternalErr("Flow Dependency requires script hash (flow version)".to_owned())
        })?;

        sqlx::query!(
            "UPDATE flow SET value = $1 WHERE path = $2 AND workspace_id = $3",
            new_flow_value,
            job_path,
            job.workspace_id
        )
        .execute(db)
        .await?;
        sqlx::query!(
            "UPDATE flow_version SET value = $1 WHERE id = $2",
            new_flow_value,
            version
        )
        .execute(db)
        .await?;

        tx.commit().await?;

        if let Err(e) = handle_deployment_metadata(
            &job.email,
            &job.created_by,
            &db,
            &job.workspace_id,
            DeployedObject::Flow { path: job_path, parent_path, version },
            deployment_message,
            rsmq.clone(),
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

async fn lock_modules<'c>(
    modules: Vec<FlowModule>,
    job: &QueuedJob,
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
    // (modules to replace old seq (even unmmodified ones), new transaction, modified ids) )
) -> Result<(
    Vec<FlowModule>,
    sqlx::Transaction<'c, sqlx::Postgres>,
    Vec<String>,
)> {
    let mut new_flow_modules = Vec::new();
    let mut modified_ids = Vec::new();
    for mut e in modules.into_iter() {
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
        } = e.get_value()?
        else {
            match e.get_value()? {
                FlowModuleValue::ForloopFlow {
                    iterator,
                    modules,
                    skip_failures,
                    parallel,
                    parallelism,
                } => {
                    let nmodules;
                    (nmodules, tx, nmodified_ids) = Box::pin(lock_modules(
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
                    ))
                    .await?;
                    e.value = FlowModuleValue::ForloopFlow {
                        iterator,
                        modules: nmodules,
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
                        (nmodules, tx, inner_modified_ids) = Box::pin(lock_modules(
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
                        ))
                        .await?;
                        nmodified_ids.extend(inner_modified_ids);
                        b.modules = nmodules;
                        nbranches.push(b)
                    }
                    e.value = FlowModuleValue::BranchAll { branches: nbranches, parallel }.into()
                }
                FlowModuleValue::WhileloopFlow { modules, skip_failures } => {
                    let nmodules;
                    (nmodules, tx, nmodified_ids) = Box::pin(lock_modules(
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
                    ))
                    .await?;
                    e.value =
                        FlowModuleValue::WhileloopFlow { modules: nmodules, skip_failures }.into()
                }
                FlowModuleValue::BranchOne { branches, default } => {
                    let mut nbranches = vec![];
                    nmodified_ids = vec![];
                    for mut b in branches {
                        let nmodules;
                        let inner_modified_ids;

                        (nmodules, tx, inner_modified_ids) = Box::pin(lock_modules(
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
                        ))
                        .await?;
                        nmodified_ids.extend(inner_modified_ids);
                        b.modules = nmodules;
                        nbranches.push(b)
                    }
                    let ndefault;
                    (ndefault, tx, nmodified_ids) = Box::pin(lock_modules(
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
                    ))
                    .await?;
                    e.value = FlowModuleValue::BranchOne { branches: nbranches, default: ndefault }
                        .into();
                }
                _ => (),
            };
            modified_ids.extend(nmodified_ids);
            new_flow_modules.push(e);
            continue;
        };

        if let Some(locks_to_reload) = locks_to_reload {
            if !locks_to_reload.contains(&e.id) {
                new_flow_modules.push(e);
                continue;
            }
        } else {
            if lock.as_ref().is_some_and(|x| !x.trim().is_empty()) {
                let skip_creating_new_lock = skip_creating_new_lock(&language, &content);
                if skip_creating_new_lock {
                    new_flow_modules.push(e);
                    continue;
                }
            }
        }

        modified_ids.push(e.id.clone());

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
            false,
            None,
            occupancy_metrics,
        )
        .await;
        //
        match new_lock {
            Ok(new_lock) => {
                let dep_path = path.clone().unwrap_or_else(|| job_path.to_string());
                tx = clear_dependency_map_for_item(
                    &job_path,
                    &job.workspace_id,
                    "flow",
                    tx,
                    &Some(e.id.clone()),
                )
                .await?;
                let relative_imports = extract_relative_imports(
                    &content,
                    &format!("{dep_path}/flow"),
                    &Some(language.clone()),
                );
                if let Some(relative_imports) = relative_imports {
                    let mut logs = "".to_string();
                    logs.push_str(format!("\n\n--- RELATIVE IMPORTS of {} ---\n\n", e.id).as_str());

                    tx = add_relative_imports_to_dependency_map(
                        &dep_path,
                        &job.workspace_id,
                        relative_imports,
                        "flow",
                        tx,
                        &mut logs,
                        Some(e.id.clone()),
                    )
                    .await?;
                    append_logs(&job.id, &job.workspace_id, logs, db).await;
                }

                if language == ScriptLang::Bun || language == ScriptLang::Bunnative {
                    let anns = get_annotation_ts(&content);
                    if anns.native_mode && language == ScriptLang::Bun {
                        language = ScriptLang::Bunnative;
                    } else if !anns.native_mode && language == ScriptLang::Bunnative {
                        language = ScriptLang::Bun;
                    };
                }
                e.value = windmill_common::worker::to_raw_value(&FlowModuleValue::RawScript {
                    lock: Some(new_lock),
                    path,
                    input_transforms,
                    content,
                    language,
                    tag,
                    custom_concurrency_key,
                    concurrent_limit,
                    concurrency_time_window_s,
                });
                new_flow_modules.push(e);
                continue;
            }
            Err(error) => {
                // TODO: Record flow raw script error lock logs
                tracing::warn!(
                    path = path,
                    language = ?language,
                    error = ?error,
                    "Failed to generate flow lock for raw script"
                );
                e.value = windmill_common::worker::to_raw_value(&FlowModuleValue::RawScript {
                    lock: None,
                    path,
                    input_transforms,
                    content,
                    language,
                    tag,
                    custom_concurrency_key,
                    concurrent_limit,
                    concurrency_time_window_s,
                });
                new_flow_modules.push(e);
                continue;
            }
        }
    }
    Ok((new_flow_modules, tx, modified_ids))
}

fn skip_creating_new_lock(language: &ScriptLang, content: &str) -> bool {
    if language == &ScriptLang::Bun || language == &ScriptLang::Bunnative {
        let anns = get_annotation_ts(&content);
        if anns.native_mode && language == &ScriptLang::Bun {
            return false;
        } else if !anns.native_mode && language == &ScriptLang::Bunnative {
            return false;
        };
    }
    true
}

#[async_recursion]
async fn lock_modules_app(
    value: Value,
    job: &QueuedJob,
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
) -> Result<Value> {
    match value {
        Value::Object(mut m) => {
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
                            if v.get("lock")
                                .is_some_and(|x| !x.as_str().unwrap().trim().is_empty())
                            {
                                if skip_creating_new_lock(&language, &content) {
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
                                &format!("{}/app", job.script_path()),
                                false,
                                None,
                                occupancy_metrics,
                            )
                            .await;
                            match new_lock {
                                Ok(new_lock) => {
                                    append_logs(&job.id, &job.workspace_id, logs, db).await;
                                    let anns = get_annotation_ts(&content);
                                    let nlang = if anns.native_mode && language == ScriptLang::Bun {
                                        Some(ScriptLang::Bunnative)
                                    } else if !anns.native_mode && language == ScriptLang::Bunnative
                                    {
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
                    )
                    .await?,
                );
            }
            Ok(Value::Array(nv))
        }
        a @ _ => Ok(a),
    }
}

pub async fn handle_app_dependency_job<R: rsmq_async::RsmqConnection + Send + Sync + Clone>(
    job: &QueuedJob,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job_dir: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    worker_name: &str,
    worker_dir: &str,
    base_internal_url: &str,
    token: &str,
    rsmq: Option<R>,
    occupancy_metrics: &mut OccupancyMetrics,
) -> error::Result<()> {
    let job_path = job.script_path.clone().ok_or_else(|| {
        error::Error::InternalErr(
            "Cannot resolve app dependencies for app without path".to_string(),
        )
    })?;

    let id = job
        .script_hash
        .clone()
        .ok_or_else(|| Error::InternalErr("App Dependency requires script hash".to_owned()))?
        .0;
    let value = sqlx::query_scalar!("SELECT value FROM app_version WHERE id = $1", id)
        .fetch_optional(db)
        .await?;

    if let Some(value) = value {
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
        )
        .await?;

        // Re-check cancelation to ensure we don't accidentially override an app.
        if sqlx::query_scalar!("SELECT canceled FROM queue WHERE id = $1", job.id)
            .fetch_optional(db)
            .await
            .map(|v| Some(true) == v)
            .unwrap_or_else(|err| {
                tracing::error!(%job.id, %err, "error checking cancelation for job {0}: {err}", job.id);
                false
            })
        {
            return Ok(());
        }

        sqlx::query!("UPDATE app_version SET value = $1 WHERE id = $2", value, id,)
            .execute(db)
            .await?;

        let (deployment_message, parent_path) =
            get_deployment_msg_and_parent_path_from_args(job.args.clone());

        if let Err(e) = handle_deployment_metadata(
            &job.email,
            &job.created_by,
            &db,
            &job.workspace_id,
            DeployedObject::App { path: job_path, version: id, parent_path },
            deployment_message,
            rsmq.clone(),
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
        //         return Err(Error::InternalErr(
        //             "Expected a transaction here".to_string(),
        //         ));
        //     }
        // }

        Ok(())
    } else {
        Ok(())
    }
}

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
) -> std::result::Result<String, Error> {
    create_dependencies_dir(job_dir).await;
    let req: std::result::Result<String, Error> = uv_pip_compile(
        job_id,
        &reqs,
        mem_peak,
        canceled_by,
        job_dir,
        db,
        worker_name,
        w_id,
        occupancy_metrics,
        false,
        false,
    )
    .await;
    // install the dependencies to pre-fill the cache
    if let Ok(req) = req.as_ref() {
        let r = handle_python_reqs(
            req.split("\n").filter(|x| !x.starts_with("--")).collect(),
            job_id,
            w_id,
            mem_peak,
            canceled_by,
            db,
            worker_name,
            job_dir,
            worker_dir,
            occupancy_metrics,
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
    worker_dir: &str,
    base_internal_url: &str,
    token: &str,
    script_path: &str,
    raw_deps: bool,
    npm_mode: Option<bool>,
    occupancy_metrics: &mut OccupancyMetrics,
) -> error::Result<String> {
    match job_language {
        ScriptLang::Python3 => {
            let reqs = if raw_deps {
                job_raw_code.to_string()
            } else {
                let mut already_visited = vec![];

                windmill_parser_py_imports::parse_python_imports(
                    job_raw_code,
                    &w_id,
                    script_path,
                    &db,
                    &mut already_visited,
                )
                .await?
                .join("\n")
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
            )
            .await
        }
        ScriptLang::Ansible => {
            if raw_deps {
                return Err(Error::ExecutionErr(
                    "Raw dependencies not supported for ansible".to_string(),
                ));
            }
            let (_logs, reqs, _) = windmill_parser_yaml::parse_ansible_reqs(job_raw_code)?;
            let reqs = reqs.map(|r| r.python_reqs.join("\n")).unwrap_or_default();

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
            )
            .await
        }
        ScriptLang::Go => {
            if raw_deps {
                return Err(Error::ExecutionErr(
                    "Raw dependencies not supported for go".to_string(),
                ));
            }
            install_go_dependencies(
                job_id,
                job_raw_code,
                mem_peak,
                canceled_by,
                job_dir,
                db,
                false,
                false,
                false,
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
                Some(db),
                w_id,
                worker_name,
                base_internal_url,
                &mut Some(occupancy_metrics),
            )
            .await
        }
        ScriptLang::Bun | ScriptLang::Bunnative => {
            let npm_mode = npm_mode.unwrap_or_else(|| {
                windmill_common::worker::get_annotation_ts(job_raw_code).npm_mode
            });
            if !raw_deps {
                let _ = write_file(job_dir, "main.ts", job_raw_code)?;
            }
            let req = gen_bun_lockfile(
                mem_peak,
                canceled_by,
                job_id,
                w_id,
                Some(db),
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
                    req.clone(),
                    script_path,
                    job_id,
                    w_id,
                    Some(db.clone()),
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
                db,
                job_dir,
                worker_name,
                reqs,
                None,
                occupancy_metrics,
            )
            .await
        }
        ScriptLang::Rust => {
            if raw_deps {
                return Err(Error::ExecutionErr(
                    "Raw dependencies not supported for rust".to_string(),
                ));
            }

            let lockfile = generate_cargo_lockfile(
                job_id,
                job_raw_code,
                mem_peak,
                canceled_by,
                job_dir,
                db,
                worker_name,
                w_id,
                occupancy_metrics,
            )
            .await?;

            build_rust_crate(
                job_id,
                mem_peak,
                canceled_by,
                job_dir,
                db,
                worker_name,
                w_id,
                base_internal_url,
                &compute_rust_hash(&job_raw_code, Some(&lockfile)),
                occupancy_metrics,
            )
            .await?;
            Ok(lockfile)
        }
        ScriptLang::Postgresql => Ok("".to_owned()),
        ScriptLang::Mysql => Ok("".to_owned()),
        ScriptLang::Bigquery => Ok("".to_owned()),
        ScriptLang::Snowflake => Ok("".to_owned()),
        ScriptLang::Mssql => Ok("".to_owned()),
        ScriptLang::Graphql => Ok("".to_owned()),
        ScriptLang::Bash => Ok("".to_owned()),
        ScriptLang::Powershell => Ok("".to_owned()),
        ScriptLang::Nativets => Ok("".to_owned()),
    }
}
