use crate::db::{ApiAuthed, DB};
use crate::jobs::{
    filter_list_completed_query, filter_list_queue_query, Job, ListCompletedQuery, ListQueueQuery,
    UnifiedJob,
};
use crate::users::check_scopes;
use axum::extract::Path;
use axum::routing::{delete, get};
use axum::{extract::Query, Extension, Json};
use serde::Deserialize;

use axum::Router;

use serde::Serialize;
use sql_builder::bind::Bind;
use sql_builder::SqlBuilder;
use uuid::Uuid;
use windmill_common::db::UserDB;
use windmill_common::error::Error::{InternalErr, PermissionDenied};
use windmill_common::error::{self, JsonResult};
use windmill_common::utils::require_admin;

pub fn global_service() -> Router {
    Router::new()
        .route("/list", get(list_concurrency_groups))
        .route("/prune/*concurrency_key", delete(prune_concurrency_group))
        .route("/:job_id/key", get(get_concurrency_key))
}

pub fn workspaced_service() -> Router {
    Router::new().route("/list_jobs", get(get_concurrent_intervals))
}

#[derive(Serialize)]
pub struct ConcurrencyGroups {
    concurrency_key: String,
    total_running: i64,
}

async fn list_concurrency_groups(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
) -> JsonResult<Vec<ConcurrencyGroups>> {
    require_admin(authed.is_admin, &authed.username)?;

    let concurrency_counts = sqlx::query_as::<_, (String, i64)>(
        "SELECT concurrency_id, (select COUNT(*) from jsonb_object_keys(job_uuids)) as n_job_uuids FROM concurrency_counter",
    ).fetch_all(&db)
    .await?;

    let mut concurrency_groups: Vec<ConcurrencyGroups> = vec![];
    for (concurrency_key, count) in concurrency_counts {
        concurrency_groups.push(ConcurrencyGroups {
            concurrency_key: concurrency_key.clone(),
            total_running: count,
        })
    }

    return Ok(Json(concurrency_groups));
}

async fn prune_concurrency_group(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(concurrency_key): Path<String>,
) -> JsonResult<()> {
    if !authed.is_admin {
        return Err(PermissionDenied(
            "Only administrators can delete concurrency groups".to_string(),
        ));
    }
    let mut tx = db.begin().await?;

    let concurrency_group = sqlx::query_as::<_, (String, i64)>(
        "SELECT concurrency_id, (select COUNT(*) from jsonb_object_keys(job_uuids)) as n_job_uuids FROM concurrency_counter WHERE concurrency_id = $1 FOR UPDATE",
    )
    .bind(concurrency_key.clone())
    .fetch_optional(&mut *tx)
    .await?;

    let n_job_uuids = concurrency_group.map(|cg| cg.1).unwrap_or_default();

    if n_job_uuids > 0 {
        tx.commit().await?;
        return Err(InternalErr(
            "Concurrency group is currently in use, unable to remove it. Retry later.".to_string(),
        ));
    }

    sqlx::query!(
        "DELETE FROM concurrency_counter WHERE concurrency_id = $1",
        concurrency_key.clone(),
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "DELETE FROM concurrency_key WHERE key = $1",
        concurrency_key.clone(),
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(Json(()))
}

#[derive(Serialize)]
struct ExtendedJobs {
    jobs: Vec<Job>,
    obscured_jobs: Vec<ObscuredJob>,
    omitted_obscured_jobs: bool,
}

#[derive(Serialize)]
struct ObscuredJob {
    typ: String,
    started_at: Option<chrono::DateTime<chrono::Utc>>,
    duration_ms: Option<i64>,
}
#[derive(Deserialize)]
struct ExtendedJobsParams {
    concurrency_key: Option<String>,
    row_limit: Option<i64>,
}

fn join_concurrency_key<'c>(concurrency_key: Option<&String>, mut sqlb: SqlBuilder) -> SqlBuilder {
    if let Some(key) = concurrency_key {
        sqlb.join("concurrency_key")
            .on_eq("id", "job_id")
            .and_where_eq("key", "?".bind(key));
    }

    sqlb
}

async fn get_concurrent_intervals(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(iq): Query<ExtendedJobsParams>,
    Query(lq): Query<ListCompletedQuery>,
) -> JsonResult<ExtendedJobs> {
    check_scopes(&authed, || format!("listjobs"))?;

    if lq.success.is_some() && lq.running.is_some_and(|x| x) {
        return Err(error::Error::BadRequest(
            "cannot specify both success and running".to_string(),
        ));
    }

    const QJ_FIELDS: &[&str] = &[
        "'QueuedJob' as typ",
        "id",
        "workspace_id",
        "parent_job",
        "created_by",
        "created_at",
        "started_at",
        "scheduled_for",
        "running",
        "script_hash",
        "script_path",
        "null as args",
        "null as duration_ms",
        "null as success",
        "false as deleted",
        "canceled",
        "canceled_by",
        "job_kind",
        "schedule_path",
        "permissioned_as",
        "is_flow_step",
        "language",
        "false as is_skipped",
        "email",
        "visible_to_owner",
        "suspend",
        "mem_peak",
        "tag",
        "concurrent_limit",
        "concurrency_time_window_s",
        "priority",
        "null as labels",
    ];
    const CJ_FIELDS: &[&str] = &[
        "'CompletedJob' as typ",
        "id",
        "workspace_id",
        "parent_job",
        "created_by",
        "created_at",
        "started_at",
        "null as scheduled_for",
        "null as running",
        "script_hash",
        "script_path",
        "null as args",
        "duration_ms",
        "success",
        "deleted",
        "canceled",
        "canceled_by",
        "job_kind",
        "schedule_path",
        "permissioned_as",
        "is_flow_step",
        "language",
        "is_skipped",
        "email",
        "visible_to_owner",
        "null as suspend",
        "mem_peak",
        "tag",
        "null as concurrent_limit",
        "null as concurrency_time_window_s",
        "priority",
        "result->'wm_labels' as labels",
    ];

    let row_limit = iq.row_limit.unwrap_or(1000);
    let concurrency_key = iq.concurrency_key;

    let lq = ListCompletedQuery {order_desc: Some(true), ..lq};
    let lqc = lq.clone();
    let lqq = ListQueueQuery {
        script_path_start: lqc.script_path_start,
        script_path_exact: lqc.script_path_exact,
        script_hash: lqc.script_hash,
        created_by: lqc.created_by,
        started_before: lqc.started_before,
        started_after: lqc.started_after,
        created_before: lqc.created_before,
        created_after: lqc.created_after,
        created_or_started_before: lqc.created_or_started_before,
        created_or_started_after: lqc.created_or_started_after,
        running: lqc.running,
        parent_job: lqc.parent_job,
        order_desc: lqc.order_desc,
        job_kinds: lqc.job_kinds,
        suspended: lqc.suspended,
        args: lqc.args,
        tag: lqc.tag,
        schedule_path: lqc.schedule_path,
        scheduled_for_before_now: lqc.scheduled_for_before_now,
        all_workspaces: lqc.all_workspaces,
        is_flow_step: lqc.is_flow_step,
        has_null_parent: lqc.has_null_parent,
        is_not_schedule: lqc.is_not_schedule,
    };
    let mut sqlb_q = SqlBuilder::select_from("queue")
        .fields(QJ_FIELDS)
        .order_by("created_at", lq.order_desc.unwrap_or(true))
        .limit(row_limit)
        .clone();
    let mut sqlb_c = SqlBuilder::select_from("completed_job")
        .fields(CJ_FIELDS)
        .order_by("started_at", lq.order_desc.unwrap_or(true))
        .limit(row_limit)
        .clone();
    let mut sqlb_q_user = SqlBuilder::select_from("queue")
        .fields(&["id"])
        .order_by("created_at", lq.order_desc.unwrap_or(true))
        .limit(row_limit)
        .clone();
    let mut sqlb_c_user = SqlBuilder::select_from("completed_job")
        .fields(&["id"])
        .order_by("started_at", lq.order_desc.unwrap_or(true))
        .limit(row_limit)
        .clone();

    sqlb_q = join_concurrency_key(concurrency_key.as_ref(), sqlb_q);
    sqlb_c = join_concurrency_key(concurrency_key.as_ref(), sqlb_c);
    sqlb_q_user = join_concurrency_key(concurrency_key.as_ref(), sqlb_q_user);
    sqlb_c_user = join_concurrency_key(concurrency_key.as_ref(), sqlb_c_user);

    let should_fetch_obscured_jobs = match lq {
        ListCompletedQuery {
            script_path_start: None,
            script_path_exact: None,
            script_hash: None,
            created_by: None,
            success: None,
            running: None,
            parent_job: None,
            is_skipped: None | Some(false),
            suspended: None,
            schedule_path: None,
            args: None,
            result: None,
            tag: None,
            scheduled_for_before_now: None,
            has_null_parent: None,
            label: None,
            is_not_schedule: None,
            started_before: _,
            started_after: _,
            created_before: _,
            created_after: _,
            created_or_started_before: _,
            created_or_started_after: _,
            order_desc: _,
            job_kinds: _,
            is_flow_step: _,
            all_workspaces: _,
        } => concurrency_key.is_some(),
        _ => false,
    };

    // When we have a concurrency key defined, fetch jobs from other workspaces
    // as obscured unless we're in the admins workspace. This is to show the
    // potential concurrency races without showing jobs that don't belong to
    // the workspace.
    // To avoid infering information through filtering, don't return obscured
    // jobs if the filters are too specific
    if should_fetch_obscured_jobs {
        let (sqlb_q, sqlb_c) = if w_id != "admin" {
            // By default get obscured jobs from all workspaces, unless in
            // admin workspace where admins can select to get all or not
            (
                filter_list_queue_query(
                    sqlb_q,
                    &ListQueueQuery { all_workspaces: Some(true), ..lqq.clone() },
                    "admins",
                ),
                filter_list_completed_query(
                    sqlb_c,
                    &ListCompletedQuery { all_workspaces: Some(true), ..lq.clone() },
                    "admins",
                ),
            )
        } else {
            (
                filter_list_queue_query(sqlb_q, &lqq, w_id.as_str()),
                filter_list_completed_query(sqlb_c, &lq, w_id.as_str()),
            )
        };

        sqlb_q_user = filter_list_queue_query(sqlb_q_user, &lqq, w_id.as_str());
        sqlb_c_user = filter_list_completed_query(sqlb_c_user, &lq, w_id.as_str());

        let sql_q_user = sqlb_q_user.query()?;
        let sql_c_user = sqlb_c_user.query()?;
        let sql_q = sqlb_q.query()?;
        let sql_c = sqlb_c.query()?;

        // This first transaction uses the user_db to know which uuids are
        // accessible to the user.
        let mut tx = user_db.begin(&authed).await?;
        let running_jobs_user: Vec<Uuid> = if lq.success.is_none() {
            sqlx::query_scalar(&sql_q_user).fetch_all(&mut *tx).await?
        } else {
            vec![]
        };
        let completed_jobs_user: Vec<Uuid> = if lq.running.is_none() {
            sqlx::query_scalar(&sql_c_user).fetch_all(&mut *tx).await?
        } else {
            vec![]
        };
        tx.commit().await?;

        // This second transaction uses the db, so it will fetch information
        // potentially forbidden to the user. It must be obscured before
        // returning it
        let running_jobs_db: Vec<UnifiedJob> = if lq.success.is_none() {
            sqlx::query_as(&sql_q).fetch_all(&db).await?
        } else {
            vec![]
        };
        let completed_jobs_db: Vec<UnifiedJob> = if lq.running.is_none() {
            sqlx::query_as(&sql_c).fetch_all(&db).await?
        } else {
            vec![]
        };

        let obscured_jobs = running_jobs_db
            .iter()
            .filter(|j| !running_jobs_user.iter().any(|id| j.id == *id))
            .chain(
                completed_jobs_db
                    .iter()
                    .filter(|j| !completed_jobs_user.iter().any(|id| j.id == *id)),
            )
            .map(|j| ObscuredJob { typ: j.typ.clone(), started_at: j.started_at, duration_ms: j.duration_ms })
            .collect();
        let jobs = running_jobs_db
            .into_iter()
            .filter(|j| running_jobs_user.iter().any(|id| j.id == *id))
            .chain(
                completed_jobs_db
                    .into_iter()
                    .filter(|j| completed_jobs_user.iter().any(|id| j.id == *id)),
            )
            .map(From::from)
            .collect();
        Ok(Json(ExtendedJobs { jobs, obscured_jobs, omitted_obscured_jobs: !should_fetch_obscured_jobs }))
    } else {
        let sql_q = sqlb_q.query()?;
        let sql_c = sqlb_c.query()?;

        let mut tx = user_db.begin(&authed).await?;
        let running_jobs: Vec<UnifiedJob> = if lq.success.is_none() {
            sqlx::query_as(&sql_q).fetch_all(&mut *tx).await?
        } else {
            vec![]
        };
        let completed_jobs: Vec<UnifiedJob> = if lq.running.is_none() {
            sqlx::query_as(&sql_c).fetch_all(&mut *tx).await?
        } else {
            vec![]
        };
        tx.commit().await?;

        let jobs = running_jobs
            .into_iter()
            .chain(completed_jobs.into_iter())
            .map(From::from)
            .collect();

        Ok(Json(ExtendedJobs { jobs, obscured_jobs: vec![], omitted_obscured_jobs: !should_fetch_obscured_jobs }))
    }
}

async fn get_concurrency_key(
    Extension(db): Extension<DB>,
    Path(job_id): Path<Uuid>,
) -> JsonResult<Option<String>> {
    let key = sqlx::query_scalar!("SELECT key FROM concurrency_key WHERE job_id = $1", job_id)
        .fetch_optional(&db)
        .await?;
    Ok(Json(key))
}
