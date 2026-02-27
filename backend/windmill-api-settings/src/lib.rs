/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2023
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::{collections::HashMap, time::Duration};

#[cfg(feature = "private")]
mod ee;
pub mod ee_oss;

#[cfg(feature = "enterprise")]
use windmill_api_auth::require_devops_role;
use windmill_api_auth::{require_super_admin, ApiAuthed};
use windmill_common::utils::HTTP_CLIENT_PERMISSIVE as HTTP_CLIENT;
use windmill_common::DB;

use ee_oss::validate_license_key;
use windmill_common::usernames::generate_instance_username_for_all_users;

#[cfg(feature = "enterprise")]
use axum::extract::Query;
use axum::{
    body::Body,
    extract::{Extension, Path},
    response::Response,
    routing::{get, post},
    Json, Router,
};
use serde_json::json;

use serde::{Deserialize, Serialize};
#[cfg(feature = "enterprise")]
use windmill_common::ee_oss::{send_critical_alert, CriticalAlertKind, CriticalErrorChannel};
#[cfg(all(feature = "private", feature = "enterprise"))]
use windmill_common::secret_backend::{SecretMigrationReport, VaultSettings};
use windmill_common::{
    email_oss::send_email_plain_text,
    error::{self, JsonResult, Result},
    get_database_url,
    global_settings::{
        APP_WORKSPACED_ROUTE_SETTING, AUTOMATE_USERNAME_CREATION_SETTING,
        CRITICAL_ALERT_MUTE_UI_SETTING, DEFAULT_TAGS_WORKSPACES_SETTING, EMAIL_DOMAIN_SETTING,
        ENV_SETTINGS, HUB_ACCESSIBLE_URL_SETTING, HUB_BASE_URL_SETTING,
    },
    instance_config::{self, ApplyMode, InstanceConfig},
    server::Smtp,
};
use windmill_common::{error::to_anyhow, PgDatabase};

pub fn global_service() -> Router {
    #[warn(unused_mut)]
    let r = Router::new()
        .route("/envs", get(get_local_settings))
        .route(
            "/global/:key",
            post(set_global_setting).get(get_global_setting),
        )
        .route("/list_global", get(list_global_settings))
        .route(
            "/instance_config",
            get(get_instance_config).put(set_instance_config),
        )
        .route("/instance_config/yaml", get(get_instance_config_yaml))
        .route("/test_smtp", post(test_email))
        .route("/test_license_key", post(test_license_key))
        .route("/send_stats", post(send_stats))
        .route("/get_stats", get(get_stats))
        .route(
            "/latest_key_renewal_attempt",
            get(get_latest_key_renewal_attempt),
        )
        .route("/renew_license_key", post(renew_license_key))
        .route("/customer_portal", post(create_customer_portal_session))
        .route("/test_critical_channels", post(test_critical_channels))
        .route("/critical_alerts", get(get_critical_alerts))
        .route(
            "/critical_alerts/:id/acknowledge",
            post(acknowledge_critical_alert),
        )
        .route(
            "/list_custom_instance_pg_databases",
            post(list_custom_instance_pg_databases),
        )
        .route(
            "/refresh_custom_instance_user_pwd",
            post(refresh_custom_instance_user_pwd),
        )
        .route(
            "/setup_custom_instance_pg_database/:name",
            post(setup_custom_instance_pg_database),
        )
        .route(
            "/critical_alerts/acknowledge_all",
            post(acknowledge_all_critical_alerts),
        )
        .route(
            "/sync_cached_resource_types",
            post(sync_cached_resource_types),
        );

    // Vault integration routes (EE only - requires both private and enterprise features)
    #[cfg(all(feature = "private", feature = "enterprise"))]
    let r = r
        .route("/test_secret_backend", post(test_secret_backend))
        .route("/migrate_secrets_to_vault", post(migrate_secrets_to_vault))
        .route(
            "/migrate_secrets_to_database",
            post(migrate_secrets_to_database),
        );

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
    send_email_plain_text(
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
use windmill_object_store::ObjectSettings;

#[cfg(feature = "parquet")]
use windmill_object_store::build_object_store_from_settings;

#[cfg(feature = "parquet")]
pub async fn test_s3_bucket(
    _authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Json(test_s3_bucket): Json<ObjectSettings>,
) -> error::Result<String> {
    use bytes::Bytes;
    use futures::StreamExt;

    let client = build_object_store_from_settings(test_s3_bucket, Some(&db))
        .await?
        .store;

    let mut list = client.list(Some(
        &windmill_object_store::object_store_reexports::Path::from("".to_string()),
    ));
    let first_file = list.next().await;
    if first_file.is_some() {
        if let Err(e) = first_file.as_ref().unwrap() {
            tracing::error!("error listing bucket: {e:#}");
            error::Error::internal_err(format!("Failed to list files in blob storage: {e:#}"));
        }
        tracing::info!("Listed files: {:?}", first_file.unwrap());
    } else {
        tracing::info!("No files in blob storage");
    }

    let path = windmill_object_store::object_store_reexports::Path::from(format!(
        "/test-s3-bucket-{uuid}",
        uuid = uuid::Uuid::new_v4()
    ));
    tracing::info!("Testing blob storage at path: {path}");
    client
        .put(
            &path,
            windmill_object_store::object_store_reexports::PutPayload::from_static(b"hello"),
        )
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
        return Err(error::Error::internal_err(
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
    let (_, expired) = validate_license_key(license_key, Some(&db)).await?;

    if expired {
        Err(error::Error::BadRequest("Expired license key".to_string()))
    } else {
        Ok("Valid license key".to_string())
    }
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
    // ducklake_user_pg_pwd and ducklake_settings were old names stored as standalone global settings.
    // Leave them for backward compatibility (CLI will try to delete them if not present in the yaml)
    if key == "ducklake_user_pg_pwd"
        || key == "ducklake_settings"
        || key == "custom_instance_pg_databases"
    {
        tracing::error!("Tried to unset global setting {}, ignored", key);
        return Ok(());
    }
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
    let value = if key == "retention_period_secs" {
        instance_config::clamp_retention_period(value)
    } else {
        value
    };

    run_setting_pre_write_hook(db, &key, &value).await?;

    match value {
        serde_json::Value::Null => {
            if instance_config::PROTECTED_SETTINGS.contains(&key.as_str()) {
                return Err(error::Error::BadRequest(format!(
                    "{key} is a protected setting and cannot be deleted"
                )));
            }
            delete_global_setting(db, &key).await?;
        }
        serde_json::Value::String(x) if x.is_empty() => {
            if instance_config::PROTECTED_SETTINGS.contains(&key.as_str()) {
                return Err(error::Error::BadRequest(format!(
                    "{key} is a protected setting and cannot be set to empty"
                )));
            }
            delete_global_setting(db, &key).await?;
        }
        v => {
            sqlx::query!(
                 "INSERT INTO global_settings (name, value) VALUES ($1, $2) ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value, updated_at = now()",
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

/// Run side-effect hooks for specific settings before writing to DB.
/// Extracted from `set_global_setting_internal` for reuse by the bulk endpoint.
async fn run_setting_pre_write_hook(
    db: &DB,
    key: &str,
    value: &serde_json::Value,
) -> error::Result<()> {
    match key {
        AUTOMATE_USERNAME_CREATION_SETTING => {
            if value.as_bool().unwrap_or(false) {
                generate_instance_username_for_all_users(db)
                    .await
                    .map_err(|err| {
                        error::Error::internal_err(format!(
                            "Failed to generate instance wide usernames: {}",
                            err
                        ))
                    })?;
            }
        }
        CRITICAL_ALERT_MUTE_UI_SETTING => {
            if value.as_bool().unwrap_or(false) {
                sqlx::query!("UPDATE alerts SET acknowledged = true")
                    .execute(db)
                    .await?;
            }
        }
        APP_WORKSPACED_ROUTE_SETTING => {
            let serde_json::Value::Bool(workspaced_route) = value else {
                return Err(error::Error::BadRequest(format!(
                    "{} setting Expected to be boolean",
                    APP_WORKSPACED_ROUTE_SETTING
                )));
            };

            if !*workspaced_route {
                #[derive(Debug, Deserialize, Serialize)]
                #[allow(unused)]
                struct DuplicateApp {
                    custom_path: Option<String>,
                    path: String,
                }
                let duplicate_app = sqlx::query_as!(
                    DuplicateApp,
                    r#"
                        SELECT
                            path,
                            custom_path
                        FROM
                            app
                        WHERE
                            custom_path IN (
                                SELECT
                                    custom_path
                                FROM
                                    app
                                GROUP
                                    BY custom_path
                                HAVING COUNT(*) > 1
                            )
                        ORDER BY custom_path
                        "#
                )
                .fetch_all(db)
                .await?;

                if !duplicate_app.is_empty() {
                    tracing::error!(
                        "Cannot disable {} setting as duplicate app with custom path were found: {:?}",
                        APP_WORKSPACED_ROUTE_SETTING,
                        &duplicate_app
                    );

                    #[derive(Serialize)]
                    struct ErrorResponse {
                        error: String,
                        details: Vec<DuplicateApp>,
                    }

                    let error_response = ErrorResponse {
                        error: "Duplicate custom paths detected".to_string(),
                        details: duplicate_app,
                    };

                    return Err(error::Error::JsonErr(
                        serde_json::to_value(error_response).unwrap(),
                    ));
                }
            }
        }
        _ => {}
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Bulk instance config endpoints
// ---------------------------------------------------------------------------

async fn get_instance_config(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
) -> JsonResult<InstanceConfig> {
    require_super_admin(&db, &authed.email).await?;
    let config = InstanceConfig::from_db(&db)
        .await
        .map_err(|e| error::Error::internal_err(e.to_string()))?;
    Ok(Json(config))
}

async fn get_instance_config_yaml(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
) -> error::Result<Response> {
    require_super_admin(&db, &authed.email).await?;
    let config = InstanceConfig::from_db(&db)
        .await
        .map_err(|e| error::Error::internal_err(e.to_string()))?;
    let yaml = config
        .to_sorted_yaml()
        .map_err(|e| error::Error::internal_err(e))?;
    Response::builder()
        .header("content-type", "application/yaml")
        .body(Body::from(yaml))
        .map_err(|e| error::Error::internal_err(e.to_string()))
}

async fn set_instance_config(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Json(desired): Json<InstanceConfig>,
) -> error::Result<()> {
    require_super_admin(&db, &authed.email).await?;

    let current = InstanceConfig::from_db(&db)
        .await
        .map_err(|e| error::Error::internal_err(e.to_string()))?;

    let desired_map = desired.global_settings.to_settings_map();
    if !desired_map.is_empty() {
        let current_map = current.global_settings.to_settings_map();
        let settings_diff =
            instance_config::diff_global_settings(&current_map, &desired_map, ApplyMode::Merge);

        for (key, value) in &settings_diff.upserts {
            run_setting_pre_write_hook(&db, key, value).await?;
        }

        instance_config::apply_settings_diff(&db, &settings_diff)
            .await
            .map_err(|e| error::Error::internal_err(e.to_string()))?;
    }

    if !desired.worker_configs.is_empty() {
        let current_wc: std::collections::BTreeMap<String, serde_json::Value> = current
            .worker_configs
            .iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    serde_json::to_value(v).expect("WorkerGroupConfig serialization cannot fail"),
                )
            })
            .collect();
        let desired_wc: std::collections::BTreeMap<String, serde_json::Value> = desired
            .worker_configs
            .iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    serde_json::to_value(v).expect("WorkerGroupConfig serialization cannot fail"),
                )
            })
            .collect();
        let configs_diff =
            instance_config::diff_worker_configs(&current_wc, &desired_wc, ApplyMode::Merge);
        instance_config::apply_configs_diff(&db, &configs_diff)
            .await
            .map_err(|e| error::Error::internal_err(e.to_string()))?;
    }

    Ok(())
}

pub async fn get_global_setting(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Path(key): Path<String>,
) -> JsonResult<serde_json::Value> {
    if !key.starts_with("default_error_handler_")
        && !key.starts_with("default_recovery_handler_")
        && !key.starts_with("default_success_handler_")
        && key != AUTOMATE_USERNAME_CREATION_SETTING
        && key != DEFAULT_TAGS_WORKSPACES_SETTING
        && key != HUB_BASE_URL_SETTING
        && key != HUB_ACCESSIBLE_URL_SETTING
        && key != EMAIL_DOMAIN_SETTING
        && key != APP_WORKSPACED_ROUTE_SETTING
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
    windmill_common::stats_oss::send_stats(
        &HTTP_CLIENT,
        &db,
        windmill_common::stats_oss::SendStatsReason::Manual,
    )
    .await?;

    Ok("Sent stats".to_string())
}

#[cfg(feature = "enterprise")]
pub async fn get_stats(Extension(db): Extension<DB>, authed: ApiAuthed) -> Result<String> {
    require_super_admin(&db, &authed.email).await?;
    let stats = windmill_common::stats_oss::get_stats_payload(
        &db,
        &windmill_common::stats_oss::SendStatsReason::Manual,
    )
    .await?;
    let encrypted = windmill_common::stats_oss::encrypt_stats(&stats)?;
    Ok(encrypted)
}

#[cfg(not(feature = "enterprise"))]
pub async fn get_stats() -> Result<String> {
    Err(error::Error::BadRequest(
        "Downloading telemetry is only available on enterprise edition".to_string(),
    ))
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
                    error::Error::internal_err(format!("Failed to parse last attempt: {}", e))
                })?;
            Ok(Json(Some(KeyRenewalAttempt {
                result: last_attempt_result,
                attempted_at: last_attempt.created_at,
            })))
        }
        None => Ok(Json(None)),
    }
}

#[cfg(feature = "enterprise")]
#[derive(Deserialize)]
pub struct LicenseQuery {
    license_key: Option<String>,
}

#[cfg(not(feature = "enterprise"))]
pub async fn renew_license_key() -> Result<String> {
    return Err(error::Error::BadRequest(
        "License key renewal not available on community edition".to_string(),
    ));
}

#[cfg(feature = "enterprise")]
pub async fn renew_license_key(
    Extension(db): Extension<DB>,
    Query(LicenseQuery { license_key }): Query<LicenseQuery>,
    authed: ApiAuthed,
) -> Result<String> {
    require_super_admin(&db, &authed.email).await?;
    let result = windmill_common::ee_oss::renew_license_key(
        &HTTP_CLIENT,
        &db,
        license_key,
        windmill_common::ee_oss::RenewReason::Manual,
    )
    .await;

    if result != "success" {
        return Err(error::Error::BadRequest(format!(
            "Failed to renew license key: {}",
            if result == "Unauthorized" {
                "Invalid key".to_string()
            } else {
                result
            }
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
pub async fn create_customer_portal_session(
    Query(LicenseQuery { license_key }): Query<LicenseQuery>,
) -> Result<String> {
    let url =
        windmill_common::ee_oss::create_customer_portal_session(&HTTP_CLIENT, license_key).await?;

    return Ok(url);
}

#[cfg(feature = "enterprise")]
pub async fn test_critical_channels(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Json(test_critical_channels): Json<Vec<CriticalErrorChannel>>,
) -> Result<String> {
    require_super_admin(&db, &authed.email).await?;

    #[cfg(feature = "enterprise")]
    send_critical_alert(
        "Test critical error".to_string(),
        &db,
        CriticalAlertKind::CriticalError,
        Some(test_critical_channels),
    )
    .await;
    Ok("Sent test critical error".to_string())
}

#[cfg(not(feature = "enterprise"))]
pub async fn test_critical_channels() -> Result<String> {
    Ok("Critical channels require EE".to_string())
}

#[cfg(feature = "enterprise")]
pub async fn get_critical_alerts(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Query(params): Query<windmill_alerting::AlertQueryParams>,
) -> JsonResult<serde_json::Value> {
    require_devops_role(&db, &authed.email).await?;

    windmill_alerting::get_critical_alerts(db, params, None).await
}

#[cfg(not(feature = "enterprise"))]
pub async fn get_critical_alerts() -> error::Error {
    error::Error::NotFound("Critical Alerts require EE".to_string())
}

#[cfg(feature = "enterprise")]
pub async fn acknowledge_critical_alert(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Path(id): Path<i32>,
) -> error::Result<String> {
    require_devops_role(&db, &authed.email).await?;
    windmill_alerting::acknowledge_critical_alert(db, None, id).await
}

#[cfg(not(feature = "enterprise"))]
pub async fn acknowledge_critical_alert() -> error::Error {
    error::Error::NotFound("Critical Alerts require EE".to_string())
}

#[cfg(feature = "enterprise")]
pub async fn acknowledge_all_critical_alerts(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
) -> error::Result<String> {
    require_super_admin(&db, &authed.email).await?;

    windmill_alerting::acknowledge_all_critical_alerts(db, None).await
}

#[cfg(not(feature = "enterprise"))]
pub async fn acknowledge_all_critical_alerts() -> error::Error {
    error::Error::NotFound("Critical Alerts require EE".to_string())
}

#[derive(Deserialize, Debug, Serialize)]
struct CustomInstanceDb {
    logs: CustomInstanceDbLogs, // (Step, Message)[]
    success: bool,
    error: Option<String>,
    tag: Option<String>,
}

#[derive(Deserialize, Debug, Serialize, Default)]
#[serde(default)]
struct CustomInstanceDbLogs {
    super_admin: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    database_credentials: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    valid_dbname: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    created_database: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    db_connect: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    grant_permissions: String,
}

async fn list_custom_instance_pg_databases(
    _authed: ApiAuthed,
    Extension(db): Extension<DB>,
) -> JsonResult<HashMap<String, CustomInstanceDb>> {
    let result = sqlx::query_scalar!(
        r#"SELECT value->'databases' FROM global_settings WHERE name = 'custom_instance_pg_databases'"#,
    )
    .fetch_one(&db)
    .await?
    .ok_or_else(|| error::Error::ExecutionErr("Couldn't find custom_instance_pg_databases".to_string()))?;
    let result = serde_json::from_value(result).map_err(|e| {
        error::Error::ExecutionErr(format!(
            "couldn't parse custom_instance_pg_databases.databases : {}",
            e.to_string()
        ))
    })?;
    return Ok(Json(result));
}

async fn refresh_custom_instance_user_pwd(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
) -> JsonResult<()> {
    require_super_admin(&db, &authed.email).await?;
    // 20251208123907_safety_custom_instance_db_user_pwd.up
    let query = r#"
    DO $$
        DECLARE
            pwd text;
        BEGIN
            SELECT gen_random_uuid()::text INTO pwd;
            
            IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'custom_instance_user') THEN
                EXECUTE format('ALTER USER custom_instance_user WITH PASSWORD %L', pwd);
                RAISE NOTICE 'Updated password for existing user custom_instance_user';
            ELSE
                EXECUTE format('CREATE USER custom_instance_user WITH PASSWORD %L', pwd);
                RAISE NOTICE 'Created new user custom_instance_user';
            END IF;

            IF NOT EXISTS (SELECT 1 FROM global_settings WHERE name = 'custom_instance_pg_databases') THEN
                INSERT INTO global_settings (name, value)
                VALUES ('custom_instance_pg_databases', jsonb_build_object(
                'user_pwd', pwd::text,
                'databases', jsonb_build_object()
                ));
                RAISE NOTICE 'Inserted new global setting for custom_instance_pg_databases';
            ELSE
                UPDATE global_settings
                SET value = jsonb_set(COALESCE(value, '{}'::jsonb), '{user_pwd}', to_jsonb(pwd::text)::jsonb)
                WHERE name = 'custom_instance_pg_databases';
                RAISE NOTICE 'Updated user_pwd in existing global setting for custom_instance_pg_databases';
            END IF;
        END
        $$;
    "#;
    sqlx::query(query).execute(&db).await?;
    Ok(Json(()))
}

#[derive(Deserialize)]
struct SetupCustomInstanceDbBody {
    tag: Option<String>,
}

async fn setup_custom_instance_pg_database(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(dbname): Path<String>,
    Json(body): Json<SetupCustomInstanceDbBody>,
) -> JsonResult<CustomInstanceDb> {
    let mut logs = CustomInstanceDbLogs::default();
    let result = setup_custom_instance_pg_database_inner(authed, &db, &dbname, &mut logs).await;
    let success = result.is_ok();
    let error = result.err().map(|e| e.to_string());
    let status = CustomInstanceDb { logs, success, error, tag: body.tag };
    let status_json = serde_json::to_value(&status).map_err(to_anyhow)?;
    // Save that the database was setup successfully
    sqlx::query!(
        r#"UPDATE global_settings SET value = jsonb_set(value, '{databases}', (COALESCE(value->'databases', '{}'::jsonb) || to_jsonb($1::json))) WHERE name = 'custom_instance_pg_databases'"#,
        json!({ dbname: status_json })
    ).execute(&db).await?;

    Ok(Json(status))
}

async fn setup_custom_instance_pg_database_inner(
    authed: ApiAuthed,
    db: &DB,
    dbname: &str,
    logs: &mut CustomInstanceDbLogs,
) -> Result<()> {
    require_super_admin(db, &authed.email).await?;
    logs.super_admin = "OK".to_string();
    let wmill_pg_creds = PgDatabase::parse_uri(&get_database_url().await?.as_str().await)?;
    logs.database_credentials = "OK".to_string();

    // Validate name to ensure it only contains alphanumeric characters
    // Prevents SQL injection on the instance database
    lazy_static::lazy_static! {
        // Must start with a letter, then alphanumeric/underscore
        static ref VALID_NAME: regex::Regex = regex::Regex::new(r"^[a-zA-Z][a-zA-Z0-9_]*$").unwrap();
    }
    let dbname = dbname.trim();
    if dbname.is_empty() {
        return Err(error::Error::BadRequest(
            "Database name cannot be empty".to_string(),
        ));
    }
    // PostgreSQL identifier limit is 63 bytes
    if dbname.len() > 63 {
        return Err(error::Error::BadRequest(
            "Database name cannot exceed 63 characters".to_string(),
        ));
    }
    if !VALID_NAME.is_match(dbname) {
        return Err(error::Error::BadRequest(
            "Database name must start with a letter and contain only alphanumeric characters or underscores".to_string(),
        ));
    }
    // Additional check: block PostgreSQL reserved/special names
    let lower = dbname.to_lowercase();
    if lower == "template0" || lower == "template1" || lower == "postgres" {
        return Err(error::Error::BadRequest(
            "Cannot use reserved PostgreSQL database names".to_string(),
        ));
    }
    if wmill_pg_creds
        .dbname
        .trim()
        .eq_ignore_ascii_case(dbname.trim())
    {
        return Err(error::Error::BadRequest(
            "Database name cannot be the same as the main database".to_string(),
        ));
    }
    logs.valid_dbname = "OK".to_string();

    let db_exists = sqlx::query_scalar!(
        "SELECT EXISTS (SELECT 1 FROM pg_catalog.pg_database WHERE datname = $1)",
        dbname
    )
    .fetch_one(db)
    .await?
    .unwrap_or(false);

    let pg_creds = PgDatabase { dbname: dbname.to_string(), ..wmill_pg_creds };

    logs.created_database = "SKIP".to_string();
    if !db_exists {
        sqlx::query(&format!("CREATE DATABASE \"{dbname}\""))
            .execute(db)
            .await?;
        logs.created_database = "OK".to_string();
    }

    // We have to connect to the newly created database as admin to grant permissions
    let (client, connection) = pg_creds.connect().await?;
    let join_handle = tokio::spawn(async move { connection.await });

    logs.db_connect = "OK".to_string();

    client
        .batch_execute(&format!(
            "GRANT CONNECT ON DATABASE \"{dbname}\" TO custom_instance_user;
             GRANT USAGE ON SCHEMA public TO custom_instance_user;
             GRANT CREATE ON SCHEMA public TO custom_instance_user;
             GRANT CREATE ON DATABASE \"{dbname}\" TO custom_instance_user;
             ALTER DEFAULT PRIVILEGES IN SCHEMA public
                 GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO custom_instance_user;
             ALTER ROLE custom_instance_user CREATEROLE;
             ALTER ROLE custom_instance_user REPLICATION;"
        ))
        .await
        .map_err(|e| {
            error::Error::ExecutionErr(format!(
                "Failed to grant permissions to custom_instance_user: {}",
                e.to_string(),
            ))
        })?;
    logs.grant_permissions = "OK".to_string();

    drop(client); // /!\ Drop before joining to avoid deadlock
    join_handle
        .await
        .map_err(|e| error::Error::ExecutionErr(format!("join error: {}", e.to_string())))?
        .map_err(|e| {
            error::Error::ExecutionErr(format!("tokio_postgres error: {}", e.to_string()))
        })?;

    Ok(())
}

// ============================================================================
// Secret Backend Settings (HashiCorp Vault Integration) - Enterprise Edition
// ============================================================================

/// Test connection to a secret backend (HashiCorp Vault)
///
/// This endpoint validates that the Vault settings are correct and that
/// Windmill can successfully authenticate and communicate with Vault.
///
/// This is an Enterprise Edition feature.
#[cfg(all(feature = "private", feature = "enterprise"))]
pub async fn test_secret_backend(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Json(settings): Json<VaultSettings>,
) -> Result<String> {
    require_super_admin(&db, &authed.email).await?;

    windmill_common::secret_backend::test_vault_connection(&settings, Some(&db)).await?;

    Ok("Successfully connected to HashiCorp Vault".to_string())
}

/// Migrate existing secrets from database to HashiCorp Vault
///
/// This endpoint reads all encrypted secrets from the database, decrypts them,
/// and stores them in HashiCorp Vault. The database values are NOT deleted
/// automatically to allow for rollback if needed.
///
/// This is an Enterprise Edition feature.
#[cfg(all(feature = "private", feature = "enterprise"))]
pub async fn migrate_secrets_to_vault(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Json(settings): Json<VaultSettings>,
) -> JsonResult<SecretMigrationReport> {
    require_super_admin(&db, &authed.email).await?;

    let report = windmill_common::secret_backend::migrate_secrets_to_vault(&db, &settings).await?;

    Ok(Json(report))
}

/// Migrate secrets from HashiCorp Vault back to database
///
/// This endpoint reads all secrets from HashiCorp Vault, encrypts them using
/// the workspace encryption keys, and stores them in the database. The Vault
/// values are NOT deleted automatically to allow for rollback if needed.
///
/// This is an Enterprise Edition feature.
#[cfg(all(feature = "private", feature = "enterprise"))]
pub async fn migrate_secrets_to_database(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Json(settings): Json<VaultSettings>,
) -> JsonResult<SecretMigrationReport> {
    require_super_admin(&db, &authed.email).await?;

    let report =
        windmill_common::secret_backend::migrate_secrets_to_database(&db, &settings).await?;

    Ok(Json(report))
}

// ============================================================================
// JWKS Endpoint for Vault JWT Authentication
// ============================================================================

/// JSON Web Key Set response structure
#[derive(Serialize)]
pub struct JwksResponse {
    pub keys: Vec<serde_json::Value>,
}

/// JWKS endpoint for HashiCorp Vault to validate JWTs
///
/// Vault calls this endpoint to fetch the public keys used to verify
/// JWTs generated by Windmill for authentication.
///
/// In the open-source version, this returns an empty JWKS.
/// The Enterprise Edition provides the actual key set.
pub async fn get_jwks() -> JsonResult<JwksResponse> {
    // Open source version returns empty JWKS
    // Enterprise Edition will override this with actual public keys
    #[cfg(not(feature = "enterprise"))]
    {
        Ok(Json(JwksResponse { keys: vec![] }))
    }

    #[cfg(feature = "enterprise")]
    {
        // In enterprise mode, the actual keys would be fetched from global settings
        // For now, return empty - the EE implementation would override this
        Ok(Json(JwksResponse { keys: vec![] }))
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
struct CachedResourceType {
    #[allow(dead_code)]
    id: i64,
    name: String,
    schema: Option<serde_json::Value>,
    #[allow(dead_code)]
    app: String,
    description: Option<String>,
}

async fn sync_cached_resource_types(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
) -> error::Result<String> {
    require_super_admin(&db, &authed.email).await?;

    use windmill_common::worker::HUB_RT_CACHE_DIR;
    let cache_path = format!("{}/resource_types.json", HUB_RT_CACHE_DIR);

    let content = tokio::fs::read_to_string(&cache_path).await.map_err(|e| {
        error::Error::NotFound(format!(
            "No cached resource types found at {}: {}",
            cache_path, e
        ))
    })?;

    let cached_types: Vec<CachedResourceType> = serde_json::from_str(&content).map_err(|e| {
        error::Error::InternalErr(format!("Failed to parse cached resource types: {}", e))
    })?;

    let mut synced_count = 0;

    for rt in &cached_types {
        let exists: Option<bool> = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM resource_type WHERE workspace_id = 'admins' AND name = $1 AND schema IS NOT DISTINCT FROM $2 AND description IS NOT DISTINCT FROM $3)",
            &rt.name,
            rt.schema.as_ref(),
            rt.description.as_deref(),
        )
        .fetch_one(&db)
        .await?;

        if exists.unwrap_or(false) {
            continue;
        }

        sqlx::query!(
            "INSERT INTO resource_type (workspace_id, name, schema, description, edited_at)
             VALUES ('admins', $1, $2, $3, now())
             ON CONFLICT (workspace_id, name) DO UPDATE
             SET schema = EXCLUDED.schema, description = EXCLUDED.description, edited_at = now()",
            &rt.name,
            rt.schema.as_ref(),
            rt.description.as_deref(),
        )
        .execute(&db)
        .await?;

        synced_count += 1;
    }

    Ok(format!(
        "Synced {} resource types ({} unchanged)",
        synced_count,
        cached_types.len() - synced_count
    ))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use windmill_common::instance_config::{GlobalSettings, InstanceConfig, WorkerGroupConfig};

    #[test]
    fn instance_config_yaml_round_trip() {
        let config = InstanceConfig {
            global_settings: GlobalSettings {
                base_url: Some("https://windmill.example.com".to_string()),
                retention_period_secs: Some(86400),
                expose_metrics: Some(true),
                ..Default::default()
            },
            worker_configs: BTreeMap::from([(
                "default".to_string(),
                WorkerGroupConfig {
                    worker_tags: Some(vec!["deno".to_string(), "python3".to_string()]),
                    init_bash: Some("apt-get update".to_string()),
                    ..Default::default()
                },
            )]),
        };

        let yaml = config.to_sorted_yaml().unwrap();

        // Verify key fields appear in the YAML output
        assert!(yaml.contains("base_url: https://windmill.example.com"));
        assert!(yaml.contains("retention_period_secs: 86400"));
        assert!(yaml.contains("expose_metrics: true"));
        assert!(yaml.contains("default:"));
        assert!(yaml.contains("- deno"));
        assert!(yaml.contains("- python3"));
        assert!(yaml.contains("init_bash: apt-get update"));

        // Round-trip back to struct
        let deserialized: InstanceConfig = serde_yml::from_str(&yaml).unwrap();
        assert_eq!(
            deserialized.global_settings.base_url.as_deref(),
            Some("https://windmill.example.com")
        );
        assert_eq!(
            deserialized.global_settings.retention_period_secs,
            Some(86400)
        );
        assert_eq!(deserialized.global_settings.expose_metrics, Some(true));
        let wc = &deserialized.worker_configs["default"];
        assert_eq!(
            wc.worker_tags.as_deref(),
            Some(["deno".to_string(), "python3".to_string()].as_slice())
        );
        assert_eq!(wc.init_bash.as_deref(), Some("apt-get update"));
    }

    #[test]
    fn sorted_yaml_global_settings_alphabetical() {
        let config = InstanceConfig {
            global_settings: GlobalSettings {
                retention_period_secs: Some(3600),
                base_url: Some("https://test.com".to_string()),
                expose_metrics: Some(true),
                email_domain: Some("example.com".to_string()),
                ..Default::default()
            },
            worker_configs: BTreeMap::new(),
        };

        let yaml = config.to_sorted_yaml().unwrap();

        // Keys must appear in alphabetical order
        let base_url_pos = yaml.find("base_url:").unwrap();
        let email_pos = yaml.find("email_domain:").unwrap();
        let expose_pos = yaml.find("expose_metrics:").unwrap();
        let retention_pos = yaml.find("retention_period_secs:").unwrap();

        assert!(
            base_url_pos < email_pos && email_pos < expose_pos && expose_pos < retention_pos,
            "global_settings keys should be alphabetically sorted, got yaml:\n{yaml}"
        );
    }

    #[test]
    fn sorted_yaml_worker_configs_default_and_native_first() {
        let config = InstanceConfig {
            global_settings: GlobalSettings::default(),
            worker_configs: BTreeMap::from([
                (
                    "gpu".to_string(),
                    WorkerGroupConfig {
                        init_bash: Some("echo gpu".to_string()),
                        ..Default::default()
                    },
                ),
                (
                    "native".to_string(),
                    WorkerGroupConfig {
                        init_bash: Some("echo native".to_string()),
                        ..Default::default()
                    },
                ),
                (
                    "default".to_string(),
                    WorkerGroupConfig {
                        init_bash: Some("echo default".to_string()),
                        ..Default::default()
                    },
                ),
                (
                    "alpha".to_string(),
                    WorkerGroupConfig {
                        init_bash: Some("echo alpha".to_string()),
                        ..Default::default()
                    },
                ),
            ]),
        };

        let yaml = config.to_sorted_yaml().unwrap();

        let default_pos = yaml.find("default:").unwrap();
        let native_pos = yaml.find("native:").unwrap();
        let alpha_pos = yaml.find("alpha:").unwrap();
        let gpu_pos = yaml.find("gpu:").unwrap();

        assert!(
            default_pos < native_pos
                && native_pos < alpha_pos
                && alpha_pos < gpu_pos,
            "worker_configs should have default, native first, then rest alphabetically, got yaml:\n{yaml}"
        );
    }

    #[test]
    fn sorted_yaml_roundtrips() {
        let config = InstanceConfig {
            global_settings: GlobalSettings {
                base_url: Some("https://rt.test".to_string()),
                retention_period_secs: Some(7200),
                expose_metrics: Some(false),
                ..Default::default()
            },
            worker_configs: BTreeMap::from([
                (
                    "default".to_string(),
                    WorkerGroupConfig {
                        worker_tags: Some(vec!["deno".to_string()]),
                        ..Default::default()
                    },
                ),
                (
                    "native".to_string(),
                    WorkerGroupConfig {
                        init_bash: Some("echo hi".to_string()),
                        ..Default::default()
                    },
                ),
            ]),
        };

        let yaml = config.to_sorted_yaml().unwrap();
        let deserialized: InstanceConfig = serde_yml::from_str(&yaml).unwrap();

        assert_eq!(
            deserialized.global_settings.base_url.as_deref(),
            Some("https://rt.test")
        );
        assert_eq!(
            deserialized.global_settings.retention_period_secs,
            Some(7200)
        );
        assert_eq!(deserialized.global_settings.expose_metrics, Some(false));
        assert_eq!(deserialized.worker_configs.len(), 2);
        assert_eq!(
            deserialized.worker_configs["default"]
                .worker_tags
                .as_deref(),
            Some(["deno".to_string()].as_slice())
        );
        assert_eq!(
            deserialized.worker_configs["native"].init_bash.as_deref(),
            Some("echo hi")
        );
    }
}
