//! Fork deployment requests + comments.
//!
//! A deployment request is a one-per-open ask from a fork author to one or
//! more assignees (admins or wm_deployers in the parent workspace) to merge
//! the fork into its parent. Assignees can leave general or anchored
//! comments; anyone with fork access can reply. Anchored comments become
//! obsolete when the underlying item changes in the fork, and the whole
//! request auto-closes on successful merge.
//!
//! Email dispatch happens via `send_email_if_possible`, which is a no-op on
//! OSS builds.

use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use windmill_api_auth::ApiAuthed;
use windmill_audit::audit_oss::{audit_log, AuditAuthorable};
use windmill_audit::ActionKind;
use windmill_common::email_oss::send_email_if_possible;
use windmill_common::{
    error::{Error, JsonResult, Result},
    BASE_URL, DB, WM_DEPLOYERS_GROUP,
};

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/eligible_deployers", get(list_eligible_deployers))
        .route("/open", get(get_open_deployment_request))
        .route("/", post(create_deployment_request))
        .route("/{id}/cancel", post(cancel_deployment_request))
        .route("/{id}/close_merged", post(close_deployment_request_merged))
        .route("/{id}/comment", post(create_deployment_request_comment))
}

// ---- DTOs ---------------------------------------------------------------

#[derive(Serialize)]
struct EligibleDeployer {
    username: String,
    email: String,
    is_admin: bool,
}

#[derive(Serialize)]
struct DeploymentRequest {
    id: i64,
    source_workspace_id: String,
    fork_workspace_id: String,
    requested_by: String,
    requested_by_email: String,
    requested_at: chrono::DateTime<chrono::Utc>,
    assignees: Vec<DeploymentRequestAssignee>,
    comments: Vec<DeploymentRequestComment>,
}

#[derive(Serialize)]
struct DeploymentRequestAssignee {
    username: String,
    email: String,
}

#[derive(Serialize)]
struct DeploymentRequestComment {
    id: i64,
    parent_id: Option<i64>,
    author: String,
    author_email: String,
    body: String,
    anchor_kind: Option<String>,
    anchor_path: Option<String>,
    obsolete: bool,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize)]
struct CreateDeploymentRequestBody {
    assignees: Vec<String>,
}

#[derive(Deserialize)]
struct CreateDeploymentRequestCommentBody {
    body: String,
    #[serde(default)]
    anchor_kind: Option<String>,
    #[serde(default)]
    anchor_path: Option<String>,
    #[serde(default)]
    parent_id: Option<i64>,
}

// ---- Endpoints ----------------------------------------------------------

/// List users eligible to be assignees on a deployment request: admins plus
/// members of the wm_deployers group in the *parent* workspace. The `{w_id}`
/// path parameter is the fork workspace; we resolve its parent and query
/// there.
///
/// Note: any member of the fork workspace can call this and see parent
/// admin/deployer usernames + emails. Intentional for the request UX — the
/// fork author has to pick an assignee somehow — but documented here as a
/// design choice for the shared-fork privacy model.
async fn list_eligible_deployers(
    _authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> JsonResult<Vec<EligibleDeployer>> {
    let parent = parent_of_fork(&db, &w_id).await?;

    let rows = sqlx::query!(
        r#"
            SELECT DISTINCT u.username, u.email, u.is_admin
            FROM usr u
            WHERE u.workspace_id = $1
              AND u.disabled = false
              AND (
                  u.is_admin = true
                  OR EXISTS (
                      SELECT 1 FROM usr_to_group g
                      WHERE g.workspace_id = $1
                        AND g.group_ = $2
                        AND g.usr = u.username
                  )
              )
            ORDER BY u.username
        "#,
        &parent,
        WM_DEPLOYERS_GROUP,
    )
    .fetch_all(&db)
    .await?;

    Ok(Json(
        rows.into_iter()
            .map(|r| EligibleDeployer {
                username: r.username,
                email: r.email,
                is_admin: r.is_admin,
            })
            .collect(),
    ))
}

/// Fetch the open deployment request (if any) for the fork's parent.
/// `{w_id}` is the fork workspace.
async fn get_open_deployment_request(
    _authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> JsonResult<Option<DeploymentRequest>> {
    let parent = parent_of_fork(&db, &w_id).await?;

    let req = sqlx::query!(
        r#"
            SELECT id, source_workspace_id, fork_workspace_id, requested_by,
                   requested_by_email, requested_at
            FROM workspace_fork_deployment_request
            WHERE source_workspace_id = $1
              AND fork_workspace_id = $2
              AND closed_at IS NULL
            LIMIT 1
        "#,
        parent,
        &w_id,
    )
    .fetch_optional(&db)
    .await?;

    let Some(req) = req else {
        return Ok(Json(None));
    };

    let assignees = sqlx::query!(
        "SELECT username, email FROM workspace_fork_deployment_request_assignee WHERE request_id = $1 ORDER BY username",
        req.id,
    )
    .fetch_all(&db)
    .await?
    .into_iter()
    .map(|r| DeploymentRequestAssignee { username: r.username, email: r.email })
    .collect();

    let comments = sqlx::query!(
        r#"
            SELECT id, parent_id, author, author_email, body,
                   anchor_kind, anchor_path, obsolete, created_at
            FROM workspace_fork_deployment_request_comment
            WHERE request_id = $1
            ORDER BY created_at ASC, id ASC
        "#,
        req.id,
    )
    .fetch_all(&db)
    .await?
    .into_iter()
    .map(|r| DeploymentRequestComment {
        id: r.id,
        parent_id: r.parent_id,
        author: r.author,
        author_email: r.author_email,
        body: r.body,
        anchor_kind: r.anchor_kind,
        anchor_path: r.anchor_path,
        obsolete: r.obsolete,
        created_at: r.created_at,
    })
    .collect();

    Ok(Json(Some(DeploymentRequest {
        id: req.id,
        source_workspace_id: req.source_workspace_id,
        fork_workspace_id: req.fork_workspace_id,
        requested_by: req.requested_by,
        requested_by_email: req.requested_by_email,
        requested_at: req.requested_at,
        assignees,
        comments,
    })))
}

/// Open a new deployment request targeting a list of assignees in the
/// parent workspace. At most one open request exists per (parent, fork)
/// pair; the DB partial unique index enforces this and we surface a 409 on
/// conflict.
async fn create_deployment_request(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(body): Json<CreateDeploymentRequestBody>,
) -> Result<Json<DeploymentRequest>> {
    // Dedupe assignees so `["alice","alice"]` isn't falsely rejected: the
    // `= ANY($...)` lookup returns each match once regardless of input
    // duplicates, so a direct `.len()` comparison would over-count.
    let unique_assignees: Vec<String> = body
        .assignees
        .iter()
        .collect::<BTreeSet<&String>>()
        .into_iter()
        .cloned()
        .collect();

    if unique_assignees.is_empty() {
        return Err(Error::BadRequest(
            "At least one assignee is required".to_string(),
        ));
    }

    let parent = parent_of_fork(&db, &w_id).await?;

    // Validate every requested assignee is admin or wm_deployers in parent.
    let assignee_rows = sqlx::query!(
        r#"
            SELECT u.username, u.email
            FROM usr u
            WHERE u.workspace_id = $1
              AND u.disabled = false
              AND u.username = ANY($2::text[])
              AND (
                  u.is_admin = true
                  OR EXISTS (
                      SELECT 1 FROM usr_to_group g
                      WHERE g.workspace_id = $1
                        AND g.group_ = $3
                        AND g.usr = u.username
                  )
              )
        "#,
        &parent,
        &unique_assignees,
        WM_DEPLOYERS_GROUP,
    )
    .fetch_all(&db)
    .await?;

    if assignee_rows.len() != unique_assignees.len() {
        return Err(Error::BadRequest(
            "All assignees must be admins or members of wm_deployers in the parent workspace"
                .to_string(),
        ));
    }

    let mut tx = db.begin().await?;

    let request_id_row = sqlx::query!(
        r#"
            INSERT INTO workspace_fork_deployment_request
                (source_workspace_id, fork_workspace_id, requested_by, requested_by_email)
            VALUES ($1, $2, $3, $4)
            RETURNING id, requested_at
        "#,
        parent,
        &w_id,
        AuditAuthorable::username(&authed),
        &authed.email,
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(db_err) if db_err.is_unique_violation() => Error::Generic(
            StatusCode::CONFLICT,
            "A deployment request is already open for this fork; cancel it first".to_string(),
        ),
        _ => Error::from(e),
    })?;

    let request_id = request_id_row.id;

    let assignee_usernames: Vec<String> =
        assignee_rows.iter().map(|r| r.username.clone()).collect();
    let assignee_emails: Vec<String> = assignee_rows.iter().map(|r| r.email.clone()).collect();
    sqlx::query!(
        r#"
            INSERT INTO workspace_fork_deployment_request_assignee (request_id, username, email)
            SELECT $1, u, e
            FROM UNNEST($2::text[], $3::text[]) AS t(u, e)
        "#,
        request_id,
        &assignee_usernames,
        &assignee_emails,
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "fork_deployment_request.create",
        ActionKind::Create,
        &w_id,
        Some(&request_id.to_string()),
        None,
    )
    .await?;

    tx.commit().await?;

    // Send a deployment-request email to each assignee.
    let base_url = (**BASE_URL.load()).clone();
    let subject = format!(
        "[Windmill] @{} requested a deployment on fork {w_id}",
        authed.username
    );
    let body_text = format!(
        "@{} asked you to deploy the fork {w_id} → {parent}.\n\nOpen the compare view: {base_url}/?workspace={w_id}",
        authed.username
    );
    for r in &assignee_rows {
        if r.email != authed.email {
            send_email_if_possible(&subject, &body_text, &r.email);
        }
    }

    let assignees: Vec<DeploymentRequestAssignee> = assignee_rows
        .into_iter()
        .map(|r| DeploymentRequestAssignee { username: r.username, email: r.email })
        .collect();

    Ok(Json(DeploymentRequest {
        id: request_id,
        source_workspace_id: parent,
        fork_workspace_id: w_id,
        requested_by: authed.username.clone(),
        requested_by_email: authed.email.clone(),
        requested_at: request_id_row.requested_at,
        assignees,
        comments: vec![],
    }))
}

/// Cancel an open deployment request. Only the original requester or an
/// admin may cancel — anyone else gets 403.
async fn cancel_deployment_request(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, id)): Path<(String, i64)>,
) -> Result<String> {
    let row = sqlx::query!(
        "SELECT requested_by, closed_at FROM workspace_fork_deployment_request WHERE id = $1 AND fork_workspace_id = $2",
        id,
        &w_id,
    )
    .fetch_optional(&db)
    .await?
    .ok_or_else(|| Error::NotFound(format!("deployment request {id} not found")))?;

    if row.closed_at.is_some() {
        return Err(Error::BadRequest(
            "Deployment request is already closed".to_string(),
        ));
    }

    if !authed.is_admin && row.requested_by != authed.username {
        return Err(Error::NotAuthorized(
            "Only the requester or an admin can cancel this request".to_string(),
        ));
    }

    let mut tx = db.begin().await?;
    let rows_affected = sqlx::query!(
        "UPDATE workspace_fork_deployment_request SET closed_at = now(), closed_reason = 'cancelled' WHERE id = $1 AND closed_at IS NULL",
        id,
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();

    if rows_affected == 0 {
        // Lost the race to another cancel / close_merged call.
        return Ok("already-closed".to_string());
    }

    audit_log(
        &mut *tx,
        &authed,
        "fork_deployment_request.cancel",
        ActionKind::Update,
        &w_id,
        Some(&id.to_string()),
        None,
    )
    .await?;
    tx.commit().await?;

    // Notify assignees that the request was cancelled.
    let assignees = sqlx::query!(
        "SELECT email FROM workspace_fork_deployment_request_assignee WHERE request_id = $1",
        id,
    )
    .fetch_all(&db)
    .await?;
    let subject = format!("[Windmill] Deployment request on fork {w_id} cancelled");
    let body_text = format!(
        "@{} cancelled the open deployment request on fork {w_id}.",
        authed.username
    );
    for r in assignees {
        if r.email != authed.email {
            send_email_if_possible(&subject, &body_text, &r.email);
        }
    }

    Ok("ok".to_string())
}

/// Called by the UI after a successful merge loop. Closes the open
/// deployment request for this fork and marks every comment obsolete. Only
/// admins and members of wm_deployers *in the parent workspace* may close —
/// same set that can actually merge. `authed.groups` here reflects the fork
/// workspace's groups, so we query the parent explicitly.
async fn close_deployment_request_merged(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, id)): Path<(String, i64)>,
) -> Result<String> {
    let row = sqlx::query!(
        "SELECT source_workspace_id, requested_by, requested_by_email, closed_at FROM workspace_fork_deployment_request WHERE id = $1 AND fork_workspace_id = $2",
        id,
        &w_id,
    )
    .fetch_optional(&db)
    .await?
    .ok_or_else(|| Error::NotFound(format!("deployment request {id} not found")))?;

    // Check deployer membership against the parent workspace, not the fork.
    let is_parent_deployer = sqlx::query_scalar!(
        r#"
            SELECT EXISTS(
                SELECT 1
                FROM usr u
                WHERE u.workspace_id = $1
                  AND u.email = $2
                  AND u.disabled = false
                  AND (
                      u.is_admin = true
                      OR EXISTS (
                          SELECT 1 FROM usr_to_group g
                          WHERE g.workspace_id = $1
                            AND g.group_ = $3
                            AND g.usr = u.username
                      )
                  )
            ) as "exists!"
        "#,
        row.source_workspace_id,
        &authed.email,
        WM_DEPLOYERS_GROUP,
    )
    .fetch_one(&db)
    .await?;

    if !authed.is_admin && !is_parent_deployer {
        return Err(Error::NotAuthorized(
            "Only admins or members of wm_deployers in the parent workspace can close a deployment request as merged"
                .to_string(),
        ));
    }

    let mut tx = db.begin().await?;
    let rows_affected = sqlx::query!(
        "UPDATE workspace_fork_deployment_request SET closed_at = now(), closed_reason = 'merged' WHERE id = $1 AND closed_at IS NULL",
        id,
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();

    if rows_affected == 0 {
        return Ok("already-closed".to_string());
    }

    sqlx::query!(
        "UPDATE workspace_fork_deployment_request_comment SET obsolete = true WHERE request_id = $1 AND obsolete = false",
        id,
    )
    .execute(&mut *tx)
    .await?;
    audit_log(
        &mut *tx,
        &authed,
        "fork_deployment_request.close_merged",
        ActionKind::Update,
        &w_id,
        Some(&id.to_string()),
        None,
    )
    .await?;
    tx.commit().await?;

    // Notify the requester + every assignee that the request was merged.
    // Skip the merger themselves to avoid self-ping.
    let assignee_emails: Vec<String> = sqlx::query_scalar!(
        "SELECT email FROM workspace_fork_deployment_request_assignee WHERE request_id = $1",
        id,
    )
    .fetch_all(&db)
    .await?;
    let mut recipients: BTreeSet<String> = assignee_emails.into_iter().collect();
    recipients.insert(row.requested_by_email);
    recipients.remove(&authed.email);
    let subject = format!("[Windmill] Deployment request on fork {w_id} merged");
    let base_url = (**BASE_URL.load()).clone();
    let body_text = format!(
        "@{} merged the deployment request from @{} on fork {w_id}.\n\n{base_url}/?workspace={w_id}",
        authed.username, row.requested_by
    );
    for email in recipients {
        send_email_if_possible(&subject, &body_text, &email);
    }

    Ok("ok".to_string())
}

/// Append a comment to an open deployment request. Can be a top-level
/// comment or a reply (set `parent_id`, must target a top-level comment),
/// and can be general or anchored to a diff row. Anyone with access to the
/// fork workspace can comment; ACL is enforced by the bearer-token →
/// workspace membership check higher up.
async fn create_deployment_request_comment(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, id)): Path<(String, i64)>,
    Json(body): Json<CreateDeploymentRequestCommentBody>,
) -> JsonResult<DeploymentRequestComment> {
    if body.body.trim().is_empty() {
        return Err(Error::BadRequest("Comment body is empty".to_string()));
    }
    if body.anchor_kind.is_some() != body.anchor_path.is_some() {
        return Err(Error::BadRequest(
            "anchor_kind and anchor_path must both be set or both omitted".to_string(),
        ));
    }

    let req = sqlx::query!(
        "SELECT id, source_workspace_id, closed_at, requested_by_email FROM workspace_fork_deployment_request WHERE id = $1 AND fork_workspace_id = $2",
        id,
        &w_id,
    )
    .fetch_optional(&db)
    .await?
    .ok_or_else(|| Error::NotFound(format!("deployment request {id} not found")))?;

    if req.closed_at.is_some() {
        return Err(Error::BadRequest(
            "Cannot comment on a closed deployment request".to_string(),
        ));
    }

    // If anchor is set, validate the (kind, path) exists on the workspace_diff
    // for this (source, fork) pair. This prevents comments against phantom
    // rows.
    if let (Some(kind), Some(path)) = (body.anchor_kind.as_ref(), body.anchor_path.as_ref()) {
        let exists: bool = sqlx::query_scalar!(
            r#"
                SELECT EXISTS(
                    SELECT 1 FROM workspace_diff
                    WHERE source_workspace_id = $1
                      AND fork_workspace_id = $2
                      AND kind = $3
                      AND path = $4
                ) as "exists!"
            "#,
            req.source_workspace_id,
            &w_id,
            kind,
            path,
        )
        .fetch_one(&db)
        .await?;
        if !exists {
            return Err(Error::BadRequest(format!(
                "No diff row exists for {kind}:{path} in this fork",
            )));
        }
    }

    // If this is a reply, look up the parent comment — must exist on the
    // same request, must itself be top-level. The parent's author_email is
    // captured here so we can include them in the notification recipients.
    let parent_author_email: Option<String> = if let Some(parent_id) = body.parent_id {
        let parent_row = sqlx::query!(
            "SELECT parent_id, author_email FROM workspace_fork_deployment_request_comment WHERE id = $1 AND request_id = $2",
            parent_id,
            id,
        )
        .fetch_optional(&db)
        .await?
        .ok_or_else(|| {
            Error::BadRequest("parent_id does not belong to this request".to_string())
        })?;
        if parent_row.parent_id.is_some() {
            return Err(Error::BadRequest(
                "Replies can only target top-level comments".to_string(),
            ));
        }
        Some(parent_row.author_email)
    } else {
        None
    };

    let mut tx = db.begin().await?;
    let row = sqlx::query!(
        r#"
            INSERT INTO workspace_fork_deployment_request_comment
                (request_id, parent_id, author, author_email, body, anchor_kind, anchor_path)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id, created_at
        "#,
        id,
        body.parent_id,
        AuditAuthorable::username(&authed),
        &authed.email,
        &body.body,
        body.anchor_kind,
        body.anchor_path,
    )
    .fetch_one(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "fork_deployment_request.comment",
        ActionKind::Create,
        &w_id,
        Some(&row.id.to_string()),
        None,
    )
    .await?;
    tx.commit().await?;

    // Recipient set: requester + every assignee + (for replies) the
    // parent-comment author, minus the comment author.
    let assignee_emails: Vec<String> = sqlx::query_scalar!(
        "SELECT email FROM workspace_fork_deployment_request_assignee WHERE request_id = $1",
        id,
    )
    .fetch_all(&db)
    .await?;

    let mut recipients: BTreeSet<String> = assignee_emails.into_iter().collect();
    recipients.insert(req.requested_by_email.clone());
    if let Some(e) = parent_author_email {
        recipients.insert(e);
    }
    recipients.remove(&authed.email);

    let subject = format!(
        "[Windmill] New comment on deployment request for fork {w_id} by @{}",
        authed.username
    );
    let base_url = (**BASE_URL.load()).clone();
    let body_text = format!(
        "@{} commented on the deployment request for fork {w_id}:\n\n{}\n\n{base_url}/?workspace={w_id}",
        authed.username, body.body
    );
    for email in recipients {
        send_email_if_possible(&subject, &body_text, &email);
    }

    Ok(Json(DeploymentRequestComment {
        id: row.id,
        parent_id: body.parent_id,
        author: authed.username.clone(),
        author_email: authed.email.clone(),
        body: body.body,
        anchor_kind: body.anchor_kind,
        anchor_path: body.anchor_path,
        obsolete: false,
        created_at: row.created_at,
    }))
}

// ---- helpers ------------------------------------------------------------

async fn parent_of_fork(db: &DB, w_id: &str) -> Result<String> {
    sqlx::query_scalar!(
        "SELECT parent_workspace_id FROM workspace WHERE id = $1",
        w_id,
    )
    .fetch_optional(db)
    .await?
    .flatten()
    .ok_or_else(|| {
        Error::BadRequest(format!(
            "workspace {w_id} is not a fork (no parent_workspace_id)"
        ))
    })
}
