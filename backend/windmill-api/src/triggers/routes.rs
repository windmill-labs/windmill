use axum::{
    extract::{Path, Query},
    http::StatusCode,
    routing::{delete, get, post},
    Extension, Json, Router,
};
use std::sync::Arc;
use windmill_audit::{audit_oss::audit_log, ActionKind};
use windmill_common::{
    db::UserDB,
    error::{self, JsonResult, Result},
    utils::StripPath,
    worker::CLOUD_HOSTED,
    DB,
};
use windmill_git_sync::handle_deployment_metadata;

use crate::{
    db::ApiAuthed,
    triggers::{CreateTrigger, EditTrigger, StandardTriggerQuery, TriggerCrud},
    utils::check_scopes,
};

pub fn trigger_routes<T: TriggerCrud + 'static>() -> Router {
    let mut router = Router::new()
        .route("/create", post(create_trigger::<T>))
        .route("/list", get(list_triggers::<T>))
        .route("/get/*path", get(get_trigger::<T>))
        .route("/update/*path", post(update_trigger::<T>))
        .route("/delete/*path", delete(delete_trigger::<T>))
        .route("/exists/*path", get(exists_trigger::<T>));

    if T::SUPPORTS_ENABLED {
        router = router.route("/setenabled/*path", post(set_enabled_trigger::<T>));
    }

    if T::SUPPORTS_TEST_CONNECTION {
        router = router.route("/test", post(test_connection::<T>));
    }

    router
}

pub fn complete_trigger_routes<T: TriggerCrud + 'static>(handler: &T) -> Router {
    let standard_routes = trigger_routes::<T>();

    let additional_routes = handler.additional_routes();

    standard_routes.merge(additional_routes)
}

async fn create_trigger<T: TriggerCrud>(
    Extension(handler): Extension<Arc<T>>,
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(workspace_id): Path<String>,
    Json(new_trigger): Json<CreateTrigger<T::NewTriggerConfig>>,
) -> Result<(StatusCode, String)> {
    check_scopes(&authed, || {
        format!("{}:write:{}", T::SCOPE_NAME, &new_trigger.base.path)
    })?;

    if *CLOUD_HOSTED {
        return Err(error::Error::BadRequest(format!(
            "{} triggers are not supported on multi-tenant cloud, use dedicated cloud or self-host",
            T::TRIGGER_TYPE
        )));
    }

    handler
        .validate_new(&workspace_id, &new_trigger.config)
        .await?;

    let mut tx = user_db.begin(&authed).await?;

    handler
        .create_trigger(&mut *tx, &authed, &workspace_id, &new_trigger)
        .await?;

    audit_log(
        &mut *tx,
        &authed,
        &format!("{}.create", T::TABLE_NAME),
        ActionKind::Create,
        &workspace_id,
        Some(new_trigger.base.path.as_str()),
        None,
    )
    .await?;

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &workspace_id,
        windmill_git_sync::DeployedObject::HttpTrigger { path: new_trigger.base.path.clone() },
        Some(format!(
            "{} '{}' created",
            T::DEPLOYMENT_NAME,
            new_trigger.base.path.clone()
        )),
        true,
    )
    .await?;

    tx.commit().await?;

    Ok((StatusCode::CREATED, new_trigger.base.path.to_string()))
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
        .list_triggers(&mut *tx, &workspace_id, &query)
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
    check_scopes(&authed, || format!("{}:read:{}", T::SCOPE_NAME, &path))?;

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
    Json(edit_trigger): Json<EditTrigger<T::EditTriggerConfig>>,
) -> Result<String> {
    let path = path.to_path();
    check_scopes(&authed, || {
        format!("{}:write:{}", T::SCOPE_NAME, &edit_trigger.base.path)
    })?;

    handler
        .validate_edit(&workspace_id, path, &edit_trigger.config)
        .await?;

    let mut tx = user_db.begin(&authed).await?;

    handler
        .update_trigger(&mut *tx, &authed, &workspace_id, path, &edit_trigger)
        .await?;

    audit_log(
        &mut *tx,
        &authed,
        &format!("{}.update", T::TABLE_NAME),
        ActionKind::Update,
        &workspace_id,
        Some(edit_trigger.base.path.as_str()),
        None,
    )
    .await?;

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &workspace_id,
        windmill_git_sync::DeployedObject::HttpTrigger { path: edit_trigger.base.path.clone() },
        Some(format!(
            "{} '{}' updated",
            T::DEPLOYMENT_NAME,
            edit_trigger.base.path.clone()
        )),
        true,
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
    check_scopes(&authed, || format!("{}:write:{}", T::SCOPE_NAME, &path))?;

    let mut tx = user_db.begin(&authed).await?;
    let deleted = handler
        .delete_by_path(&mut *tx, &workspace_id, path)
        .await?;

    if !deleted {
        return Err(error::Error::NotFound(format!(
            "Trigger not found at path: {}",
            path
        )));
    }

    audit_log(
        &mut *tx,
        &authed,
        &format!("triggers.delete.{}:{}", T::TRIGGER_TYPE, path),
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
    Extension(user_db): Extension<UserDB>,
    Path((workspace_id, path)): Path<(String, StripPath)>,
) -> JsonResult<bool> {
    let path = path.to_path();
    let mut tx = user_db.begin(&authed).await?;
    let exists = handler.exists(&mut *tx, &workspace_id, path).await?;
    tx.commit().await?;

    Ok(Json(exists))
}

#[derive(serde::Deserialize)]
struct SetEnabledPayload {
    enabled: bool,
}

async fn set_enabled_trigger<T: TriggerCrud>(
    Extension(handler): Extension<Arc<T>>,
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((workspace_id, path)): Path<(String, StripPath)>,
    Json(payload): Json<SetEnabledPayload>,
) -> Result<String> {
    let path = path.to_path();
    check_scopes(&authed, || format!("{}:write", T::SCOPE_NAME))?;

    let mut tx = user_db.begin(&authed).await?;
    let updated = handler
        .set_enabled(&mut *tx, &workspace_id, path, payload.enabled)
        .await?;

    if !updated {
        return Err(error::Error::NotFound(format!(
            "Trigger not found at path: {}",
            path
        )));
    }

    tx.commit().await?;

    Ok(format!(
        "Trigger '{}' {}",
        path,
        if payload.enabled {
            "enabled"
        } else {
            "disabled"
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
            .test_connection(&db, &authed, &user_db, &workspace_id, &config)
            .await
    };

    tokio::time::timeout(tokio::time::Duration::from_secs(30), connect_f)
        .await
        .map_err(|_| {
            error::Error::BadConfig(format!("Timeout connecting to service after 30 seconds"))
        })??;
    Ok(())
}

#[macro_export]
macro_rules! register_trigger_routes {
    ($app:expr, $path:literal, $handler:expr) => {
        $app.nest(
            $path,
            $crate::triggers::routes::complete_trigger_routes(&$handler)
                .layer(Extension(Arc::new($handler))),
        )
    };
}
