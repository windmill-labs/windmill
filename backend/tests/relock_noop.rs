// These pin language-independent behaviour, and Python is the cheapest runtime that still
// generates a real lock out of relative imports, so the whole file needs that feature.
#![cfg(feature = "python")]

use sqlx::{Pool, Postgres};
use tokio_stream::StreamExt;
use windmill_api_client::types::NewScript;
use windmill_common::scripts::{deploy_relocked_version, fetch_script_for_update};
use windmill_test_utils::*;

const W: &str = "test-workspace";

/// Budget for one wait below, counted completions and drain together. Even against a cold cache
/// these settle in a few seconds, so it only bounds a step that is stuck, and it stays under the
/// 60s cap `in_test_worker` puts on the whole body so the panic names what was being waited on
/// rather than surfacing as a worker timeout.
const WAIT_BUDGET: std::time::Duration = std::time::Duration::from_secs(30);

/// How often the waits below re-check. The drain reads the queue once per completion it waits
/// this long for, so it doubles as the floor on one turn of that loop.
const POLL: std::time::Duration = std::time::Duration::from_millis(20);

const A: &str = "def main():\n    return 'a'\n";
const A_COMMENTED: &str = "# same dependencies, different content\ndef main():\n    return 'a'\n";
const A_WITH_TINY: &str = "import tiny\n\ndef main():\n    return 'a'\n";
const B: &str = "from f.rel.a import main as a\n\ndef main():\n    return 'b' + a()\n";
const C: &str = "from f.rel.b import main as b\n\ndef main():\n    return 'c' + b()\n";

fn py_script(path: &str, content: &str, parent_hash: Option<String>) -> NewScript {
    NewScript {
        draft_only: None,
        content: content.into(),
        language: windmill_api_client::types::ScriptLang::Python3,
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

#[derive(sqlx::FromRow, Debug)]
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

/// `(path, status, logs)` of every dependency job created after `since`, in completion order.
async fn dependency_jobs_since(
    db: &Pool<Postgres>,
    since: chrono::DateTime<chrono::Utc>,
) -> Vec<(String, String, String)> {
    sqlx::query_as(
        "SELECT j.runnable_path, c.status::text, COALESCE(l.logs, '') FROM v2_job_completed c
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
    db: &Pool<Postgres>,
    completed: &mut (impl futures::Stream<Item = uuid::Uuid> + Unpin),
    count: usize,
) {
    let deadline = tokio::time::Instant::now() + WAIT_BUDGET;
    for i in 0..count {
        tokio::time::timeout_at(deadline, completed.next())
            .await
            .unwrap_or_else(|_| panic!("only {i} of {count} jobs completed"));
    }
    // Then let anything else that was queued run out, so a job the assertions say must not
    // exist would have shown up here. A dependency job queues its fan-out before it completes,
    // so an empty queue is a fixpoint rather than a lull.
    loop {
        while let Ok(Some(_)) = tokio::time::timeout(POLL, completed.next()).await {}
        let queued: i64 = sqlx::query_scalar("SELECT count(*) FROM v2_job_queue")
            .fetch_one(db)
            .await
            .unwrap();
        if queued == 0 {
            return;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "the queue never emptied"
        );
    }
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
                    .create_script(W, &py_script(path, content, None))
                    .await
                    .unwrap();
                wait_for_jobs(&db, &mut completed, 1).await;
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
                    &py_script("f/rel/a", A_COMMENTED, Some(format!("{a_hash:016x}"))),
                )
                .await
                .unwrap();
            wait_for_jobs(&db, &mut completed, 2).await;

            let jobs = dependency_jobs_since(&db, since).await;
            let paths: Vec<&str> = jobs.iter().map(|(p, _, _)| p.as_str()).collect();
            assert_eq!(
                paths,
                ["f/rel/a", "f/rel/b"],
                "the leaf's own job and one no-op relock of its importer, and nothing for c"
            );
            assert!(
                jobs[1]
                    .2
                    .contains("Lock unchanged: no new version deployed"),
                "b's relock should have found its lock unchanged: {}",
                jobs[1].2
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
                    &py_script("f/rel/a", A_WITH_TINY, Some(format!("{a_hash:016x}"))),
                )
                .await
                .unwrap();
            wait_for_jobs(&db, &mut completed, 3).await;

            let jobs = dependency_jobs_since(&db, since).await;
            let paths: Vec<&str> = jobs.iter().map(|(p, _, _)| p.as_str()).collect();
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
                    vs[1].lock.as_deref().unwrap_or("").contains("tiny"),
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

/// A relock that has to wait on its head's row lock, because a deploy of the same path holds
/// it, must find the version that deploy left and requeue itself for it rather than fail. The
/// blocked statement re-checks only the row it selected, which the deploy archived, and comes
/// back empty; the successor is only visible to a fresh read.
#[sqlx::test(fixtures("base"))]
async fn relock_waiting_on_a_deploy_requeues_for_its_successor(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    std::env::set_var("DEPENDENCY_JOB_DEBOUNCE_DELAY", "0");
    let (client, port, _s) = init_client(db.clone()).await;
    let mut completed = listen_for_completed_jobs(&db).await;

    in_test_worker(
        &db,
        async {
            for (path, content) in [("f/rel/a", A), ("f/rel/b", B)] {
                client
                    .create_script(W, &py_script(path, content, None))
                    .await
                    .unwrap();
                wait_for_jobs(&db, &mut completed, 1).await;
            }

            // A deploy of b that holds its head's row lock for as long as this transaction lives.
            let mut deploy = db.begin().await.unwrap();
            let head = fetch_script_for_update("f/rel/b", W, &mut *deploy)
                .await
                .unwrap()
                .unwrap();

            let since = chrono::Utc::now();
            let a_hash = live(&versions(&db, "f/rel/a").await).hash;
            client
                .create_script(
                    W,
                    &py_script("f/rel/a", A_COMMENTED, Some(format!("{a_hash:016x}"))),
                )
                .await
                .unwrap();

            // b's relock skips generation and reaches its commit, where it waits on the lock.
            let deadline = std::time::Instant::now() + WAIT_BUDGET;
            let mut waiting = false;
            while !waiting && std::time::Instant::now() < deadline {
                waiting = sqlx::query_scalar(
                    "SELECT EXISTS (SELECT 1 FROM pg_stat_activity
                     WHERE datname = current_database() AND wait_event_type = 'Lock'
                       AND query LIKE '%FROM script WHERE path = $1%FOR UPDATE%')",
                )
                .fetch_one(&db)
                .await
                .unwrap();
                if !waiting {
                    tokio::time::sleep(POLL).await;
                }
            }
            assert!(waiting, "b's relock never reached the row lock");

            // The deploy lands: the head is archived and a successor with its own lock takes
            // its place, while the relock is still waiting.
            let lock = head.lock.clone().unwrap();
            let successor =
                deploy_relocked_version(&mut deploy, head, None, Some(&lock), None, None)
                    .await
                    .unwrap();
            deploy.commit().await.unwrap();

            // a's own job, the relock that waited, and the relock it queued for the successor.
            wait_for_jobs(&db, &mut completed, 3).await;

            let jobs = dependency_jobs_since(&db, since).await;
            let paths: Vec<&str> = jobs.iter().map(|(p, _, _)| p.as_str()).collect();
            assert_eq!(paths, ["f/rel/a", "f/rel/b", "f/rel/b"], "{jobs:?}");
            assert!(
                jobs.iter().all(|(_, status, _)| status == "success"),
                "no relock may fail on the wait: {jobs:?}"
            );
            assert!(
                jobs[1]
                    .2
                    .contains("was deployed while this lock was generated"),
                "the waiting relock should have seen the successor: {}",
                jobs[1].2
            );
            assert!(
                jobs[2]
                    .2
                    .contains("Lock unchanged: no new version deployed"),
                "the requeued relock should find the successor's lock current: {}",
                jobs[2].2
            );
            let vs = versions(&db, "f/rel/b").await;
            assert_eq!(
                vs.len(),
                2,
                "the deploy's successor and nothing else: {vs:?}"
            );
            assert_eq!(live(&vs).hash, successor);
        },
        port,
    )
    .await;

    Ok(())
}

/// A multi-file importer: on a skipped relock each module gets its own last lock back, not the
/// parent script's, so an import's content-only redeploy leaves the importer alone as well.
#[sqlx::test(fixtures("base"))]
async fn multi_file_importer_relock_is_a_no_op_too(db: Pool<Postgres>) -> anyhow::Result<()> {
    std::env::set_var("DEPENDENCY_JOB_DEBOUNCE_DELAY", "0");
    let (client, port, _s) = init_client(db.clone()).await;
    let mut completed = listen_for_completed_jobs(&db).await;

    let py = |path: &str, content: &str, parent_hash: Option<String>, with_module: bool| {
        let mut ns = py_script(path, content, parent_hash);
        if with_module {
            ns.modules = Some(std::collections::HashMap::from([(
                "helper.py".to_string(),
                serde_json::json!({
                    "content": "def greet(x):\n    return 'hi ' + x\n",
                    "language": "python3"
                }),
            )]));
        }
        ns
    };
    async fn module_lock(db: &Pool<Postgres>) -> Option<String> {
        sqlx::query_scalar(
            "SELECT modules->'helper.py'->>'lock' FROM script
             WHERE workspace_id = $1 AND path = 'f/rel/pb' AND archived = false",
        )
        .bind(W)
        .fetch_one(db)
        .await
        .unwrap()
    }

    in_test_worker(
        &db,
        async {
            client
                .create_script(W, &py("f/rel/pa", "def main():\n    return 'a'\n", None, false))
                .await
                .unwrap();
            wait_for_jobs(&db, &mut completed, 1).await;
            client
                .create_script(
                    W,
                    &py(
                        "f/rel/pb",
                        "from f.rel.pa import main as a\nfrom .helper import greet\n\ndef main():\n    return greet(a())\n",
                        None,
                        true,
                    ),
                )
                .await
                .unwrap();
            wait_for_jobs(&db, &mut completed, 1).await;
            let lock_before = module_lock(&db).await;
            assert!(lock_before.is_some(), "the module got a lock of its own on deploy");

            let since = chrono::Utc::now();
            let pa_hash = live(&versions(&db, "f/rel/pa").await).hash;
            client
                .create_script(
                    W,
                    &py(
                        "f/rel/pa",
                        "# same dependencies\ndef main():\n    return 'a'\n",
                        Some(format!("{pa_hash:016x}")),
                        false,
                    ),
                )
                .await
                .unwrap();
            wait_for_jobs(&db, &mut completed, 2).await;

            let jobs = dependency_jobs_since(&db, since).await;
            let paths: Vec<&str> = jobs.iter().map(|(p, _, _)| p.as_str()).collect();
            assert_eq!(paths, ["f/rel/pa", "f/rel/pb"], "{jobs:?}");
            assert!(
                jobs[1].2.contains("Lock unchanged: no new version deployed"),
                "the multi-file importer's relock should be a no-op: {}",
                jobs[1].2
            );
            assert_eq!(versions(&db, "f/rel/pb").await.len(), 1);
            assert_eq!(module_lock(&db).await, lock_before, "the module keeps its own lock");
        },
        port,
    )
    .await;

    Ok(())
}
