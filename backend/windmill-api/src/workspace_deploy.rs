//! Copies one item from another workspace of this instance into the request's workspace, or
//! removes it there: the write a fork ↔ parent merge applies per item.
//!
//! Every read and write goes through the item's own route handler, so the checks those carry
//! (scopes, deploy rules, roles, `on_behalf_of` resolution, git sync, audit, the fork tally,
//! CI tests) apply as they do to a client calling the routes. What the router itself adds is
//! applied here instead: the caller's identity is resolved per workspace and per route by the
//! same extractor (so a token's workspace binding and route scopes hold), and the trigger and
//! schedule routers' operator write gate is checked before each of their writes.

use std::{collections::HashMap, sync::Arc};

use axum::{
    body::Bytes,
    extract::{FromRequest, FromRequestParts, Multipart, OriginalUri, Path, Query},
    http::{self, Method, StatusCode},
    Extension, Json,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Map, Value};
use windmill_api_auth::{AuthCache, Tokened};
use windmill_common::{
    db::UserDB,
    error::{to_anyhow, Error, JsonResult, Result},
    utils::StripPath,
    workspaces::{check_operator_can_manage, ManageKind},
    DB,
};
use tower::{Layer, ServiceExt};
use windmill_trigger::handler::TriggerCrud;

use crate::{apps, db::ApiAuthed, webhook_util::WebhookShared};
use windmill_api_flows::flows;
use windmill_api_groups::folders;
use windmill_api_schedule as schedules;
use windmill_api_scripts::scripts;
use windmill_api_workspaces::datatable_migrations as migrations;
use windmill_store::{resources, variables};
use windmill_trigger::handler as triggers;

#[derive(Deserialize)]
pub(crate) struct DeployFromWorkspace {
    kind: String,
    path: String,
    /// Where the item is read from. Unused when `delete` is set.
    source_workspace: Option<String>,
    /// Remove the item here (scripts and flows are archived) instead of copying it.
    #[serde(default)]
    delete: bool,
    /// Who the item runs as here, as the item kind stores it: an email for scripts, flows,
    /// apps and agents, a `u/`/`g/` principal for schedules and triggers. Unset makes it the
    /// caller.
    on_behalf_of: Option<String>,
    /// The `u/`/`g/` principal naming the same identity as `on_behalf_of`, when the caller
    /// picked a user. Unset leaves the handler to derive this workspace's own from the email.
    on_behalf_of_principal: Option<String>,
    /// Refuse with a 409 rather than overwrite when this workspace already has the item. A
    /// folder is the exception: it is created with its principals translated into this
    /// workspace's, and one that already exists is left alone and counts as done.
    #[serde(default)]
    create_only: bool,
    /// Hold back the CI tests the write would queue, for a caller deploying a batch.
    #[serde(default)]
    skip_ci_tests: bool,
    /// Root URL a GCP or Azure push subscription posts to. Defaults to this instance's.
    push_base_url: Option<String>,
}

#[derive(Serialize, Default)]
pub(crate) struct Deployed {
    /// Folder owners and ACL entries left out because they have no account here.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    dropped_access: Vec<String>,
}

pub(crate) async fn deploy_from_workspace(
    _authed: ApiAuthed,
    Tokened { token }: Tokened,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Extension(webhook): Extension<WebhookShared>,
    Extension(cache): Extension<Arc<AuthCache>>,
    Path(w_id): Path<String>,
    Json(req): Json<DeployFromWorkspace>,
) -> JsonResult<Deployed> {
    let cx = Cx { db, user_db, webhook, cache, token, to: w_id };
    if req.delete {
        cx.delete(&req.kind, &req.path).await?;
        return Ok(Json(Deployed::default()));
    }
    let from = req
        .source_workspace
        .clone()
        .ok_or_else(|| Error::BadRequest("source_workspace is required".to_string()))?;
    cx.deploy(&from, &req).await.map(Json)
}

struct Cx {
    db: DB,
    user_db: UserDB,
    webhook: WebhookShared,
    cache: Arc<AuthCache>,
    token: String,
    to: String,
}

fn at(w_id: &str, path: &str) -> Path<(String, StripPath)> {
    Path((w_id.to_string(), StripPath(path.to_string())))
}

fn q<T: DeserializeOwned>(query: &str) -> Result<Query<T>> {
    let uri: http::Uri = format!("/?{query}").parse().map_err(to_anyhow)?;
    Query::try_from_uri(&uri).map_err(|e| Error::internal_err(e.to_string()))
}

fn ci(skip: bool) -> &'static str {
    if skip {
        "skip_ci_tests=true"
    } else {
        ""
    }
}

fn val<T: Serialize>(x: T) -> Result<Value> {
    Ok(serde_json::to_value(x).map_err(to_anyhow)?)
}

fn body<T: DeserializeOwned>(v: Value) -> Result<Json<T>> {
    serde_json::from_value(v)
        .map(Json)
        .map_err(|e| Error::BadRequest(format!("item cannot be deployed as read: {e}")))
}

/// Sets `key`, or removes it for `None`: an absent field is how the handlers read "unset".
fn put(v: &mut Value, key: &str, x: Option<Value>) {
    if let Some(o) = v.as_object_mut() {
        match x {
            Some(x) => o.insert(key.to_string(), x),
            None => o.remove(key),
        };
    }
}

/// `keys` of `v` that are present, null included, as a new object.
fn pick(v: &Value, keys: &[&str]) -> Value {
    Value::Object(
        keys.iter()
            .filter_map(|k| v.get(*k).map(|x| (k.to_string(), x.clone())))
            .collect::<Map<_, _>>(),
    )
}

/// Sends `key` as null when the source has none: the handler keeps what an omitted field
/// leaves out.
fn send_unset(v: &mut Value, key: &str) {
    if let Some(o) = v.as_object_mut() {
        o.entry(key).or_insert(Value::Null);
    }
}

fn or(v: &Value, key: &str, default: Value) -> Value {
    v.get(key)
        .filter(|x| !x.is_null())
        .cloned()
        .unwrap_or(default)
}

fn conflict() -> Error {
    Error::Generic(
        StatusCode::CONFLICT,
        "item already exists in the target workspace".to_string(),
    )
}

/// A flow step pinned to a script hash names a version of the source workspace; cleared, the
/// target resolves it by path. Only steps are walked: a static input can hold any object,
/// including one shaped like a step.
fn clear_script_hashes(module: &mut Value) {
    let Some(v) = module.get_mut("value").and_then(Value::as_object_mut) else {
        return;
    };
    let steps: &[&str] = match v.get("type").and_then(Value::as_str) {
        Some("script") => {
            v.remove("hash");
            &[]
        }
        Some("forloopflow" | "whileloopflow") => &["modules"],
        Some("branchall" | "branchone") => &["branches", "default"],
        Some("aiagent") => &["tools"],
        _ => &[],
    };
    for key in steps {
        for x in v.get_mut(*key).and_then(Value::as_array_mut).into_iter().flatten() {
            // A branch holds its steps under `modules`; the others are steps themselves.
            match x.get_mut("modules").and_then(Value::as_array_mut) {
                Some(modules) => modules.iter_mut().for_each(clear_script_hashes),
                None => clear_script_hashes(x),
            }
        }
    }
}

fn parse_migration_path(path: &str) -> Result<(String, i64)> {
    let invalid = || Error::BadRequest(format!("Invalid datatable migration path: {path}"));
    let (datatable, rest) = path.split_once('/').ok_or_else(invalid)?;
    let (ts, _name) = rest.split_once('_').ok_or_else(invalid)?;
    Ok((datatable.to_string(), ts.parse().map_err(|_| invalid())?))
}

impl Cx {
    /// The caller as `w_id` sees them, resolved for `route` exactly as a request to it would be.
    async fn authed(&self, w_id: &str, method: Method, route: &str) -> Result<ApiAuthed> {
        let uri: http::Uri = format!("/api/w/{w_id}/{route}")
            .parse()
            .map_err(to_anyhow)?;
        let mut req = http::Request::builder()
            .method(method)
            .uri(uri.clone())
            .header(
                http::header::AUTHORIZATION,
                format!("Bearer {}", self.token),
            )
            .body(())
            .map_err(to_anyhow)?;
        req.extensions_mut().insert(self.cache.clone());
        req.extensions_mut().insert(OriginalUri(uri));
        let (mut parts, _) = req.into_parts();
        ApiAuthed::from_request_parts(&mut parts, &()).await
    }

    fn db(&self) -> Extension<DB> {
        Extension(self.db.clone())
    }
    fn udb(&self) -> Extension<UserDB> {
        Extension(self.user_db.clone())
    }
    fn hook(&self) -> Extension<WebhookShared> {
        Extension(self.webhook.clone())
    }
    fn to(&self) -> Path<String> {
        Path(self.to.clone())
    }

    async fn deploy(&self, from: &str, req: &DeployFromWorkspace) -> Result<Deployed> {
        let path = req.path.as_str();
        let obo = req.on_behalf_of.clone().map(Value::String);
        let principal = req.on_behalf_of_principal.clone().map(Value::String);
        let ci = ci(req.skip_ci_tests);
        // Script, flow and app bodies carry the identity the same way.
        let identity = |v: &mut Value| {
            put(v, "preserve_on_behalf_of", Some(json!(obo.is_some())));
            put(v, "on_behalf_of_email", obo.clone());
            // Usernames are per workspace: the source's principal names nobody here, or
            // someone else.
            put(v, "on_behalf_of", principal.clone());
        };
        match req.kind.as_str() {
            "script" => {
                let read = |w: String| async move {
                    let a = self
                        .authed(&w, Method::GET, &format!("scripts/get/p/{path}"))
                        .await?;
                    val(
                        scripts::get_script_by_path(a, self.udb(), self.db(), at(&w, path), q("")?)
                            .await?
                            .0,
                    )
                };
                let mut v = read(from.to_string()).await?;
                let exists = scripts::exists_script_by_path(self.db(), at(&self.to, path))
                    .await?
                    .0;
                if exists && req.create_only {
                    return Err(conflict());
                }
                // A parent makes the create a new version of the head here rather than a fork.
                let parent = if exists {
                    read(self.to.clone()).await?.get("hash").cloned()
                } else {
                    None
                };
                put(&mut v, "parent_hash", parent);
                send_unset(&mut v, "job_token_scopes");
                identity(&mut v);
                let a = self
                    .authed(&self.to, Method::POST, "scripts/create")
                    .await?;
                scripts::create_script(
                    a,
                    self.udb(),
                    self.hook(),
                    self.db(),
                    self.to(),
                    q(ci)?,
                    body(v)?,
                )
                .await?;
            }
            "flow" => {
                let a = self
                    .authed(from, Method::GET, &format!("flows/get/{path}"))
                    .await?;
                let mut v =
                    val(
                        flows::get_flow_by_path(a, self.udb(), self.db(), at(from, path), q("")?)
                            .await?
                            .0,
                    )?;
                if let Some(value) = v.get_mut("value") {
                    if let Some(modules) = value.get_mut("modules").and_then(Value::as_array_mut) {
                        modules.iter_mut().for_each(clear_script_hashes);
                    }
                    if let Some(failure) = value.get_mut("failure_module") {
                        clear_script_hashes(failure);
                    }
                }
                send_unset(&mut v, "job_token_scopes");
                identity(&mut v);
                if flows::exists_flow_by_path(self.db(), at(&self.to, path))
                    .await?
                    .0
                {
                    if req.create_only {
                        return Err(conflict());
                    }
                    let a = self
                        .authed(&self.to, Method::POST, &format!("flows/update/{path}"))
                        .await?;
                    flows::update_flow(
                        a,
                        self.udb(),
                        self.db(),
                        self.hook(),
                        at(&self.to, path),
                        q(ci)?,
                        body(v)?,
                    )
                    .await?;
                } else {
                    let a = self.authed(&self.to, Method::POST, "flows/create").await?;
                    flows::create_flow(
                        a,
                        self.db(),
                        self.udb(),
                        self.hook(),
                        self.to(),
                        q(ci)?,
                        body(v)?,
                    )
                    .await?;
                }
            }
            "app" | "raw_app" => self.deploy_app(from, req, &obo, &principal).await?,
            "variable" => {
                let a = self
                    .authed(from, Method::GET, &format!("variables/get/{path}"))
                    .await?;
                let read = |decrypt: bool| {
                    variables::get_variable(
                        a.clone(),
                        self.udb(),
                        self.db(),
                        q(&format!("decrypt_secret={decrypt}")).unwrap(),
                        at(from, path),
                    )
                };
                let mut v = val(read(false).await?.0)?;
                // An external secret reference is copied as the reference, never resolved.
                let external = v["value"]
                    .as_str()
                    .is_some_and(|s| s.starts_with("$external:"));
                if v["is_secret"].as_bool() == Some(true) && !external {
                    v = val(read(true).await?.0)?;
                }
                let mut b = pick(&v, &["is_secret", "labels"]);
                put(&mut b, "path", Some(json!(path)));
                put(&mut b, "value", Some(or(&v, "value", json!(""))));
                put(
                    &mut b,
                    "description",
                    Some(or(&v, "description", json!(""))),
                );
                // Null clears; absent would keep the target's stored date.
                put(
                    &mut b,
                    "value_valid_until",
                    Some(or(&v, "value_valid_until", Value::Null)),
                );
                if variables::exists_variable(self.db(), at(&self.to, path))
                    .await?
                    .0
                {
                    if req.create_only {
                        return Err(conflict());
                    }
                    let a = self
                        .authed(&self.to, Method::POST, &format!("variables/update/{path}"))
                        .await?;
                    variables::update_variable(
                        a,
                        self.db(),
                        self.udb(),
                        self.hook(),
                        at(&self.to, path),
                        q("already_encrypted=false")?,
                        body(b)?,
                    )
                    .await?;
                } else {
                    let a = self
                        .authed(&self.to, Method::POST, "variables/create")
                        .await?;
                    variables::create_variable(
                        a,
                        self.db(),
                        self.udb(),
                        self.hook(),
                        self.to(),
                        q("")?,
                        body(b)?,
                    )
                    .await?;
                }
            }
            "resource" => {
                let a = self
                    .authed(from, Method::GET, &format!("resources/get/{path}"))
                    .await?;
                let v =
                    val(
                        resources::get_resource(a, self.udb(), self.db(), at(from, path), q("")?)
                            .await?
                            .0,
                    )?;
                let mut value = or(&v, "value", json!(""));
                let mut b = pick(&v, &["resource_type", "labels"]);
                // An agent's value carries the principal it runs as, as a trigger's row does: the
                // source's names nobody here, so it is replaced by the chosen one or dropped,
                // leaving the handler to make it the deployer's.
                if v["resource_type"] == "ai_agent" && value.is_object() {
                    put(&mut value, "on_behalf_of", obo.clone());
                    put(&mut b, "preserve_on_behalf_of", Some(json!(obo.is_some())));
                }
                put(&mut b, "path", Some(json!(path)));
                put(&mut b, "value", Some(value));
                put(
                    &mut b,
                    "description",
                    Some(or(&v, "description", json!(""))),
                );
                if resources::exists_resource(self.db(), at(&self.to, path))
                    .await?
                    .0
                {
                    if req.create_only {
                        return Err(conflict());
                    }
                    let a = self
                        .authed(&self.to, Method::POST, &format!("resources/update/{path}"))
                        .await?;
                    resources::update_resource(
                        a,
                        self.db(),
                        self.udb(),
                        self.hook(),
                        at(&self.to, path),
                        q(ci)?,
                        body(b)?,
                    )
                    .await?;
                } else {
                    let a = self
                        .authed(&self.to, Method::POST, "resources/create")
                        .await?;
                    resources::create_resource(
                        a,
                        self.db(),
                        self.udb(),
                        self.hook(),
                        self.to(),
                        q(ci)?,
                        body(b)?,
                    )
                    .await?;
                }
            }
            "resource_type" => {
                let a = self
                    .authed(from, Method::GET, &format!("resources/type/get/{path}"))
                    .await?;
                let v = val(resources::get_resource_type(
                    a,
                    self.udb(),
                    Path((from.to_string(), path.to_string())),
                )
                .await?
                .0)?;
                let name = || Path((self.to.clone(), path.to_string()));
                if resources::exists_resource_type(self.db(), name()).await?.0 {
                    if req.create_only {
                        return Err(conflict());
                    }
                    // An omitted extension or name is left unchanged, so unset ones are sent as
                    // null to clear the target's.
                    let b = json!({
                        "schema": v["schema"],
                        "description": or(&v, "description", json!("")),
                        "is_fileset": or(&v, "is_fileset", json!(false)),
                        "format_extension": v["format_extension"],
                        "display_name": v["display_name"],
                    });
                    let a = self
                        .authed(
                            &self.to,
                            Method::POST,
                            &format!("resources/type/update/{path}"),
                        )
                        .await?;
                    resources::update_resource_type(
                        a,
                        self.db(),
                        self.udb(),
                        self.hook(),
                        name(),
                        body(b)?,
                    )
                    .await?;
                } else {
                    let mut b = pick(
                        &v,
                        &[
                            "name",
                            "schema",
                            "is_fileset",
                            "format_extension",
                            "display_name",
                        ],
                    );
                    put(
                        &mut b,
                        "description",
                        Some(or(&v, "description", json!(""))),
                    );
                    let a = self
                        .authed(&self.to, Method::POST, "resources/type/create")
                        .await?;
                    resources::create_resource_type(
                        a,
                        self.db(),
                        self.udb(),
                        self.hook(),
                        self.to(),
                        body(b)?,
                    )
                    .await?;
                }
            }
            "folder" => return self.deploy_folder(from, req).await,
            "schedule" => {
                let a = self
                    .authed(from, Method::GET, &format!("schedules/get/{path}"))
                    .await?;
                let v =
                    val(
                        schedules::get_schedule(a, self.udb(), self.db(), at(from, path), q("")?)
                            .await?
                            .0,
                    )?;
                let exists = schedules::exists_schedule(self.db(), at(&self.to, path))
                    .await?
                    .0;
                let b = self.runs_as(v, exists, req)?;
                if exists {
                    let a = self
                        .authed(&self.to, Method::POST, &format!("schedules/update/{path}"))
                        .await?;
                    check_operator_can_manage(&self.db, &self.to, &a, ManageKind::Schedules)
                        .await?;
                    schedules::edit_schedule(
                        a,
                        self.db(),
                        self.udb(),
                        at(&self.to, path),
                        body(b)?,
                    )
                    .await?;
                } else {
                    let a = self
                        .authed(&self.to, Method::POST, "schedules/create")
                        .await?;
                    check_operator_can_manage(&self.db, &self.to, &a, ManageKind::Schedules)
                        .await?;
                    schedules::create_schedule(a, self.db(), self.udb(), self.to(), body(b)?)
                        .await?;
                }
            }
            "datatable_migration" => {
                let (datatable, timestamp) = parse_migration_path(path)?;
                let a = self
                    .authed(from, Method::GET, "workspaces/list_datatable_migrations")
                    .await?;
                let m = migrations::list_datatable_migrations(a, self.db(), Path(from.to_string()))
                    .await?
                    .0
                    .into_iter()
                    .find(|m| m.datatable == datatable && m.timestamp == timestamp)
                    .ok_or_else(|| {
                        Error::NotFound(format!("Datatable migration {path} not found in {from}"))
                    })?;
                if req.create_only {
                    let a = self
                        .authed(
                            &self.to,
                            Method::GET,
                            "workspaces/list_datatable_migrations",
                        )
                        .await?;
                    let here = migrations::list_datatable_migrations(a, self.db(), self.to())
                        .await?
                        .0;
                    if here
                        .iter()
                        .any(|m| m.datatable == datatable && m.timestamp == timestamp)
                    {
                        return Err(conflict());
                    }
                }
                let a = self
                    .authed(
                        &self.to,
                        Method::POST,
                        &format!("workspaces/upsert_datatable_migration/{datatable}"),
                    )
                    .await?;
                let up = migrations::UpsertDatatableMigration {
                    timestamp: m.timestamp,
                    name: m.name,
                    code_up: m.code_up,
                    code_down: m.code_down,
                };
                migrations::upsert_datatable_migration(a, self.db(), Path((self.to.clone(), datatable.clone())), Json(up))
                    .await
                    .map_err(|e| match e.to_string() {
                        s if s.to_lowercase().contains("migrations are not enabled") => Error::BadRequest(format!(
                            "Data table '{datatable}' has not opted in to migrations on the target workspace; enable migrations for it there before deploying its migrations."
                        )),
                        _ => e,
                    })?;
            }
            kind => {
                return self
                    .trigger(kind, TriggerOp::Deploy { from, req })
                    .await
                    .map(|_| Deployed::default())
            }
        }
        Ok(Deployed::default())
    }

    /// A schedule's or trigger's body: who it runs as, and on update nothing of its operational
    /// state, which the target keeps.
    fn runs_as(&self, mut v: Value, exists: bool, req: &DeployFromWorkspace) -> Result<Value> {
        if exists && req.create_only {
            return Err(conflict());
        }
        if exists {
            put(&mut v, "enabled", None);
            put(&mut v, "mode", None);
        }
        put(
            &mut v,
            "permissioned_as",
            req.on_behalf_of.clone().map(Value::String),
        );
        put(
            &mut v,
            "preserve_permissioned_as",
            Some(json!(req.on_behalf_of.is_some())),
        );
        Ok(v)
    }

    async fn deploy_app(
        &self,
        from: &str,
        req: &DeployFromWorkspace,
        obo: &Option<Value>,
        principal: &Option<Value>,
    ) -> Result<()> {
        let path = req.path.as_str();
        let a = self
            .authed(from, Method::GET, &format!("apps/get/p/{path}"))
            .await?;
        let mut v = val(
            apps::get_app(a, self.udb(), self.db(), at(from, path), q("")?)
                .await?
                .0,
        )?;
        // The policy holds the app's identity in both formats; see `identity` in `deploy`. A
        // group travels as its synthetic `group-*@windmill.dev` address, which an admin-created
        // account holding it would win: known and accepted, see
        // `users::permissioned_as_from_email`.
        if let Some(policy) = v.get_mut("policy") {
            put(policy, "on_behalf_of", principal.clone());
            put(policy, "on_behalf_of_email", obo.clone());
        }
        put(&mut v, "preserve_on_behalf_of", Some(json!(obo.is_some())));
        let exists = apps::exists_app(self.db(), at(&self.to, path)).await?.0;
        if exists && req.create_only {
            return Err(conflict());
        }
        if v["raw_app"].as_bool() != Some(true) {
            if exists {
                let a = self
                    .authed(&self.to, Method::POST, &format!("apps/update/{path}"))
                    .await?;
                let _ = apps::update_app(
                    a,
                    self.db(),
                    self.udb(),
                    self.hook(),
                    at(&self.to, path),
                    body(v)?,
                )
                .await?;
            } else {
                let a = self.authed(&self.to, Method::POST, "apps/create").await?;
                apps::create_app(a, self.udb(), self.db(), self.hook(), self.to(), body(v)?)
                    .await?;
            }
            return Ok(());
        }
        let a = self
            .authed(
                from,
                Method::GET,
                &format!("apps/secret_of_latest_version/{path}"),
            )
            .await?;
        let secret =
            apps::get_latest_version_secret_id(a, self.udb(), self.db(), at(from, path)).await?;
        let js = self.bundle_part(from, &secret, "js").await?;
        let css = self.bundle_part(from, &secret, "css").await?;
        let multipart = raw_app_multipart(&v, js, css).await?;
        if exists {
            let a = self
                .authed(&self.to, Method::POST, &format!("apps/update_raw/{path}"))
                .await?;
            let _ = apps::update_app_raw(
                a,
                self.udb(),
                self.db(),
                self.hook(),
                at(&self.to, path),
                multipart,
            )
            .await?;
        } else {
            let a = self
                .authed(&self.to, Method::POST, "apps/create_raw")
                .await?;
            apps::create_app_raw(a, self.udb(), self.db(), self.hook(), self.to(), multipart)
                .await?;
        }
        Ok(())
    }

    /// One part of a raw app's compiled bundle. A bundle with no styles may have no `css` stored
    /// at all, which reads as empty; a missing `js` is a broken bundle.
    async fn bundle_part(&self, from: &str, secret: &str, ext: &str) -> Result<Bytes> {
        let res = apps::get_raw_app_data(
            Path((from.to_string(), format!("{secret}.{ext}"))),
            Query(HashMap::new()),
            self.db(),
        )
        .await;
        match res {
            Ok(r) if r.status().is_success() => axum::body::to_bytes(r.into_body(), usize::MAX)
                .await
                .map_err(to_anyhow)
                .map_err(Into::into),
            Ok(r) if ext == "css" && r.status() == StatusCode::NOT_FOUND => Ok(Bytes::new()),
            Err(Error::NotFound(_)) if ext == "css" => Ok(Bytes::new()),
            Ok(r) => Err(Error::internal_err(format!(
                "reading the app bundle's {ext}: {}",
                r.status()
            ))),
            Err(e) => Err(e),
        }
    }

    async fn deploy_folder(&self, from: &str, req: &DeployFromWorkspace) -> Result<Deployed> {
        let name = req.path.trim_start_matches("f/");
        let at_to = || Path((self.to.clone(), name.to_string()));
        let exists = folders::exists_folder(self.db(), at_to()).await?.0;
        if exists && req.create_only {
            return Ok(Deployed::default());
        }
        let a = self
            .authed(from, Method::GET, &format!("folders/get/{name}"))
            .await?;
        let f = folders::get_folder(a, self.udb(), Path((from.to_string(), name.to_string())))
            .await?
            .0;
        if exists {
            let mut b = json!({
                "display_name": f.display_name,
                "owners": f.owners,
                "extra_perms": f.extra_perms,
                // A folder stores cleared labels as NULL and reads back without the field, so an
                // absent value is sent as [] or the target keeps labels the source removed.
                "labels": f.labels.clone().unwrap_or_default(),
            });
            put(&mut b, "summary", f.summary.clone().map(Value::String));
            let a = self
                .authed(&self.to, Method::POST, &format!("folders/update/{name}"))
                .await?;
            folders::update_folder(a, self.db(), self.udb(), self.hook(), at_to(), body(b)?)
                .await?;
            return Ok(Deployed::default());
        }
        let mut b = json!({
            "name": name,
            "display_name": f.display_name,
            "owners": f.owners,
            "extra_perms": f.extra_perms,
        });
        put(&mut b, "summary", f.summary.clone().map(Value::String));
        put(&mut b, "labels", f.labels.clone().map(|l| json!(l)));
        let mut dropped_access = vec![];
        if req.create_only {
            dropped_access = self.translate_folder(from, &f, &mut b).await?;
        }
        let a = self
            .authed(&self.to, Method::POST, "folders/create")
            .await?;
        let created = folders::create_folder(
            a,
            Tokened { token: self.token.clone() },
            self.db(),
            self.udb(),
            self.hook(),
            Extension(self.cache.clone()),
            self.to(),
            body(b)?,
        )
        .await;
        match created {
            Ok(_) => Ok(Deployed { dropped_access }),
            // Created meanwhile: the folder exists, which is all a create-only caller needs. The
            // name conflict is not part of the handler's contract, so ask again.
            Err(_)
                if req.create_only
                    && folders::exists_folder(
                        self.db(),
                        Path((self.to.clone(), name.to_string())),
                    )
                    .await?
                    .0 =>
            {
                Ok(Deployed::default())
            }
            Err(e) => Err(e),
        }
    }

    /// Rewrite a folder's principals into this workspace's own naming, for a folder created to
    /// give items somewhere to land.
    ///
    /// A `u/<username>` is workspace-local, so a user is matched by email. An owner or ACL entry
    /// with no account here is dropped: the folder ends up more restrictive than its source,
    /// and its creator becomes an owner. An identity rule with no account here refuses the copy:
    /// dropped, items inside would run as their deployer, and kept, every deploy into the folder
    /// would be rejected.
    async fn translate_folder(
        &self,
        from: &str,
        f: &folders::Folder,
        b: &mut Value,
    ) -> Result<Vec<String>> {
        let users: HashMap<String, String> = sqlx::query!(
            r#"SELECT s.username AS "from!", t.username AS "to!" FROM usr s
               JOIN usr t ON t.email = s.email AND t.workspace_id = $2
               WHERE s.workspace_id = $1"#,
            from,
            &self.to
        )
        .fetch_all(&self.db)
        .await?
        .into_iter()
        .map(|r| (r.from, r.to))
        .collect();
        let emails: Vec<String> =
            sqlx::query_scalar!("SELECT email FROM usr WHERE workspace_id = $1", &self.to)
                .fetch_all(&self.db)
                .await?;
        let groups: Vec<String> =
            sqlx::query_scalar!("SELECT name FROM group_ WHERE workspace_id = $1", &self.to)
                .fetch_all(&self.db)
                .await?;
        let translate = |p: &str| -> Option<String> {
            if let Some(u) = p.strip_prefix("u/") {
                users.get(u).map(|t| format!("u/{t}"))
            } else if let Some(g) = p.strip_prefix("g/") {
                groups.iter().any(|x| x == g).then(|| p.to_string())
            } else {
                emails.iter().any(|x| x == p).then(|| p.to_string())
            }
        };

        let mut rules = f
            .default_permissioned_as
            .as_array()
            .cloned()
            .unwrap_or_default();
        for rule in rules.iter_mut() {
            let p = rule["permissioned_as"]
                .as_str()
                .unwrap_or_default()
                .to_string();
            let t = translate(&p).ok_or_else(|| {
                Error::BadRequest(format!(
                    "f/{} runs items on behalf of {p}, which has no account in the target workspace. Bring the folder across from the compare page first.",
                    f.name
                ))
            })?;
            rule["permissioned_as"] = json!(t);
        }
        let acl = f.extra_perms.as_object().cloned().unwrap_or_default();
        let dropped = f
            .owners
            .iter()
            .chain(acl.keys())
            .filter(|p| translate(p).is_none())
            .cloned()
            .collect();
        put(
            b,
            "owners",
            Some(json!(f
                .owners
                .iter()
                .filter_map(|p| translate(p))
                .collect::<Vec<_>>())),
        );
        put(
            b,
            "extra_perms",
            Some(Value::Object(
                acl.into_iter()
                    .filter_map(|(p, w)| translate(&p).map(|t| (t, w)))
                    .collect(),
            )),
        );
        put(b, "default_permissioned_as", Some(Value::Array(rules)));
        Ok(dropped)
    }

    async fn delete(&self, kind: &str, path: &str) -> Result<()> {
        match kind {
            "script" => {
                let a = self
                    .authed(&self.to, Method::POST, &format!("scripts/archive/p/{path}"))
                    .await?;
                scripts::archive_script_by_path(
                    a,
                    self.hook(),
                    self.udb(),
                    self.db(),
                    at(&self.to, path),
                )
                .await?;
            }
            "flow" => {
                let a = self
                    .authed(&self.to, Method::POST, &format!("flows/archive/{path}"))
                    .await?;
                flows::archive_flow_by_path(
                    a,
                    self.db(),
                    self.udb(),
                    self.hook(),
                    at(&self.to, path),
                    body(json!({ "archived": true }))?,
                )
                .await?;
            }
            "app" | "raw_app" => {
                let a = self
                    .authed(&self.to, Method::DELETE, &format!("apps/delete/{path}"))
                    .await?;
                apps::delete_app(a, self.db(), self.udb(), self.hook(), at(&self.to, path)).await?;
            }
            "variable" => {
                let a = self
                    .authed(
                        &self.to,
                        Method::DELETE,
                        &format!("variables/delete/{path}"),
                    )
                    .await?;
                variables::delete_variable(
                    a,
                    self.db(),
                    self.udb(),
                    self.hook(),
                    at(&self.to, path),
                )
                .await?;
            }
            "resource" => {
                let a = self
                    .authed(
                        &self.to,
                        Method::DELETE,
                        &format!("resources/delete/{path}"),
                    )
                    .await?;
                resources::delete_resource(
                    a,
                    self.db(),
                    self.udb(),
                    self.hook(),
                    at(&self.to, path),
                )
                .await?;
            }
            "resource_type" => {
                let a = self
                    .authed(
                        &self.to,
                        Method::DELETE,
                        &format!("resources/type/delete/{path}"),
                    )
                    .await?;
                resources::delete_resource_type(
                    a,
                    self.db(),
                    self.udb(),
                    self.hook(),
                    Path((self.to.clone(), path.to_string())),
                )
                .await?;
            }
            "folder" => {
                let name = path.trim_start_matches("f/");
                let a = self
                    .authed(&self.to, Method::DELETE, &format!("folders/delete/{name}"))
                    .await?;
                folders::delete_folder(
                    a,
                    self.db(),
                    self.udb(),
                    self.hook(),
                    Path((self.to.clone(), name.to_string())),
                )
                .await?;
            }
            "schedule" => {
                let a = self
                    .authed(
                        &self.to,
                        Method::DELETE,
                        &format!("schedules/delete/{path}"),
                    )
                    .await?;
                check_operator_can_manage(&self.db, &self.to, &a, ManageKind::Schedules).await?;
                schedules::delete_schedule(a, self.db(), self.udb(), at(&self.to, path)).await?;
            }
            "datatable_migration" => {
                let (datatable, timestamp) = parse_migration_path(path)?;
                let a = self
                    .authed(
                        &self.to,
                        Method::DELETE,
                        &format!("workspaces/delete_datatable_migration/{datatable}/{timestamp}"),
                    )
                    .await?;
                migrations::delete_datatable_migration(
                    a,
                    self.db(),
                    Path((self.to.clone(), datatable, timestamp)),
                )
                .await?;
            }
            kind => self.trigger(kind, TriggerOp::Delete { path }).await?,
        }
        Ok(())
    }

    async fn trigger(&self, kind: &str, op: TriggerOp<'_>) -> Result<()> {
        match kind {
            #[cfg(feature = "http_trigger")]
            "http_trigger" => {
                self.trigger_op(crate::triggers::http::HttpTrigger, op)
                    .await
            }
            #[cfg(feature = "websocket")]
            "websocket_trigger" => {
                self.trigger_op(crate::triggers::websocket::WebsocketTrigger, op)
                    .await
            }
            #[cfg(all(feature = "enterprise", feature = "kafka", feature = "private"))]
            "kafka_trigger" => {
                self.trigger_op(crate::triggers::kafka::KafkaTrigger, op)
                    .await
            }
            #[cfg(all(feature = "enterprise", feature = "nats", feature = "private"))]
            "nats_trigger" => {
                self.trigger_op(crate::triggers::nats::NatsTrigger, op)
                    .await
            }
            #[cfg(feature = "mqtt_trigger")]
            "mqtt_trigger" => {
                self.trigger_op(crate::triggers::mqtt::MqttTrigger, op)
                    .await
            }
            #[cfg(feature = "amqp_trigger")]
            "amqp_trigger" => {
                self.trigger_op(crate::triggers::amqp::AmqpTrigger, op)
                    .await
            }
            #[cfg(all(feature = "enterprise", feature = "sqs_trigger", feature = "private"))]
            "sqs_trigger" => self.trigger_op(crate::triggers::sqs::SqsTrigger, op).await,
            #[cfg(all(feature = "enterprise", feature = "gcp_trigger", feature = "private"))]
            "gcp_trigger" => self.trigger_op(crate::triggers::gcp::GcpTrigger, op).await,
            #[cfg(all(feature = "enterprise", feature = "azure_trigger", feature = "private"))]
            "azure_trigger" => {
                self.trigger_op(crate::triggers::azure::AzureTrigger, op)
                    .await
            }
            #[cfg(feature = "postgres_trigger")]
            "postgres_trigger" => {
                self.trigger_op(crate::triggers::postgres::PostgresTrigger, op)
                    .await
            }
            #[cfg(all(feature = "smtp", feature = "private"))]
            "email_trigger" => {
                self.trigger_op(crate::triggers::email::EmailTrigger, op)
                    .await
            }
            _ => {
                let _ = op;
                Err(Error::BadRequest(format!(
                    "Cannot deploy an item of kind {kind} on this instance"
                )))
            }
        }
    }

    #[allow(dead_code)]
    async fn trigger_op<T: TriggerCrud + 'static>(
        &self,
        handler: T,
        op: TriggerOp<'_>,
    ) -> Result<()> {
        let h = Extension(Arc::new(handler));
        let prefix = T::ROUTE_PREFIX.trim_start_matches('/');
        let (from, req) = match op {
            TriggerOp::Delete { path } => {
                let a = self
                    .authed(&self.to, Method::DELETE, &format!("{prefix}/delete/{path}"))
                    .await?;
                check_operator_can_manage(&self.db, &self.to, &a, ManageKind::Triggers).await?;
                triggers::delete_trigger::<T>(h, a, self.udb(), self.db(), at(&self.to, path))
                    .await?;
                return Ok(());
            }
            TriggerOp::Deploy { from, req } => (from, req),
        };
        let path = req.path.as_str();
        let a = self
            .authed(from, Method::GET, &format!("{prefix}/get/{path}"))
            .await?;
        let mut v = val(triggers::get_trigger::<T>(
            h.clone(),
            a,
            self.udb(),
            self.db(),
            at(from, path),
            q("")?,
        )
        .await?
        .0)?;
        let push_base = || {
            req.push_base_url
                .clone()
                .unwrap_or_else(|| windmill_common::BASE_URL.load().to_string())
        };
        if req.kind == "gcp_trigger" {
            // The target creates its own subscription; the source's id and push audience name
            // the source's.
            put(&mut v, "subscription_id", Some(json!("")));
            put(&mut v, "subscription_mode", Some(json!("create_update")));
            if v["delivery_config"].is_object() {
                put(&mut v["delivery_config"], "audience", Some(json!("")));
            } else {
                put(&mut v, "delivery_config", None);
            }
            let push = (v["delivery_type"] == "push").then(|| json!(push_base()));
            put(&mut v, "base_endpoint", push);
        } else if req.kind == "azure_trigger" {
            // Not stored on the trigger, but a push subscription is created from it.
            let push = (v["azure_mode"] != "namespace_pull").then(|| json!(push_base()));
            put(&mut v, "base_endpoint", push);
        }
        let a = self
            .authed(&self.to, Method::GET, &format!("{prefix}/exists/{path}"))
            .await?;
        let exists = triggers::exists_trigger::<T>(h.clone(), a, self.db(), at(&self.to, path))
            .await?
            .0;
        let b = self.runs_as(v, exists, req)?;
        if exists {
            let a = self
                .authed(&self.to, Method::POST, &format!("{prefix}/update/{path}"))
                .await?;
            check_operator_can_manage(&self.db, &self.to, &a, ManageKind::Triggers).await?;
            triggers::update_trigger::<T>(
                h,
                a,
                self.db(),
                self.udb(),
                at(&self.to, path),
                body(b)?,
            )
            .await?;
        } else {
            let a = self
                .authed(&self.to, Method::POST, &format!("{prefix}/create"))
                .await?;
            check_operator_can_manage(&self.db, &self.to, &a, ManageKind::Triggers).await?;
            triggers::create_trigger::<T>(h, a, self.db(), self.udb(), self.to(), body(b)?).await?;
        }
        Ok(())
    }
}

enum TriggerOp<'a> {
    Deploy { from: &'a str, req: &'a DeployFromWorkspace },
    Delete { path: &'a str },
}

async fn raw_app_multipart(app: &Value, js: Bytes, css: Bytes) -> Result<Multipart> {
    let boundary = format!("wm-{}", uuid::Uuid::new_v4().simple());
    let mut b = Vec::new();
    let app = serde_json::to_vec(app).map_err(to_anyhow)?;
    for (name, data) in [("app", &app[..]), ("js", &js[..]), ("css", &css[..])] {
        b.extend_from_slice(
            format!("--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n")
                .as_bytes(),
        );
        b.extend_from_slice(data);
        b.extend_from_slice(b"\r\n");
    }
    b.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    let req = http::Request::builder()
        .header(
            http::header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(axum::body::Body::from(b))
        .map_err(to_anyhow)?;
    // The bundle passed the raw-app routes' size limit when it was stored; axum's default
    // 2 MiB cap on an extracted body would refuse a larger one here.
    let extract = tower::service_fn(|req: http::Request<axum::body::Body>| async move {
        Multipart::from_request(req, &())
            .await
            .map_err(|e| Error::internal_err(e.to_string()))
    });
    axum::extract::DefaultBodyLimit::disable()
        .layer(extract)
        .oneshot(req)
        .await
}
