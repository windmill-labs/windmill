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
