#[cfg(feature = "private")]
#[allow(unused)]
pub use crate::ci_tests_ee::*;

use std::collections::BTreeSet;

use uuid::Uuid;
use windmill_common::error;

use crate::scoped_dependency_map::ScopedDependencyMap;

/// `skip_ci_tests` on a deploy endpoint. A caller deploying several items in a row sets it
/// and queues the CI tests once the whole batch has landed (`trigger_ci_tests_for_batch`): a
/// test run between two of its deploys resolves imports against a half-deployed workspace.
#[derive(serde::Deserialize, Default, Clone, Copy)]
pub struct SkipCiTestsQuery {
    #[serde(default)]
    pub skip_ci_tests: bool,
}

#[derive(serde::Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CiTestBatchItem {
    pub kind: String,
    pub path: String,
}

#[derive(serde::Serialize, Default)]
pub struct CiTestBatchOutcome {
    /// Paths with a dependency job still queued or running. When non-empty and the caller
    /// did not ask to ignore it, nothing was queued.
    pub pending: Vec<String>,
    pub job_ids: Vec<Uuid>,
}

/// Queue the CI tests of a batch deployed with `skip_ci_tests`, once nothing those tests read
/// is still being locked. The relocks a deployed script causes in the scripts importing it
/// carry the flag too, so those importers' tests are part of the batch.
pub async fn trigger_ci_tests_for_batch(
    db: &sqlx::Pool<sqlx::Postgres>,
    w_id: &str,
    items: Vec<CiTestBatchItem>,
    ignore_pending: bool,
    email: &str,
    username: &str,
) -> error::Result<CiTestBatchOutcome> {
    let mut items: BTreeSet<CiTestBatchItem> = items
        .into_iter()
        .filter(|i| matches!(i.kind.as_str(), "script" | "flow" | "resource"))
        .collect();

    let mut frontier: Vec<String> = items
        .iter()
        .filter(|i| i.kind == "script")
        .map(|i| i.path.clone())
        .collect();
    while let Some(path) = frontier.pop() {
        for d in ScopedDependencyMap::get_dependents(&path, w_id, db).await? {
            if d.importer_kind == "script"
                && items.insert(CiTestBatchItem {
                    kind: "script".to_string(),
                    path: d.importer_path.clone(),
                })
            {
                frontier.push(d.importer_path);
            }
        }
    }

    let refs = sqlx::query!(
        "SELECT test_script_path, tested_item_path, tested_item_kind, has_wildcard AS \"has_wildcard!\" \
         FROM ci_test_reference WHERE workspace_id = $1",
        w_id
    )
    .fetch_all(db)
    .await?;
    // What a test run reads: the tested item, the test script and everything it imports.
    // Any of them still being locked would have the test run on its previous lock.
    let mut watched: BTreeSet<String> = BTreeSet::new();
    let mut visited: BTreeSet<String> = BTreeSet::new();
    let mut to_visit: Vec<String> = vec![];
    for r in refs {
        let mut tested = items
            .iter()
            .filter(|i| {
                i.kind == r.tested_item_kind
                    && if r.has_wildcard {
                        windmill_common::schema::ci_test_path_matches(&i.path, &r.tested_item_path)
                    } else {
                        i.path == r.tested_item_path
                    }
            })
            .peekable();
        if tested.peek().is_none() {
            continue;
        }
        watched.extend(tested.map(|i| i.path.clone()));
        if visited.insert(r.test_script_path.clone()) {
            to_visit.push(r.test_script_path);
        }
    }
    // Waiting on the batch's locks is only worth it when there is a test to run after them.
    if to_visit.is_empty() {
        return Ok(CiTestBatchOutcome::default());
    }
    while let Some(path) = to_visit.pop() {
        let imported = sqlx::query_scalar!(
            "SELECT DISTINCT imported_path FROM dependency_map \
             WHERE workspace_id = $1 AND importer_path = $2 AND importer_kind = 'script'",
            w_id,
            &path
        )
        .fetch_all(db)
        .await?;
        for p in imported {
            if visited.insert(p.clone()) {
                to_visit.push(p);
            }
        }
    }
    watched.extend(visited);

    let watched: Vec<String> = watched.into_iter().collect();
    let pending = sqlx::query_scalar!(
        "SELECT DISTINCT j.runnable_path AS \"path!\" FROM v2_job_queue q
         JOIN v2_job j ON j.id = q.id
         WHERE q.workspace_id = $1
           AND j.kind IN ('dependencies', 'flowdependencies')
           AND j.runnable_path = ANY($2)",
        w_id,
        &watched
    )
    .fetch_all(db)
    .await?;
    if !pending.is_empty() && !ignore_pending {
        return Ok(CiTestBatchOutcome { pending, job_ids: vec![] });
    }

    let mut job_ids = vec![];
    for item in items {
        job_ids.extend(
            trigger_ci_tests_for_item(db, w_id, &item.path, &item.kind, email, username).await?,
        );
    }
    Ok(CiTestBatchOutcome { pending, job_ids })
}

#[cfg(not(feature = "private"))]
pub async fn trigger_ci_tests_for_item(
    _db: &sqlx::Pool<sqlx::Postgres>,
    _w_id: &str,
    _item_path: &str,
    _item_kind: &str,
    _email: &str,
    _username: &str,
) -> error::Result<Vec<Uuid>> {
    Ok(vec![])
}

#[cfg(not(feature = "private"))]
pub async fn trigger_all_ci_tests(
    _db: &sqlx::Pool<sqlx::Postgres>,
    _w_id: &str,
    _email: &str,
    _username: &str,
) -> error::Result<Vec<Uuid>> {
    Ok(vec![])
}
