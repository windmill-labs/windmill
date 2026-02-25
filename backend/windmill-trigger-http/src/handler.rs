use super::{
    validate_authentication_method, HttpConfig, HttpConfigRequest, HttpMethod, HttpTrigger,
    RouteExists, ROUTE_PATH_KEY_RE, VALID_ROUTE_PATH_RE,
};
use axum::{async_trait, extract::Path, routing::post, Extension, Json, Router};
use http::StatusCode;
use sqlx::PgConnection;
use std::collections::HashSet;
use windmill_api_auth::ApiAuthed;
use windmill_audit::{audit_oss::audit_log, ActionKind};
use windmill_common::{
    db::UserDB,
    error::{Error, Result},
    utils::require_admin,
    worker::CLOUD_HOSTED,
    DB,
};
use windmill_git_sync::{handle_deployment_metadata, DeployedObject};
use windmill_trigger::{Trigger, TriggerCrud, TriggerData};

pub async fn increase_trigger_version(tx: &mut PgConnection) -> Result<()> {
    sqlx::query!("SELECT nextval('http_trigger_version_seq')")
        .fetch_one(tx)
        .await?;
    Ok(())
}

pub fn generate_route_path_key(route_path: &str) -> String {
    ROUTE_PATH_KEY_RE
        .replace_all(route_path, "${1}${2}key")
        .to_string()
}

pub async fn route_path_key_exists(
    route_path_key: &str,
    http_method: &HttpMethod,
    w_id: &str,
    trigger_path: Option<&str>,
    workspaced_route: Option<bool>,
    db: &DB,
) -> Result<bool> {
    let exists = if *CLOUD_HOSTED {
        sqlx::query_scalar!(
            r#"
            SELECT EXISTS(
                SELECT 1
                FROM http_trigger
                WHERE
                    route_path_key = $1
                    AND workspace_id = $2
                    AND http_method = $3
                    AND ($4::TEXT IS NULL OR path != $4)
            )
            "#,
            &route_path_key,
            w_id,
            http_method as &HttpMethod,
            trigger_path
        )
        .fetch_one(db)
        .await?
        .unwrap_or(false)
    } else {
        let route_path_key = match workspaced_route {
            Some(true) => {
                std::borrow::Cow::Owned(format!("{}/{}", w_id, route_path_key.trim_matches('/')))
            }
            _ => std::borrow::Cow::Borrowed(route_path_key),
        };

        sqlx::query_scalar!(
            r#"
            SELECT EXISTS(
                SELECT 1
                FROM http_trigger
                WHERE
                    ((workspaced_route IS TRUE AND workspace_id || '/' || route_path_key = $1)
                    OR (workspaced_route IS FALSE AND route_path_key = $1))
                    AND http_method = $2
                    AND ($3::TEXT IS NULL OR path != $3)
            )
            "#,
            &route_path_key,
            http_method as &HttpMethod,
            trigger_path
        )
        .fetch_one(db)
        .await?
        .unwrap_or(false)
    };

    Ok(exists)
}

pub async fn exists_route(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(RouteExists { route_path, http_method, trigger_path, workspaced_route }): Json<
        RouteExists,
    >,
) -> Result<Json<bool>> {
    let route_path_key = generate_route_path_key(&route_path);

    let exists = route_path_key_exists(
        &route_path_key,
        &http_method,
        &w_id,
        trigger_path.as_deref(),
        workspaced_route,
        &db,
    )
    .await?;

    Ok(Json(exists))
}

fn check_no_duplicates(
    new_http_triggers: &[TriggerData<HttpConfigRequest>],
    route_path_key: &[String],
) -> Result<()> {
    let mut seen = HashSet::with_capacity(new_http_triggers.len());

    for (i, trigger) in new_http_triggers.iter().enumerate() {
        if !seen.insert((
            &route_path_key[i],
            trigger.config.http_method,
            trigger.config.workspaced_route,
        )) {
            return Err(Error::BadRequest(format!(
            "Duplicate HTTP route detected: '{}'. Each HTTP route must have a unique 'route_path'.",
            &trigger.config.route_path
        )));
        }
    }

    Ok(())
}

pub async fn insert_new_trigger_into_db(
    authed: &ApiAuthed,
    db: &DB,
    tx: &mut PgConnection,
    w_id: &str,
    trigger: &TriggerData<HttpConfigRequest>,
    route_path_key: &str,
) -> Result<()> {
    require_admin(authed.is_admin, &authed.username)?;

    let request_type = trigger.config.request_type;
    let resolved_edited_by = trigger.base.resolve_edited_by(authed);
    let resolved_email = trigger.base.resolve_email(authed, db, w_id).await?;

    sqlx::query!(
            r#"
            INSERT INTO http_trigger (
                workspace_id,
                path,
                route_path,
                route_path_key,
                workspaced_route,
                authentication_resource_path,
                wrap_body,
                raw_string,
                script_path,
                summary,
                description,
                is_flow,
                mode,
                request_type,
                authentication_method,
                http_method,
                static_asset_config,
                edited_by,
                email,
                edited_at,
                is_static_website,
                error_handler_path,
                error_handler_args,
                retry
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, now(), $20, $21, $22, $23
            )
            "#,
            w_id,
            trigger.base.path,
            trigger.config.route_path,
            route_path_key,
            trigger.config.workspaced_route.unwrap_or(false),
            trigger.config.authentication_resource_path,
            trigger.config.wrap_body.unwrap_or(false),
            trigger.config.raw_string.unwrap_or(false),
            trigger.base.script_path,
            trigger.config.summary,
            trigger.config.description,
            trigger.base.is_flow,
            trigger.base.mode() as _,
            request_type as _,
            trigger.config.authentication_method as _,
            trigger.config.http_method as _,
            trigger.config.static_asset_config as _,
            &resolved_edited_by,
            resolved_email,
            trigger.config.is_static_website,
            trigger.error_handling.error_handler_path,
            trigger.error_handling.error_handler_args as _,
            trigger.error_handling.retry as _
        )
        .execute(&mut *tx)
        .await?;
    Ok(())
}

pub async fn create_many_http_triggers(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(new_http_triggers): Json<Vec<TriggerData<HttpConfigRequest>>>,
) -> Result<(StatusCode, String)> {
    require_admin(authed.is_admin, &authed.username)?;

    let handler = HttpTrigger;

    let error_wrapper = |route_path: &str, error: Error| -> Error {
        anyhow::anyhow!(
            "Error occurred for HTTP route at route path: {}, error: {}",
            route_path,
            error
        )
        .into()
    };

    let mut route_path_keys = Vec::with_capacity(new_http_triggers.len());

    for new_http_trigger in new_http_triggers.iter() {
        handler
            .validate_new(&db, &w_id, &new_http_trigger.config)
            .await
            .map_err(|err| error_wrapper(&new_http_trigger.config.route_path, err))?;

        let route_path_key =
            check_if_route_exist(&db, &new_http_trigger.config, &w_id, None).await?;

        route_path_keys.push(route_path_key.clone());
    }

    check_no_duplicates(&new_http_triggers, &route_path_keys)?;

    let mut tx = user_db.begin(&authed).await?;

    for (new_http_trigger, route_path_key) in new_http_triggers.iter().zip(route_path_keys.iter()) {
        insert_new_trigger_into_db(
            &authed,
            &db,
            &mut tx,
            &w_id,
            new_http_trigger,
            route_path_key,
        )
        .await
        .map_err(|err| error_wrapper(&new_http_trigger.config.route_path, err))?;

        audit_log(
            &mut *tx,
            &authed,
            "http_trigger.create",
            ActionKind::Create,
            &w_id,
            Some(&new_http_trigger.base.path),
            None,
        )
        .await
        .map_err(|err| error_wrapper(&new_http_trigger.config.route_path, err.into()))?;

        increase_trigger_version(&mut tx)
            .await
            .map_err(|err| error_wrapper(&new_http_trigger.config.route_path, err.into()))?;
    }

    tx.commit().await?;

    for http_trigger in new_http_triggers.into_iter() {
        handle_deployment_metadata(
            &authed.email,
            &authed.username,
            &db,
            &w_id,
            DeployedObject::HttpTrigger { path: http_trigger.base.path.clone(), parent_path: None },
            Some(format!("HTTP trigger '{}' created", http_trigger.base.path)),
            true,
            None,
        )
        .await
        .map_err(|err| error_wrapper(&http_trigger.config.route_path, err.into()))?;
    }
    Ok((StatusCode::CREATED, "Created all HTTP routes".to_string()))
}

async fn check_if_route_exist(
    db: &DB,
    config: &HttpConfigRequest,
    workspace_id: &str,
    trigger_path: Option<&str>,
) -> Result<String> {
    let route_path_key = generate_route_path_key(&config.route_path);

    let exists = route_path_key_exists(
        &route_path_key,
        &config.http_method,
        workspace_id,
        trigger_path,
        config.workspaced_route,
        db,
    )
    .await?;

    if exists {
        return Err(Error::BadRequest(
            "A route already exists with this path".to_string(),
        ));
    }

    Ok(route_path_key)
}

#[async_trait]
impl TriggerCrud for HttpTrigger {
    type TriggerConfig = HttpConfig;
    type Trigger = Trigger<Self::TriggerConfig>;
    type TriggerConfigRequest = HttpConfigRequest;
    type TestConnectionConfig = ();

    const TABLE_NAME: &'static str = "http_trigger";
    const TRIGGER_TYPE: &'static str = "http";
    const SUPPORTS_SERVER_STATE: bool = false;
    const SUPPORTS_TEST_CONNECTION: bool = false;
    const ROUTE_PREFIX: &'static str = "/http_triggers";
    const DEPLOYMENT_NAME: &'static str = "HTTP trigger";
    const IS_ALLOWED_ON_CLOUD: bool = true;
    const ADDITIONAL_SELECT_FIELDS: &[&'static str] = &[
        "route_path",
        "route_path_key",
        "request_type",
        "authentication_method",
        "http_method",
        "summary",
        "description",
        "static_asset_config",
        "is_static_website",
        "authentication_resource_path",
        "workspaced_route",
        "wrap_body",
        "raw_string",
    ];

    fn get_deployed_object(path: String, parent_path: Option<String>) -> DeployedObject {
        DeployedObject::HttpTrigger { path, parent_path }
    }

    fn additional_routes(&self) -> Router {
        Router::new()
            .route("/create_many", post(create_many_http_triggers))
            .route("/route_exists", post(exists_route))
    }

    async fn validate_new(
        &self,
        _db: &DB,
        _workspace_id: &str,
        new: &Self::TriggerConfigRequest,
    ) -> Result<()> {
        if *CLOUD_HOSTED && (new.is_static_website || new.static_asset_config.is_some()) {
            return Err(Error::BadRequest(
                "Static website and static asset are not supported on cloud".to_string(),
            ));
        }

        if !VALID_ROUTE_PATH_RE.is_match(&new.route_path) {
            return Err(Error::BadRequest("Invalid route path".to_string()));
        }

        validate_authentication_method(new.authentication_method, new.raw_string)?;

        Ok(())
    }

    async fn validate_edit(
        &self,
        _db: &DB,
        _workspace_id: &str,
        edit: &Self::TriggerConfigRequest,
        _path: &str,
    ) -> Result<()> {
        if *CLOUD_HOSTED && (edit.is_static_website || edit.static_asset_config.is_some()) {
            return Err(Error::BadRequest(
                "Static website and static asset are not supported on cloud".to_string(),
            ));
        }

        validate_authentication_method(edit.authentication_method, edit.raw_string)?;

        Ok(())
    }

    async fn create_trigger(
        &self,
        db: &DB,
        tx: &mut PgConnection,
        authed: &ApiAuthed,
        w_id: &str,
        trigger: TriggerData<Self::TriggerConfigRequest>,
    ) -> Result<()> {
        let route_path_key = check_if_route_exist(db, &trigger.config, &w_id, None).await?;

        insert_new_trigger_into_db(authed, db, tx, w_id, &trigger, &route_path_key).await?;

        increase_trigger_version(tx).await?;

        Ok(())
    }

    async fn update_trigger(
        &self,
        db: &DB,
        tx: &mut PgConnection,
        authed: &ApiAuthed,
        workspace_id: &str,
        path: &str,
        trigger: TriggerData<Self::TriggerConfigRequest>,
    ) -> Result<()> {
        let resolved_edited_by = trigger.base.resolve_edited_by(authed);
        let resolved_email = trigger.base.resolve_email(authed, db, workspace_id).await?;

        if authed.is_admin {
            if trigger.config.route_path.is_empty() {
                return Err(Error::BadRequest("route_path is required".to_string()));
            };

            let route_path = &trigger.config.route_path;
            if !VALID_ROUTE_PATH_RE.is_match(route_path) {
                return Err(Error::BadRequest("Invalid route path".to_string()));
            }

            let route_path_key =
                check_if_route_exist(db, &trigger.config, workspace_id, Some(path)).await?;

            let request_type = trigger.config.request_type;

            sqlx::query!(
                r#"
            UPDATE
                http_trigger
            SET
                route_path = $1,
                route_path_key = $2,
                workspaced_route = $3,
                wrap_body = $4,
                raw_string = $5,
                authentication_resource_path = $6,
                script_path = $7,
                path = $8,
                is_flow = $9,
                mode = $10,
                http_method = $11,
                static_asset_config = $12,
                edited_by = $13,
                email = $14,
                request_type = $15,
                authentication_method = $16,
                summary = $17,
                description = $18,
                edited_at = now(),
                is_static_website = $19,
                error_handler_path = $20,
                error_handler_args = $21,
                retry = $22
            WHERE
                workspace_id = $23 AND
                path = $24
            "#,
                route_path,
                &route_path_key,
                trigger.config.workspaced_route,
                trigger.config.wrap_body,
                trigger.config.raw_string,
                trigger.config.authentication_resource_path,
                trigger.base.script_path,
                trigger.base.path,
                trigger.base.is_flow,
                trigger.base.mode() as _,
                trigger.config.http_method as _,
                trigger.config.static_asset_config as _,
                &resolved_edited_by,
                resolved_email,
                request_type as _,
                trigger.config.authentication_method as _,
                trigger.config.summary,
                trigger.config.description,
                trigger.config.is_static_website,
                trigger.error_handling.error_handler_path,
                trigger.error_handling.error_handler_args as _,
                trigger.error_handling.retry as _,
                workspace_id,
                path,
            )
            .execute(&mut *tx)
            .await?;
        } else {
            let request_type = trigger.config.request_type;

            sqlx::query!(
                r#"
            UPDATE
                http_trigger
            SET
                wrap_body = $1,
                raw_string = $2,
                authentication_resource_path = $3,
                script_path = $4,
                path = $5,
                is_flow = $6,
                mode = $7,
                http_method = $8,
                static_asset_config = $9,
                edited_by = $10,
                email = $11,
                request_type = $12,
                authentication_method = $13,
                summary = $14,
                description = $15,
                edited_at = now(),
                is_static_website = $16,
                error_handler_path = $17,
                error_handler_args = $18,
                retry = $19
            WHERE
                workspace_id = $20 AND
                path = $21
            "#,
                trigger.config.wrap_body,
                trigger.config.raw_string,
                trigger.config.authentication_resource_path,
                trigger.base.script_path,
                trigger.base.path,
                trigger.base.is_flow,
                trigger.base.mode() as _,
                trigger.config.http_method as _,
                trigger.config.static_asset_config as _,
                &resolved_edited_by,
                resolved_email,
                request_type as _,
                trigger.config.authentication_method as _,
                trigger.config.summary,
                trigger.config.description,
                trigger.config.is_static_website,
                trigger.error_handling.error_handler_path,
                trigger.error_handling.error_handler_args as _,
                trigger.error_handling.retry as _,
                workspace_id,
                path,
            )
            .execute(&mut *tx)
            .await?;
        }

        increase_trigger_version(tx).await?;

        Ok(())
    }

    async fn set_trigger_mode_extra_action(&self, tx: &mut PgConnection) -> Result<()> {
        increase_trigger_version(tx).await
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

        increase_trigger_version(tx).await?;

        Ok(deleted > 0)
    }
}
