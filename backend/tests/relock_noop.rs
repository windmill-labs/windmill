use sqlx::{Pool, Postgres};
use tokio_stream::StreamExt;
use windmill_api_client::types::NewScript;
use windmill_test_utils::*;

const W: &str = "test-workspace";

const A: &str = r#"export async function main() { return "a" }"#;
const A_COMMENTED: &str = r#"// same dependencies, different content
export async function main() { return "a" }"#;
const A_WITH_LODASH: &str = r#"import _ from "lodash@4.17.21";
export async function main() { return _.trim(" a ") }"#;
const B: &str = r#"import { main as a } from "/f/rel/a.ts";
export async function main() { return "b" + (await a()) }"#;
const C: &str = r#"import { main as b } from "/f/rel/b.ts";
export async function main() { return "c" + (await b()) }"#;

fn bun_script(path: &str, content: &str, parent_hash: Option<String>) -> NewScript {
    NewScript {
        draft_only: None,
        content: content.into(),
        language: windmill_api_client::types::ScriptLang::Bun,
        lock: None,
        parent_hash,
        path: path.into(),
        concurrent_limit: None,
        concurrency_time_window_s: None,
        cache_ttl: None,
        dedicated_worker: None,
        description: "".to_string(),
        envs: vec![],
        is_template: None,
        kind: None,
        summary: "".to_string(),
        tag: None,
        schema: std::collections::HashMap::new(),
        ws_error_handler_muted: Some(false),
        priority: None,
        delete_after_secs: None,
        timeout: None,
        restart_unless_cancelled: None,
        deployment_message: None,
        concurrency_key: None,
        visible_to_runner_only: None,
        auto_kind: None,
        codebase: None,
        has_preprocessor: None,
        on_behalf_of_email: None,
        assets: vec![],
        modules: None,
    }
}

#[derive(sqlx::FromRow)]
struct Version {
    hash: i64,
    archived: bool,
    lock: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
}

/// Every version of `path`, oldest first.
async fn versions(db: &Pool<Postgres>, path: &str) -> Vec<Version> {
    sqlx::query_as(
        "SELECT hash, archived, lock, created_at FROM script
         WHERE workspace_id = $1 AND path = $2 ORDER BY created_at",
    )
    .bind(W)
    .bind(path)
    .fetch_all(db)
    .await
    .unwrap()
}

fn live(versions: &[Version]) -> &Version {
    versions.iter().rev().find(|v| !v.archived).unwrap()
}

/// `(path, logs)` of every dependency job created after `since`, in completion order.
async fn dependency_jobs_since(
    db: &Pool<Postgres>,
    since: chrono::DateTime<chrono::Utc>,
) -> Vec<(String, String)> {
    sqlx::query_as(
        "SELECT j.runnable_path, COALESCE(l.logs, '') FROM v2_job_completed c
         JOIN v2_job j ON j.id = c.id
         LEFT JOIN job_logs l ON l.job_id = c.id
         WHERE j.kind = 'dependencies' AND j.created_at > $1
         ORDER BY c.started_at",
    )
    .bind(since)
    .fetch_all(db)
    .await
    .unwrap()
}

async fn wait_for_jobs(
    completed: &mut (impl futures::Stream<Item = uuid::Uuid> + Unpin),
    count: usize,
) {
    for _ in 0..count {
        completed.next().await;
    }
    // Then let anything else that was queued run out, so a job the assertions say must not
    // exist would have shown up here.
    while let Ok(Some(_)) =
        tokio::time::timeout(std::time::Duration::from_secs(2), completed.next()).await
    {}
}

/// A redeploy of an imported script whose dependencies did not move relocks its importer,
/// and that relock must deploy nothing: no new version, and no dependency job for the
/// importer's own importers. A redeploy that does change the dependencies still walks the
/// whole chain with a new version at each step.
#[sqlx::test(fixtures("base"))]
async fn relative_import_relock_deploys_only_when_the_lock_changed(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    std::env::set_var("DEPENDENCY_JOB_DEBOUNCE_DELAY", "0");
    let (client, port, _s) = init_client(db.clone()).await;
    let mut completed = listen_for_completed_jobs(&db).await;

    in_test_worker(
        &db,
        async {
            // One at a time: each deploy's dependency job records the importer's edges, and an
            // importer whose edges are recorded is what a later relock of it can skip on.
            for (path, content) in [("f/rel/a", A), ("f/rel/b", B), ("f/rel/c", C)] {
                client
                    .create_script(W, &bun_script(path, content, None))
                    .await
                    .unwrap();
                wait_for_jobs(&mut completed, 1).await;
            }
            let b_before = versions(&db, "f/rel/b").await;
            let c_before = versions(&db, "f/rel/c").await;
            assert_eq!(b_before.len(), 1);
            assert_eq!(c_before.len(), 1);

            // Content-only change on the leaf.
            let since = chrono::Utc::now();
            let a_hash = live(&versions(&db, "f/rel/a").await).hash;
            client
                .create_script(
                    W,
                    &bun_script("f/rel/a", A_COMMENTED, Some(format!("{a_hash:016x}"))),
                )
                .await
                .unwrap();
            wait_for_jobs(&mut completed, 2).await;

            let jobs = dependency_jobs_since(&db, since).await;
            let paths: Vec<&str> = jobs.iter().map(|(p, _)| p.as_str()).collect();
            assert_eq!(
                paths,
                ["f/rel/a", "f/rel/b"],
                "the leaf's own job and one no-op relock of its importer, and nothing for c"
            );
            assert!(
                jobs[1]
                    .1
                    .contains("Lock unchanged: no new version deployed"),
                "b's relock should have found its lock unchanged: {}",
                jobs[1].1
            );
            let b_after = versions(&db, "f/rel/b").await;
            let c_after = versions(&db, "f/rel/c").await;
            assert_eq!(
                b_after.len(),
                1,
                "an unchanged relock must not mint a version"
            );
            assert_eq!(live(&b_after).hash, live(&b_before).hash);
            assert_eq!(c_after.len(), 1);
            assert_eq!(live(&c_after).hash, live(&c_before).hash);

            // A dependency change on the leaf.
            let since = chrono::Utc::now();
            let a_hash = live(&versions(&db, "f/rel/a").await).hash;
            client
                .create_script(
                    W,
                    &bun_script("f/rel/a", A_WITH_LODASH, Some(format!("{a_hash:016x}"))),
                )
                .await
                .unwrap();
            wait_for_jobs(&mut completed, 3).await;

            let jobs = dependency_jobs_since(&db, since).await;
            let paths: Vec<&str> = jobs.iter().map(|(p, _)| p.as_str()).collect();
            assert_eq!(paths, ["f/rel/a", "f/rel/b", "f/rel/c"]);
            for path in ["f/rel/b", "f/rel/c"] {
                let vs = versions(&db, path).await;
                assert_eq!(
                    vs.len(),
                    2,
                    "{path}: a changed relock deploys a new version"
                );
                assert!(
                    vs[0].archived && !vs[1].archived,
                    "{path}: parent archived, child live"
                );
                assert!(vs[0].created_at < vs[1].created_at, "{path}: lineage order");
                assert!(
                    vs[1].lock.as_deref().unwrap_or("").contains("lodash"),
                    "{path}: the new version carries the new lock: {:?}",
                    vs[1].lock
                );
            }
        },
        port,
    )
    .await;

    Ok(())
}
