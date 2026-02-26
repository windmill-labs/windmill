use windmill_api_auth::{check_scopes, ApiAuthed};
use windmill_common::{
    db::{UserDB, DB},
    error::Error::PermissionDenied,
    error::{self, JsonResult},
    utils::require_admin,
};

use crate::query::{filter_list_completed_query, filter_list_queue_query};
use crate::types::{ListCompletedQuery, ListQueueQuery, UnifiedJob};
use windmill_api_sse::Job;

use axum::extract::Path;
use axum::routing::{delete, get};
use axum::{extract::Query, Extension, Json};
use serde::Deserialize;

use axum::Router;

use serde::Serialize;
use sql_builder::bind::Bind;
use sql_builder::SqlBuilder;
use uuid::Uuid;

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
        return Err(error::Error::internal_err(
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
    row_limit: Option<i64>,
}

pub fn join_concurrency_key<'c>(
    concurrency_key: Option<&String>,
    mut sqlb: SqlBuilder,
) -> SqlBuilder {
    if let Some(key) = concurrency_key {
        sqlb.join("concurrency_key")
            .on_eq("id", "concurrency_key.job_id")
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
    check_scopes(&authed, || format!("jobs:read"))?;

    if lq.success.is_some() && lq.running.is_some_and(|x| x) {
        return Err(error::Error::BadRequest(
            "cannot specify both success and running".to_string(),
        ));
    }

    let row_limit = iq.row_limit.unwrap_or(1000);

    let lq = ListCompletedQuery { order_desc: Some(true), ..lq };
    let lqc = lq.clone();
    let lqq: ListQueueQuery = lqc.into();
    let mut sqlb_q = SqlBuilder::select_from("v2_job_queue")
        .fields(UnifiedJob::queued_job_fields())
        .order_by("created_at", lq.order_desc.unwrap_or(true))
        .limit(row_limit)
        .clone();
    let mut sqlb_c = SqlBuilder::select_from("v2_job_completed")
        .fields(UnifiedJob::completed_job_fields())
        .order_by("completed_at", lq.order_desc.unwrap_or(true))
        .limit(row_limit)
        .clone();
    let mut sqlb_q_user = SqlBuilder::select_from("v2_job_queue")
        .fields(&["id"])
        .order_by("created_at", lq.order_desc.unwrap_or(true))
        .limit(row_limit)
        .clone();
    let mut sqlb_c_user = SqlBuilder::select_from("v2_job_completed")
        .fields(&["id"])
        .order_by("completed_at", lq.order_desc.unwrap_or(true))
        .limit(row_limit)
        .clone();

    sqlb_q = join_concurrency_key(lq.concurrency_key.as_ref(), sqlb_q);
    sqlb_c = join_concurrency_key(lq.concurrency_key.as_ref(), sqlb_c);
    sqlb_q_user = join_concurrency_key(lq.concurrency_key.as_ref(), sqlb_q_user);
    sqlb_c_user = join_concurrency_key(lq.concurrency_key.as_ref(), sqlb_c_user);

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
            has_null_parent: None,
            worker: None,
            label: None,
            trigger_path: None,
            scheduled_for_before_now: _,
            is_not_schedule: _,
            started_before: _,
            started_after: _,
            created_before: _,
            created_after: _,
            created_before_queue: _,
            created_after_queue: _,
            completed_after: _,
            completed_before: _,
            created_or_started_before: _,
            created_or_started_after: _,
            created_or_started_after_completed_jobs: _,
            order_desc: _,
            job_kinds: _,
            is_flow_step: _,
            all_workspaces: _,
            concurrency_key: Some(_),
            allow_wildcards: None,
            trigger_kind: _,
            include_args: _,
            broad_filter: _,
        } => true,
        _ => false,
    };

    // When we have a concurrency key defined, fetch jobs from other workspaces
    // as obscured unless we're in the admins workspace. This is to show the
    // potential concurrency races without showing jobs that don't belong to
    // the workspace.
    // To avoid infering information through filtering, don't return obscured
    // jobs if the filters are too specific
    if should_fetch_obscured_jobs && w_id != "admins" {
        // Get the obscured jobs from all workspaces (concurrency key could be global)
        let (sqlb_q, sqlb_c) = (
            filter_list_queue_query(
                sqlb_q,
                &ListQueueQuery { all_workspaces: Some(true), ..lqq.clone() },
                "admins",
                true,
            ),
            filter_list_completed_query(
                sqlb_c,
                &ListCompletedQuery { all_workspaces: Some(true), ..lq.clone() },
                "admins",
                true,
            ),
        );

        sqlb_q_user = filter_list_queue_query(sqlb_q_user, &lqq, w_id.as_str(), true);
        sqlb_c_user = filter_list_completed_query(sqlb_c_user, &lq, w_id.as_str(), true);

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
            .map(|j| ObscuredJob {
                typ: j.typ.clone(),
                started_at: j.started_at,
                duration_ms: j.duration_ms,
            })
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

        Ok(Json(ExtendedJobs {
            jobs,
            obscured_jobs,
            omitted_obscured_jobs: !should_fetch_obscured_jobs,
        }))
    } else {
        sqlb_q = filter_list_queue_query(sqlb_q, &lqq, w_id.as_str(), true);
        sqlb_c = filter_list_completed_query(sqlb_c, &lq, w_id.as_str(), true);
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

        Ok(Json(ExtendedJobs {
            jobs,
            obscured_jobs: vec![],
            omitted_obscured_jobs: !should_fetch_obscured_jobs,
        }))
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
