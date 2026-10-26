/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! A runnable's run history listed by how each run started. Its runs inside other runs are
//! grouped by the top-level run that contains them.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{types::Uuid, FromRow, Postgres, Transaction};
use windmill_common::{
    error::{Error, Result},
    jobs::JobKind,
};

use crate::{singlestepflow_filter, Input, RunnableType};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStartedBy {
    Manual,
    Api,
    Schedule,
    App,
    Trigger,
    Preview,
    FlowStep,
    AgentTool,
    ScriptCall,
    Handler,
}

impl RunStartedBy {
    const ALL: [RunStartedBy; 10] = [
        Self::Manual,
        Self::Api,
        Self::Schedule,
        Self::App,
        Self::Trigger,
        Self::Preview,
        Self::FlowStep,
        Self::AgentTool,
        Self::ScriptCall,
        Self::Handler,
    ];

    fn as_str(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Api => "api",
            Self::Schedule => "schedule",
            Self::App => "app",
            Self::Trigger => "trigger",
            Self::Preview => "preview",
            Self::FlowStep => "flow_step",
            Self::AgentTool => "agent_tool",
            Self::ScriptCall => "script_call",
            Self::Handler => "handler",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.as_str() == s)
    }

    fn is_inside(self) -> bool {
        matches!(
            self,
            Self::FlowStep | Self::AgentTool | Self::ScriptCall | Self::Handler
        )
    }
}

pub(crate) fn parse_started_by(list: &str) -> Result<Vec<RunStartedBy>> {
    list.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| {
            RunStartedBy::parse(s)
                .ok_or_else(|| Error::BadRequest(format!("unknown started_by value: {s}")))
        })
        .collect()
}

/// The history fields of a run, flattened into [`Input`].
#[derive(Debug, Serialize, Deserialize)]
pub struct RunDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    started_by: Option<RunStartedBy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    trigger_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    trigger: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    running: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    suspended: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    flow_step_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    retry: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parent_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_run: Option<TopRun>,
    #[serde(skip_serializing_if = "Option::is_none")]
    group: Option<RunGroup>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TopRun {
    id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    success: Option<bool>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    running: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    trigger_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    trigger: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RunGroup {
    runs: i64,
    failed: i64,
    running: i64,
    retries: i64,
    handlers: i64,
    complete: bool,
}

pub(crate) struct HistoryQuery<'a> {
    pub w_id: &'a str,
    pub path: &'a str,
    pub runnable_type: &'a RunnableType,
    pub started_by: &'a [RunStartedBy],
    pub args: Option<serde_json::Value>,
}

/// Runs examined per side (top-level, inside) when a category of that side is unticked, and the
/// most the inside window grows to.
const CAP: i64 = 5000;
const MIN_INSIDE_WINDOW: i64 = 200;
const INSIDE_RUNS_LIMIT: i64 = 500;

const HANDLER_CREATED_BY: &str = "('error_handler', 'schedule_error_handler', \
    'schedule_recovery_handler', 'success_handler', 'global')";

/// Category of a run with no parent, from its own columns. `prefix` is the table alias with its
/// dot, or empty.
fn top_level_category(prefix: &str) -> String {
    format!(
        "CASE WHEN {prefix}kind IN ('preview', 'flowpreview') THEN 'preview' \
              WHEN {prefix}trigger_kind IS NULL OR {prefix}trigger_kind = 'ui' THEN 'manual' \
              WHEN {prefix}trigger_kind = 'webhook' THEN 'api' \
              WHEN {prefix}trigger_kind = 'schedule' THEN 'schedule' \
              WHEN {prefix}trigger_kind = 'app' THEN 'app' \
              ELSE 'trigger' END"
    )
}

/// A handler job has the job it handles as both parent and innermost root. The steps of a
/// handler flow and the runs a handler starts through the SDK inherit its `created_by`, but
/// not that shape.
fn is_handler(prefix: &str) -> String {
    format!(
        "({prefix}flow_step_id IS NULL AND {prefix}created_by IN {HANDLER_CREATED_BY} \
          AND {prefix}parent_job = {prefix}flow_innermost_root_job)"
    )
}

/// Category of a run with a parent as far as its own columns tell: NULL means it needs its
/// parent row (agent tool, script call, or a retry, which takes its first attempt's category).
fn own_category(prefix: &str) -> String {
    format!(
        "CASE WHEN {prefix}flow_step_id IS NOT NULL THEN 'flow_step' \
              WHEN {handler} THEN 'handler' END",
        handler = is_handler(prefix)
    )
}

/// Lateral subquery `x` over the run `s` (with `s.own_category`): its full category, retry
/// attempt, parent path, whether its first attempt is top-level, and whether it is hidden.
/// A run whose parent shares its path is a workflow-as-code task or the inner run of a
/// single-step-flow wrapper, both already listed through that parent. A parent that row-level
/// security hides from the viewer reads as NULL, and the run must stay listed.
fn classify_inside() -> String {
    format!(
        "CROSS JOIN LATERAL (\
            SELECT ra.attempt AS retry, p.runnable_path AS parent_path, \
                   p.id IS NOT NULL AND p.parent_job IS NULL AS parent_is_top, \
                   CASE WHEN s.own_category IS NOT NULL THEN s.own_category \
                        WHEN ra.attempt IS NOT NULL AND p.id IS NOT NULL AND p.parent_job IS NULL \
                             THEN {parent_top_level} \
                        WHEN ra.attempt IS NOT NULL AND p.id IS NOT NULL THEN \
                             CASE WHEN {parent_handler} THEN 'handler' \
                                  WHEN (SELECT pp.kind FROM v2_job pp \
                                        WHERE pp.id = p.parent_job AND pp.workspace_id = $2) = 'aiagent' \
                                       THEN 'agent_tool' \
                                  ELSE 'script_call' END \
                        WHEN p.kind = 'aiagent' THEN 'agent_tool' \
                        ELSE 'script_call' END AS started_by, \
                   ra.attempt IS NULL AND p.runnable_path IS NOT DISTINCT FROM $1 \
                       AND s.own_category IS DISTINCT FROM 'handler' AS hidden \
            FROM (SELECT 1) one \
            LEFT JOIN v2_job p ON p.id = s.parent_job AND p.workspace_id = $2 \
            LEFT JOIN native_retry_attempt ra ON ra.job_id = s.id AND s.own_category IS NULL \
            LIMIT 1\
        ) x",
        parent_top_level = top_level_category("p."),
        parent_handler = is_handler("p."),
    )
}

/// Lateral subquery `st` over the run `{alias}`: completion or, while it runs, queue state.
/// The LIMIT keeps it a per-row lookup, so the scan above it keeps its index order.
fn run_state(alias: &str) -> String {
    format!(
        "CROSS JOIN LATERAL (\
            SELECT c.status, c.completed_at, q.running IS TRUE AS running, \
                   COALESCE(q.suspend, 0) > 0 AS suspended, q.started_at \
            FROM (SELECT 1) one \
            LEFT JOIN v2_job_completed c ON c.id = {alias}.id \
            LEFT JOIN v2_job_queue q ON q.id = {alias}.id \
            LIMIT 1\
        ) st"
    )
}

/// Runs that completed (except skipped ones) or that a worker has started.
const STARTED: &str =
    "CASE WHEN st.status IS NOT NULL THEN st.status <> 'skipped' ELSE st.running END";

fn inside_kind(runnable_type: &RunnableType) -> &'static str {
    match runnable_type {
        RunnableType::FlowPath => "'flow'",
        _ => "'script'",
    }
}

/// A page entry, or the one row with no entry that carries the bounds of an empty page.
#[derive(FromRow)]
struct EntryRow {
    run_id: Option<Uuid>,
    top: Option<Uuid>,
    latest_created: Option<DateTime<Utc>>,
    created_by: Option<String>,
    started_by: Option<String>,
    is_preview: Option<bool>,
    trigger_kind: Option<String>,
    trigger: Option<String>,
    flow_step_id: Option<String>,
    retry: Option<i32>,
    parent_path: Option<String>,
    status: Option<String>,
    running: Option<bool>,
    suspended: Option<bool>,
    time: Option<DateTime<Utc>>,
    runs: Option<i64>,
    failed: Option<i64>,
    running_runs: Option<i64>,
    retries: Option<i64>,
    handlers: Option<i64>,
    top_created_at: Option<DateTime<Utc>>,
    root_retries: Option<i64>,
    root_retries_failed: Option<i64>,
    root_retries_running: Option<i64>,
    top_kind: Option<String>,
    top_path: Option<String>,
    top_trigger_kind: Option<String>,
    top_trigger: Option<String>,
    top_status: Option<String>,
    top_running: Option<bool>,
    window_bound: Option<DateTime<Utc>>,
    count_bound: Option<DateTime<Utc>>,
    above: i64,
}

/// One page of history entries, newest activity first. An entry is a top-level run of the
/// runnable (with a count of its own retries and of the runs of the runnable inside it), or the
/// runs of the runnable inside one other top-level run.
///
/// Runs come from two windows over `created_at`: the latest top-level runs, and the latest runs
/// inside other runs. A run inside another starts after it, so a group's count is exact when its
/// top-level run started after the oldest run the inside window examined.
pub(crate) async fn list(
    tx: &mut Transaction<'static, Postgres>,
    q: &HistoryQuery<'_>,
    per_page: usize,
    offset: usize,
) -> Result<Vec<Input>> {
    let needed = (offset + per_page) as i64;
    let mut inside_window = (10 * needed).clamp(MIN_INSIDE_WINDOW, CAP);
    let mut rows = list_page(tx, q, per_page, offset, inside_window).await?;
    // The page may hold entries older than the full inside window, where an older inner run
    // could still belong before them: grow the window once, in proportion to the shortfall.
    if let Some(first) = rows.first() {
        if first.window_bound.is_some() && first.above < needed && inside_window < CAP {
            inside_window = (inside_window * needed * 3 / (2 * first.above.max(1)))
                .max(2 * inside_window)
                .min(CAP);
            rows = list_page(tx, q, per_page, offset, inside_window).await?;
        }
    }
    Ok(rows
        .into_iter()
        .filter(|row| row.run_id.is_some() || row.top.is_some())
        .map(entry_to_input)
        .collect())
}

async fn list_page(
    tx: &mut Transaction<'static, Postgres>,
    q: &HistoryQuery<'_>,
    per_page: usize,
    offset: usize,
    inside_window: i64,
) -> Result<Vec<EntryRow>> {
    let needed = (offset + per_page) as i64;
    let root_categories: Vec<&str> = q
        .started_by
        .iter()
        .filter(|c| !c.is_inside())
        .map(|c| c.as_str())
        .collect();
    let inside_categories: Vec<&str> = q
        .started_by
        .iter()
        .filter(|c| c.is_inside())
        .map(|c| c.as_str())
        .collect();
    let include_preview = q.started_by.contains(&RunStartedBy::Preview);
    let root_kinds = match q.runnable_type {
        RunnableType::FlowPath if include_preview => {
            vec![JobKind::Flow, JobKind::SingleStepFlow, JobKind::FlowPreview]
        }
        RunnableType::FlowPath => vec![JobKind::Flow, JobKind::SingleStepFlow],
        _ if include_preview => vec![JobKind::Script, JobKind::SingleStepFlow, JobKind::Preview],
        _ => vec![JobKind::Script, JobKind::SingleStepFlow],
    };
    // Previews are left out through the kinds; any other unticked top-level category filters
    // rows, so the scan is capped like the inside one.
    let root_filtered = [
        RunStartedBy::Manual,
        RunStartedBy::Api,
        RunStartedBy::Schedule,
        RunStartedBy::App,
        RunStartedBy::Trigger,
    ]
    .iter()
    .any(|c| !q.started_by.contains(c));
    let (root_cap, root_filter) = if root_filtered {
        ("LIMIT $8", "AND j.started_by = ANY($4)")
    } else {
        ("", "")
    };
    // Only a failed script run can have native retries (windmill-queue
    // maybe_enqueue_native_script_retry), so every other run skips the walk over its children.
    let retry_gate = match q.runnable_type {
        RunnableType::FlowPath => "AND false",
        _ => "AND r.status = 'failure'",
    };

    // `kind = 'script'` (or 'flow') stays a literal: the planner can only use the partial index
    // ix_v2_job_child_by_path when it proves the index predicate from the query text.
    // root_runs adds the runs in progress that the window passed over, however old: a flow
    // suspended at an approval step or a hung run must stay in sight.
    let sql = format!(
        "WITH root_window AS (\
            SELECT j.id, j.created_at, j.created_by, j.kind, j.trigger_kind::text AS trigger_kind, \
                   j.trigger, j.started_by, st.status, st.completed_at, st.running, st.suspended, \
                   st.started_at \
            FROM (\
                SELECT id, created_at, created_by, kind, trigger_kind, trigger, \
                       {root_category} AS started_by \
                FROM v2_job \
                WHERE workspace_id = $2 AND runnable_path = $1 AND parent_job IS NULL \
                  AND kind = ANY($3) AND script_entrypoint_override IS NULL \
                  AND cardinality($4) > 0 {singlestepflow_filter} \
                  AND ($11::jsonb IS NULL OR args @> $11::jsonb) \
                ORDER BY created_at DESC {root_cap}\
            ) j \
            {root_state} \
            WHERE {STARTED} {root_filter} \
            ORDER BY j.created_at DESC \
            LIMIT $6\
        ), \
        root_runs AS (\
            SELECT id, created_at, created_by, kind, trigger_kind, trigger, started_by, status, \
                   completed_at, running, suspended, started_at \
            FROM root_window \
            UNION ALL \
            (SELECT j.id, j.created_at, j.created_by, j.kind, j.trigger_kind::text AS trigger_kind, \
                   j.trigger, j.started_by, st.status, st.completed_at, st.running, st.suspended, \
                   st.started_at \
            FROM (\
                SELECT q.id FROM v2_job_queue q \
                WHERE q.workspace_id = $2 AND q.running AND cardinality($4) > 0\
            ) running \
            CROSS JOIN LATERAL (\
                SELECT id, created_at, created_by, kind, trigger_kind, trigger, \
                       {root_category} AS started_by \
                FROM v2_job \
                WHERE id = running.id AND workspace_id = $2 AND runnable_path = $1 \
                  AND parent_job IS NULL AND kind = ANY($3) \
                  AND script_entrypoint_override IS NULL {singlestepflow_filter} \
                  AND ($11::jsonb IS NULL OR args @> $11::jsonb) \
                LIMIT 1\
            ) j \
            {root_state} \
            WHERE st.running {root_filter} \
              AND NOT EXISTS (SELECT 1 FROM root_window w WHERE w.id = j.id) \
            ORDER BY j.created_at DESC LIMIT 100)\
        ), \
        inside_runs AS (\
            SELECT s.id, s.created_at, s.created_by, s.trigger_kind, s.trigger, s.parent_job, \
                   s.root_job, s.flow_innermost_root_job, s.flow_step_id, st.status, \
                   st.completed_at, st.running, st.suspended, st.started_at, x.retry, \
                   x.parent_path, x.started_by \
            FROM (\
                SELECT id, created_at, created_by, trigger_kind, trigger, parent_job, root_job, \
                       flow_innermost_root_job, flow_step_id, script_entrypoint_override, args, \
                       {own_category} AS own_category \
                FROM v2_job \
                WHERE workspace_id = $2 AND runnable_path = $1 AND parent_job IS NOT NULL \
                  AND kind = {inside_kind} AND cardinality($5) > 0 \
                ORDER BY created_at DESC LIMIT $8\
            ) s \
            {classify} \
            {inside_state} \
            WHERE s.script_entrypoint_override IS NULL \
              AND ($11::jsonb IS NULL OR s.args @> $11::jsonb) \
              AND (s.own_category = ANY($5) \
                   OR (s.own_category IS NULL AND $5 && '{{agent_tool,script_call,handler}}')) \
              AND NOT x.hidden AND NOT (x.retry IS NOT NULL AND x.parent_is_top) \
              AND x.started_by = ANY($5) AND {STARTED} \
            ORDER BY s.created_at DESC \
            LIMIT $7\
        ), \
        firsts AS (\
            SELECT DISTINCT COALESCE(root_job, flow_innermost_root_job, parent_job) AS first \
            FROM inside_runs\
        ), \
        tops AS (\
            SELECT f.first, COALESCE(a4.id, a3.id, a2.id, a1.id, f.first) AS top, \
                   COALESCE(a4.created_at, a3.created_at, a2.created_at, a1.created_at) AS top_created_at \
            FROM firsts f \
            LEFT JOIN LATERAL (SELECT id, created_at, COALESCE(root_job, flow_innermost_root_job, parent_job) AS nxt \
                               FROM v2_job WHERE id = f.first AND workspace_id = $2) a1 ON true \
            LEFT JOIN LATERAL (SELECT id, created_at, COALESCE(root_job, flow_innermost_root_job, parent_job) AS nxt \
                               FROM v2_job WHERE id = a1.nxt AND workspace_id = $2) a2 ON true \
            LEFT JOIN LATERAL (SELECT id, created_at, COALESCE(root_job, flow_innermost_root_job, parent_job) AS nxt \
                               FROM v2_job WHERE id = a2.nxt AND workspace_id = $2) a3 ON true \
            LEFT JOIN LATERAL (SELECT id, created_at FROM v2_job WHERE id = a3.nxt AND workspace_id = $2) a4 ON true\
        ), \
        inside_keyed AS (\
            SELECT r.*, t.top, t.top_created_at \
            FROM inside_runs r \
            JOIN tops t ON t.first = COALESCE(r.root_job, r.flow_innermost_root_job, r.parent_job)\
        ), \
        groups AS (\
            SELECT top, min(top_created_at) AS top_created_at, count(*) AS runs, \
                   count(*) FILTER (WHERE status IN ('failure', 'canceled')) AS failed, \
                   count(*) FILTER (WHERE running) AS running, \
                   count(*) FILTER (WHERE retry IS NOT NULL) AS retries, \
                   count(*) FILTER (WHERE started_by = 'handler') AS handlers, \
                   max(COALESCE(completed_at, 'infinity')) AS latest, \
                   max(created_at) AS latest_created, \
                   (array_agg(id ORDER BY created_at DESC))[1] AS last_id \
            FROM inside_keyed GROUP BY top\
        ), \
        bounds AS (\
            SELECT window_bound, \
                   CASE WHEN window_bound IS NOT NULL THEN window_bound \
                        ELSE (SELECT created_at FROM v2_job \
                              WHERE workspace_id = $2 AND runnable_path = $1 AND parent_job IS NOT NULL \
                                AND kind IN ('script', 'flow') AND cardinality($5) > 0 \
                              ORDER BY created_at DESC OFFSET $8 - 1 LIMIT 1) END AS count_bound \
            FROM (SELECT CASE WHEN count(*) >= $7 THEN min(created_at) END AS window_bound \
                  FROM inside_runs) w\
        ), \
        entries AS (\
            SELECT r.id AS run_id, NULL::uuid AS top, \
                   GREATEST(COALESCE(r.completed_at, 'infinity'), g.latest) AS latest, \
                   GREATEST(r.created_at, g.latest_created) AS latest_created \
            FROM root_runs r LEFT JOIN groups g ON g.top = r.id \
            UNION ALL \
            SELECT g.last_id, g.top, g.latest, g.latest_created \
            FROM groups g WHERE NOT EXISTS (SELECT 1 FROM root_runs r WHERE r.id = g.top)\
        ), \
        page AS (\
            SELECT run_id, top, latest, latest_created FROM entries \
            ORDER BY latest DESC, latest_created DESC, COALESCE(top, run_id) \
            LIMIT $10 OFFSET $9\
        ) \
        SELECT e.run_id, e.top, e.latest_created, \
               (SELECT count(*) FROM entries a \
                WHERE b.window_bound IS NULL OR a.latest_created >= b.window_bound) AS above, \
               COALESCE(r.created_by, i.created_by) AS created_by, \
               COALESCE(r.started_by, i.started_by) AS started_by, \
               r.kind IN ('preview', 'flowpreview') AS is_preview, \
               COALESCE(r.trigger_kind, i.trigger_kind::text) AS trigger_kind, \
               COALESCE(r.trigger, i.trigger) AS trigger, i.flow_step_id, i.retry, \
               CASE WHEN i.started_by = 'script_call' THEN i.parent_path END AS parent_path, \
               COALESCE(r.status, i.status)::text AS status, \
               COALESCE(r.running, i.running) AS running, \
               COALESCE(r.suspended, i.suspended) AS suspended, \
               COALESCE(r.completed_at, i.completed_at, r.started_at, i.started_at, \
                        r.created_at, i.created_at) AS time, \
               g.runs, g.failed, g.running AS running_runs, g.retries, g.handlers, \
               COALESCE(g.top_created_at, r.created_at) AS top_created_at, \
               rt.n AS root_retries, rt.failed AS root_retries_failed, \
               rt.running AS root_retries_running, \
               t.kind::text AS top_kind, t.runnable_path AS top_path, \
               t.trigger_kind::text AS top_trigger_kind, t.trigger AS top_trigger, \
               tc.status::text AS top_status, tq.running IS TRUE AS top_running, \
               b.window_bound, b.count_bound \
        FROM bounds b \
        LEFT JOIN page e ON true \
        LEFT JOIN root_runs r ON e.top IS NULL AND r.id = e.run_id \
        LEFT JOIN inside_keyed i ON e.top IS NOT NULL AND i.id = e.run_id \
        LEFT JOIN groups g ON g.top = COALESCE(e.top, e.run_id) \
        LEFT JOIN v2_job t ON t.id = e.top AND t.workspace_id = $2 \
        LEFT JOIN v2_job_completed tc ON tc.id = t.id \
        LEFT JOIN v2_job_queue tq ON tq.id = t.id \
        LEFT JOIN LATERAL (\
            SELECT count(*) AS n, \
                   count(*) FILTER (WHERE xc.status IN ('failure', 'canceled')) AS failed, \
                   count(*) FILTER (WHERE xq.running) AS running \
            FROM v2_job x \
            JOIN native_retry_attempt n ON n.job_id = x.id \
            LEFT JOIN v2_job_completed xc ON xc.id = x.id \
            LEFT JOIN v2_job_queue xq ON xq.id = x.id \
            WHERE x.parent_job = e.run_id AND x.workspace_id = $2 {retry_gate} \
              AND CASE WHEN xc.status IS NOT NULL THEN xc.status <> 'skipped' \
                       ELSE xq.running IS TRUE END\
        ) rt ON e.top IS NULL \
        ORDER BY e.latest DESC, e.latest_created DESC, COALESCE(e.top, e.run_id)",
        root_category = top_level_category(""),
        singlestepflow_filter = singlestepflow_filter(q.runnable_type),
        root_state = run_state("j"),
        own_category = own_category(""),
        inside_kind = inside_kind(q.runnable_type),
        classify = classify_inside(),
        inside_state = run_state("s"),
    );

    let rows = sqlx::query_as::<_, EntryRow>(&sql)
        .bind(q.path)
        .bind(q.w_id)
        .bind(root_kinds)
        .bind(root_categories.as_slice())
        .bind(inside_categories.as_slice())
        .bind(needed)
        .bind(inside_window)
        .bind(CAP)
        .bind(offset as i64)
        .bind(per_page as i64)
        .bind(q.args.as_ref())
        .fetch_all(&mut **tx)
        .await?;
    Ok(rows)
}

fn entry_to_input(row: EntryRow) -> Input {
    let complete = match row.count_bound {
        None => true,
        Some(bound) => row.top_created_at.is_some_and(|t| t > bound),
    };
    let time = row.time.or(row.latest_created).unwrap_or_default();
    let created_by = row.created_by.unwrap_or_default();
    let success = row.status.as_deref() == Some("success");
    let run = |run_id: Uuid, top_run: Option<TopRun>, group: Option<RunGroup>| {
        Input::history_entry(
            run_id,
            time,
            created_by.clone(),
            success,
            row.is_preview.unwrap_or(false),
            RunDetails {
                started_by: row.started_by.as_deref().and_then(RunStartedBy::parse),
                trigger_kind: row.trigger_kind.clone(),
                trigger: row.trigger.clone(),
                running: row.running.unwrap_or(false),
                suspended: row.suspended.unwrap_or(false),
                flow_step_id: row.flow_step_id.clone(),
                retry: row.retry,
                parent_path: row.parent_path.clone(),
                top_run,
                group,
            },
        )
    };

    let Some(top) = row.top else {
        let retries = row.root_retries.unwrap_or(0);
        let inner = row.runs.unwrap_or(0) + retries;
        let group = (inner > 0).then(|| RunGroup {
            runs: inner,
            failed: row.failed.unwrap_or(0) + row.root_retries_failed.unwrap_or(0),
            running: row.running_runs.unwrap_or(0) + row.root_retries_running.unwrap_or(0),
            retries: row.retries.unwrap_or(0) + retries,
            handlers: row.handlers.unwrap_or(0),
            complete,
        });
        return run(row.run_id.unwrap_or_default(), None, group);
    };

    let top_run = TopRun {
        id: top,
        kind: row.top_kind.clone(),
        path: row.top_path.clone(),
        success: row.top_status.as_deref().map(|s| s == "success"),
        running: row.top_running.unwrap_or(false),
        trigger_kind: row.top_trigger_kind.clone(),
        trigger: row.top_trigger.clone(),
    };
    let runs = row.runs.unwrap_or(0);
    match row.run_id {
        Some(run_id) if runs == 1 && complete => run(run_id, Some(top_run), None),
        _ => {
            let group = RunGroup {
                runs,
                failed: row.failed.unwrap_or(0),
                running: row.running_runs.unwrap_or(0),
                retries: row.retries.unwrap_or(0),
                handlers: row.handlers.unwrap_or(0),
                complete,
            };
            Input::history_entry(
                top,
                time,
                created_by,
                group.failed == 0,
                false,
                RunDetails {
                    started_by: None,
                    trigger_kind: None,
                    trigger: None,
                    running: group.running > 0,
                    suspended: false,
                    flow_step_id: None,
                    retry: None,
                    parent_path: None,
                    top_run: Some(top_run),
                    group: Some(group),
                },
            )
        }
    }
}

#[derive(FromRow)]
struct InsideRow {
    id: Uuid,
    created_by: String,
    trigger_kind: Option<String>,
    trigger: Option<String>,
    flow_step_id: Option<String>,
    retry: Option<i32>,
    parent_path: Option<String>,
    started_by: String,
    status: Option<String>,
    running: bool,
    suspended: bool,
    time: DateTime<Utc>,
    args: sqlx::types::Json<Box<serde_json::value::RawValue>>,
}

/// The runs of the runnable inside the top-level run `top_run`, in execution order, with their
/// args (or `"WINDMILL_TOO_BIG"`), since an opened group shows them all at once.
pub(crate) async fn list_inside(
    tx: &mut Transaction<'static, Postgres>,
    q: &HistoryQuery<'_>,
    top_run: Uuid,
) -> Result<Vec<Input>> {
    let categories: Vec<&str> = q.started_by.iter().map(|c| c.as_str()).collect();
    let sql = format!(
        "WITH RECURSIVE inside(id, depth) AS (\
            SELECT id, 1 FROM v2_job WHERE parent_job = $4 AND workspace_id = $2 \
            UNION ALL \
            SELECT j.id, i.depth + 1 FROM v2_job j JOIN inside i ON j.parent_job = i.id \
            WHERE i.depth < 10 AND j.workspace_id = $2\
        ) \
        SELECT s.id, s.created_by, s.trigger_kind::text AS trigger_kind, s.trigger, \
               s.flow_step_id, x.retry, \
               CASE WHEN x.started_by = 'script_call' THEN x.parent_path END AS parent_path, \
               x.started_by, st.status::text AS status, st.running, st.suspended, \
               COALESCE(st.completed_at, st.started_at, s.created_at) AS time, \
               CASE WHEN pg_column_size(s.args) < 40000 THEN COALESCE(s.args, '{{}}'::jsonb) \
                    ELSE '\"WINDMILL_TOO_BIG\"'::jsonb END AS args \
        FROM inside i \
        JOIN (\
            SELECT id, created_at, created_by, kind, runnable_path, trigger_kind, trigger, \
                   parent_job, flow_innermost_root_job, flow_step_id, script_entrypoint_override, \
                   args, {own_category} AS own_category \
            FROM v2_job\
        ) s ON s.id = i.id \
        {classify} \
        {state} \
        WHERE s.runnable_path = $1 AND s.kind = {inside_kind} \
          AND s.script_entrypoint_override IS NULL \
          AND ($6::jsonb IS NULL OR s.args @> $6::jsonb) \
          AND NOT x.hidden AND x.started_by = ANY($3) AND {STARTED} \
        ORDER BY s.created_at \
        LIMIT $5",
        own_category = own_category(""),
        classify = classify_inside(),
        state = run_state("s"),
        inside_kind = inside_kind(q.runnable_type),
    );
    let rows = sqlx::query_as::<_, InsideRow>(&sql)
        .bind(q.path)
        .bind(q.w_id)
        .bind(categories.as_slice())
        .bind(top_run)
        .bind(INSIDE_RUNS_LIMIT)
        .bind(q.args.as_ref())
        .fetch_all(&mut **tx)
        .await?;
    Ok(rows
        .into_iter()
        .map(|row| {
            let mut input = Input::history_entry(
                row.id,
                row.time,
                row.created_by,
                row.status.as_deref() == Some("success"),
                false,
                RunDetails {
                    started_by: RunStartedBy::parse(&row.started_by),
                    trigger_kind: row.trigger_kind,
                    trigger: row.trigger,
                    running: row.running,
                    suspended: row.suspended,
                    flow_step_id: row.flow_step_id,
                    retry: row.retry,
                    parent_path: row.parent_path,
                    top_run: None,
                    group: None,
                },
            );
            input.args = row.args;
            input
        })
        .collect())
}
