/*
 * Author: Windmill Labs, Inc
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use crate::types::{HasPath, StandardTriggerQuery, TriggerData, TriggerMode};
use async_trait::async_trait;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sql_builder::{bind::Bind, SqlBuilder};
use sqlx::{FromRow, PgConnection};
use std::fmt::Debug;
use windmill_api_auth::{build_scope_path_predicate, check_scopes, ApiAuthed};
use windmill_common::{
    db::UserDB,
    error::{Error, JsonResult, Result},
    user_drafts::{
        delete_all_drafts_for_path, delete_own_draft_for_path, fetch_draft_only_list_rows,
        overlay_or_draft_only, UserDraftItemKind, WithDraftOverlay, WithDraftQuery,
    },
    utils::{paginate, Pagination, StripPath},
    worker::CLOUD_HOSTED,
    DB,
};
use windmill_git_sync::DeployedObject;

use axum::{
    extract::{Path, Query},
    http::StatusCode,
    routing::{delete, get, post},
    Extension, Json, Router,
};
use std::sync::Arc;
use windmill_audit::{audit_oss::audit_log, ActionKind};
use windmill_git_sync::handle_deployment_metadata;

/// True when the workspace is a fork (`parent_workspace_id IS NOT NULL`).
///
/// Operational state (`mode`) belongs to the parent workspace: a git-sync /
/// merge / clone / UI-create write into a fork must never set it. On create we
/// force `disabled` so a fork trigger can't compete with the parent's listener;
/// on update we preserve the fork's existing value. `setmode` is the intended
/// explicit mutator of a fork's mode (and carries its own conflict warning) —
/// runtime error handling may still auto-disable an errored trigger, which is
/// orthogonal to this rule. This is the write half whose read half lives in
/// `workspaces_export.rs` (parent-value substitution on fork export), and it is
/// the single authority shared by both the git-sync round-trip and the in-app
/// compare-workspaces merge.
async fn workspace_is_fork(db: &DB, workspace_id: &str) -> Result<bool> {
    let is_fork: Option<bool> =
        sqlx::query_scalar("SELECT parent_workspace_id IS NOT NULL FROM workspace WHERE id = $1")
            .bind(workspace_id)
            .fetch_optional(db)
            .await?;
    Ok(is_fork.unwrap_or(false))
}

#[async_trait]
pub trait TriggerCrud: Send + Sync + 'static {
    type Trigger: Serialize
        + DeserializeOwned
        + for<'r> FromRow<'r, sqlx::postgres::PgRow>
        + Send
        + Sync
        + Unpin
        // Path accessor so `list_triggers` can apply scoped-token filtering.
        + HasPath
        // `'static` so the deployed trigger can be boxed into
        // `WithDraftOverlay`'s erased-serde inner (it's an owned row).
        + 'static;

    type TriggerConfig: Debug
        + DeserializeOwned
        + for<'r> FromRow<'r, sqlx::postgres::PgRow>
        + Serialize
        + Send
        + Sync
        + Unpin;

    type TriggerConfigRequest: Debug + DeserializeOwned + Serialize + Send + Sync;
    type TestConnectionConfig: Debug + DeserializeOwned + Serialize + Send + Sync;

    /// Table name used in dynamic SQL queries via `format!()`. This is a compile-time
    /// constant set by each trigger impl — it is never user-controllable.
    const TABLE_NAME: &'static str;
    const TRIGGER_TYPE: &'static str;
    /// `UserDraftItemKind` for this trigger's per-user `draft` rows. Required (no
    /// default) so a trigger that forgets it is a compile error, not a runtime panic.
    const DRAFT_KIND: UserDraftItemKind;
    const SUPPORTS_SERVER_STATE: bool;
    const SUPPORTS_TEST_CONNECTION: bool;
    const ROUTE_PREFIX: &'static str;
    const DEPLOYMENT_NAME: &'static str;
    const ADDITIONAL_SELECT_FIELDS: &[&'static str] = &[];
    const IS_ALLOWED_ON_CLOUD: bool;
    /// Whether enabling this trigger in a fork while the parent has the same
    /// path enabled is a real conflict (shared upstream resource). True for
    /// listener-based kinds where two consumers compete (Kafka group, PG slot,
    /// SQS queue, etc.) and for Websocket where both subscribers fire on every
    /// broadcast. False for kinds whose upstream identifier is implicitly
    /// workspace-scoped at runtime (HTTP routes, Email local_part — clones for
    /// the non-workspaced sub-case are filtered out, so any cloned row is
    /// already collision-free vs. the parent).
    const FORK_CONFLICT_ON_ENABLE: bool = true;

    fn get_deployed_object(path: String, parent_path: Option<String>) -> DeployedObject;

    async fn validate_new(
        &self,
        db: &DB,
        workspace_id: &str,
        new: &Self::TriggerConfigRequest,
    ) -> Result<()> {
        self.validate_config(db, new, workspace_id).await
    }

    async fn validate_edit(
        &self,
        db: &DB,
        workspace_id: &str,
        edit: &Self::TriggerConfigRequest,
        _path: &str,
    ) -> Result<()> {
        self.validate_config(db, edit, workspace_id).await
    }

    async fn validate_config(
        &self,
        _db: &DB,
        _config: &Self::TriggerConfigRequest,
        _workspace_id: &str,
    ) -> Result<()> {
        Ok(())
    }

    fn scope_domain_name() -> &'static str {
        &Self::ROUTE_PREFIX[1..]
    }

    /// Accessor for `DRAFT_KIND` used at the draft-lookup call sites.
    fn user_draft_item_kind() -> UserDraftItemKind {
        Self::DRAFT_KIND
    }

    async fn create_trigger(
        &self,
        db: &DB,
        tx: &mut PgConnection,
        authed: &ApiAuthed,
        w_id: &str,
        trigger: TriggerData<Self::TriggerConfigRequest>,
    ) -> Result<()>;

    async fn update_trigger(
        &self,
        db: &DB,
        tx: &mut PgConnection,
        authed: &ApiAuthed,
        workspace_id: &str,
        path: &str,
        trigger: TriggerData<Self::TriggerConfigRequest>,
    ) -> Result<()>;

    async fn test_connection(
        &self,
        _db: &DB,
        _authed: &ApiAuthed,
        _user_db: &UserDB,
        _workspace_id: &str,
        _config: Self::TestConnectionConfig,
    ) -> Result<()> {
        Err(
            anyhow::anyhow!("Test connection not supported for this trigger type".to_string(),)
                .into(),
        )
    }

    fn additional_routes(&self) -> axum::Router {
        axum::Router::new()
    }

    async fn get_trigger_by_path(
        &self,
        tx: &mut PgConnection,
        workspace_id: &str,
        path: &str,
    ) -> Result<Self::Trigger> {
        let mut fields = vec![
            "workspace_id",
            "path",
            "script_path",
            "is_flow",
            "edited_by",
            "permissioned_as",
            "edited_at",
            "extra_perms",
            "mode",
            "labels",
        ];

        if Self::SUPPORTS_SERVER_STATE {
            fields.extend_from_slice(&["server_id", "last_server_ping", "error"]);
        }

        fields.extend_from_slice(&["error_handler_path", "error_handler_args", "retry"]);
        fields.extend_from_slice(Self::ADDITIONAL_SELECT_FIELDS);

        let sql = format!(
            r#"SELECT
                {}
            FROM
                {}
            WHERE
                workspace_id = $1 AND
                path = $2
            "#,
            fields.join(", "),
            Self::TABLE_NAME
        );

        sqlx::query_as(&sql)
            .bind(workspace_id)
            .bind(path)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| Error::NotFound(format!("Trigger not found at path: {}", path)))
    }

    async fn exists(&self, db: &DB, workspace_id: &str, path: &str) -> Result<bool> {
        // SAFETY: Self::TABLE_NAME is a compile-time constant, not user input.
        let exists = sqlx::query_scalar(&format!(
            "SELECT EXISTS(SELECT 1 FROM {} WHERE workspace_id = $1 AND path = $2)",
            Self::TABLE_NAME
        ))
        .bind(workspace_id)
        .bind(path)
        .fetch_one(db)
        .await?;

        Ok(exists)
    }

    async fn delete_by_path(
        &self,
        tx: &mut PgConnection,
        workspace_id: &str,
        path: &str,
    ) -> Result<bool> {
        // SAFETY: Self::TABLE_NAME is a compile-time constant, not user input.
        let deleted = sqlx::query(&format!(
            "DELETE FROM {} WHERE workspace_id = $1 AND path = $2",
            Self::TABLE_NAME
        ))
        .bind(workspace_id)
        .bind(path)
        .execute(&mut *tx)
        .await?
        .rows_affected();

        Ok(deleted > 0)
    }

    async fn set_trigger_mode_extra_action(&self, _: &mut PgConnection) -> Result<()> {
        Ok(())
    }

    async fn set_trigger_mode(
        &self,
        authed: &ApiAuthed,
        tx: &mut PgConnection,
        workspace_id: &str,
        path: &str,
        mode: &TriggerMode,
    ) -> Result<bool> {
        let permissioned_as = windmill_common::users::username_to_permissioned_as(&authed.username);
        let updated = if Self::SUPPORTS_SERVER_STATE {
            // SAFETY: Self::TABLE_NAME is a compile-time constant.
            sqlx::query(&format!(
                r#"
                UPDATE
                    {}
                SET
                    mode = $1,
                    permissioned_as = $2,
                    edited_by = $3,
                    edited_at = now(),
                    server_id = NULL,
                    error = NULL
                WHERE
                    workspace_id = $4 AND
                    path = $5
                "#,
                Self::TABLE_NAME
            ))
            .bind(mode)
            .bind(&permissioned_as)
            .bind(&authed.username)
            .bind(workspace_id)
            .bind(path)
            .execute(&mut *tx)
            .await?
            .rows_affected()
        } else {
            // SAFETY: Self::TABLE_NAME is a compile-time constant.
            sqlx::query(&format!(
                r#"
                UPDATE
                    {}
                SET
                    mode = $1,
                    permissioned_as = $2,
                    edited_by = $3,
                    edited_at = now()
                WHERE
                    workspace_id = $4 AND
                    path = $5
                "#,
                Self::TABLE_NAME
            ))
            .bind(mode)
            .bind(&permissioned_as)
            .bind(&authed.username)
            .bind(workspace_id)
            .bind(path)
            .execute(&mut *tx)
            .await?
            .rows_affected()
        };

        self.set_trigger_mode_extra_action(&mut *tx).await?;

        Ok(updated > 0)
    }

    #[allow(unused)]
    async fn trigger_count(
        &self,
        tx: &mut PgConnection,
        workspace_id: &str,
        is_flow: bool,
        script_path: &str,
    ) -> i64 {
        // SAFETY: Self::TABLE_NAME is a compile-time constant.
        let count = sqlx::query_scalar(&format!(
            r#"
                SELECT
                    COUNT(*)
                FROM
                    {}
                WHERE
                    workspace_id = $1 AND
                    is_flow = $2 AND
                    script_path = $3
            "#,
            Self::TABLE_NAME
        ))
        .bind(workspace_id)
        .bind(is_flow)
        .bind(script_path)
        .fetch_one(&mut *tx)
        .await
        .unwrap_or(0);

        count
    }

    /// `authed_email = Some` adds the per-user `is_draft` flag (scalar EXISTS);
    /// `None` (e.g. workspace export) leaves it omitted.
    async fn list_triggers(
        &self,
        tx: &mut PgConnection,
        workspace_id: &str,
        query: Option<&StandardTriggerQuery>,
        authed_email: Option<&str>,
    ) -> Result<Vec<Self::Trigger>> {
        let mut fields = vec![
            "workspace_id",
            "path",
            "script_path",
            "is_flow",
            "edited_by",
            "permissioned_as",
            "edited_at",
            "extra_perms",
            "mode",
            "labels",
        ];

        if Self::SUPPORTS_SERVER_STATE {
            fields.extend_from_slice(&["server_id", "last_server_ping", "error"]);
        }

        fields.extend_from_slice(&["error_handler_path", "error_handler_args", "retry"]);
        fields.extend_from_slice(Self::ADDITIONAL_SELECT_FIELDS);

        let mut sqlb = SqlBuilder::select_from(Self::TABLE_NAME);

        sqlb.fields(&fields)
            .order_by("edited_at", true)
            .and_where("workspace_id = ?".bind(&workspace_id));

        if let Some(email) = authed_email {
            // SAFETY: interpolated TABLE_NAME and draft kind are compile-time constants; email is bound.
            sqlb.field(
                &format!(
                    "EXISTS(SELECT 1 FROM draft WHERE draft.workspace_id = {t}.workspace_id \
                     AND draft.path = {t}.path AND draft.typ = '{k}' AND draft.email = ?) as is_draft",
                    t = Self::TABLE_NAME,
                    k = Self::user_draft_item_kind().as_str(),
                )
                .bind(&email),
            );
        }

        if let Some(query) = query {
            let (per_page, offset) =
                paginate(Pagination { per_page: query.per_page, page: query.page });
            if let Some(path) = &query.path {
                sqlb.and_where_eq("script_path", "?".bind(path));
            }

            if let Some(is_flow) = query.is_flow {
                sqlb.and_where_eq("is_flow", "?".bind(&is_flow));
            }

            if let Some(path_start) = &query.path_start {
                sqlb.and_where_like_left("path", path_start);
            }

            if let Some(label) = &query.label {
                for l in label.split(',') {
                    sqlb.and_where("labels @> ARRAY[?]".bind(&l.trim()));
                }
            }

            sqlb.offset(offset).limit(per_page);
        }

        let sql = sqlb
            .sql()
            .map_err(|e| Error::InternalErr(format!("SQL error: {}", e)))?;

        let triggers = sqlx::query_as(&sql).fetch_all(&mut *tx).await?;

        Ok(triggers)
    }
}

pub fn trigger_routes<T: TriggerCrud + 'static>() -> Router {
    let mut router = Router::new()
        .route("/create", post(create_trigger::<T>))
        .route("/list", get(list_triggers::<T>))
        .route("/get/{*path}", get(get_trigger::<T>))
        .route("/update/{*path}", post(update_trigger::<T>))
        .route("/delete/{*path}", delete(delete_trigger::<T>))
        .route("/exists/{*path}", get(exists_trigger::<T>))
        .route("/setmode/{*path}", post(set_trigger_mode::<T>));

    if T::SUPPORTS_TEST_CONNECTION {
        router = router.route("/test", post(test_connection::<T>));
    }

    router
}

async fn create_trigger<T: TriggerCrud>(
    Extension(handler): Extension<Arc<T>>,
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(workspace_id): Path<String>,
    Json(mut new_trigger): Json<TriggerData<T::TriggerConfigRequest>>,
) -> Result<(StatusCode, String)> {
    check_scopes(&authed, || {
        format!(
            "{}:write:{}",
            T::scope_domain_name(),
            &new_trigger.base.path
        )
    })?;

    if *CLOUD_HOSTED && !T::IS_ALLOWED_ON_CLOUD {
        return Err(Error::BadRequest(format!(
            "{} triggers are not supported on multi-tenant cloud, use dedicated cloud or self-host",
            T::TRIGGER_TYPE
        )));
    }

    handler
        .validate_new(&db, &workspace_id, &new_trigger.config)
        .await?;

    let mut tx = user_db.begin(&authed).await?;

    // Writing into a fork never sets operational state: force `disabled` so a
    // cloned / synced / merged / UI-created trigger can't compete with the
    // parent's listener. The fork owner re-enables locally via `setmode`.
    if workspace_is_fork(&db, &workspace_id).await? {
        new_trigger.base.set_mode(TriggerMode::Disabled);
    }

    let new_path = new_trigger.base.path.clone();
    let labels = new_trigger.base.labels.clone();

    // If the caller did not preserve a value but the user can preserve, fall back
    // to the folder's default_permissioned_as rule (create-time only).
    let explicit_preserve = new_trigger.base.permissioned_as.is_some()
        && new_trigger.base.preserve_permissioned_as.unwrap_or(false)
        && windmill_common::can_preserve_on_behalf_of(&authed);
    if !explicit_preserve && windmill_common::can_preserve_on_behalf_of(&authed) {
        if let Some(default) = windmill_common::folders::resolve_folder_default_permissioned_as(
            &db,
            &workspace_id,
            &new_trigger.base.path,
        )
        .await?
        {
            new_trigger.base.permissioned_as = Some(default);
            new_trigger.base.preserve_permissioned_as = Some(true);
        }
    }

    let on_behalf_of_info = windmill_common::check_on_behalf_of_preservation(
        new_trigger.base.permissioned_as.as_deref(),
        new_trigger.base.preserve_permissioned_as.unwrap_or(false),
        &authed,
        &authed.username,
    );

    handler
        .create_trigger(&db, &mut *tx, &authed, &workspace_id, new_trigger)
        .await?;

    if let Some(ref labels) = labels {
        // SAFETY: T::TABLE_NAME is a compile-time constant.
        sqlx::query(&format!(
            "UPDATE {} SET labels = $1 WHERE workspace_id = $2 AND path = $3",
            T::TABLE_NAME
        ))
        .bind(labels)
        .bind(&workspace_id)
        .bind(&new_path)
        .execute(&mut *tx)
        .await?;
    }

    audit_log(
        &mut *tx,
        &authed,
        &format!("{}_triggers.create", T::TRIGGER_TYPE),
        ActionKind::Create,
        &workspace_id,
        Some(&new_path),
        None,
    )
    .await?;
    if let Some(on_behalf_of) = on_behalf_of_info {
        audit_log(
            &mut *tx,
            &authed,
            &format!("{}_triggers.on_behalf_of", T::TRIGGER_TYPE),
            ActionKind::Create,
            &workspace_id,
            Some(&new_path),
            Some(
                [
                    ("on_behalf_of", on_behalf_of.as_str()),
                    ("action", "create"),
                ]
                .into(),
            ),
        )
        .await?;
    }

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &workspace_id,
        T::get_deployed_object(new_path.clone(), None),
        Some(format!("{} '{}' created", T::DEPLOYMENT_NAME, new_path)),
        true,
        None,
    )
    .await?;

    tx.commit().await?;

    Ok((StatusCode::CREATED, new_path))
}

async fn list_triggers<T: TriggerCrud>(
    Extension(handler): Extension<Arc<T>>,
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path(workspace_id): Path<String>,
    Query(query): Query<StandardTriggerQuery>,
) -> JsonResult<Vec<T::Trigger>> {
    let mut tx = user_db.begin(&authed).await?;
    let mut triggers = handler
        .list_triggers(&mut *tx, &workspace_id, Some(&query), Some(&authed.email))
        .await?;
    tx.commit().await?;

    // Append the authed user's draft-only triggers of this kind; see scripts.rs.
    // Best-effort: the editor's TriggerData shape overlaps T::Trigger but a per-kind
    // config can deviate, so drop a row on deserialize failure rather than fail the list.
    if query.include_draft_only.unwrap_or(false)
        && !authed.is_operator
        && query.page.unwrap_or(0) == 0
        && query.path.is_none()
        && query.is_flow.is_none()
        && query.path_start.is_none()
        && query.label.is_none()
    {
        let draft_only_rows = fetch_draft_only_list_rows(
            &db,
            &workspace_id,
            &authed.email,
            T::user_draft_item_kind(),
        )
        .await?;

        for row in draft_only_rows {
            let created_at = row.created_at;
            let v: serde_json::Value = match serde_json::from_str(row.value.0.get()) {
                Ok(v) => v,
                Err(_) => continue,
            };
            let serde_json::Value::Object(mut map) = v else {
                continue;
            };
            // Fill operational fields the editor draft omits so the merged JSON matches
            // `Trigger<T::TriggerConfig>`'s flattened shape (mode derived from `enabled`).
            map.insert(
                "workspace_id".into(),
                serde_json::Value::String(workspace_id.clone()),
            );
            map.insert("edited_by".into(), serde_json::Value::String(String::new()));
            if let Ok(at) = serde_json::to_value(&created_at) {
                map.insert("edited_at".into(), at);
            }
            map.entry("permissioned_as")
                .or_insert(serde_json::Value::String(String::new()));
            map.entry("extra_perms").or_insert(serde_json::Value::Null);
            if !map.contains_key("mode") {
                let enabled = map.get("enabled").and_then(|x| x.as_bool()).unwrap_or(true);
                map.insert(
                    "mode".into(),
                    serde_json::Value::String(if enabled { "enabled" } else { "disabled" }.into()),
                );
            }
            map.insert("draft_only".into(), serde_json::Value::Bool(true));
            // Synthesized rows are the authed user's draft.
            map.insert("is_draft".into(), serde_json::Value::Bool(true));
            match serde_json::from_value::<T::Trigger>(serde_json::Value::Object(map)) {
                Ok(t) => triggers.push(t),
                Err(_) => continue,
            }
        }
    }

    let allowed = build_scope_path_predicate(&authed, T::scope_domain_name(), "read");
    triggers.retain(|t| allowed(t.trigger_path()));

    Ok(Json(triggers))
}

async fn get_trigger<T: TriggerCrud>(
    Extension(handler): Extension<Arc<T>>,
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((workspace_id, path)): Path<(String, StripPath)>,
    Query(q): Query<WithDraftQuery>,
) -> JsonResult<WithDraftOverlay> {
    let path = path.to_path();
    check_scopes(&authed, || {
        format!("{}:read:{}", T::scope_domain_name(), &path)
    })?;

    let mut tx = user_db.begin(&authed).await?;
    let trigger_res = handler
        .get_trigger_by_path(&mut *tx, &workspace_id, path)
        .await;
    tx.commit().await?;

    // Map "no deployed trigger" to `None` and let the shared choreography
    // handle the draft overlay / draft-only fallback / 404.
    let deployed = match trigger_res {
        Ok(t) => Some(t),
        Err(Error::NotFound(_)) => None,
        Err(e) => return Err(e),
    };

    let overlay = overlay_or_draft_only(
        &db,
        &workspace_id,
        &authed.email,
        T::user_draft_item_kind(),
        path,
        q.get_draft,
        deployed,
        || Error::NotFound(format!("Trigger not found at path: {}", path)),
    )
    .await?;
    Ok(Json(overlay))
}

async fn update_trigger<T: TriggerCrud>(
    Extension(handler): Extension<Arc<T>>,
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((workspace_id, path)): Path<(String, StripPath)>,
    Json(mut edit_trigger): Json<TriggerData<T::TriggerConfigRequest>>,
) -> Result<String> {
    let path = path.to_path();
    // A scoped token must be allowed on both the existing path (URL) and the
    // new path (body); checking only the latter would let it move a trigger it
    // can't touch into its scope.
    check_scopes(&authed, || {
        format!("{}:write:{}", T::scope_domain_name(), &path)
    })?;
    check_scopes(&authed, || {
        format!(
            "{}:write:{}",
            T::scope_domain_name(),
            &edit_trigger.base.path
        )
    })?;

    handler
        .validate_edit(&db, &workspace_id, &edit_trigger.config, path)
        .await?;

    let mut tx = user_db.begin(&authed).await?;

    // Preserve the existing DB `mode` instead of writing the incoming value
    // when either:
    //   * the target is a fork — a fork's operational state is fork-local and
    //     is never set through a git-sync/merge write (only via `setmode`); or
    //   * the request omits `mode`/`enabled` (legacy clients / YAML round-trip),
    //     where falling back to the BaseTriggerData default (Enabled) would flip
    //     the parent on a fork→parent merge.
    // Read half of the rule: parent-value substitution on fork export in
    // workspaces_export.rs.
    if workspace_is_fork(&db, &workspace_id).await? || edit_trigger.base.is_mode_unspecified() {
        let existing_mode: Option<TriggerMode> = sqlx::query_scalar(&format!(
            "SELECT mode FROM {} WHERE workspace_id = $1 AND path = $2",
            T::TABLE_NAME
        ))
        .bind(&workspace_id)
        .bind(path)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(m) = existing_mode {
            edit_trigger.base.set_mode(m);
        }
    }

    let new_path = edit_trigger.base.path.to_string();
    let labels = edit_trigger.base.labels.clone();
    let on_behalf_of_info = windmill_common::check_on_behalf_of_preservation(
        edit_trigger.base.permissioned_as.as_deref(),
        edit_trigger.base.preserve_permissioned_as.unwrap_or(false),
        &authed,
        &authed.username,
    );

    handler
        .update_trigger(&db, &mut *tx, &authed, &workspace_id, path, edit_trigger)
        .await?;

    if let Some(ref labels) = labels {
        // SAFETY: T::TABLE_NAME is a compile-time constant.
        sqlx::query(&format!(
            "UPDATE {} SET labels = $1 WHERE workspace_id = $2 AND path = $3",
            T::TABLE_NAME
        ))
        .bind(labels)
        .bind(&workspace_id)
        .bind(&new_path)
        .execute(&mut *tx)
        .await?;
    }

    audit_log(
        &mut *tx,
        &authed,
        &format!("{}_triggers.update", T::TRIGGER_TYPE),
        ActionKind::Update,
        &workspace_id,
        Some(&new_path),
        None,
    )
    .await?;
    if let Some(on_behalf_of) = on_behalf_of_info {
        audit_log(
            &mut *tx,
            &authed,
            &format!("{}_triggers.on_behalf_of", T::TRIGGER_TYPE),
            ActionKind::Update,
            &workspace_id,
            Some(&new_path),
            Some(
                [
                    ("on_behalf_of", on_behalf_of.as_str()),
                    ("action", "update"),
                ]
                .into(),
            ),
        )
        .await?;
    }

    let parent_path = if path != new_path {
        Some(path.to_string())
    } else {
        None
    };

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &workspace_id,
        T::get_deployed_object(new_path.clone(), parent_path),
        Some(format!("{} '{}' updated", T::DEPLOYMENT_NAME, new_path)),
        true,
        None,
    )
    .await?;

    tx.commit().await?;

    // On rename the old-path draft orphans (no SQL FK); clear the deployer's own
    // (+ legacy NULL) there, teammates keep theirs (StaleDraftModal). See scripts.rs.
    if path != new_path {
        delete_own_draft_for_path(
            &db,
            &workspace_id,
            T::user_draft_item_kind(),
            path,
            &authed.email,
        )
        .await?;
    }

    Ok(format!("Trigger '{}' updated", path))
}

async fn delete_trigger<T: TriggerCrud>(
    Extension(handler): Extension<Arc<T>>,
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((workspace_id, path)): Path<(String, StripPath)>,
) -> Result<String> {
    let path = path.to_path();
    check_scopes(&authed, || {
        format!("{}:write:{}", T::scope_domain_name(), &path)
    })?;

    let mut tx = user_db.begin(&authed).await?;

    // Capture trigger data for trashbin before deleting
    // SAFETY: T::TABLE_NAME is a compile-time constant.
    let trash_data: Option<serde_json::Value> = sqlx::query_scalar(&format!(
        "SELECT jsonb_build_object('row', to_jsonb(t), 'table_name', '{table}') FROM {table} t WHERE path = $1 AND workspace_id = $2",
        table = T::TABLE_NAME
    ))
    .bind(path)
    .bind(&workspace_id)
    .fetch_optional(&mut *tx)
    .await?;

    let deleted = handler
        .delete_by_path(&mut *tx, &workspace_id, path)
        .await?;

    if !deleted {
        return Err(Error::NotFound(format!(
            "Trigger not found at path: {}",
            path
        )));
    }

    if let Some(data) = trash_data {
        let item_kind = format!("{}_trigger", T::TRIGGER_TYPE);
        windmill_common::trashbin::move_to_trash(
            &mut *tx,
            &workspace_id,
            &item_kind,
            path,
            data,
            &authed.username,
        )
        .await?;
    }

    audit_log(
        &mut *tx,
        &authed,
        &format!("{}_triggers.delete", T::TRIGGER_TYPE),
        ActionKind::Delete,
        &workspace_id,
        Some(&path),
        None,
    )
    .await?;

    tx.commit().await?;

    // Reset the fork/parent workspace_diff tally for this path, exactly as
    // create/update and every other kind's delete does. Without this a deleted
    // trigger leaves its cached `has_changes=true` diff row behind: the compare
    // trusts it (triggers aren't re-validated like scripts/flows), then drops it
    // as it no longer exists in the table — a phantom "ahead" item that reads as
    // "changes not visible to your user" and hides the deploy button, even for
    // superadmins. Re-tallying sets has_changes=NULL so the next compare
    // re-evaluates and corrects/removes the row.
    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &workspace_id,
        T::get_deployed_object(path.to_string(), None),
        Some(format!("{} '{}' deleted", T::DEPLOYMENT_NAME, path)),
        true,
        None,
    )
    .await?;

    // Trigger gone for everyone: wipe ALL users' drafts at this path; see scripts.rs.
    delete_all_drafts_for_path(&db, &workspace_id, T::user_draft_item_kind(), path).await?;

    Ok(format!("Trigger '{}' deleted", path))
}

async fn exists_trigger<T: TriggerCrud>(
    Extension(handler): Extension<Arc<T>>,
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((workspace_id, path)): Path<(String, StripPath)>,
) -> JsonResult<bool> {
    let path = path.to_path();
    check_scopes(&authed, || {
        format!("{}:read:{}", T::scope_domain_name(), path)
    })?;
    let exists = handler.exists(&db, &workspace_id, path).await?;

    Ok(Json(exists))
}

#[derive(serde::Deserialize)]
struct SetTriggerModePayload {
    mode: TriggerMode,
    /// When true, bypass the parent-state warning that would otherwise reject
    /// enabling a trigger that's already enabled in the parent workspace.
    /// The frontend sets this after the user confirms the duplicate-execution
    /// dialog. See windmill-trigger/src/handler.rs::set_trigger_mode for the
    /// full check.
    #[serde(default)]
    force: bool,
}

/// Returns the parent workspace id when this workspace is a fork *and* the
/// parent has a row at the same trigger path. Used to gate enabling a trigger
/// in a fork behind an explicit `force=true` confirmation: the fork's row was
/// cloned from the parent, so its upstream identifier (Kafka group, PG slot,
/// SQS queue URL, etc.) is shared by construction. The risk is independent of
/// the parent's current `mode`: if the parent is enabled, the two listeners
/// compete; if it's disabled, the fork can destructively take over shared
/// state (e.g. advance the PG WAL, claim an MQTT client_id) before the parent
/// re-enables. Either way, the user should be asked to confirm.
async fn parent_has_trigger(
    tx: &mut PgConnection,
    table_name: &str,
    workspace_id: &str,
    path: &str,
) -> Result<Option<String>> {
    let parent: Option<String> =
        sqlx::query_scalar("SELECT parent_workspace_id FROM workspace WHERE id = $1")
            .bind(workspace_id)
            .fetch_optional(&mut *tx)
            .await?
            .flatten();
    let Some(parent_id) = parent else {
        return Ok(None);
    };
    let exists: Option<bool> = sqlx::query_scalar(&format!(
        "SELECT EXISTS(SELECT 1 FROM {} WHERE workspace_id = $1 AND path = $2)",
        table_name
    ))
    .bind(&parent_id)
    .bind(path)
    .fetch_one(&mut *tx)
    .await?;
    Ok(if exists == Some(true) {
        Some(parent_id)
    } else {
        None
    })
}

async fn set_trigger_mode<T: TriggerCrud>(
    Extension(handler): Extension<Arc<T>>,
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((workspace_id, path)): Path<(String, StripPath)>,
    Json(payload): Json<SetTriggerModePayload>,
) -> Result<String> {
    let path = path.to_path();
    check_scopes(&authed, || format!("{}:write", T::scope_domain_name()))?;

    let mut tx = user_db.begin(&authed).await?;

    // Block transitioning a trigger in a fork to any mode that attaches a
    // listener (Enabled or Suspended) when the parent has the same path,
    // unless the caller passes force=true. Suspended still keeps the
    // listener attached — it just stops auto-running queued jobs — so a
    // suspended fork would still split Kafka events / share a PG slot
    // with the parent. The cloned upstream identifier is shared by
    // construction; the risk is independent of the parent's current mode.
    // Skipped for kinds where the upstream identifier is already
    // workspace-scoped at runtime (HTTP, Email).
    if T::FORK_CONFLICT_ON_ENABLE && payload.mode != TriggerMode::Disabled && !payload.force {
        if let Some(parent_id) =
            parent_has_trigger(&mut *tx, T::TABLE_NAME, &workspace_id, path).await?
        {
            return Err(Error::BadRequest(format!(
                "fork-conflict:{}:{}",
                T::TRIGGER_TYPE,
                parent_id
            )));
        }
    }

    let updated = handler
        .set_trigger_mode(&authed, &mut *tx, &workspace_id, path, &payload.mode)
        .await?;

    if !updated {
        return Err(Error::NotFound(format!(
            "Trigger not found at path: {}",
            path
        )));
    }

    tx.commit().await?;

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &workspace_id,
        T::get_deployed_object(path.to_owned(), None),
        Some(format!("{} trigger '{}' updated", T::DEPLOYMENT_NAME, path)),
        true,
        None,
    )
    .await?;

    Ok(format!(
        "Trigger '{}' {}",
        path,
        if payload.mode == TriggerMode::Enabled {
            "enabled"
        } else if payload.mode == TriggerMode::Disabled {
            "disabled"
        } else {
            "suspended"
        }
    ))
}

async fn test_connection<T: TriggerCrud>(
    Extension(handler): Extension<Arc<T>>,
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(workspace_id): Path<String>,
    Json(config): Json<T::TestConnectionConfig>,
) -> Result<()> {
    // Test connection opens an outbound connection to a caller-supplied target,
    // so gate it behind write access like the other mutating trigger routes.
    check_scopes(&authed, || format!("{}:write", T::scope_domain_name()))?;

    let connect_f = async move {
        handler
            .test_connection(&db, &authed, &user_db, &workspace_id, config)
            .await
    };

    tokio::time::timeout(tokio::time::Duration::from_secs(30), connect_f)
        .await
        .map_err(|_| {
            Error::BadConfig(format!("Timeout connecting to service after 30 seconds"))
        })??;
    Ok(())
}

#[allow(unused)]
pub fn complete_trigger_routes<T: TriggerCrud + 'static>(handler: T) -> Router {
    let standard_routes = trigger_routes::<T>();

    let additional_routes = handler.additional_routes();

    standard_routes
        .merge(additional_routes)
        .layer(Extension(Arc::new(handler)))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TriggerPrimarySchedule {
    pub schedule: String,
}

// generate_trigger_routers(), get_triggers_count_internal(), TriggersCount stay in windmill-api
