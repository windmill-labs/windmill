use std::borrow::Cow;
use std::collections::HashMap;
use std::fs::{create_dir_all, remove_dir_all};
use std::path::{Component, Path, PathBuf};

#[cfg(feature = "python")]
use crate::ansible_executor::{get_git_repos_lock, AnsibleDependencyLocks};
use crate::scoped_dependency_map::{DependencyDependent, ScopedDependencyMap};
use async_recursion::async_recursion;
use chrono::{Duration, Utc};
use itertools::Itertools;
use serde::Serialize;
use serde_json::value::RawValue;
use serde_json::{from_value, json, Value};
use sha2::Digest;
use sqlx::types::Json;
use tokio::time::timeout;
use uuid::Uuid;
use windmill_common::assets::{clear_asset_usage, insert_asset_usage, AssetUsageKind};
use windmill_common::error::Error;
use windmill_common::error::Result;
use windmill_common::flows::{FlowModule, FlowModuleValue, FlowNodeId};
use windmill_common::jobs::JobPayload;
use windmill_common::scripts::ScriptHash;
use windmill_common::utils::WarnAfterExt;
#[cfg(feature = "python")]
use windmill_common::worker::PythonAnnotations;
use windmill_common::worker::{to_raw_value, to_raw_value_owned, write_file, Connection};
use windmill_common::workspace_dependencies::{
    RawWorkspaceDependencies, WorkspaceDependencies, WorkspaceDependenciesPrefetched,
};
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
    static ref DEPENDENCY_JOB_DEBOUNCE_DELAY: usize = std::env::var("DEPENDENCY_JOB_DEBOUNCE_DELAY").ok().and_then(|flag| flag.parse().ok()).unwrap_or(
        if cfg!(test) { /* if test we want increased debouncing delay */ 15 } else { 5 }
    );
}

use crate::common::{MaybeLock, OccupancyMetrics};
use crate::csharp_executor::generate_nuget_lockfile;

#[cfg(feature = "java")]
use crate::java_executor;

#[cfg(feature = "ruby")]
use crate::ruby_executor;

#[cfg(feature = "php")]
use crate::php_executor::{composer_install, parse_php_imports};
#[cfg(feature = "python")]
use crate::python_executor::{create_dependencies_dir, handle_python_reqs, uv_pip_compile};
#[cfg(feature = "rust")]
use crate::rust_executor::generate_cargo_lockfile;
use crate::{
    bun_executor::gen_bun_lockfile, deno_executor::generate_deno_lock,
    go_executor::install_go_dependencies,
};

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

pub fn extract_referenced_paths(
    raw_code: &str,
    script_path: &str,
    language: Option<ScriptLang>,
) -> Option<Vec<String>> {
    let mut referenced_paths = vec![];
    if let Some(wk_deps_refs) = language
        .and_then(|l| l.extract_workspace_dependencies_annotated_refs(raw_code, script_path))
        .map(|r| r.external)
    {
        let l = language.expect("should be some");
        for wk_deps_ref in wk_deps_refs {
            if let Some(path) = WorkspaceDependencies::to_path(&Some(wk_deps_ref), l).ok() {
                referenced_paths.push(path);
            };
        }
    } else if let (Some(l), true /* Only if it is not blacklisted */) = (
        language,
        WorkspaceDependenciesPrefetched::is_external_references_permitted(script_path),
    ) {
        // we assume all runnables without annotated dependencies reference default dependencies file.
        WorkspaceDependencies::to_path(&None, l)
            .ok()
            .inspect(|p| referenced_paths.push(p.to_owned()));
    }

    if let Some(relative_imports) = extract_relative_imports(raw_code, script_path, &language) {
        referenced_paths.extend(relative_imports);
    }

    if referenced_paths.is_empty() {
        None
    } else {
        Some(referenced_paths)
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
    raw_workspace_dependencies_o: Option<RawWorkspaceDependencies>,
) -> error::Result<Box<RawValue>> {
    // Processing a dependency job - these jobs handle lockfile generation and dependency updates
    // for scripts, flows, and apps when their dependencies or imported scripts change
    tracing::debug!(
        "Processing dependency job for path: {:?}",
        job.runnable_path()
    );
    let script_path = job.runnable_path();

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
        occupancy_metrics,
        &raw_workspace_dependencies_o,
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

            if let Err(e) = handle_deployment_metadata(
                &job.permissioned_as_email,
                &job.created_by,
                &db,
                &w_id,
                DeployedObject::Script {
                    hash: current_hash,
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
    _job_id: Option<Uuid>,
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
) -> error::Result<()> {
    // TODO: Should be moved into handle_dependency_job body to be more consistent with how flows and apps are handled
    {
        let mut tx = db.begin().await?;
        let mut dependency_map = ScopedDependencyMap::fetch_maybe_rearranged(
            &w_id,
            script_path,
            "script",
            &parent_path,
            db,
        )
        .await?;

        tx = dependency_map
            .patch(
                extract_referenced_paths(&code, script_path, *script_lang),
                // Ideally should be None, but due to current implementation will use empty string to represent None.
                "".into(),
                tx,
            )
            .await?;

        dependency_map.dissolve(tx).await.commit().await?;
    }

    {
        let mut already_visited = args
            .map(|x| {
                x.get("already_visited")
                    .map(|v| serde_json::from_str::<Vec<String>>(v.get()).ok())
                    .flatten()
            })
            .flatten()
            .unwrap_or_default();

        // TODO: There is a race-condition.
        // This can be old version.

        // Check lines of code below, you will find that we get the latest version of the script/app/flow

        // However the latest version does not necessarily mean that it is finalized.
        // Instead we assume that this would be the version we would base on.

        // So the script_importers might be behind. Thus some information like nodes_to_relock might be lost.
        let importers = crate::scoped_dependency_map::ScopedDependencyMap::get_dependents(
            script_path,
            w_id,
            db,
        )
        .await?;

        already_visited.push(script_path.to_string());
        // But currently we will do this extra db call for every script regardless of whether they have relative imports or not
        // Script might have no relative imports but still be referenced by someone else.
        match timeout(
            core::time::Duration::from_secs(60),
            Box::pin(trigger_dependents_to_recompute_dependencies(
                w_id,
                importers,
                deployment_message,
                parent_path,
                permissioned_as_email,
                created_by,
                permissioned_as,
                db,
                already_visited,
            )),
        )
        .warn_after_seconds(10)
        .await
        {
            Ok(Err(e)) => {
                tracing::error!(%e, "error triggering dependents to recompute dependencies")
            }
            Err(e) => {
                tracing::error!(%e, "triggering dependents to recompute dependencies has timed out")
            }
            _ => {}
        }
    }

    Ok(())
}

pub async fn trigger_dependents_to_recompute_dependencies(
    w_id: &str,
    importers: Vec<DependencyDependent>,
    // imported_path: &str,
    deployment_message: Option<String>,
    parent_path: Option<String>,
    email: &str,
    created_by: &str,
    permissioned_as: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    already_visited: Vec<String>,
) -> error::Result<()> {
    tracing::debug!(
        "Triggering dependents to recompute dependencies: {}",
        importers.iter().map(|dd| &dd.importer_path).join(",")
    );
    for DependencyDependent { importer_path, importer_kind, importer_node_ids } in importers.iter()
    {
        tracing::trace!("Processing dependency: {:?}", importer_path);
        if already_visited.contains(importer_path) {
            tracing::trace!("Skipping already visited dependency");
            continue;
        }

        let mut tx = db.clone().begin().await?;
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

        args.insert(
            "triggered_by_relative_import".to_string(),
            to_raw_value(&()),
        );

        // Lock the debounce_key entry FOR UPDATE to coordinate with the push side.
        // This prevents concurrent modifications during dependency job scheduling.
        //
        // The lock serves two purposes:
        // 1. Ensures we get the current debounce_job_id atomically
        // 2. Blocks new push requests from modifying this key until we commit
        // 3. Blocks puller from actually starting the job and gives us a chance to still squeeze the debounce in.
        //
        // After our transaction commits, any pending push/pull requests can proceed with
        // their debounce logic.
        let debounce_job_id_o =
            windmill_common::jobs::lock_debounce_key(w_id, &importer_path, &mut tx).await?;

        tracing::debug!(
            debounce_job_id = ?debounce_job_id_o,
            importer_path = %importer_path,
            "Retrieved debounce job ID (if exists)"
        );

        let job_payload = match importer_kind.as_str() {
            // TODO: Make it query only non-archived
            // Scripts
            "script" => match sqlx::query_scalar!(
                "SELECT hash FROM script WHERE path = $1 AND workspace_id = $2 AND deleted = false ORDER BY created_at DESC LIMIT 1",
                importer_path,
                w_id
            )
            .fetch_optional(&mut *tx)
            .await?
            {
                Some(hash) => {
                    tracing::debug!("newest hash for {} is: {hash}", importer_path);

                    let info =
                        windmill_common::get_script_info_for_hash(None, db, w_id, hash).await?;

                    JobPayload::Dependencies {
                        path: importer_path.clone(),
                        hash: ScriptHash(hash),
                        language: info.language,
                        dedicated_worker: info.dedicated_worker,
                    }
                }
                None => {
                    ScopedDependencyMap::clear_map_for_item(
                        importer_path,
                        w_id,
                        "script",
                        tx,
                        &None,
                    )
                    .await
                    .commit()
                    .await?;
                    continue;
                }
            },

            // Flows
            "flow" => match sqlx::query_scalar!(
                "SELECT id FROM flow_version WHERE path = $1 AND workspace_id = $2 ORDER BY created_at DESC LIMIT 1",
                importer_path,
                w_id
            )
            .fetch_optional(&mut *tx)
            .await?
            {
                Some(version) => {
                    tracing::debug!("Handling flow dependency update for: {}", importer_path);

                    args.insert(
                        "nodes_to_relock".to_string(),
                        to_raw_value(&importer_node_ids),
                    );

                    JobPayload::FlowDependencies {
                        path: importer_path.clone(),
                        version,
                        dedicated_worker: None,
                    }
                }
                None => {
                    ScopedDependencyMap::clear_map_for_item(importer_path, w_id, "flow", tx, &None)
                        .await
                        .commit()
                        .await?;
                    continue;
                }
            },

            // Apps
            "app" => match sqlx::query_scalar!(
                "SELECT id FROM app_version WHERE app_id = (SELECT id FROM app WHERE path = $1 AND workspace_id = $2) ORDER BY created_at DESC LIMIT 1",
                importer_path,
                w_id
            )
            .fetch_optional(&mut *tx)
            .await?
            {
                Some(version) => {
                    tracing::debug!("Handling app dependency update for: {}", importer_path);

                    args.insert(
                        "components_to_relock".to_string(),
                        // TODO: unsafe. Importer Node Ids are not checked. They can simply be array of empty strings!
                        to_raw_value(importer_node_ids),
                    );

                    JobPayload::AppDependencies { path: importer_path.clone(), version }
                }
                None => {
                    ScopedDependencyMap::clear_map_for_item(importer_path, w_id, "app", tx, &None)
                        .await
                        .commit()
                        .await?;
                    continue;
                }
            },

            _ => {
                tracing::error!(
                    "unexpected importer kind: {kind:?} for path {path}",
                    kind = importer_kind,
                    path = importer_path
                );
                continue;
            }
        };

        tracing::debug!("Pushing dependency job for: {}", importer_path);
        let (job_uuid, new_tx) = windmill_queue::push(
            db,
            PushIsolationLevel::Transaction(tx),
            &w_id,
            job_payload,
            windmill_queue::PushArgs { args: &args, extra: None },
            &created_by,
            email,
            permissioned_as.to_string(),
            Some("trigger.dependents.to.recompute.dependencies"),
            // Schedule for future for debouncing.
            Some(Utc::now() + Duration::seconds(*DEPENDENCY_JOB_DEBOUNCE_DELAY as i64)),
            None,
            None,
            None,
            None,
            None,
            false,
            false,
            None,
            true,
            Some("dependency".into()),
            None,
            None,
            None,
            None,
            false,
            None,
            debounce_job_id_o,
            None,
            None,
        )
        .await?;

        tracing::info!(
            "pushed dependency job due to common python path: {job_uuid} for path {path}",
            path = importer_path,
        );
        new_tx.commit().await?;
    }
    Ok(())
}

pub async fn handle_flow_dependency_job(
    job: MiniPulledJob,
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
    raw_workspace_dependencies_o: Option<RawWorkspaceDependencies>,
) -> error::Result<Box<serde_json::value::RawValue>> {
    tracing::debug!("Processing flow dependency job");
    tracing::trace!("Job details: {:?}", &job);
    tracing::trace!("Preview data: {:?}", &preview_data);
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

    let triggered_by_relative_import = job
        .args
        .as_ref()
        .map(|x| x.get("triggered_by_relative_import").is_some())
        .unwrap_or_default();

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

    tracing::trace!("Job details: {:?}", &job);
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

    tracing::debug!("Nodes to relock: {:?}", &nodes_to_relock);
    let raw_deps = job
        .args
        .as_ref()
        .map(|x| {
            x.get("raw_deps")
                .map(|v| serde_json::from_str::<HashMap<String, String>>(v.get()).ok())
                .flatten()
        })
        .flatten();

    // `JobKind::FlowDependencies` job store either:
    // - A saved flow version `id` in the `script_hash` column.
    // - Preview raw flow in the `queue` or `job` table.
    let (mut flow, notes) = match job.runnable_id {
        Some(ScriptHash(id)) => {
            let flow = cache::flow::fetch_version(db, id).await?;
            (flow.value().clone(), flow.notes())
        }
        _ => match preview_data {
            Some(RawData::Flow(data)) => (data.value().clone(), data.notes()),
            _ => return Err(Error::internal_err("expected script hash")),
        },
    };

    let mut tx = db.begin().await?;

    let mut dependency_map = ScopedDependencyMap::fetch_maybe_rearranged(
        &job.workspace_id,
        &job_path,
        "flow",
        &parent_path,
        &mut *tx,
    )
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
    (flow, tx, modified_ids, errors) = lock_flow_value(
        flow,
        &job,
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
        &raw_deps,
        &mut dependency_map,
        &raw_workspace_dependencies_o,
    )
    .await?;

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

    #[derive(Debug, Clone, Serialize)]
    struct FlowValueWithNotes<'a> {
        #[serde(flatten)]
        value: &'a FlowValue,

        #[serde(skip_serializing_if = "Option::is_none")]
        notes: Option<Box<RawValue>>, // TODO: Make this a Vec<FlowNote>
    }

    let new_flow_value = Json(
        serde_json::value::to_raw_value(&FlowValueWithNotes {
            value: &flow,
            notes: notes.and_then(|n| n.notes).map(|n| n.into()),
        })
        .map_err(to_anyhow)?,
    );

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
        // Drop tx and thus cancel any changes
        return Ok(to_raw_value_owned(json!({
            "status": "Flow lock generation was canceled",
        })));
    }

    if !skip_flow_update {
        let version = version.ok_or_else(|| {
            Error::internal_err("Flow Dependency requires script hash (flow version)".to_owned())
        })?;

        tx = dependency_map.dissolve(tx).await;

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
        let mut value_lite = flow.clone();

        tx = reduce_flow(
            tx,
            &mut value_lite.modules,
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
            tracing::debug!("Marked flow version as latest");
            tracing::debug!("Flow version: {}", version);
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

// Process entire FlowValue including failure_module and preprocessor_module
async fn lock_flow_value<'c>(
    mut flow: FlowValue,
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
    raw_deps: &Option<HashMap<String, String>>,
    dependency_map: &mut ScopedDependencyMap,
    raw_workspace_dependencies_o: &Option<RawWorkspaceDependencies>,
) -> Result<(
    FlowValue,
    sqlx::Transaction<'c, sqlx::Postgres>,
    Vec<String>,
    Vec<LockModuleError>,
)> {
    let mut all_modified_ids = Vec::new();
    let mut all_errors = Vec::new();

    // Process main modules
    let (updated_modules, updated_tx, modules_modified_ids, modules_errors) = lock_modules(
        flow.modules,
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
        &raw_deps,
        dependency_map,
        &raw_workspace_dependencies_o,
    )
    .await?;

    tx = updated_tx;
    flow.modules = updated_modules;
    all_modified_ids.extend(modules_modified_ids);
    all_errors.extend(modules_errors);

    // Process failure_module if it exists
    if let Some(failure_module) = flow.failure_module {
        let (updated_failure_modules, updated_tx, failure_modified_ids, failure_errors) =
            lock_modules(
                vec![*failure_module],
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
                &raw_deps,
                dependency_map,
                &raw_workspace_dependencies_o,
            )
            .await?;

        tx = updated_tx;
        all_modified_ids.extend(failure_modified_ids);
        all_errors.extend(failure_errors);

        flow.failure_module = updated_failure_modules.into_iter().next().map(Box::new);
    }

    // Process preprocessor_module if it exists
    if let Some(preprocessor_module) = flow.preprocessor_module {
        let (
            updated_preprocessor_modules,
            updated_tx,
            preprocessor_modified_ids,
            preprocessor_errors,
        ) = lock_modules(
            vec![*preprocessor_module],
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
            &raw_deps,
            dependency_map,
            raw_workspace_dependencies_o,
        )
        .await?;

        tx = updated_tx;
        all_modified_ids.extend(preprocessor_modified_ids);
        all_errors.extend(preprocessor_errors);

        flow.preprocessor_module = updated_preprocessor_modules
            .into_iter()
            .next()
            .map(Box::new);
    }

    Ok((flow, tx, all_modified_ids, all_errors))
}

// TODO: Maybe use [FlowValue::traverse_leafs]
// IMPORTANT: If updating this function, make sure you also update [FlowValue::traverse_leafs]
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
    raw_deps: &Option<HashMap<String, String>>,
    dependency_map: &mut ScopedDependencyMap, // (modules to replace old seq (even unmmodified ones), new transaction, modified ids) )
    raw_workspace_dependencies_o: &Option<RawWorkspaceDependencies>,
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
        let FlowModuleValue::RawScript {
            lock,
            path,
            content,
            mut language,
            input_transforms,
            tag,
            is_trigger,
            assets,
            concurrency_settings,
        } = e.get_value()?
        else {
            let mut nmodified_ids = Vec::new();
            let mut nerrors = Vec::new();
            match e.get_value()? {
                FlowModuleValue::ForloopFlow {
                    iterator,
                    modules,
                    modules_node,
                    skip_failures,
                    parallel,
                    parallelism,
                    squash,
                } => {
                    let nmodules;
                    (nmodules, tx, nmodified_ids, nerrors) = Box::pin(lock_modules(
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
                        &raw_deps,
                        dependency_map,
                        &raw_workspace_dependencies_o,
                    ))
                    .await?;
                    e.value = FlowModuleValue::ForloopFlow {
                        iterator,
                        modules: nmodules,
                        modules_node,
                        skip_failures,
                        parallel,
                        parallelism,
                        squash,
                    }
                    .into()
                }
                FlowModuleValue::BranchAll { branches, parallel } => {
                    let mut nbranches = vec![];
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
                            &raw_deps,
                            dependency_map,
                            raw_workspace_dependencies_o,
                        ))
                        .await?;
                        nmodified_ids.extend(inner_modified_ids);
                        errors.extend(inner_errors);
                        b.modules = nmodules;
                        nbranches.push(b)
                    }
                    e.value = FlowModuleValue::BranchAll { branches: nbranches, parallel }.into()
                }
                FlowModuleValue::WhileloopFlow { modules, modules_node, skip_failures, squash } => {
                    let nmodules;
                    (nmodules, tx, nmodified_ids, nerrors) = Box::pin(lock_modules(
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
                        &raw_deps,
                        dependency_map,
                        raw_workspace_dependencies_o,
                    ))
                    .await?;
                    e.value = FlowModuleValue::WhileloopFlow {
                        modules: nmodules,
                        modules_node,
                        skip_failures,
                        squash,
                    }
                    .into()
                }
                FlowModuleValue::BranchOne { branches, default, default_node } => {
                    let mut nbranches = vec![];
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
                            &raw_deps,
                            dependency_map,
                            raw_workspace_dependencies_o,
                        ))
                        .await?;
                        nmodified_ids.extend(inner_modified_ids);
                        errors.extend(inner_errors);
                        b.modules = nmodules;
                        nbranches.push(b)
                    }
                    let ndefault;
                    let ninner_errors;
                    let ninner_modified_ids;
                    (ndefault, tx, ninner_modified_ids, ninner_errors) = Box::pin(lock_modules(
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
                        &raw_deps,
                        dependency_map,
                        raw_workspace_dependencies_o,
                    ))
                    .await?;
                    errors.extend(ninner_errors);
                    nmodified_ids.extend(ninner_modified_ids);
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
                FlowModuleValue::AIAgent { input_transforms, mut tools } => {
                    // Extract FlowModules from tools and track their original indices
                    // MCP tools don't need locking, so we filter them out
                    let mut flow_modules = Vec::new();
                    let mut flow_module_indices = Vec::new();

                    for (idx, tool) in tools.iter().enumerate() {
                        if let Some(flow_module) = Option::<FlowModule>::from(tool) {
                            // Convert AgentTool -> FlowModule for locking
                            flow_modules.push(flow_module);
                            flow_module_indices.push(idx);
                        }
                    }

                    // Lock only the FlowModule-type tools
                    let locked_flow_modules;
                    (locked_flow_modules, tx, nmodified_ids, nerrors) = Box::pin(lock_modules(
                        flow_modules,
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
                        &raw_deps,
                        dependency_map,
                        raw_workspace_dependencies_o,
                    ))
                    .await?;

                    let mut locked_iter = locked_flow_modules.into_iter();
                    for idx in flow_module_indices {
                        let locked = locked_iter.next().ok_or_else(|| {
                            Error::internal_err("locked tool module should exist".to_string())
                        })?;
                        tools[idx] = locked.into();
                    }

                    e.value = FlowModuleValue::AIAgent { input_transforms, tools }.into();
                }
                _ => (),
            };
            modified_ids.extend(nmodified_ids);
            errors.extend(nerrors);
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

        let get_references = || {
            let dep_path = path.clone().unwrap_or_else(|| job_path.to_string());
            extract_referenced_paths(&content, &format!("{dep_path}/flow"), Some(language))
        };

        if let Some(locks_to_reload) = locks_to_reload {
            if !locks_to_reload.contains(&e.id) {
                tx = dependency_map
                    .patch(get_references(), e.id.clone(), tx)
                    .await?;
                new_flow_modules.push(e);
                continue;
            }
        } else {
            if lock.as_ref().is_some_and(|x| !x.trim().is_empty()) {
                let skip_creating_new_lock = skip_creating_new_lock(&language, &content);
                if skip_creating_new_lock {
                    tx = dependency_map
                        .patch(get_references(), e.id.clone(), tx)
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
        let (content_for_capture, raw_deps) = raw_deps
            .as_ref()
            .and_then(|llfs| llfs.get(language.as_str()))
            .map(|lock| (lock.to_owned(), true))
            .unwrap_or((content.clone(), false));

        let new_lock = capture_dependency_job(
            &job.id,
            &language,
            &content_for_capture,
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
            occupancy_metrics,
            raw_workspace_dependencies_o,
        )
        .await;
        //
        let lock = match new_lock {
            Ok(new_lock) => {
                if !raw_deps && !skip_flow_update {
                    let relative_imports = get_references();
                    tx = dependency_map
                        .patch(relative_imports.clone(), e.id.clone(), tx)
                        .await?;
                }
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
                errors.push(LockModuleError { id: e.id.clone(), error });
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
            is_trigger,
            assets,
            concurrency_settings,
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
) -> Result<sqlx::Transaction<'c, sqlx::Postgres>> {
    tx = Box::pin(reduce_flow(
        tx,
        modules,
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
    path: &str,
    workspace_id: &str,
    failure_module: Option<&Box<FlowModule>>,
    same_worker: bool,
) -> Result<sqlx::Transaction<'c, sqlx::Postgres>> {
    use FlowModuleValue::*;
    for module in &mut *modules {
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
                    is_trigger,
                    assets,
                    concurrency_settings,
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

                val = FlowScript {
                    input_transforms,
                    id,
                    tag,
                    language,
                    is_trigger,
                    assets,
                    concurrency_settings,
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
// TODO: Use abstracted traverse function.
//
// IMPORTANT: If updating this function, make sure you also update [traverse_app_inline_scripts]
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
    raw_workspace_dependencies_o: &Option<RawWorkspaceDependencies>,
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

                            let referenced_paths = extract_referenced_paths(
                                &content,
                                &format!("{job_path}/app"),
                                Some(language),
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
                                        .patch(
                                            referenced_paths.clone(),
                                            container_id.unwrap_or_default(),
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
                                        .patch(
                                            referenced_paths.clone(),
                                            container_id.unwrap_or_default(),
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
                                occupancy_metrics,
                                // TODO:
                                &None,
                            )
                            .await;
                            match new_lock {
                                Ok(new_lock) => {
                                    append_logs(&job.id, &job.workspace_id, logs, &db.into()).await;

                                    dependency_map
                                        .patch(
                                            referenced_paths.clone(),
                                            container_id.unwrap_or_default(),
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
                        raw_workspace_dependencies_o,
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
                        raw_workspace_dependencies_o,
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
    job: MiniPulledJob,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job_dir: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    worker_name: &str,
    worker_dir: &str,
    base_internal_url: &str,
    token: &str,
    occupancy_metrics: &mut OccupancyMetrics,
    raw_workspace_dependencies_o: Option<RawWorkspaceDependencies>,
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

    let (_, parent_path) = get_deployment_msg_and_parent_path_from_args(job.args.clone());

    let mut dependency_map = ScopedDependencyMap::fetch_maybe_rearranged(
        &job.workspace_id,
        &job_path,
        "app",
        &parent_path,
        db,
    )
    .await?;

    // TODO: Use transaction for entire segment?
    if let Some((app_id, value)) = record {
        let value = lock_modules_app(
            value,
            &job,
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
            &raw_workspace_dependencies_o,
        )
        .await?;

        // TODO: Dissolve in the end?
        dependency_map
            .dissolve(db.begin().await?)
            .await
            .commit()
            .await?;

        // Compute a lite version of the app value (w/ `inlineScript.{lock,code}`).
        let mut value_lite = value.clone();
        reduce_app(db, &job_path, &mut value_lite, app_id).await?;
        if let Value::Object(object) = &mut value_lite {
            object.insert("version".to_string(), json!(id));
            object.remove("files");
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
    use windmill_common::worker::{split_python_requirements, PyVAlias};

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
            split_python_requirements(req),
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
            PyVAlias::default().into(),
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
    occupancy_metrics: &mut OccupancyMetrics,
    raw_workspace_dependencies_o: &Option<RawWorkspaceDependencies>,
) -> error::Result<String> {
    let workspace_dependencies = WorkspaceDependenciesPrefetched::extract(
        job_raw_code,
        *job_language,
        w_id,
        raw_workspace_dependencies_o,
        script_path,
        db.into(),
    )
    .await?;

    let lock = match job_language {
        ScriptLang::Python3 => {
            #[cfg(not(feature = "python"))]
            return Err(Error::internal_err(
                "Python requires the python feature to be enabled".to_string(),
            ));
            #[cfg(feature = "python")]
            {
                let annotations = PythonAnnotations::parse(job_raw_code);

                let (pyv, reqs) = {
                    let (mut version_specifiers, mut locked_v) = (vec![], None);
                    let reqs = windmill_parser_py_imports::parse_python_imports(
                        job_raw_code,
                        &w_id,
                        script_path,
                        &db,
                        &mut version_specifiers,
                        &mut locked_v,
                        raw_workspace_dependencies_o,
                    )
                    .await?
                    .0
                    .join("\n");

                    // Resolve python version
                    // It is based on version specifiers
                    let pyv = if let Some(v) = locked_v {
                        v.into()
                    } else {
                        crate::PyV::resolve(
                            version_specifiers,
                            job_id,
                            w_id,
                            annotations.py_select_latest,
                            Some(db.clone().into()),
                            None,
                            None,
                        )
                        .await?
                    };

                    (pyv, reqs)
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
                    pyv,
                    annotations,
                )
                .await?
            }
        }
        ScriptLang::Ansible => {
            #[cfg(not(feature = "python"))]
            return Err(Error::internal_err(
                "Ansible requires the python feature to be enabled".to_string(),
            ));

            #[cfg(feature = "python")]
            {
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
                .await?
            }
        }
        ScriptLang::Go => {
            install_go_dependencies(
                job_id,
                job_raw_code,
                MaybeLock::Unresolved { workspace_dependencies: workspace_dependencies.clone() },
                mem_peak,
                canceled_by,
                job_dir,
                &db.into(),
                false,
                false,
                false,
                worker_name,
                w_id,
                occupancy_metrics,
            )
            .await?
        }
        ScriptLang::Deno => {
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
            .await?
        }
        ScriptLang::Bun | ScriptLang::Bunnative => {
            let wd_exist = workspace_dependencies.get_bun()?.is_some();
            // TODO: move inside gen_bun_lockfile
            if !wd_exist {
                write_file(job_dir, "main.ts", job_raw_code)?;
            }
            if let Some(lock) = gen_bun_lockfile(
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
                &workspace_dependencies,
                windmill_common::worker::TypeScriptAnnotations::parse(job_raw_code).npm,
                &mut Some(occupancy_metrics),
            )
            .await?
            {
                if !wd_exist {
                    crate::bun_executor::prebundle_bun_script(
                        job_raw_code,
                        &lock,
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

                lock
            } else {
                Default::default()
            }
        }
        ScriptLang::Php => {
            #[cfg(not(feature = "php"))]
            return Err(Error::internal_err(
                "PHP requires the php feature to be enabled".to_string(),
            ));

            #[cfg(feature = "php")]
            {
                let composer_content = if let Some(c) = workspace_dependencies.get_php()? {
                    c
                } else {
                    match parse_php_imports(job_raw_code)? {
                        Some(reqs) => reqs,
                        None => return Ok("".to_string()),
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
                    composer_content,
                    None,
                    occupancy_metrics,
                )
                .await?
            }
        }
        ScriptLang::Rust => {
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
            lockfile
        }
        ScriptLang::CSharp => {
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
            .await?
        }
        #[cfg(feature = "java")]
        ScriptLang::Java => {
            java_executor::resolve(
                job_id,
                job_raw_code,
                job_dir,
                &Connection::Sql(db.clone()),
                w_id,
            )
            .await?
        }
        #[cfg(feature = "ruby")]
        ScriptLang::Ruby => {
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
            .await?
        }
        // for related places search: ADD_NEW_LANG
        _ => "".to_owned(),
    };
    {
        let mut lines = vec![];
        add_lock_header(&mut lines, workspace_dependencies, *job_language, w_id, db).await?;
        Ok(if lines.is_empty() {
            lock
        } else {
            format!("{}\n{lock}", lines.join("\n"))
        })
    }
}

async fn add_lock_header(
    lines: &mut Vec<String>,
    wd: WorkspaceDependenciesPrefetched,
    _language: ScriptLang,
    _workspace_id: &str,
    _db: &sqlx::Pool<sqlx::Postgres>,
) -> error::Result<()> {
    if let Some(header) = wd.to_lock_header().await {
        lines.push(header);
    }

    Ok(())
}
