use std::collections::HashMap;

use chrono::{Duration, Utc};
use itertools::Itertools;
use serde_json::value::RawValue;
use windmill_common::error;
use windmill_common::jobs::JobPayload;
use windmill_common::runnable_settings::DebouncingSettings;
use windmill_common::scripts::ScriptHash;
use windmill_common::worker::to_raw_value;
use windmill_queue::PushIsolationLevel;

use crate::scoped_dependency_map::{DependencyDependent, ScopedDependencyMap};

lazy_static::lazy_static! {
    static ref DEPENDENCY_JOB_DEBOUNCE_DELAY: usize = std::env::var("DEPENDENCY_JOB_DEBOUNCE_DELAY").ok().and_then(|flag| flag.parse().ok()).unwrap_or(
        if cfg!(test) { 15 } else { 5 }
    );
}

pub async fn trigger_dependents_to_recompute_dependencies(
    w_id: &str,
    importers: Vec<DependencyDependent>,
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
            args.insert("common_dependency_path".to_string(), to_raw_value(&p_path));
        }

        args.insert(
            "already_visited".to_string(),
            to_raw_value(&already_visited),
        );

        args.insert(
            "triggered_by_relative_import".to_string(),
            to_raw_value(&true),
        );

        let mut debouncing_settings = DebouncingSettings {
            debounce_key: Some(format!("{w_id}:{importer_path}:dependency")),
            debounce_delay_s: Some(5),
            ..Default::default()
        };

        let job_payload = match importer_kind.as_str() {
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
                        debouncing_settings,
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

                    debouncing_settings.debounce_args_to_accumulate = Some(vec!["nodes_to_relock".into()]);

                    JobPayload::FlowDependencies {
                        path: importer_path.clone(),
                        version,
                        dedicated_worker: None,
                        debouncing_settings,
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
                        to_raw_value(importer_node_ids),
                    );

                    debouncing_settings.debounce_args_to_accumulate = Some(vec!["components_to_relock".into()]);

                    JobPayload::AppDependencies { path: importer_path.clone(), version, debouncing_settings }
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
