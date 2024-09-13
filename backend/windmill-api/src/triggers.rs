use axum::{
    extract::{FromRequestParts, Path, Query},
    response::IntoResponse,
    routing::{any, delete, get, post},
    Extension, Json, Router,
};
use http::{request::Parts, StatusCode};
use serde::{Deserialize, Serialize};
use sql_builder::{bind::Bind, SqlBuilder};
use sqlx::prelude::FromRow;
use std::collections::HashMap;
use windmill_audit::{audit_ee::audit_log, ActionKind};
use windmill_common::{
    auth::fetch_authed_from_permissioned_as,
    db::UserDB,
    error::{self, JsonResult},
    users::username_to_permissioned_as,
    utils::{not_found_if_none, paginate, require_admin, Pagination, StripPath},
    worker::{to_raw_value, CLOUD_HOSTED},
};
use windmill_queue::PushArgsOwned;

use crate::{
    db::{ApiAuthed, DB},
    jobs::{
        run_flow_by_path_inner, run_script_by_path_inner, run_wait_result_flow_by_path_internal,
        run_wait_result_script_by_path_internal, RunJobQuery,
    },
    users::OptAuthed,
};

pub fn routes_global_service() -> Router {
    Router::new().route("/*path", any(route_job))
}

pub fn triggers_workspaced_service() -> Router {
    Router::new()
        .route("/create", post(create_trigger))
        .route("/list/:kind", get(list_triggers))
        .route("/get/:kind/*path", get(get_trigger))
        .route("/update/:kind/*path", post(update_trigger))
        .route("/delete/:kind/*path", delete(delete_trigger))
        .route("/exists/:kind/*path", get(exists_trigger))
        .route("/used", get(used_trigger_kinds))
        .route("/route_exists", post(exists_route))
}

#[derive(Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TRIGGER_KIND", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum TriggerKind {
    Http,
    Email,
}

impl TriggerKind {
    pub fn as_str(&self) -> &str {
        match self {
            TriggerKind::Http => "http",
            TriggerKind::Email => "email",
        }
    }
}

#[derive(Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "HTTP_METHOD", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
}

impl From<HttpMethod> for http::Method {
    fn from(method: HttpMethod) -> Self {
        match method {
            HttpMethod::Get => http::Method::GET,
            HttpMethod::Post => http::Method::POST,
            HttpMethod::Put => http::Method::PUT,
            HttpMethod::Delete => http::Method::DELETE,
            HttpMethod::Patch => http::Method::PATCH,
        }
    }
}

#[derive(Deserialize)]
struct NewTrigger {
    path: String,
    route_path: String,
    script_path: String,
    is_flow: bool,
    kind: TriggerKind,
    summary: String,
    is_async: bool,
    requires_auth: bool,
    http_method: Option<HttpMethod>,
}

#[derive(FromRow, Serialize)]
struct Trigger {
    workspace_id: String,
    path: String,
    route_path: String,
    script_path: String,
    is_flow: bool,
    kind: TriggerKind,
    edited_by: String,
    email: String,
    edited_at: chrono::DateTime<chrono::Utc>,
    extra_perms: serde_json::Value,
    summary: String,
    is_async: bool,
    requires_auth: bool,
    http_method: Option<HttpMethod>,
}

#[derive(Deserialize)]
struct EditTrigger {
    path: String,
    route_path: Option<String>,
    script_path: String,
    is_flow: bool,
    kind: TriggerKind,
    summary: String,
    is_async: bool,
    requires_auth: bool,
    http_method: Option<HttpMethod>,
}

#[derive(Deserialize)]
pub struct ListTriggerQuery {
    pub page: Option<usize>,
    pub per_page: Option<usize>,
    pub path: Option<String>,
    pub is_flow: Option<bool>,
    pub path_start: Option<String>,
}

async fn list_triggers(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, kind)): Path<(String, TriggerKind)>,
    Query(lst): Query<(ListTriggerQuery)>,
) -> error::JsonResult<Vec<Trigger>> {
    let mut tx = user_db.begin(&authed).await?;
    let (per_page, offset) = paginate(Pagination { per_page: lst.per_page, page: lst.page });
    let mut sqlb = SqlBuilder::select_from("trigger")
        .field("*")
        .order_by("edited_at", true)
        .and_where("workspace_id = ?".bind(&w_id))
        .and_where_eq("kind", "?".bind(&kind.as_str()))
        .offset(offset)
        .limit(per_page)
        .clone();
    if let Some(path) = lst.path {
        sqlb.and_where_eq("script_path", "?".bind(&path));
    }
    if let Some(is_flow) = lst.is_flow {
        sqlb.and_where_eq("is_flow", "?".bind(&is_flow));
    }
    if let Some(path_start) = &lst.path_start {
        sqlb.and_where_like_left("path", path_start);
    }
    let sql = sqlb
        .sql()
        .map_err(|e| error::Error::InternalErr(e.to_string()))?;
    let rows = sqlx::query_as::<_, Trigger>(&sql)
        .fetch_all(&mut *tx)
        .await?;
    tx.commit().await?;

    Ok(Json(rows))
}

async fn get_trigger(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, kind, path)): Path<(String, TriggerKind, StripPath)>,
) -> error::JsonResult<Trigger> {
    let mut tx = user_db.begin(&authed).await?;
    let path = path.to_path();
    let trigger = sqlx::query_as!(
        Trigger,
        r#"SELECT workspace_id, path, route_path, script_path, is_flow, kind as "kind: _", http_method as "http_method: _", edited_by, email, edited_at, extra_perms, summary, is_async, requires_auth
            FROM trigger
            WHERE workspace_id = $1 AND path = $2 AND kind = $3"#,
        w_id,
        path,
        kind as TriggerKind
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;

    let trigger = not_found_if_none(trigger, "Trigger", path)?;

    Ok(Json(trigger))
}

async fn create_trigger(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(ct): Json<NewTrigger>,
) -> error::Result<(StatusCode, String)> {
    require_admin(authed.is_admin, &authed.username)?;
    let mut tx = user_db.begin(&authed).await?;
    sqlx::query!(
        "INSERT INTO trigger (workspace_id, summary, path, route_path, script_path, is_flow, kind, is_async, requires_auth, http_method, edited_by, email, edited_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, now())",
        w_id,
        ct.summary,
        ct.path,
        ct.route_path,
        ct.script_path,
        ct.is_flow,
        ct.kind as TriggerKind,
        ct.is_async,
        ct.requires_auth,
        ct.http_method as Option<HttpMethod>,
        &authed.username,
        &authed.email
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
    Path((w_id, kind, path)): Path<(String, TriggerKind, StripPath)>,
    Json(ct): Json<EditTrigger>,
) -> error::Result<String> {
    let path = path.to_path();
    let mut tx = user_db.begin(&authed).await?;
    if authed.is_admin {
        sqlx::query!(
            "UPDATE trigger 
                SET route_path = $1, script_path = $2, path = $3, is_flow = $4, kind = $5, http_method = $6, edited_by = $7, email = $8, summary = $9, edited_at = now() 
                WHERE workspace_id = $10 AND path = $11 AND kind = $12",
            ct.route_path,
            ct.script_path,
            ct.path,
            ct.is_flow,
            ct.kind as TriggerKind,
            ct.http_method as Option<HttpMethod>,
            &authed.username,
            &authed.email,
            ct.summary,
            w_id,
            path,
            kind as TriggerKind
        )
        .execute(&mut *tx).await?;
    } else {
        sqlx::query!(
            "UPDATE trigger SET script_path = $1, path = $2, is_flow = $3, kind = $4, http_method = $5, edited_by = $6, email = $7, summary = $8, is_async = $9, requires_auth = $10, edited_at = now() 
                WHERE workspace_id = $11 AND path = $12 AND kind = $13",
            ct.script_path,
            ct.path,
            ct.is_flow,
            ct.kind as TriggerKind,
            ct.http_method as Option<HttpMethod>,
            &authed.username,
            &authed.email,
            ct.summary,
            ct.is_async,
            ct.requires_auth,
            w_id,
            path,
            kind as TriggerKind
        )
        .execute(&mut *tx).await?;
    }

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
    Path((w_id, kind, path)): Path<(String, TriggerKind, StripPath)>,
) -> error::Result<String> {
    require_admin(authed.is_admin, &authed.username)?;
    let path = path.to_path();
    let mut tx = user_db.begin(&authed).await?;
    sqlx::query!(
        "DELETE FROM trigger WHERE workspace_id = $1 AND path = $2 AND kind = $3",
        w_id,
        path,
        kind as TriggerKind
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

async fn used_trigger_kinds(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> JsonResult<Vec<TriggerKind>> {
    let used_kinds = sqlx::query_scalar!(
        r#"SELECT DISTINCT kind as "kind: _" FROM trigger WHERE workspace_id = $1"#,
        w_id,
    )
    .fetch_all(&db)
    .await?;
    Ok(Json(used_kinds))
}

async fn exists_trigger(
    Extension(db): Extension<DB>,
    Path((w_id, kind, path)): Path<(String, TriggerKind, StripPath)>,
) -> JsonResult<bool> {
    let path = path.to_path();
    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM trigger WHERE path = $1 AND workspace_id = $2 AND kind = $3)",
        path,
        w_id,
        kind as TriggerKind
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(false);
    Ok(Json(exists))
}

#[derive(Deserialize)]
struct RouteExists {
    route_path: String,
    kind: TriggerKind,
}

async fn exists_route(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(RouteExists { route_path, kind }): Json<RouteExists>,
) -> JsonResult<bool> {
    let exists = if *CLOUD_HOSTED {
        sqlx::query_scalar!(
                    "SELECT EXISTS(SELECT 1 FROM trigger WHERE route_path = $1 AND workspace_id = $2 AND kind = $3)",
                    route_path,
                    w_id,
                    kind as TriggerKind
                )
                .fetch_one(&db)
                .await?
                .unwrap_or(false)
    } else {
        sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM trigger WHERE route_path = $1 AND kind = $2)",
            route_path,
            kind as TriggerKind
        )
        .fetch_one(&db)
        .await?
        .unwrap_or(false)
    };
    Ok(Json(exists))
}

struct TriggerRoute {
    script_path: String,
    is_flow: bool,
    route_path: String,
    workspace_id: String,
    is_async: bool,
    requires_auth: bool,
    edited_by: String,
    email: String,
    http_method: Option<HttpMethod>,
}

async fn fetch_api_authed(
    username: String,
    email: String,
    w_id: &str,
    db: &DB,
    username_override: String,
) -> error::Result<ApiAuthed> {
    let permissioned_as = username_to_permissioned_as(username.as_str());
    let authed =
        fetch_authed_from_permissioned_as(permissioned_as, email.clone(), w_id, db).await?;
    Ok(ApiAuthed {
        username: username,
        email: email,
        is_admin: authed.is_admin,
        is_operator: authed.is_operator,
        groups: authed.groups,
        folders: authed.folders,
        scopes: authed.scopes,
        username_override: Some(username_override),
    })
}

async fn get_http_route_trigger(
    route_path: &str,
    opt_authed: Option<ApiAuthed>,
    db: &DB,
    user_db: UserDB,
) -> error::Result<(TriggerRoute, HashMap<String, String>, ApiAuthed)> {
    let (mut triggers, route_path) = if *CLOUD_HOSTED {
        let mut splitted = route_path.split("/");
        let w_id = splitted.next().ok_or_else(|| {
            error::Error::BadRequest("Missing workspace id in route path".to_string())
        })?;
        let route_path = StripPath(splitted.collect::<Vec<_>>().join("/"));
        let triggers = sqlx::query_as!(
            TriggerRoute,
            r#"SELECT script_path, is_flow, route_path, workspace_id, is_async, requires_auth, edited_by, email, http_method as "http_method: _" FROM trigger WHERE workspace_id = $1 AND kind = 'http'"#,
            w_id
        )
        .fetch_all(db)
        .await?;
        (triggers, route_path)
    } else {
        let triggers = sqlx::query_as!(
            TriggerRoute,
            r#"SELECT script_path, is_flow, route_path, workspace_id, is_async, requires_auth, edited_by, email, http_method as "http_method: _" FROM trigger WHERE kind = 'http'"#,
        )
        .fetch_all(db)
        .await?;
        (triggers, StripPath(route_path.to_string()))
    };

    let mut router = matchit::Router::new();

    for (idx, trigger) in triggers.iter().enumerate() {
        let route_path = trigger.route_path.clone();
        router.insert(route_path.as_str(), idx).unwrap_or_else(|e| {
            tracing::warn!(
                "Failed to consider http trigger route {}: {:?}",
                route_path,
                e,
            );
        });
    }

    let trigger_idx = router.at(route_path.0.as_str()).ok();

    let matchit::Match { value: trigger_idx, params } =
        not_found_if_none(trigger_idx, "Trigger", route_path.0.as_str())?;

    let trigger = triggers.remove(trigger_idx.to_owned());

    let params: HashMap<String, String> = params
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

    let username_override = if trigger.requires_auth {
        if let Some(authed) = opt_authed {
            // check that the user has access to the trigger
            let mut tx = user_db.begin(&authed).await?;
            let exists = sqlx::query_scalar!(
                "SELECT EXISTS(SELECT 1 FROM trigger WHERE workspace_id = $1 AND path = $2)",
                trigger.workspace_id,
                trigger.route_path
            )
            .fetch_one(&mut *tx)
            .await?
            .unwrap_or(false);
            tx.commit().await?;
            if exists {
                Some(authed.display_username().to_owned())
            } else {
                return Err(error::Error::NotAuthorized("Unauthorized".to_string()));
            }
        } else {
            return Err(error::Error::NotAuthorized(
                "Requires authentication".to_string(),
            ));
        }
    } else {
        None
    };

    let authed = fetch_api_authed(
        trigger.edited_by.clone(),
        trigger.email.clone(),
        &trigger.workspace_id,
        &db,
        username_override.unwrap_or("anonymous".to_string()),
    )
    .await?;

    Ok((trigger, params, authed))
}

struct Method(http::Method);

#[axum::async_trait]
impl<S> FromRequestParts<S> for Method
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);
    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        let method = parts.method.clone();
        Ok(Method(method))
    }
}

async fn route_job(
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Extension(rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,
    Path(route_path): Path<StripPath>,
    Query(run_query): Query<RunJobQuery>,
    OptAuthed(opt_authed): OptAuthed,
    Method(method): Method,
    mut args: PushArgsOwned,
) -> impl IntoResponse {
    let route_path = route_path.to_path();
    let (trigger, params, authed) =
        match get_http_route_trigger(route_path, opt_authed, &db, user_db.clone()).await {
            Ok(trigger) => trigger,
            Err(e) => return e.into_response(),
        };

    let params = to_raw_value(&params);
    match args.extra.as_mut() {
        Some(extra) => {
            extra.insert("wm_path_params".to_string(), params);
            extra.insert("wm_route_path".to_string(), to_raw_value(&route_path));
        }
        None => {
            let extra = HashMap::from([
                ("wm_path_params".to_string(), params),
                ("wm_route_path".to_string(), to_raw_value(&route_path)),
            ]);
            args.extra = Some(extra);
        }
    }

    if let Some(http_method) = trigger.http_method {
        if http::Method::from(http_method) != method {
            return error::Error::BadRequest("Invalid HTTP method".to_string()).into_response();
        }
    }

    let label_prefix = Some(format!("http-route-{}-", trigger.route_path));

    if trigger.is_flow {
        if trigger.is_async {
            run_flow_by_path_inner(
                authed,
                db,
                user_db,
                rsmq,
                trigger.workspace_id.clone(),
                StripPath(trigger.script_path.to_owned()),
                run_query,
                args,
                label_prefix,
            )
            .await
            .into_response()
        } else {
            run_wait_result_flow_by_path_internal(
                db,
                run_query,
                StripPath(trigger.script_path.to_owned()),
                authed,
                rsmq,
                user_db,
                args,
                trigger.workspace_id.clone(),
                label_prefix,
            )
            .await
            .into_response()
        }
    } else {
        if trigger.is_async {
            run_script_by_path_inner(
                authed,
                db,
                user_db,
                rsmq,
                trigger.workspace_id.clone(),
                StripPath(trigger.script_path.to_owned()),
                run_query,
                args,
                label_prefix,
            )
            .await
            .into_response()
        } else {
            run_wait_result_script_by_path_internal(
                db,
                run_query,
                StripPath(trigger.script_path.to_owned()),
                authed,
                rsmq,
                user_db,
                trigger.workspace_id.clone(),
                args,
                label_prefix,
            )
            .await
            .into_response()
        }
    }
}
