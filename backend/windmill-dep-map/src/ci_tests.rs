use std::collections::HashMap;

use serde_json::value::RawValue;
use uuid::Uuid;
use windmill_common::error;
use windmill_common::jobs::{JobPayload, JobTriggerKind};
use windmill_common::runnable_settings::DebouncingSettings;
use windmill_common::scripts::ScriptHash;
use windmill_common::triggers::TriggerMetadata;
use windmill_common::users::username_to_permissioned_as;
use windmill_queue::PushIsolationLevel;

struct CiTestRef {
    test_script_path: String,
}

pub async fn trigger_ci_tests_for_item(
    db: &sqlx::Pool<sqlx::Postgres>,
    w_id: &str,
    item_path: &str,
    item_kind: &str,
    email: &str,
    username: &str,
) -> error::Result<Vec<Uuid>> {
    let test_refs = sqlx::query_as!(
        CiTestRef,
        "SELECT test_script_path FROM ci_test_reference \
         WHERE workspace_id = $1 AND tested_item_path = $2 AND tested_item_kind = $3",
        w_id,
        item_path,
        item_kind
    )
    .fetch_all(db)
    .await?;

    if test_refs.is_empty() {
        return Ok(vec![]);
    }

    tracing::info!(
        "Triggering {} CI test(s) for {item_kind}:{item_path} in workspace {w_id}",
        test_refs.len()
    );

    let mut job_ids = Vec::new();
    let empty_args: HashMap<String, Box<RawValue>> = HashMap::new();

    for test_ref in test_refs {
        let latest_hash = sqlx::query_scalar!(
            "SELECT hash FROM script WHERE path = $1 AND workspace_id = $2 \
             AND deleted = false AND archived = false ORDER BY created_at DESC LIMIT 1",
            &test_ref.test_script_path,
            w_id
        )
        .fetch_optional(db)
        .await?;

        let Some(hash) = latest_hash else {
            tracing::warn!(
                "CI test script {} not found, skipping",
                test_ref.test_script_path
            );
            continue;
        };

        let info = windmill_common::get_script_info_for_hash(None, db, w_id, hash).await?;

        let tx = db.begin().await?;
        let (job_id, new_tx) = windmill_queue::push(
            db,
            PushIsolationLevel::Transaction(tx),
            w_id,
            JobPayload::ScriptHash {
                hash: ScriptHash(hash),
                path: test_ref.test_script_path.clone(),
                cache_ttl: None,
                cache_ignore_s3_path: None,
                dedicated_worker: info.dedicated_worker,
                language: info.language,
                priority: info.priority,
                apply_preprocessor: false,
                concurrency_settings: Default::default(),
                debouncing_settings: DebouncingSettings {
                    debounce_key: Some(format!("{w_id}:{}:ci_test", test_ref.test_script_path)),
                    debounce_delay_s: Some(5),
                    ..Default::default()
                },
                labels: None,
            },
            windmill_queue::PushArgs::from(&empty_args),
            username,
            email,
            username_to_permissioned_as(username),
            Some("ci_test_trigger"),
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
            info.tag,
            info.timeout,
            None,
            None,
            None,
            false,
            None,
            Some(TriggerMetadata::new(
                Some(item_path.to_string()),
                JobTriggerKind::CiTest,
            )),
            None,
        )
        .await?;

        new_tx.commit().await?;

        tracing::info!(
            "Pushed CI test job {job_id} for test script {} (triggered by {item_kind}:{item_path})",
            test_ref.test_script_path
        );
        job_ids.push(job_id);
    }

    Ok(job_ids)
}
