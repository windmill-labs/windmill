/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use argon2::Argon2;
use axum::{middleware::from_extractor, routing::get, Extension, Router};
use db::DB;
use git_version::git_version;
use std::{net::SocketAddr, sync::Arc};
use tower::ServiceBuilder;
use tower_cookies::CookieManagerLayer;
use tower_http::trace::TraceLayer;
use windmill_common::{error::to_anyhow, utils::rd_string};

use crate::{
    db::UserDB,
    oauth2::{build_oauth_clients, SlackVerifier},
    tracing_init::{MyMakeSpan, MyOnResponse},
    users::{Authed, OptAuthed},
};

mod apps;
mod audit;
mod capture;
mod db;
mod favorite;
mod flows;
mod folders;
mod granular_acls;
mod groups;
pub mod jobs;
mod oauth2;
mod resources;
mod schedule;
mod scripts;
mod static_assets;
mod tracing_init;
mod users;
mod utils;
mod variables;
mod worker_ping;
mod workspaces;

pub const GIT_VERSION: &str =
    git_version!(args = ["--tag", "--always"], fallback = "unknown-version");

pub struct BaseUrl(String);
pub struct IsSecure(bool);
pub struct CookieDomain(Option<String>);
pub struct CloudHosted(bool);
pub struct ContentSecurityPolicy(String);
pub struct TimeoutWaitResult(i32);

pub use users::delete_expired_items_perdiodically;

pub async fn run_server(
    db: DB,
    addr: SocketAddr,
    base_url: String,
    mut rx: tokio::sync::broadcast::Receiver<()>,
) -> anyhow::Result<()> {
    let user_db = UserDB::new(db.clone());

    let auth_cache = Arc::new(users::AuthCache::new(
        db.clone(),
        std::env::var("SUPERADMIN_SECRET").ok(),
    ));
    let argon2 = Arc::new(Argon2::default());
    let basic_clients = Arc::new(build_oauth_clients(&base_url).await?);
    let slack_verifier = Arc::new(
        std::env::var("SLACK_SIGNING_SECRET")
            .ok()
            .map(|x| SlackVerifier::new(x).unwrap()),
    );
    let http_client = reqwest::ClientBuilder::new()
        .user_agent("windmill/beta")
        .build()
        .map_err(to_anyhow)?;
    let middleware_stack = ServiceBuilder::new()
        .layer(
            TraceLayer::new_for_http()
                .on_response(MyOnResponse {})
                .make_span_with(MyMakeSpan {})
                .on_request(()),
        )
        .layer(Extension(db.clone()))
        .layer(Extension(user_db))
        .layer(Extension(auth_cache.clone()))
        .layer(Extension(basic_clients))
        .layer(Extension(Arc::new(BaseUrl(base_url.to_string()))))
        .layer(Extension(Arc::new(ContentSecurityPolicy(
            std::env::var("SERVE_CSP").unwrap_or("".to_owned()),
        ))))
        .layer(Extension(Arc::new(CloudHosted(
            std::env::var("CLOUD_HOSTED").is_ok(),
        ))))
        .layer(Extension(Arc::new(IsSecure(
            base_url.starts_with("https://"),
        ))))
        .layer(Extension(Arc::new(CookieDomain(
            std::env::var("COOKIE_DOMAIN").ok(),
        ))))
        .layer(Extension(http_client))
        .layer(CookieManagerLayer::new());
    // build our application with a route
    let app = Router::new()
        .nest(
            "/api",
            Router::new()
                .nest(
                    "/w/:workspace_id",
                    Router::new()
                        .nest("/scripts", scripts::workspaced_service())
                        .nest(
                            "/jobs",
                            jobs::workspaced_service().layer(Extension(Arc::new(
                                TimeoutWaitResult(
                                    std::env::var("TIMEOUT_WAIT_RESULT")
                                        .ok()
                                        .and_then(|x| x.parse().ok())
                                        .unwrap_or(20),
                                ),
                            ))),
                        )
                        .nest(
                            "/users",
                            users::workspaced_service().layer(Extension(argon2.clone())),
                        )
                        .nest("/variables", variables::workspaced_service())
                        .nest("/oauth", oauth2::workspaced_service())
                        .nest("/resources", resources::workspaced_service())
                        .nest("/schedules", schedule::workspaced_service())
                        .nest("/groups", groups::workspaced_service())
                        .nest("/audit", audit::workspaced_service())
                        .nest("/acls", granular_acls::workspaced_service())
                        .nest("/workspaces", workspaces::workspaced_service())
                        .nest("/apps", apps::workspaced_service())
                        .nest("/flows", flows::workspaced_service())
                        .nest("/capture", capture::workspaced_service())
                        .nest("/favorites", favorite::workspaced_service())
                        .nest("/folders", folders::workspaced_service()),
                )
                .nest("/workspaces", workspaces::global_service())
                .nest(
                    "/users",
                    users::global_service().layer(Extension(argon2.clone())),
                )
                .nest("/workers", worker_ping::global_service())
                .nest("/scripts", scripts::global_service())
                .nest("/flows", flows::global_service())
                .nest("/apps", apps::global_service())
                .nest("/schedules", schedule::global_service())
                .route_layer(from_extractor::<Authed>())
                .route_layer(from_extractor::<users::Tokened>())
                .nest(
                    "/w/:workspace_id/apps_u",
                    apps::unauthed_service().layer(from_extractor::<OptAuthed>()),
                )
                .nest("/w/:workspace_id/jobs_u", jobs::global_service())
                .nest("/w/:workspace_id/capture_u", capture::global_service())
                .nest(
                    "/auth",
                    users::make_unauthed_service().layer(Extension(argon2)),
                )
                .nest(
                    "/oauth",
                    oauth2::global_service().layer(Extension(slack_verifier)),
                )
                .route("/version", get(git_v))
                .route("/openapi.yaml", get(openapi)),
        )
        .fallback(static_assets::static_handler)
        .layer(middleware_stack);

    let instance_name = rd_string(5);

    tracing::info!(addr = %addr.to_string(), instance = %instance_name, "server started listening");
    let server = axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(async {
            rx.recv().await.ok();
            println!("Graceful shutdown of server");
        });

    tokio::spawn(async move { auth_cache.monitor().await });

    server.await?;
    Ok(())
}

async fn git_v() -> &'static str {
    GIT_VERSION
}

async fn openapi() -> &'static str {
    include_str!("../openapi-deref.yaml")
}
pub async fn migrate_db(db: &DB) -> anyhow::Result<()> {
    db::migrate(db).await?;
    Ok(())
}
