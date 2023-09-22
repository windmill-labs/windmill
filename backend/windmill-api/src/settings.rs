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
    error::{self, to_anyhow, JsonResult},
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
        .implicit_tls(smtp.tls_implicit)
        .credentials((smtp.username, smtp.password));
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

pub async fn test_license_key(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Json(license_key): Json<String>,
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
pub async fn set_global_setting(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Path(key): Path<String>,
    Json(value): Json<Value>,
) -> error::Result<()> {
    require_super_admin(&db, &authed.email).await?;
    sqlx::query!(
        "INSERT INTO global_settings (name, value) VALUES ($1, $2) ON CONFLICT (name) DO UPDATE SET value = $2, updated_at = now()",
        key,
        value.value
    )
    .execute(&db)
    .await?;
    tracing::info!("Set global setting {} to {}", key, value.value);
    Ok(())
}

pub async fn get_global_setting(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Path(key): Path<String>,
) -> JsonResult<serde_json::Value> {
    require_super_admin(&db, &authed.email).await?;
    let value = sqlx::query!("SELECT value FROM global_settings WHERE name = $1", key)
        .fetch_optional(&db)
        .await?
        .map(|x| x.value);

    Ok(Json(value.unwrap_or_else(|| serde_json::Value::Null)))
}
