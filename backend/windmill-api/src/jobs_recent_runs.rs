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

use crate::{db::ApiAuthed, users::get_scope_tags};

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
    /// When the run started, or was created for one that has not reported a start.
    ran_at: DateTime<Utc>,
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

    // A run counts when it completed in the window or is running now, whenever it was queued:
    // a schedule is queued a full period ahead of its run. An agent's run is filed under the
    // agent's path as a raw flow; an editor's flow preview also carries a path, so only a raw
    // flow whose path names a saved agent counts. A `singlestepflow` wraps a script or a flow.
    let scope_tags =
        get_scope_tags(&authed).map(|v| v.into_iter().map(str::to_string).collect::<Vec<_>>());
    let runs = sqlx::query_as::<_, RecentRun>(
        "WITH ran AS (
            SELECT c.id, c.status::text AS status, c.started_at, c.duration_ms::bigint AS duration_ms
            FROM v2_job_completed c
            WHERE c.workspace_id = $1 AND c.completed_at > now() - make_interval(hours => $2)
            UNION ALL
            SELECT q.id, NULL, q.started_at, NULL
            FROM v2_job_queue q
            WHERE q.workspace_id = $1 AND q.running
              AND NOT EXISTS (SELECT 1 FROM v2_job_completed c WHERE c.id = q.id)
         )
         SELECT j.id, j.runnable_path AS path,
            CASE WHEN j.kind = 'flow' THEN 'flow'
                 WHEN j.kind = 'flowpreview' THEN 'agent'
                 WHEN j.kind = 'singlestepflow' AND COALESCE(
                        (SELECT m->'value'->>'type'
                            FROM jsonb_array_elements(j.raw_flow->'modules') m
                            WHERE m->>'id' IN ('a', 'main')
                            LIMIT 1),
                        'script') = 'flow' THEN 'flow'
                 ELSE 'script' END AS kind,
            CASE WHEN r.status IS NULL THEN 'running'
                 WHEN r.status IN ('success', 'skipped') THEN 'success'
                 WHEN r.status = 'canceled' THEN 'canceled'
                 ELSE 'failure' END AS status,
            COALESCE(r.started_at, j.created_at) AS ran_at,
            r.duration_ms
         FROM ran r
         JOIN v2_job j ON j.id = r.id
         WHERE j.workspace_id = $1
           AND j.parent_job IS NULL
           AND j.runnable_path IS NOT NULL
           AND ($3::text IS NULL OR starts_with(j.runnable_path, $3))
           AND ($5::text[] IS NULL OR j.tag = ANY($5))
           AND (j.kind IN ('script', 'singlestepflow', 'flow')
                OR (j.kind = 'flowpreview' AND EXISTS (
                    SELECT 1 FROM resource res
                    WHERE res.workspace_id = j.workspace_id AND res.path = j.runnable_path
                      AND res.resource_type = 'ai_agent')))
         ORDER BY ran_at DESC
         LIMIT $4",
    )
    .bind(&w_id)
    .bind(WINDOW_HOURS)
    .bind(&folder)
    .bind(SCAN_LIMIT)
    .bind(&scope_tags)
    .fetch_all(&mut *tx)
    .await?;

    let capped = runs.len() as i64 == SCAN_LIMIT;
    let total_runs = runs.len() as i64;
    let failed_runs = runs.iter().filter(|r| r.status == "failure").count() as i64;

    // A script and a flow can share a path: they are two items.
    let mut items: Vec<RecentRunItem> = vec![];
    let mut index: HashMap<(String, String), usize> = HashMap::new();
    for run in runs {
        match index.get(&(run.kind.clone(), run.path.clone())) {
            Some(&i) => {
                if items[i].runs.len() < RUNS_PER_ITEM {
                    items[i].runs.push(run);
                }
            }
            None => {
                index.insert((run.kind.clone(), run.path.clone()), items.len());
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

    let kinds: Vec<String> = items.iter().map(|i| i.kind.clone()).collect();
    let paths: Vec<String> = items.iter().map(|i| i.path.clone()).collect();
    let summaries: HashMap<(String, String), String> =
        sqlx::query_as::<_, (String, String, Option<String>)>(
            "SELECT p.kind, p.path, CASE p.kind
                WHEN 'script' THEN (SELECT NULLIF(s.summary, '') FROM script s
                    WHERE s.workspace_id = $1 AND s.path = p.path AND NOT s.archived
                    ORDER BY s.created_at DESC LIMIT 1)
                WHEN 'flow' THEN (SELECT NULLIF(f.summary, '') FROM flow f
                    WHERE f.workspace_id = $1 AND f.path = p.path)
                ELSE (SELECT NULLIF(r.description, '') FROM resource r
                    WHERE r.workspace_id = $1 AND r.path = p.path AND r.resource_type = 'ai_agent')
             END
             FROM unnest($2::text[], $3::text[]) AS p(kind, path)",
        )
        .bind(&w_id)
        .bind(&kinds)
        .bind(&paths)
        .fetch_all(&mut *tx)
        .await?
        .into_iter()
        .filter_map(|(k, p, s)| s.map(|s| ((k, p), s)))
        .collect();
    tx.commit().await?;

    for item in items.iter_mut() {
        item.summary = summaries
            .get(&(item.kind.clone(), item.path.clone()))
            .cloned();
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
