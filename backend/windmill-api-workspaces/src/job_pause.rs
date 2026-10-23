/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2026
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use axum::{
    extract::{Extension, Path},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Duration, Utc};
use hyper::StatusCode;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

use windmill_api_auth::ApiAuthed;
use windmill_audit::audit_oss::audit_log;
use windmill_audit::ActionKind;
use windmill_common::error::{Error, JsonResult};
use windmill_common::feature_usage::log_feature_usage;
use windmill_common::job_pause::{self, MAX_PAUSE_DAYS};
use windmill_common::utils::require_admin;
use windmill_common::DB;

pub(crate) fn routes() -> Router {
    Router::new()
        .route("/jobs_paused", get(get_jobs_paused))
        .route("/pause_jobs", post(pause_jobs))
        .route("/resume_jobs", post(resume_jobs))
}

#[derive(Serialize)]
struct JobsPaused {
    paused_until: DateTime<Utc>,
    paused_by: String,
    paused_at: DateTime<Utc>,
    reason: Option<String>,
    held_count: i64,
    pause_id: Uuid,
}

async fn read_pause(db: &DB, w_id: &str) -> windmill_common::error::Result<Option<JobsPaused>> {
    Ok(sqlx::query_as!(
        JobsPaused,
        r#"SELECT paused_until, paused_by, paused_at, reason, pause_id,
            (SELECT count(*) FROM workspace_held_job h WHERE h.workspace_id = p.workspace_id) AS "held_count!"
        FROM workspace_job_pause p
        WHERE workspace_id = $1 AND paused_until > now()"#,
        w_id
    )
    .fetch_optional(db)
    .await?)
}

async fn get_jobs_paused(
    _authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> JsonResult<Option<JobsPaused>> {
    Ok(Json(read_pause(&db, &w_id).await?))
}

#[derive(Deserialize)]
struct PauseJobsRequest {
    until: Option<DateTime<Utc>>,
    duration_s: Option<i64>,
    reason: Option<String>,
    /// Where the pause was asked from, for telemetry only.
    source: Option<String>,
}

async fn pause_jobs(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(req): Json<PauseJobsRequest>,
) -> JsonResult<Option<JobsPaused>> {
    require_admin(authed.is_admin, &authed.username)?;

    let now = Utc::now();
    let max = Duration::days(MAX_PAUSE_DAYS);
    let until = match (req.until, req.duration_s) {
        (Some(until), None) => until,
        // Bounded before the arithmetic, which panics on a large enough duration.
        (None, Some(secs)) if secs > 0 => now + Duration::seconds(secs.min(max.num_seconds() + 1)),
        _ => {
            return Err(Error::BadRequest(
                "give either `until` or a positive `duration_s`".to_string(),
            ))
        }
    };
    if until <= now {
        return Err(Error::BadRequest(
            "`until` must be in the future".to_string(),
        ));
    }
    if until > now + max {
        return Err(Error::BadRequest(format!(
            "jobs cannot be paused for more than {MAX_PAUSE_DAYS} days"
        )));
    }

    let mut tx = db.begin().await?;
    job_pause::lock_workspace(&mut tx, &w_id).await?;
    // Extending a pause keeps when it started, and a pause past its deadline counts as a new one.
    sqlx::query!(
        "INSERT INTO workspace_job_pause (workspace_id, paused_until, paused_by, reason)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (workspace_id) DO UPDATE
        SET paused_until = EXCLUDED.paused_until, paused_by = EXCLUDED.paused_by,
            reason = EXCLUDED.reason, pause_id = gen_random_uuid(),
            paused_at = CASE WHEN workspace_job_pause.paused_until <= now() THEN now()
                ELSE workspace_job_pause.paused_at END",
        &w_id,
        until,
        &authed.username,
        req.reason,
    )
    .execute(&mut *tx)
    .await?;
    let held = job_pause::hold_queued_jobs(&mut tx, &w_id).await?;
    let until_s = until.to_rfc3339();
    let held_s = held.to_string();
    audit_log(
        &mut *tx,
        &authed,
        "workspaces.pause_jobs",
        ActionKind::Update,
        &w_id,
        None,
        Some(
            [
                ("until", until_s.as_str()),
                ("held_queued", held_s.as_str()),
            ]
            .into(),
        ),
    )
    .await?;
    tx.commit().await?;
    job_pause::evict(&w_id);

    let source = match req.source.as_deref() {
        Some(s @ ("cli" | "ui" | "sdk")) => s,
        _ => "api",
    };
    log_feature_usage("workspace_job_pause", "paused", source);

    Ok(Json(read_pause(&db, &w_id).await?))
}

#[derive(Deserialize, Default)]
struct ResumeJobsRequest {
    /// Resume only if the pause is still the one this id names: no one paused again since.
    pause_id: Option<Uuid>,
}

async fn resume_jobs(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    req: Option<Json<ResumeJobsRequest>>,
) -> JsonResult<i64> {
    require_admin(authed.is_admin, &authed.username)?;
    let req = req.map(|Json(r)| r).unwrap_or_default();

    let mut tx = db.begin().await?;
    job_pause::lock_workspace(&mut tx, &w_id).await?;
    if let Some(expected) = req.pause_id {
        let current = sqlx::query!(
            "SELECT pause_id, paused_by FROM workspace_job_pause WHERE workspace_id = $1",
            &w_id
        )
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(current) = current.filter(|c| c.pause_id != expected) {
            return Err(Error::Generic(
                StatusCode::CONFLICT,
                format!(
                    "jobs were paused again since, by {}: they stay paused",
                    current.paused_by
                ),
            ));
        }
    }
    let released = job_pause::resume(&mut tx, &w_id).await?;
    let released_s = released.to_string();
    audit_log(
        &mut *tx,
        &authed,
        "workspaces.resume_jobs",
        ActionKind::Update,
        &w_id,
        None,
        Some([("released", released_s.as_str())].into()),
    )
    .await?;
    tx.commit().await?;
    job_pause::evict(&w_id);
    log_feature_usage("workspace_job_pause", "resumed", "manual");

    Ok(Json(released))
}
