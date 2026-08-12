use std::sync::Arc;

use axum::{extract::Path, routing::get, Extension, Json, Router};
use uuid::Uuid;
use windmill_common::{
    db::UserDB,
    error::{Error, JsonResult},
};

pub use windmill_api_jobs::concurrency_groups::{join_concurrency_key, workspaced_service};

use crate::{
    auth::{AuthCache, Tokened},
    db::{ApiAuthed, DB},
    jobs::{require_job_read_access, OptViewToken},
};

pub fn global_service() -> Router {
    windmill_api_jobs::concurrency_groups::global_service()
        .route("/{job_id}/key", get(get_concurrency_key))
}

/// A concurrency key embeds the job's workspace, its runnable path and any
/// `$args[...]`-templated argument values, so reading one is gated on the same access as
/// reading the run itself. `ApiAuthed` is taken here rather than left to the router: this
/// service is nested after `route_layer(from_extractor::<ApiAuthed>())`, which only wraps
/// the routes present at its call site.
///
/// The route carries no workspace, so `authed` is not workspace-scoped (no groups, no
/// folders, `is_admin` only for superadmins). Re-resolve the caller's token against the
/// job's own workspace before evaluating access — that also rejects non-members, for whom
/// the job does not exist. Members reaching [`require_job_read_access`] get its 403, which
/// discloses existence only inside a workspace they belong to.
async fn get_concurrency_key(
    _authed: ApiAuthed,
    Tokened { token }: Tokened,
    Extension(cache): Extension<Arc<AuthCache>>,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    OptViewToken(view_token): OptViewToken,
    Path(job_id): Path<Uuid>,
) -> JsonResult<Option<String>> {
    let not_found = || Error::NotFound(format!("Job {job_id} not found"));

    // A caller who cannot reach the job must not be able to tell it apart from a job
    // that does not exist, so both answer with the same 404.
    let job = sqlx::query!(
        "SELECT workspace_id, created_by FROM v2_job WHERE id = $1",
        job_id
    )
    .fetch_optional(&db)
    .await?
    .ok_or_else(not_found)?;

    let authed_in_workspace = cache
        .get_authed(Some(job.workspace_id.clone()), &token)
        .await
        .ok_or_else(not_found)?;

    require_job_read_access(
        &db,
        &user_db,
        &authed_in_workspace,
        &job.workspace_id,
        &job_id,
        &job.created_by,
        view_token.as_deref(),
    )
    .await?;

    let key = sqlx::query_scalar!("SELECT key FROM concurrency_key WHERE job_id = $1", job_id)
        .fetch_optional(&db)
        .await?;
    Ok(Json(key))
}
