/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use crate::db::ApiAuthed;
use crate::oauth2::AllClients;
use crate::saml::{SamlSsoLogin, ServiceProviderExt};
use crate::scim::has_scim_token;
use crate::tracing_init::MyOnFailure;
use crate::workers::ALL_TAGS;
use crate::{
    oauth2::{build_oauth_clients, SlackVerifier},
    tracing_init::{MyMakeSpan, MyOnResponse},
    users::OptAuthed,
    webhook_util::WebhookShared,
};
use anyhow::Context;
use argon2::Argon2;
use axum::extract::DefaultBodyLimit;
use axum::{middleware::from_extractor, routing::get, Extension, Router};
use db::DB;
use git_version::git_version;
use hyper::{http, Method};
use mail_send::SmtpClientBuilder;
use reqwest::Client;
use std::{net::SocketAddr, sync::Arc};
use tower::ServiceBuilder;
use tower_cookies::CookieManagerLayer;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use windmill_common::db::UserDB;
use windmill_common::utils::rd_string;

use windmill_common::error::AppError;

mod apps;
mod audit;
mod capture;
mod db;
mod drafts;
mod favorite;
mod flows;
mod folders;
mod granular_acls;
mod groups;
mod inputs;
pub mod jobs;
mod oauth2;
mod openai;
mod raw_apps;
mod resources;
mod saml;
mod schedule;
mod scim;
mod scripts;
mod settings;
mod static_assets;
mod tracing_init;
mod users;
mod utils;
mod variables;
mod webhook_util;
mod workers;
mod workspaces;

pub const GIT_VERSION: &str =
    git_version!(args = ["--tag", "--always"], fallback = "unknown-version");

pub use users::delete_expired_items_perdiodically;

pub const DEFAULT_BODY_LIMIT: usize = 2097152; // 2MB
lazy_static::lazy_static! {
    pub static ref BASE_URL: String = std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost".to_string());

    pub static ref REQUEST_SIZE_LIMIT: usize = std::env::var("REQUEST_SIZE_LIMIT")
    .ok()
    .and_then(|x| x.parse::<usize>().ok())
    .unwrap_or(DEFAULT_BODY_LIMIT);


    pub static ref COOKIE_DOMAIN: Option<String> = std::env::var("COOKIE_DOMAIN").ok();

    pub static ref SLACK_SIGNING_SECRET: Option<SlackVerifier> = std::env::var("SLACK_SIGNING_SECRET")
        .ok()
        .map(|x| SlackVerifier::new(x).unwrap());

        static ref IS_SECURE: bool = BASE_URL.starts_with("https://");

    pub static ref HTTP_CLIENT: Client = reqwest::ClientBuilder::new()
        .user_agent("windmill/beta")
        .danger_accept_invalid_certs(std::env::var("ACCEPT_INVALID_CERTS").is_ok())
        .build().unwrap();

    pub static ref OAUTH_CLIENTS: AllClients = build_oauth_clients(&BASE_URL)
        .map_err(|e| tracing::error!("Error building oauth clients (is the oauth.json mounted and in correct format? Use '{}' as minimal oauth.json): {}", "{}", e))
        .unwrap();

    pub static ref SMTP_CLIENT: Option<SmtpClientBuilder<String>> = {
        let smtp = parse_smtp();
        if let Some(smtp) = smtp {
            match smtp {
                Ok(smtp) => Some(smtp),
                Err(e) => {
                    tracing::error!("SMTP is not configured correctly, emails will not be sent: {}", e);
                    None
                }
            }
        } else {
            tracing::warn!("SMTP is not configured, emails will not be sent");
            None
        }
    };

    pub static ref SMTP_FROM: String = std::env::var("SMTP_FROM").unwrap_or_else(|_| "noreply@getwindmill.com".to_string());

    pub static ref LICENSE_KEY: Option<String> = std::env::var("LICENSE_KEY").ok();
}

pub fn parse_smtp() -> Option<windmill_common::error::Result<SmtpClientBuilder<String>>> {
    let username = std::env::var("SMTP_USERNAME").ok();
    let port = std::env::var("SMTP_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(587);
    let password = std::env::var("SMTP_PASSWORD").ok();
    let host = std::env::var("SMTP_HOST").ok();
    let tls_implicit = std::env::var("SMTP_TLS_IMPLICIT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(false);

    if username.is_some() && password.is_some() && host.is_some() {
        let smtp = SmtpClientBuilder::new(host.unwrap(), port)
            .implicit_tls(tls_implicit)
            .credentials((username.unwrap(), password.unwrap()));
        Some(Ok(smtp))
    } else {
        None
    }
}

pub async fn run_server(
    db: DB,
    rsmq: Option<rsmq_async::MultiplexedRsmq>,
    addr: SocketAddr,
    mut rx: tokio::sync::broadcast::Receiver<()>,
    port_tx: tokio::sync::oneshot::Sender<u16>,
) -> anyhow::Result<()> {
    if let Some(mut rsmq) = rsmq.clone() {
        for tag in ALL_TAGS.clone() {
            let r =
                rsmq_async::RsmqConnection::create_queue(&mut rsmq, &tag, None, None, None).await;
            if let Err(e) = r {
                tracing::info!("Redis queue {tag} could not be created: {e}");
            } else {
                tracing::info!("Redis queue {tag} created");
            }
        }
    }
    let user_db = UserDB::new(db.clone());

    let auth_cache = Arc::new(users::AuthCache::new(
        db.clone(),
        std::env::var("SUPERADMIN_SECRET").ok(),
    ));
    let argon2 = Arc::new(Argon2::default());

    let middleware_stack = ServiceBuilder::new()
        .layer(
            TraceLayer::new_for_http()
                .on_response(MyOnResponse {})
                .make_span_with(MyMakeSpan {})
                .on_request(())
                .on_failure(MyOnFailure {}),
        )
        .layer(Extension(db.clone()))
        .layer(Extension(rsmq))
        .layer(Extension(user_db))
        .layer(Extension(auth_cache.clone()))
        .layer(CookieManagerLayer::new())
        .layer(Extension(WebhookShared::new(rx.resubscribe(), db.clone())))
        .layer(DefaultBodyLimit::max(*REQUEST_SIZE_LIMIT));

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([http::header::CONTENT_TYPE, http::header::AUTHORIZATION])
        .allow_origin(Any);

    #[cfg(feature = "enterprise")]
    let sp_extension: (ServiceProviderExt, SamlSsoLogin) = saml::build_sp_extension().await?;

    #[cfg(not(feature = "enterprise"))]
    let sp_extension = (ServiceProviderExt(), SamlSsoLogin(None));

    // build our application with a route
    let app = Router::new()
        .nest(
            "/api",
            Router::new()
                .nest(
                    "/w/:workspace_id",
                    Router::new()
                        // Reordered alphabetically
                        .nest("/acls", granular_acls::workspaced_service())
                        .nest("/apps", apps::workspaced_service())
                        .nest("/raw_apps", raw_apps::workspaced_service())
                        .nest("/audit", audit::workspaced_service())
                        .nest("/capture", capture::workspaced_service())
                        .nest("/favorites", favorite::workspaced_service())
                        .nest("/flows", flows::workspaced_service())
                        .nest("/folders", folders::workspaced_service())
                        .nest("/groups", groups::workspaced_service())
                        .nest("/inputs", inputs::workspaced_service())
                        .nest("/jobs", jobs::workspaced_service())
                        .nest("/oauth", oauth2::workspaced_service())
                        .nest("/resources", resources::workspaced_service())
                        .nest("/schedules", schedule::workspaced_service())
                        .nest("/scripts", scripts::workspaced_service())
                        .nest("/drafts", drafts::workspaced_service())
                        .nest(
                            "/users",
                            users::workspaced_service().layer(Extension(argon2.clone())),
                        )
                        .nest("/variables", variables::workspaced_service())
                        .nest("/workspaces", workspaces::workspaced_service())
                        .nest("/openai", openai::workspaced_service()),
                )
                .nest("/workspaces", workspaces::global_service())
                .nest(
                    "/users",
                    users::global_service().layer(Extension(argon2.clone())),
                )
                .nest("/settings", settings::global_service())
                .nest("/jobs", jobs::global_root_service())
                .nest("/workers", workers::global_service())
                .nest("/scripts", scripts::global_service())
                .nest("/groups", groups::global_service())
                .nest("/flows", flows::global_service())
                .nest("/apps", apps::global_service().layer(cors.clone()))
                .nest("/schedules", schedule::global_service())
                .route_layer(from_extractor::<ApiAuthed>())
                .route_layer(from_extractor::<users::Tokened>())
                .nest(
                    "/saml",
                    saml::global_service().layer(Extension(Arc::new(sp_extension.0))),
                )
                .nest(
                    "/scim",
                    scim::global_service().route_layer(axum::middleware::from_fn(has_scim_token)),
                )
                .nest("/scripts_u", scripts::global_unauthed_service())
                .nest(
                    "/w/:workspace_id/apps_u",
                    apps::unauthed_service()
                        .layer(from_extractor::<OptAuthed>())
                        .layer(cors.clone()),
                )
                .nest(
                    "/w/:workspace_id/jobs_u",
                    jobs::global_service().layer(cors.clone()),
                )
                .nest(
                    "/w/:workspace_id/capture_u",
                    capture::global_service().layer(cors),
                )
                .nest(
                    "/auth",
                    users::make_unauthed_service().layer(Extension(argon2)),
                )
                .nest(
                    "/oauth",
                    oauth2::global_service().layer(Extension(Arc::new(sp_extension.1))),
                )
                .route("/version", get(git_v))
                .route("/uptodate", get(is_up_to_date))
                .route("/ee_license", get(ee_license))
                .route("/openapi.yaml", get(openapi)),
        )
        .fallback(static_assets::static_handler)
        .layer(middleware_stack);

    let instance_name = rd_string(5);

    tracing::info!(addr = %addr.to_string(), instance = %instance_name, "server started listening");
    let server = axum::Server::bind(&addr).serve(app.into_make_service());

    let port = server.local_addr().port();
    tracing::info!(
        "server started on port={} and addr={}",
        port,
        server.local_addr().ip()
    );
    port_tx
        .send(server.local_addr().port())
        .expect("Failed to send port");

    let server = server.with_graceful_shutdown(async {
        rx.recv().await.ok();
        println!("Graceful shutdown of server");
    });

    tokio::spawn(async move { auth_cache.monitor().await });

    server.await?;
    Ok(())
}

async fn is_up_to_date() -> Result<String, AppError> {
    let error_reading_version = || anyhow::anyhow!("Error reading latest released version");
    let version = HTTP_CLIENT
        .get("https://api.github.com/repos/windmill-labs/windmill/releases/latest")
        .send()
        .await
        .context("Impossible to reach api.github")?
        .json::<serde_json::Value>()
        .await?
        .get("tag_name")
        .ok_or_else(error_reading_version)?
        .as_str()
        .ok_or_else(error_reading_version)?
        .to_string();
    let release = GIT_VERSION
        .split('-')
        .next()
        .ok_or_else(error_reading_version)?
        .to_string();
    if version == release {
        Ok("yes".to_string())
    } else {
        Ok(format!("Update: {GIT_VERSION} -> {version}"))
    }
}

#[cfg(feature = "enterprise")]
async fn git_v() -> String {
    format!("EE {GIT_VERSION}")
}

#[cfg(not(feature = "enterprise"))]
async fn git_v() -> String {
    format!("CE {GIT_VERSION}")
}

#[cfg(not(feature = "enterprise"))]
async fn ee_license() -> &'static str {
    ""
}

#[cfg(feature = "enterprise")]
async fn ee_license() -> String {
    LICENSE_KEY
        .as_ref()
        .unwrap()
        .split(".")
        .next()
        .unwrap()
        .to_string()
}

async fn openapi() -> &'static str {
    include_str!("../openapi-deref.yaml")
}
pub async fn migrate_db(db: &DB) -> anyhow::Result<()> {
    db::migrate(db).await?;
    Ok(())
}
