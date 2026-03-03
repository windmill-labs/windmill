/*
 * Author: Windmill Labs, Inc
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use crate::types::{StandardTriggerQuery, TriggerData, TriggerMode};
use async_trait::async_trait;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sql_builder::{bind::Bind, SqlBuilder};
use sqlx::{FromRow, PgConnection};
use std::fmt::Debug;
use windmill_api_auth::{check_scopes, ApiAuthed};
use windmill_common::{
    db::UserDB,
    error::{Error, JsonResult, Result},
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

#[async_trait]
pub trait TriggerCrud: Send + Sync + 'static {
    type Trigger: Serialize
        + DeserializeOwned
        + for<'r> FromRow<'r, sqlx::postgres::PgRow>
        + Send
        + Sync
        + Unpin;

    type TriggerConfig: Debug
        + DeserializeOwned
        + for<'r> FromRow<'r, sqlx::postgres::PgRow>
        + Serialize
        + Send
        + Sync
        + Unpin;

    type TriggerConfigRequest: Debug + DeserializeOwned + Serialize + Send + Sync;
    type TestConnectionConfig: Debug + DeserializeOwned + Serialize + Send + Sync;

    const TABLE_NAME: &'static str;
    const TRIGGER_TYPE: &'static str;
    const SUPPORTS_SERVER_STATE: bool;
    const SUPPORTS_TEST_CONNECTION: bool;
    const ROUTE_PREFIX: &'static str;
    const DEPLOYMENT_NAME: &'static str;
    const ADDITIONAL_SELECT_FIELDS: &[&'static str] = &[];
    const IS_ALLOWED_ON_CLOUD: bool;

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
            "email",
            "edited_at",
            "extra_perms",
            "mode",
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
        let updated = if Self::SUPPORTS_SERVER_STATE {
            sqlx::query(&format!(
                r#"
                UPDATE
                    {}
                SET
                    mode = $1,
                    email = $2,
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
            .bind(&authed.email)
            .bind(&authed.username)
            .bind(workspace_id)
            .bind(path)
            .execute(&mut *tx)
            .await?
            .rows_affected()
        } else {
            sqlx::query(&format!(
                r#"
                UPDATE
                    {}
                SET
                    mode = $1,
                    email = $2,
                    edited_by = $3,
                    edited_at = now()
                WHERE
                    workspace_id = $4 AND
                    path = $5
                "#,
                Self::TABLE_NAME
            ))
            .bind(mode)
            .bind(&authed.email)
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

    async fn list_triggers(
        &self,
        tx: &mut PgConnection,
        workspace_id: &str,
        query: Option<&StandardTriggerQuery>,
    ) -> Result<Vec<Self::Trigger>> {
        let mut fields = vec![
            "workspace_id",
            "path",
            "script_path",
            "is_flow",
            "edited_by",
            "email",
            "edited_at",
            "extra_perms",
            "mode",
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
        .route("/get/*path", get(get_trigger::<T>))
        .route("/update/*path", post(update_trigger::<T>))
        .route("/delete/*path", delete(delete_trigger::<T>))
        .route("/exists/*path", get(exists_trigger::<T>))
        .route("/setmode/*path", post(set_trigger_mode::<T>));

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
    Json(new_trigger): Json<TriggerData<T::TriggerConfigRequest>>,
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

    let new_path = new_trigger.base.path.clone();
    let on_behalf_of_info = windmill_common::check_on_behalf_of_preservation(
        new_trigger.base.email.as_deref(),
        new_trigger.base.preserve_email.unwrap_or(false),
        &authed,
        &authed.username,
    );

    handler
        .create_trigger(&db, &mut *tx, &authed, &workspace_id, new_trigger)
        .await?;

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
    Path(workspace_id): Path<String>,
    Query(query): Query<StandardTriggerQuery>,
) -> JsonResult<Vec<T::Trigger>> {
    let mut tx = user_db.begin(&authed).await?;
    let triggers = handler
        .list_triggers(&mut *tx, &workspace_id, Some(&query))
        .await?;
    tx.commit().await?;

    Ok(Json(triggers))
}

async fn get_trigger<T: TriggerCrud>(
    Extension(handler): Extension<Arc<T>>,
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((workspace_id, path)): Path<(String, StripPath)>,
) -> JsonResult<T::Trigger> {
    let path = path.to_path();
    check_scopes(&authed, || {
        format!("{}:read:{}", T::scope_domain_name(), &path)
    })?;

    let mut tx = user_db.begin(&authed).await?;
    let trigger = handler
        .get_trigger_by_path(&mut *tx, &workspace_id, path)
        .await?;

    tx.commit().await?;

    Ok(Json(trigger))
}

async fn update_trigger<T: TriggerCrud>(
    Extension(handler): Extension<Arc<T>>,
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((workspace_id, path)): Path<(String, StripPath)>,
    Json(edit_trigger): Json<TriggerData<T::TriggerConfigRequest>>,
) -> Result<String> {
    let path = path.to_path();
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

    let new_path = edit_trigger.base.path.to_string();
    let on_behalf_of_info = windmill_common::check_on_behalf_of_preservation(
        edit_trigger.base.email.as_deref(),
        edit_trigger.base.preserve_email.unwrap_or(false),
        &authed,
        &authed.username,
    );

    handler
        .update_trigger(&db, &mut *tx, &authed, &workspace_id, path, edit_trigger)
        .await?;

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

    Ok(format!("Trigger '{}' updated", path))
}

async fn delete_trigger<T: TriggerCrud>(
    Extension(handler): Extension<Arc<T>>,
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((workspace_id, path)): Path<(String, StripPath)>,
) -> Result<String> {
    let path = path.to_path();
    check_scopes(&authed, || {
        format!("{}:write:{}", T::scope_domain_name(), &path)
    })?;

    let mut tx = user_db.begin(&authed).await?;
    let deleted = handler
        .delete_by_path(&mut *tx, &workspace_id, path)
        .await?;

    if !deleted {
        return Err(Error::NotFound(format!(
            "Trigger not found at path: {}",
            path
        )));
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
