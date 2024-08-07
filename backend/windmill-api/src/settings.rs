/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2023
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::time::Duration;

use crate::{
    db::{ApiAuthed, DB},
    ee::validate_license_key,
    utils::{generate_instance_username_for_all_users, require_super_admin},
    HTTP_CLIENT,
};

use axum::{
    extract::{Extension, Path},
    routing::{get, post},
    Json, Router,
};

use serde::Deserialize;
use windmill_common::{
    error::{self, JsonResult, Result},
    global_settings::{
        AUTOMATE_USERNAME_CREATION_SETTING, EMAIL_DOMAIN_SETTING, ENV_SETTINGS,
        HUB_BASE_URL_SETTING,
    },
    server::Smtp,
    utils::send_email,
};

#[cfg(feature = "parquet")]
use windmill_common::error::to_anyhow;

pub fn global_service() -> Router {
    #[warn(unused_mut)]
    let r = Router::new()
        .route("/envs", get(get_local_settings))
        .route(
            "/global/:key",
            post(set_global_setting).get(get_global_setting),
        )
        .route("/list_global", get(list_global_settings))
        .route("/test_smtp", post(test_email))
        .route("/test_license_key", post(test_license_key))
        .route("/send_stats", post(send_stats))
        .route(
            "/latest_key_renewal_attempt",
            get(get_latest_key_renewal_attempt),
        )
        .route("/renew_license_key", post(renew_license_key))
        .route("/customer_portal", post(create_customer_portal_session));

    #[cfg(feature = "parquet")]
    {
        return r.route("/test_object_storage_config", post(test_s3_bucket));
    }

    #[cfg(not(feature = "parquet"))]
    {
        return r;
    }
}

#[derive(Deserialize)]
pub struct TestEmail {
    pub to: String,
    pub smtp: Smtp,
}

pub async fn test_email(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Json(test_email): Json<TestEmail>,
) -> error::Result<String> {
    require_super_admin(&db, &authed.email).await?;
    let smtp = test_email.smtp;
    let to = test_email.to;

    let client_timeout = Duration::from_secs(3);
    send_email(
        "Test email from Windmill",
        "Test email content",
        vec![to],
        smtp,
        Some(client_timeout),
    )
    .await?;

    Ok("Sent test email".to_string())
}

#[cfg(feature = "parquet")]
use windmill_common::s3_helpers::ObjectSettings;

#[cfg(feature = "parquet")]
use windmill_common::s3_helpers::build_object_store_from_settings;

#[cfg(feature = "parquet")]
pub async fn test_s3_bucket(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Json(test_s3_bucket): Json<ObjectSettings>,
) -> error::Result<String> {
    use bytes::Bytes;
    use futures::StreamExt;
    use windmill_common::ee::{get_license_plan, LicensePlan};

    if matches!(get_license_plan().await, LicensePlan::Pro) {
        return Err(error::Error::InternalErr(
            "This feature is only available in Enterprise, not Pro".to_string(),
        ));
    }

    require_super_admin(&db, &authed.email).await?;
    let client = build_object_store_from_settings(test_s3_bucket).await?;

    let mut list = client.list(Some(&object_store::path::Path::from("".to_string())));
    let first_file = list.next().await;
    if first_file.is_some() {
        if let Err(e) = first_file.as_ref().unwrap() {
            tracing::error!("error listing bucket: {e:#}");
            error::Error::InternalErr(format!("Failed to list files in blob storage: {e:#}"));
        }
        tracing::info!("Listed files: {:?}", first_file.unwrap());
    } else {
        tracing::info!("No files in blob storage");
    }

    let path = object_store::path::Path::from(format!(
        "/test-s3-bucket-{uuid}",
        uuid = uuid::Uuid::new_v4()
    ));
    tracing::info!("Testing blob storage at path: {path}");
    client
        .put(&path, object_store::PutPayload::from_static(b"hello"))
        .await
        .map_err(|e| anyhow::anyhow!("error writing file to {path}: {e:#}"))?;
    let content = client
        .get(&path)
        .await
        .map_err(to_anyhow)?
        .bytes()
        .await
        .map_err(to_anyhow)?;
    if content != Bytes::from_static(b"hello") {
        return Err(error::Error::InternalErr(
            "Failed to read back from blob storage".to_string(),
        ));
    }
    client.delete(&path).await.map_err(to_anyhow)?;
    Ok("Tested blob storage successfully".to_string())
}

#[derive(Deserialize)]
pub struct TestKey {
    pub license_key: String,
}

pub async fn test_license_key(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Json(TestKey { license_key }): Json<TestKey>,
) -> error::Result<String> {
    require_super_admin(&db, &authed.email).await?;
    validate_license_key(license_key).await?;
    Ok("Sent test email".to_string())
}

pub async fn get_local_settings(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
) -> error::JsonResult<serde_json::Value> {
    require_super_admin(&db, &authed.email).await?;

    let mut settings = serde_json::Map::new();
    for key in ENV_SETTINGS.iter() {
        if let Some(value) = std::env::var(key).ok() {
            settings.insert(key.to_string(), serde_json::Value::String(value));
        }
    }
    Ok(Json(serde_json::Value::Object(settings)))
}

#[derive(serde::Deserialize)]
pub struct Value {
    pub value: Option<serde_json::Value>,
}

pub async fn delete_global_setting(db: &DB, key: &str) -> error::Result<()> {
    sqlx::query!("DELETE FROM global_settings WHERE name = $1", key,)
        .execute(db)
        .await?;
    tracing::info!("Unset global setting {}", key);
    Ok(())
}
pub async fn set_global_setting(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Path(key): Path<String>,
    Json(value): Json<Value>,
) -> error::Result<()> {
    require_super_admin(&db, &authed.email).await?;
    set_global_setting_internal(&db, key, value.value.unwrap_or(serde_json::Value::Null)).await
}

pub async fn set_global_setting_internal(
    db: &DB,
    key: String,
    value: serde_json::Value,
) -> error::Result<()> {
    match key.as_str() {
        AUTOMATE_USERNAME_CREATION_SETTING => {
            if value.clone().as_bool().unwrap_or(false) {
                generate_instance_username_for_all_users(db)
                    .await
                    .map_err(|err| {
                        error::Error::InternalErr(format!(
                            "Failed to generate instance wide usernames: {}",
                            err
                        ))
                    })?;
            }
        }
        _ => {}
    }

    match value {
        serde_json::Value::Null => {
            delete_global_setting(db, &key).await?;
        }
        serde_json::Value::String(x) if x.is_empty() => {
            delete_global_setting(db, &key).await?;
        }
        v => {
            sqlx::query!(
                "INSERT INTO global_settings (name, value) VALUES ($1, $2) ON CONFLICT (name) DO UPDATE SET value = $2, updated_at = now()",
                key,
                v
            )
            .execute(db)
            .await?;
            tracing::info!("Set global setting {} to {}", key, v);
        }
    };

    Ok(())
}

pub async fn get_global_setting(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Path(key): Path<String>,
) -> JsonResult<serde_json::Value> {
    if !key.starts_with("default_error_handler_")
        && !key.starts_with("default_recovery_handler_")
        && key != AUTOMATE_USERNAME_CREATION_SETTING
        && key != HUB_BASE_URL_SETTING
        && key != EMAIL_DOMAIN_SETTING
    {
        require_super_admin(&db, &authed.email).await?;
    }
    let value = sqlx::query!("SELECT value FROM global_settings WHERE name = $1", key)
        .fetch_optional(&db)
        .await?
        .map(|x| x.value);

    Ok(Json(value.unwrap_or_else(|| serde_json::Value::Null)))
}

#[cfg(feature = "enterprise")]
#[derive(Deserialize, serde::Serialize)]
struct GlobalSetting {
    name: String,
    value: serde_json::Value,
}

#[cfg(feature = "enterprise")]
async fn list_global_settings(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
) -> JsonResult<Vec<GlobalSetting>> {
    require_super_admin(&db, &authed.email).await?;
    let settings = sqlx::query_as!(GlobalSetting, "SELECT name, value FROM global_settings")
        .fetch_all(&db)
        .await?;

    Ok(Json(settings))
}

#[cfg(not(feature = "enterprise"))]
async fn list_global_settings() -> JsonResult<String> {
    return Err(error::Error::BadRequest(
        "Listing global settings not available on community edition".to_string(),
    ));
}

pub async fn send_stats(Extension(db): Extension<DB>, authed: ApiAuthed) -> Result<String> {
    require_super_admin(&db, &authed.email).await?;
    windmill_common::stats_ee::send_stats(&"manual".to_string(), &HTTP_CLIENT, &db).await?;

    Ok("Sent stats".to_string())
}

#[derive(serde::Serialize)]
pub struct KeyRenewalAttempt {
    result: String,
    attempted_at: chrono::DateTime<chrono::Utc>,
}

pub async fn get_latest_key_renewal_attempt(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
) -> JsonResult<Option<KeyRenewalAttempt>> {
    require_super_admin(&db, &authed.email).await?;

    let last_attempt = sqlx::query!(
        "SELECT value, created_at FROM metrics WHERE id = $1 ORDER BY created_at DESC LIMIT 1",
        "license_key_renewal"
    )
    .fetch_optional(&db)
    .await?;

    match last_attempt {
        Some(last_attempt) => {
            let last_attempt_result = serde_json::from_value::<String>(last_attempt.value)
                .map_err(|e| {
                    error::Error::InternalErr(format!("Failed to parse last attempt: {}", e))
                })?;
            Ok(Json(Some(KeyRenewalAttempt {
                result: last_attempt_result,
                attempted_at: last_attempt.created_at,
            })))
        }
        None => Ok(Json(None)),
    }
}

#[cfg(not(feature = "enterprise"))]
pub async fn renew_license_key() -> Result<String> {
    return Err(error::Error::BadRequest(
        "License key renewal not available on community edition".to_string(),
    ));
}

#[cfg(feature = "enterprise")]
pub async fn renew_license_key(Extension(db): Extension<DB>, authed: ApiAuthed) -> Result<String> {
    require_super_admin(&db, &authed.email).await?;
    windmill_common::stats_ee::send_stats(&"manual".to_string(), &HTTP_CLIENT, &db).await?;
    let result = windmill_common::ee::renew_license_key(&HTTP_CLIENT, &db).await;

    if result != "success" {
        return Err(error::Error::BadRequest(format!(
            "Failed to renew license key: {}",
            result
        )));
    } else {
        return Ok("Renewed license key".to_string());
    }
}

#[cfg(not(feature = "enterprise"))]
pub async fn create_customer_portal_session() -> Result<String> {
    return Err(error::Error::BadRequest(
        "Customer portal is not available on community edition".to_string(),
    ));
}

#[cfg(feature = "enterprise")]
pub async fn create_customer_portal_session() -> Result<String> {
    let url = windmill_common::ee::create_customer_portal_session(&HTTP_CLIENT).await?;

    return Ok(url);
}
