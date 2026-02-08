/*
 * Author: Windmill Labs, Inc
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Query builders for filtering job lists (queue and completed).

use sql_builder::prelude::*;
use sql_builder::SqlBuilder;
use windmill_common::utils::{paginate_without_limits, Pagination};

use crate::types::{ListCompletedQuery, ListQueueQuery};

pub fn filter_list_queue_query(
    mut sqlb: SqlBuilder,
    lq: &ListQueueQuery,
    w_id: &str,
    join_outstanding_wait_times: bool,
) -> SqlBuilder {
    sqlb.join("v2_job").on_eq("v2_job_queue.id", "v2_job.id");

    if join_outstanding_wait_times {
        sqlb.left()
            .join("outstanding_wait_time")
            .on_eq("v2_job_queue.id", "outstanding_wait_time.job_id");
    }

    if w_id != "admins" || !lq.all_workspaces.is_some_and(|x| x) {
        sqlb.and_where_eq("v2_job_queue.workspace_id", "?".bind(&w_id));
    }

    if let Some(w) = &lq.worker {
        if lq.allow_wildcards.unwrap_or(false) {
            sqlb.and_where_like_left("v2_job_queue.worker", w.replace("*", "%"));
        } else {
            sqlb.and_where_eq("v2_job_queue.worker", "?".bind(w));
        }
    }

    if let Some(ps) = &lq.script_path_start {
        sqlb.and_where_like_left("runnable_path", ps);
    }
    if let Some(p) = &lq.script_path_exact {
        sqlb.and_where_eq("runnable_path", "?".bind(p));
    }
    if let Some(p) = &lq.schedule_path {
        sqlb.and_where_eq("trigger", "?".bind(p));
        sqlb.and_where_eq("trigger_kind", "'schedule'");
    }
    if let Some(h) = &lq.script_hash {
        sqlb.and_where_eq("runnable_id", "?".bind(h));
    }
    if let Some(cb) = &lq.created_by {
        sqlb.and_where_eq("created_by", "?".bind(cb));
    }
    if let Some(t) = &lq.tag {
        if lq.allow_wildcards.unwrap_or(false) {
            sqlb.and_where_like_left("v2_job.tag", t.replace("*", "%"));
        } else {
            sqlb.and_where_eq("v2_job.tag", "?".bind(t));
        }
    }

    if let Some(r) = &lq.running {
        sqlb.and_where_eq("running", &r);
    }
    if let Some(pj) = &lq.parent_job {
        sqlb.and_where_eq("parent_job", "?".bind(pj));
    }
    if let Some(dt) = &lq.started_before {
        sqlb.and_where_le("started_at", "?".bind(&dt.to_rfc3339()));
    }
    if let Some(dt) = &lq.started_after {
        sqlb.and_where_ge("started_at", "?".bind(&dt.to_rfc3339()));
    }
    if let Some(fs) = &lq.is_flow_step {
        if *fs {
            sqlb.and_where_is_not_null("flow_step_id");
        } else {
            sqlb.and_where_is_null("flow_step_id");
        }
    }
    if let Some(fs) = &lq.has_null_parent {
        if *fs {
            sqlb.and_where_is_null("parent_job");
        }
    }

    if let Some(dt) = &lq.created_before {
        sqlb.and_where_le("v2_job.created_at", "?".bind(&dt.to_rfc3339()));
    }
    if let Some(dt) = &lq.created_after {
        sqlb.and_where_ge("v2_job.created_at", "?".bind(&dt.to_rfc3339()));
    }

    if let Some(dt) = &lq.created_or_started_after {
        let ts = dt.timestamp_millis();
        sqlb.and_where(format!("(started_at IS NOT NULL AND started_at >= to_timestamp({}  / 1000.0)) OR (started_at IS NULL AND v2_job.created_at >= to_timestamp({}  / 1000.0))", ts, ts));
    }

    if let Some(dt) = &lq.created_or_started_before {
        let ts = dt.timestamp_millis();
        sqlb.and_where(format!("(started_at IS NOT NULL AND started_at < to_timestamp({}  / 1000.0)) OR (started_at IS NULL AND v2_job.created_at < to_timestamp({}  / 1000.0))", ts, ts));
    }

    if let Some(s) = &lq.suspended {
        if *s {
            sqlb.and_where_gt("suspend", 0);
        } else {
            sqlb.and_where_is_null("suspend_until");
        }
    }

    if let Some(jk) = &lq.job_kinds {
        sqlb.and_where_in(
            "kind",
            &jk.split(',').into_iter().map(quote).collect::<Vec<_>>(),
        );
    }

    if let Some(args) = &lq.args {
        sqlb.and_where("args @> ?".bind(&args.replace("'", "''")));
    }

    if lq.scheduled_for_before_now.is_some_and(|x| x) {
        sqlb.and_where_le("scheduled_for", "now()");
    }

    if lq.is_not_schedule.unwrap_or(false) {
        sqlb.and_where("trigger_kind IS DISTINCT FROM 'schedule'");
    }

    if let Some(tk) = &lq.trigger_kind {
        sqlb.and_where_eq("trigger_kind", "?".bind(&format!("{}", tk)));
    }

    if let Some(tp) = &lq.trigger_path {
        sqlb.and_where_eq("trigger", "?".bind(tp));
    }

    sqlb
}

pub fn list_queue_jobs_query(
    w_id: &str,
    lq: &ListQueueQuery,
    fields: &[&str],
    pagination: Pagination,
    join_outstanding_wait_times: bool,
    tags: Option<Vec<&str>>,
) -> SqlBuilder {
    let (limit, offset) = paginate_without_limits(pagination);
    let mut sqlb = SqlBuilder::select_from("v2_job_queue")
        .fields(fields)
        .order_by("v2_job.created_at", lq.order_desc.unwrap_or(true))
        .limit(limit)
        .offset(offset)
        .clone();

    if let Some(tags) = tags {
        sqlb.and_where_in(
            "v2_job.tag",
            &tags.iter().map(|x| quote(x)).collect::<Vec<_>>(),
        );
    }

    filter_list_queue_query(sqlb, lq, w_id, join_outstanding_wait_times)
}

pub fn filter_list_completed_query(
    mut sqlb: SqlBuilder,
    lq: &ListCompletedQuery,
    w_id: &str,
    join_outstanding_wait_times: bool,
) -> SqlBuilder {
    sqlb.join("v2_job")
        .on_eq("v2_job_completed.id", "v2_job.id");

    if join_outstanding_wait_times {
        sqlb.left()
            .join("outstanding_wait_time")
            .on_eq("v2_job_completed.id", "outstanding_wait_time.job_id");
    }

    if let Some(label) = &lq.label {
        if lq.allow_wildcards.unwrap_or(false) {
            let wh = format!(
                    "EXISTS (SELECT 1 FROM jsonb_array_elements_text(result->'wm_labels') label WHERE jsonb_typeof(result->'wm_labels') = 'array' AND label LIKE '{}')",
                    &label.replace("*", "%").replace("'", "''")
                );
            sqlb.and_where("result ? 'wm_labels'");
            sqlb.and_where(&wh);
        } else {
            let mut wh = format!("result->'wm_labels' ? ");
            wh.push_str(&format!("'{}'", &label.replace("'", "''")));
            sqlb.and_where("result ? 'wm_labels'");
            sqlb.and_where(&wh);
        }
    }

    if let Some(worker) = &lq.worker {
        if lq.allow_wildcards.unwrap_or(false) {
            sqlb.and_where_like_left("v2_job_completed.worker", worker.replace("*", "%"));
        } else {
            sqlb.and_where_eq("v2_job_completed.worker", "?".bind(worker));
        }
    }

    if w_id != "admins" || !lq.all_workspaces.is_some_and(|x| x) {
        sqlb.and_where_eq("v2_job_completed.workspace_id", "?".bind(&w_id))
            .and_where_eq("v2_job.workspace_id", "?".bind(&w_id));
    }

    if let Some(p) = &lq.schedule_path {
        sqlb.and_where_eq("trigger", "?".bind(p));
        sqlb.and_where_eq("trigger_kind", "'schedule'");
    }

    if let Some(ps) = &lq.script_path_start {
        sqlb.and_where_like_left("runnable_path", ps);
    }
    if let Some(p) = &lq.script_path_exact {
        sqlb.and_where_eq("runnable_path", "?".bind(p));
    }
    if let Some(h) = &lq.script_hash {
        sqlb.and_where_eq("runnable_id", "?".bind(h));
    }
    if let Some(t) = &lq.tag {
        if lq.allow_wildcards.unwrap_or(false) {
            sqlb.and_where_like_left("v2_job.tag", t.replace("*", "%"));
        } else {
            sqlb.and_where_eq("v2_job.tag", "?".bind(t));
        }
    }

    if let Some(cb) = &lq.created_by {
        sqlb.and_where_eq("created_by", "?".bind(cb));
    }
    if let Some(r) = &lq.success {
        if *r {
            sqlb.and_where_eq("status", "'success'")
                .or_where_eq("status", "'skipped'");
        } else {
            sqlb.and_where_eq("status", "'failure'")
                .or_where_eq("status", "'canceled'");
        }
    }
    if let Some(pj) = &lq.parent_job {
        sqlb.and_where_eq("parent_job", "?".bind(pj));
    }
    if let Some(dt) = &lq.started_before {
        sqlb.and_where_le("started_at", "?".bind(&dt.to_rfc3339()));
    }
    if let Some(dt) = &lq.started_after {
        sqlb.and_where_ge("started_at", "?".bind(&dt.to_rfc3339()));
    }

    if let Some(dt) = &lq.created_or_started_before {
        sqlb.and_where_le("started_at", "?".bind(&dt.to_rfc3339()));
    }
    if let Some(dt) = &lq.created_or_started_after {
        let ts = dt.to_rfc3339();
        sqlb.and_where(format!(
            "(created_at >= '{}' OR started_at >= '{}')",
            ts.replace("'", "''"),
            ts.replace("'", "''")
        ));
    }

    if let Some(dt) = &lq.created_before {
        sqlb.and_where_le("created_at", "?".bind(&dt.to_rfc3339()));
    }
    if let Some(dt) = &lq.created_after {
        sqlb.and_where_ge("created_at", "?".bind(&dt.to_rfc3339()));
    }

    if let Some(dt) = &lq.created_or_started_after_completed_jobs {
        sqlb.and_where_ge("started_at", "?".bind(&dt.to_rfc3339()));
    }

    if let Some(dt) = &lq.completed_after {
        sqlb.and_where_ge("completed_at", "?".bind(&dt.to_rfc3339()));
    }
    if let Some(dt) = &lq.completed_before {
        sqlb.and_where_le("completed_at", "?".bind(&dt.to_rfc3339()));
    }

    if let Some(sk) = &lq.is_skipped {
        if *sk {
            sqlb.and_where_eq("status", "'skipped'");
        } else {
            sqlb.and_where_ne("status", "'skipped'");
        }
    }
    if let Some(fs) = &lq.is_flow_step {
        if *fs {
            sqlb.and_where_is_not_null("flow_step_id");
        } else {
            sqlb.and_where_is_null("flow_step_id");
        }
    }
    if let Some(fs) = &lq.has_null_parent {
        if *fs {
            sqlb.and_where_is_null("parent_job");
        }
    }
    if let Some(jk) = &lq.job_kinds {
        sqlb.and_where_in(
            "kind",
            &jk.split(',').into_iter().map(quote).collect::<Vec<_>>(),
        );
    }

    if let Some(args) = &lq.args {
        sqlb.and_where("args @> ?".bind(&args.replace("'", "''")));
    }

    if let Some(result) = &lq.result {
        sqlb.and_where("result @> ?".bind(&result.replace("'", "''")));
    }

    if lq.is_not_schedule.unwrap_or(false) {
        sqlb.and_where("trigger_kind IS DISTINCT FROM 'schedule'");
    }

    if let Some(tk) = &lq.trigger_kind {
        sqlb.and_where_eq("trigger_kind", "?".bind(&format!("{}", tk)));
    }

    if let Some(tp) = &lq.trigger_path {
        sqlb.and_where_eq("trigger", "?".bind(tp));
    }

    sqlb
}

pub fn list_completed_jobs_query(
    w_id: &str,
    per_page: Option<usize>,
    offset: usize,
    lq: &ListCompletedQuery,
    fields: &[&str],
    join_outstanding_wait_times: bool,
    tags: Option<Vec<&str>>,
) -> SqlBuilder {
    let mut sqlb = SqlBuilder::select_from("v2_job_completed")
        .fields(fields)
        .order_by(
            if lq.completed_before.is_some() || lq.completed_after.is_some() {
                "v2_job_completed.completed_at"
            } else {
                "v2_job.created_at"
            },
            lq.order_desc.unwrap_or(true),
        )
        .offset(offset)
        .clone();
    if let Some(per_page) = per_page {
        sqlb.limit(per_page);
    }

    if let Some(tags) = tags {
        sqlb.and_where_in(
            "v2_job.tag",
            &tags.iter().map(|x| quote(x)).collect::<Vec<_>>(),
        );
    }

    filter_list_completed_query(sqlb, lq, w_id, join_outstanding_wait_times)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_queue_query() -> ListQueueQuery {
        ListQueueQuery {
            script_path_start: None,
            script_path_exact: None,
            script_hash: None,
            created_by: None,
            started_before: None,
            started_after: None,
            created_before: None,
            created_after: None,
            created_or_started_before: None,
            created_or_started_after: None,
            running: None,
            parent_job: None,
            order_desc: None,
            job_kinds: None,
            suspended: None,
            args: None,
            tag: None,
            schedule_path: None,
            scheduled_for_before_now: None,
            all_workspaces: None,
            is_flow_step: None,
            has_null_parent: None,
            is_not_schedule: None,
            concurrency_key: None,
            worker: None,
            allow_wildcards: None,
            trigger_kind: None,
            trigger_path: None,
            include_args: None,
        }
    }

    fn empty_completed_query() -> ListCompletedQuery {
        ListCompletedQuery {
            script_path_start: None,
            script_path_exact: None,
            script_hash: None,
            created_by: None,
            started_before: None,
            started_after: None,
            created_before: None,
            created_after: None,
            created_or_started_before: None,
            created_or_started_after: None,
            created_or_started_after_completed_jobs: None,
            created_before_queue: None,
            created_after_queue: None,
            completed_after: None,
            completed_before: None,
            success: None,
            running: None,
            parent_job: None,
            order_desc: None,
            job_kinds: None,
            is_skipped: None,
            is_flow_step: None,
            suspended: None,
            schedule_path: None,
            args: None,
            result: None,
            tag: None,
            scheduled_for_before_now: None,
            all_workspaces: None,
            has_null_parent: None,
            label: None,
            is_not_schedule: None,
            concurrency_key: None,
            worker: None,
            allow_wildcards: None,
            trigger_kind: None,
            trigger_path: None,
            include_args: None,
        }
    }

    fn build_sql(sqlb: SqlBuilder) -> String {
        sqlb.sql().unwrap_or_default()
    }

    // --- Queue query tests ---

    #[test]
    fn test_queue_basic_query() {
        let lq = empty_queue_query();
        let sqlb = list_queue_jobs_query(
            "test_ws",
            &lq,
            &["v2_job_queue.id"],
            Pagination { page: Some(1), per_page: Some(10) },
            false,
            None,
        );
        let sql = build_sql(sqlb);
        assert!(sql.contains("v2_job_queue"));
        assert!(sql.contains("v2_job_queue.workspace_id"));
    }

    #[test]
    fn test_queue_filter_script_path_start() {
        let lq = ListQueueQuery {
            script_path_start: Some("f/test".to_string()),
            ..empty_queue_query()
        };
        let sqlb = filter_list_queue_query(
            SqlBuilder::select_from("v2_job_queue").clone(),
            &lq,
            "ws",
            false,
        );
        let sql = build_sql(sqlb);
        assert!(sql.contains("runnable_path"));
        assert!(sql.contains("LIKE"));
    }

    #[test]
    fn test_queue_filter_script_path_exact() {
        let lq = ListQueueQuery {
            script_path_exact: Some("f/test/script".to_string()),
            ..empty_queue_query()
        };
        let sqlb = filter_list_queue_query(
            SqlBuilder::select_from("v2_job_queue").clone(),
            &lq,
            "ws",
            false,
        );
        let sql = build_sql(sqlb);
        assert!(sql.contains("runnable_path"));
    }

    #[test]
    fn test_queue_filter_running() {
        let lq = ListQueueQuery {
            running: Some(true),
            ..empty_queue_query()
        };
        let sqlb = filter_list_queue_query(
            SqlBuilder::select_from("v2_job_queue").clone(),
            &lq,
            "ws",
            false,
        );
        let sql = build_sql(sqlb);
        assert!(sql.contains("running"));
    }

    #[test]
    fn test_queue_filter_job_kinds() {
        let lq = ListQueueQuery {
            job_kinds: Some("script,flow".to_string()),
            ..empty_queue_query()
        };
        let sqlb = filter_list_queue_query(
            SqlBuilder::select_from("v2_job_queue").clone(),
            &lq,
            "ws",
            false,
        );
        let sql = build_sql(sqlb);
        assert!(sql.contains("kind"));
        assert!(sql.contains("IN"));
    }

    #[test]
    fn test_queue_filter_suspended() {
        let lq = ListQueueQuery {
            suspended: Some(true),
            ..empty_queue_query()
        };
        let sqlb = filter_list_queue_query(
            SqlBuilder::select_from("v2_job_queue").clone(),
            &lq,
            "ws",
            false,
        );
        let sql = build_sql(sqlb);
        assert!(sql.contains("suspend"));
    }

    #[test]
    fn test_queue_filter_is_not_schedule() {
        let lq = ListQueueQuery {
            is_not_schedule: Some(true),
            ..empty_queue_query()
        };
        let sqlb = filter_list_queue_query(
            SqlBuilder::select_from("v2_job_queue").clone(),
            &lq,
            "ws",
            false,
        );
        let sql = build_sql(sqlb);
        assert!(sql.contains("trigger_kind IS DISTINCT FROM"));
    }

    #[test]
    fn test_queue_filter_has_null_parent() {
        let lq = ListQueueQuery {
            has_null_parent: Some(true),
            ..empty_queue_query()
        };
        let sqlb = filter_list_queue_query(
            SqlBuilder::select_from("v2_job_queue").clone(),
            &lq,
            "ws",
            false,
        );
        let sql = build_sql(sqlb);
        assert!(sql.contains("parent_job IS NULL"));
    }

    #[test]
    fn test_queue_filter_is_flow_step_true() {
        let lq = ListQueueQuery {
            is_flow_step: Some(true),
            ..empty_queue_query()
        };
        let sqlb = filter_list_queue_query(
            SqlBuilder::select_from("v2_job_queue").clone(),
            &lq,
            "ws",
            false,
        );
        let sql = build_sql(sqlb);
        assert!(sql.contains("flow_step_id IS NOT NULL"));
    }

    #[test]
    fn test_queue_filter_is_flow_step_false() {
        let lq = ListQueueQuery {
            is_flow_step: Some(false),
            ..empty_queue_query()
        };
        let sqlb = filter_list_queue_query(
            SqlBuilder::select_from("v2_job_queue").clone(),
            &lq,
            "ws",
            false,
        );
        let sql = build_sql(sqlb);
        assert!(sql.contains("flow_step_id IS NULL"));
    }

    #[test]
    fn test_queue_admins_all_workspaces() {
        let lq = ListQueueQuery {
            all_workspaces: Some(true),
            ..empty_queue_query()
        };
        let sqlb = filter_list_queue_query(
            SqlBuilder::select_from("v2_job_queue").clone(),
            &lq,
            "admins",
            false,
        );
        let sql = build_sql(sqlb);
        assert!(!sql.contains("workspace_id"));
    }

    #[test]
    fn test_queue_non_admins_ignores_all_workspaces() {
        let lq = ListQueueQuery {
            all_workspaces: Some(true),
            ..empty_queue_query()
        };
        let sqlb = filter_list_queue_query(
            SqlBuilder::select_from("v2_job_queue").clone(),
            &lq,
            "other_ws",
            false,
        );
        let sql = build_sql(sqlb);
        assert!(sql.contains("workspace_id"));
    }

    #[test]
    fn test_queue_with_outstanding_wait_times() {
        let lq = empty_queue_query();
        let sqlb = filter_list_queue_query(
            SqlBuilder::select_from("v2_job_queue").clone(),
            &lq,
            "ws",
            true,
        );
        let sql = build_sql(sqlb);
        assert!(sql.contains("outstanding_wait_time"));
    }

    #[test]
    fn test_queue_schedule_path_filter() {
        let lq = ListQueueQuery {
            schedule_path: Some("f/test/schedule".to_string()),
            ..empty_queue_query()
        };
        let sqlb = filter_list_queue_query(
            SqlBuilder::select_from("v2_job_queue").clone(),
            &lq,
            "ws",
            false,
        );
        let sql = build_sql(sqlb);
        assert!(sql.contains("trigger"));
        assert!(sql.contains("'schedule'"));
    }

    // --- Completed query tests ---

    #[test]
    fn test_completed_basic_query() {
        let lq = empty_completed_query();
        let sqlb = list_completed_jobs_query("test_ws", Some(10), 0, &lq, &["id"], false, None);
        let sql = build_sql(sqlb);
        assert!(sql.contains("v2_job_completed"));
    }

    #[test]
    fn test_completed_filter_success_true() {
        let lq = ListCompletedQuery {
            success: Some(true),
            ..empty_completed_query()
        };
        let sqlb = filter_list_completed_query(
            SqlBuilder::select_from("v2_job_completed").clone(),
            &lq,
            "ws",
            false,
        );
        let sql = build_sql(sqlb);
        assert!(sql.contains("'success'"));
    }

    #[test]
    fn test_completed_filter_success_false() {
        let lq = ListCompletedQuery {
            success: Some(false),
            ..empty_completed_query()
        };
        let sqlb = filter_list_completed_query(
            SqlBuilder::select_from("v2_job_completed").clone(),
            &lq,
            "ws",
            false,
        );
        let sql = build_sql(sqlb);
        assert!(sql.contains("'failure'"));
    }

    #[test]
    fn test_completed_order_by_completed_at() {
        let lq = ListCompletedQuery {
            completed_after: Some(chrono::Utc::now()),
            ..empty_completed_query()
        };
        let sqlb = list_completed_jobs_query("ws", Some(10), 0, &lq, &["id"], false, None);
        let sql = build_sql(sqlb);
        assert!(sql.contains("completed_at"));
    }

    #[test]
    fn test_completed_filter_label() {
        let lq = ListCompletedQuery {
            label: Some("deploy".to_string()),
            ..empty_completed_query()
        };
        let sqlb = filter_list_completed_query(
            SqlBuilder::select_from("v2_job_completed").clone(),
            &lq,
            "ws",
            false,
        );
        let sql = build_sql(sqlb);
        assert!(sql.contains("wm_labels"));
    }

    #[test]
    fn test_completed_filter_is_skipped() {
        let lq = ListCompletedQuery {
            is_skipped: Some(true),
            ..empty_completed_query()
        };
        let sqlb = filter_list_completed_query(
            SqlBuilder::select_from("v2_job_completed").clone(),
            &lq,
            "ws",
            false,
        );
        let sql = build_sql(sqlb);
        assert!(sql.contains("'skipped'"));
    }

    #[test]
    fn test_completed_with_tags() {
        let lq = empty_completed_query();
        let sqlb = list_completed_jobs_query(
            "ws",
            Some(10),
            0,
            &lq,
            &["id"],
            false,
            Some(vec!["tag1", "tag2"]),
        );
        let sql = build_sql(sqlb);
        assert!(sql.contains("v2_job.tag"));
        assert!(sql.contains("IN"));
    }

    #[test]
    fn test_completed_no_limit() {
        let lq = empty_completed_query();
        let sqlb = list_completed_jobs_query("ws", None, 0, &lq, &["id"], false, None);
        let sql = build_sql(sqlb);
        assert!(!sql.contains("LIMIT"));
    }

    #[test]
    fn test_completed_result_filter() {
        let lq = ListCompletedQuery {
            result: Some(r#"{"status": "ok"}"#.to_string()),
            ..empty_completed_query()
        };
        let sqlb = filter_list_completed_query(
            SqlBuilder::select_from("v2_job_completed").clone(),
            &lq,
            "ws",
            false,
        );
        let sql = build_sql(sqlb);
        assert!(sql.contains("result @>"));
    }
}
