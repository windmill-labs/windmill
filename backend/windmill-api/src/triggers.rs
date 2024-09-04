use axum::{
    extract::{Path, Query},
    routing::{get, post},
    Extension, Json, Router,
};
use http::StatusCode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use windmill_audit::{audit_ee::audit_log, ActionKind};
use windmill_common::{
    db::UserDB,
    error,
    utils::{not_found_if_none, StripPath},
    worker::to_raw_value,
};
use windmill_queue::PushArgsOwned;

use crate::{
    db::{ApiAuthed, DB},
    jobs::{run_flow_by_path_inner, run_script_by_path_inner, RunJobQuery},
};

pub fn routes_workspaced_service() -> Router {
    Router::new().route("/*path", post(route_job))
}

pub fn triggers_workspaced_service() -> Router {
    Router::new()
        .route("/create", post(create_trigger))
        .route("/list", post(list_triggers))
        .route("/:path", get(get_trigger))
        .route("/update/:path", post(update_trigger))
        .route("/delete/:path", post(delete_trigger))
}

#[derive(Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TRIGGER_KIND", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
enum TriggerKind {
    Http,
    Email,
}

#[derive(Deserialize)]
struct CreateTrigger {
    path: String,
    route_path: String,
    job_path: String,
    is_flow: bool,
    kind: TriggerKind,
}

#[derive(Serialize)]
struct Trigger {
    workspace_id: String,
    path: String,
    route_path: String,
    job_path: String,
    is_flow: bool,
    kind: TriggerKind,
    edited_by: String,
    edited_at: chrono::DateTime<chrono::Utc>,
    extra_perms: serde_json::Value,
}

#[derive(Deserialize)]
struct UpdateTrigger {
    route_path: String,
    job_path: String,
    is_flow: bool,
    kind: TriggerKind,
}

async fn list_triggers(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
) -> error::JsonResult<Vec<Trigger>> {
    let mut tx = user_db.begin(&authed).await?;
    let triggers = sqlx::query_as!(
        Trigger,
        r#"SELECT workspace_id, path, route_path, job_path, is_flow, kind as "kind: _", edited_by, edited_at, extra_perms FROM trigger WHERE workspace_id = $1"#,
        w_id
    )
    .fetch_all(&mut *tx)
    .await?;

    Ok(Json(triggers))
}

async fn get_trigger(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
) -> error::JsonResult<Trigger> {
    let mut tx = user_db.begin(&authed).await?;
    let trigger = sqlx::query_as!(
        Trigger,
        r#"SELECT workspace_id, path, route_path, job_path, is_flow, kind as "kind: _", edited_by, edited_at, extra_perms FROM trigger WHERE workspace_id = $1"#,
        w_id
    )
    .fetch_optional(&mut *tx)
    .await?;

    let trigger = not_found_if_none(trigger, "Trigger", w_id)?;

    Ok(Json(trigger))
}

async fn create_trigger(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(ct): Json<CreateTrigger>,
) -> error::Result<(StatusCode, String)> {
    let mut tx = user_db.begin(&authed).await?;
    sqlx::query!(
        "INSERT INTO trigger (workspace_id, path, route_path, job_path, is_flow, kind, edited_by, edited_at) VALUES ($1, $2, $3, $4, $5, $6, $7, now())",
        w_id,
        ct.path,
        ct.route_path,
        ct.job_path,
        ct.is_flow,
        ct.kind as TriggerKind,
        &authed.username
    )
    .execute(&mut *tx).await?;

    audit_log(
        &mut *tx,
        &authed,
        "triggers.create",
        ActionKind::Create,
        &w_id,
        Some(ct.path.as_str()),
        None,
    )
    .await?;

    tx.commit().await?;

    Ok((StatusCode::CREATED, format!("{}", ct.path)))
}

async fn update_trigger(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Json(ct): Json<UpdateTrigger>,
) -> error::Result<String> {
    let path = path.to_path();
    let mut tx = user_db.begin(&authed).await?;
    sqlx::query!(
        "UPDATE trigger SET route_path = $1, job_path = $2, is_flow = $3, kind = $4, edited_by = $5, edited_at = now() WHERE workspace_id = $6 AND path = $7",
        ct.route_path,
        ct.job_path,
        ct.is_flow,
        ct.kind as TriggerKind,
        &authed.username,
        w_id,
        path
    )
    .execute(&mut *tx).await?;

    audit_log(
        &mut *tx,
        &authed,
        "triggers.update",
        ActionKind::Create,
        &w_id,
        Some(path),
        None,
    )
    .await?;

    tx.commit().await?;

    Ok(path.to_string())
}

async fn delete_trigger(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> error::Result<String> {
    let path = path.to_path();
    let mut tx = user_db.begin(&authed).await?;
    sqlx::query!(
        "DELETE FROM trigger WHERE workspace_id = $1 AND path = $2",
        w_id,
        path
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "triggers.delete",
        ActionKind::Delete,
        &w_id,
        Some(path),
        None,
    )
    .await?;

    tx.commit().await?;

    Ok(format!("Trigger {path} deleted"))
}

async fn route_job(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Extension(rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,
    Path((w_id, route_path)): Path<(String, StripPath)>,
    Query(run_query): Query<RunJobQuery>,
    mut args: PushArgsOwned,
) -> error::Result<(StatusCode, String)> {
    let triggers = sqlx::query!(
        "SELECT job_path, is_flow, route_path FROM trigger WHERE workspace_id = $1 AND kind = 'http'",
        w_id
    )
    .fetch_all(&db)
    .await?;

    let mut router = matchit::Router::new();

    for trigger in triggers {
        let route_path = trigger.route_path.clone();

        router
            .insert(route_path.as_str(), trigger)
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to consider http trigger route {}: {:?}",
                    route_path,
                    e,
                );
            });
    }

    let route_path = format!("/{}", route_path.to_path());
    let trigger = router.at(&route_path).ok();

    let matchit::Match { value: trigger, params } =
        not_found_if_none(trigger, "Trigger", route_path.as_str())?;

    let params: HashMap<_, _> = params.iter().collect();
    let params = to_raw_value(&params);
    match &mut args.extra {
        Some(extra) => {
            extra.insert("path_params".to_string(), params);
        }
        None => {
            let extra = HashMap::from([("path_params".to_string(), params)]);
            args.extra = Some(extra);
        }
    }

    let label_prefix = Some("http-route-".to_string());
    if trigger.is_flow {
        run_flow_by_path_inner(
            authed,
            db,
            user_db,
            rsmq,
            w_id,
            StripPath(trigger.job_path.to_owned()),
            run_query,
            args,
            label_prefix,
        )
        .await
    } else {
        run_script_by_path_inner(
            authed,
            db,
            user_db,
            rsmq,
            w_id,
            StripPath(trigger.job_path.to_owned()),
            run_query,
            args,
            label_prefix,
        )
        .await
    }
}
