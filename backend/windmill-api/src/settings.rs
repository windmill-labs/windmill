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
    utils::require_super_admin,
    HTTP_CLIENT,
};

use axum::{
    extract::{Extension, Path},
    routing::{get, post},
    Json, Router,
};

use mail_send::{mail_builder::MessageBuilder, SmtpClientBuilder};
use serde::Deserialize;
use tokio::time::timeout;
use windmill_common::{
    error::{self, to_anyhow, JsonResult, Result},
    global_settings::ENV_SETTINGS,
    server::Smtp,
};

pub fn global_service() -> Router {
    Router::new()
        .route("/envs", get(get_local_settings))
        .route(
            "/global/:key",
            post(set_global_setting).get(get_global_setting),
        )
        .route("/test_smtp", post(test_email))
        .route("/test_license_key", post(test_license_key))
        .route("/send_stats", post(send_stats))
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
    let client = SmtpClientBuilder::new(smtp.host, smtp.port)
        .implicit_tls(smtp.tls_implicit.unwrap_or(false));
    let client = if let (Some(username), Some(password)) = (smtp.username, smtp.password) {
        if !username.is_empty() {
            client.credentials((username, password))
        } else {
            client
        }
    } else {
        client
    };
    let message = MessageBuilder::new()
        .from(("Windmill", smtp.from.as_str()))
        .to(to.clone())
        .subject("Test email from Windmill")
        .text_body("Test email content");
    let dur = Duration::from_secs(3);
    timeout(dur, client.connect())
        .await
        .map_err(to_anyhow)?
        .map_err(to_anyhow)?
        .send(message)
        .await
        .map_err(to_anyhow)?;
    tracing::info!("Sent test email to {to}");
    Ok("Sent test email".to_string())
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
    pub value: serde_json::Value,
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
    set_global_setting_internal(&db, key, value.value).await
}

pub async fn set_global_setting_internal(
    db: &DB,
    key: String,
    value: serde_json::Value,
) -> error::Result<()> {
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
    if !key.starts_with("default_error_handler_") && !key.starts_with("default_recovery_handler_") {
        require_super_admin(&db, &authed.email).await?;
    }
    let value = sqlx::query!("SELECT value FROM global_settings WHERE name = $1", key)
        .fetch_optional(&db)
        .await?
        .map(|x| x.value);

    Ok(Json(value.unwrap_or_else(|| serde_json::Value::Null)))
}

pub async fn send_stats(Extension(db): Extension<DB>, authed: ApiAuthed) -> Result<String> {
    require_super_admin(&db, &authed.email).await?;
    windmill_common::stats::send_stats(
        &"manual".to_string(),
        &windmill_common::utils::Mode::Server,
        &HTTP_CLIENT,
        &db,
        cfg!(feature = "enterprise"),
    )
    .await?;

    Ok("Sent stats".to_string())
}
