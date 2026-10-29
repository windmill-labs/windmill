//! What ran recently in a workspace, one entry per runnable: the workspace home's "Recently run"
//! panel. Statuses and counts are settled here so the page only displays them.

use std::collections::HashMap;

use axum::{
    extract::{Path, Query},
    Extension, Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use windmill_common::{db::UserDB, error::JsonResult};

use crate::db::ApiAuthed;

const WINDOW_HOURS: i32 = 24;
/// Newest root runs read from the window. A workspace running more than this in the window
/// gets counts over these runs only, and `capped` says so.
const SCAN_LIMIT: i64 = 5000;
const MAX_ITEMS: usize = 50;
const RUNS_PER_ITEM: usize = 10;

#[derive(Deserialize)]
pub(crate) struct RecentRunsQuery {
    folder: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct RecentRuns {
    window_hours: i32,
    capped: bool,
    total_runs: i64,
    failed_runs: i64,
    failing_items: i64,
    running_items: i64,
    items: Vec<RecentRunItem>,
}

#[derive(Serialize)]
pub(crate) struct RecentRunItem {
    path: String,
    kind: String,
    summary: Option<String>,
    /// The status of the item's newest run.
    status: String,
    /// Newest first.
    runs: Vec<RecentRun>,
}

#[derive(Serialize, sqlx::FromRow)]
pub(crate) struct RecentRun {
    id: uuid::Uuid,
    #[serde(skip)]
    path: String,
    #[serde(skip)]
    kind: String,
    status: String,
    created_at: DateTime<Utc>,
    started_at: Option<DateTime<Utc>>,
    duration_ms: Option<i64>,
}

pub(crate) async fn list_recent_runs(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(q): Query<RecentRunsQuery>,
) -> JsonResult<RecentRuns> {
    let folder = q
        .folder
        .filter(|f| !f.is_empty())
        .map(|f| format!("f/{f}/"));
    let mut tx = user_db.begin(&authed).await?;

    // An agent's run is filed under the agent's path as a raw flow; an editor's flow preview
    // also carries a path, so only a raw flow whose path names a saved agent counts. Queued
    // jobs that have not started (schedules pushed ahead of time included) did not run yet.
    let runs = sqlx::query_as::<_, RecentRun>(
        "SELECT j.id, j.runnable_path AS path,
            CASE WHEN j.kind = 'flow' THEN 'flow'
                 WHEN j.kind = 'flowpreview' THEN 'agent'
                 ELSE 'script' END AS kind,
            CASE WHEN c.id IS NULL THEN 'running'
                 WHEN c.status IN ('success', 'skipped') THEN 'success'
                 ELSE 'failure' END AS status,
            j.created_at,
            COALESCE(c.started_at, q.started_at) AS started_at,
            c.duration_ms::bigint AS duration_ms
         FROM v2_job j
         LEFT JOIN v2_job_completed c ON c.id = j.id
         LEFT JOIN v2_job_queue q ON q.id = j.id
         WHERE j.workspace_id = $1
           AND j.parent_job IS NULL
           AND j.runnable_path IS NOT NULL
           AND j.created_at > now() - make_interval(hours => $2)
           AND ($3::text IS NULL OR starts_with(j.runnable_path, $3))
           AND (c.id IS NOT NULL OR q.running)
           AND (j.kind IN ('script', 'singlestepflow', 'flow')
                OR (j.kind = 'flowpreview' AND EXISTS (
                    SELECT 1 FROM resource r
                    WHERE r.workspace_id = j.workspace_id AND r.path = j.runnable_path
                      AND r.resource_type = 'ai_agent')))
         ORDER BY j.created_at DESC
         LIMIT $4",
    )
    .bind(&w_id)
    .bind(WINDOW_HOURS)
    .bind(&folder)
    .bind(SCAN_LIMIT)
    .fetch_all(&mut *tx)
    .await?;

    let capped = runs.len() as i64 == SCAN_LIMIT;
    let total_runs = runs.len() as i64;
    let failed_runs = runs.iter().filter(|r| r.status == "failure").count() as i64;

    let mut items: Vec<RecentRunItem> = vec![];
    let mut index: HashMap<String, usize> = HashMap::new();
    for run in runs {
        match index.get(&run.path) {
            Some(&i) => {
                if items[i].runs.len() < RUNS_PER_ITEM {
                    items[i].runs.push(run);
                }
            }
            None => {
                index.insert(run.path.clone(), items.len());
                items.push(RecentRunItem {
                    path: run.path.clone(),
                    kind: run.kind.clone(),
                    summary: None,
                    status: run.status.clone(),
                    runs: vec![run],
                });
            }
        }
    }
    let failing_items = items.iter().filter(|i| i.status == "failure").count() as i64;
    let running_items = items.iter().filter(|i| i.status == "running").count() as i64;
    items.truncate(MAX_ITEMS);

    let paths: Vec<String> = items.iter().map(|i| i.path.clone()).collect();
    let summaries: HashMap<String, String> = sqlx::query_as::<_, (String, Option<String>)>(
        "SELECT p.path, COALESCE(
            (SELECT NULLIF(s.summary, '') FROM script s
             WHERE s.workspace_id = $1 AND s.path = p.path AND NOT s.archived
             ORDER BY s.created_at DESC LIMIT 1),
            (SELECT NULLIF(f.summary, '') FROM flow f
             WHERE f.workspace_id = $1 AND f.path = p.path),
            (SELECT NULLIF(r.description, '') FROM resource r
             WHERE r.workspace_id = $1 AND r.path = p.path AND r.resource_type = 'ai_agent'))
         FROM unnest($2::text[]) AS p(path)",
    )
    .bind(&w_id)
    .bind(&paths)
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .filter_map(|(p, s)| s.map(|s| (p, s)))
    .collect();
    tx.commit().await?;

    for item in items.iter_mut() {
        item.summary = summaries.get(&item.path).cloned();
    }

    Ok(Json(RecentRuns {
        window_hours: WINDOW_HOURS,
        capped,
        total_runs,
        failed_runs,
        failing_items,
        running_items,
        items,
    }))
}
