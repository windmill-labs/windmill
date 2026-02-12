/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use crate::db::ApiAuthed;
#[cfg(feature = "embedding")]
use crate::embeddings::load_embeddings_db;
#[cfg(feature = "oauth2")]
use crate::oauth2_oss::SlackVerifier;
#[cfg(feature = "smtp")]
use crate::smtp_server_oss::SmtpServer;
#[cfg(feature = "enterprise")]
use windmill_api_auth::ee_oss::ExternalJwks;
use windmill_store::resources::public_service;

#[cfg(feature = "mcp")]
use crate::mcp::{extract_and_store_workspace_id, setup_mcp_server};
use crate::triggers::start_all_listeners;
use tower_http::catch_panic::CatchPanicLayer;

use crate::tracing_init::MyOnFailure;
use crate::{
    tracing_init::{MyMakeSpan, MyOnResponse},
    users::OptAuthed,
    webhook_util::WebhookShared,
};
#[cfg(feature = "agent_worker_server")]
use windmill_api_agent_workers::AgentCache;

use anyhow::Context;
use argon2::Argon2;
use axum::body::Body;
use axum::extract::DefaultBodyLimit;
use axum::http::HeaderValue;
use axum::response::Response;
use axum::{middleware::from_extractor, routing::get, routing::post, Extension, Json, Router};
use db::DB;
use tokio::task::JoinHandle;
use windmill_common::global_settings::load_value_from_global_settings;
use windmill_common::global_settings::EMAIL_DOMAIN_SETTING;
use windmill_common::worker::HUB_CACHE_DIR;

use std::fs::DirBuilder;
use std::time::Duration;
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;
use tower::ServiceBuilder;
use tower_cookies::CookieManagerLayer;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use windmill_common::db::UserDB;
use windmill_common::worker::CLOUD_HOSTED;
#[allow(unused_imports)]
pub(crate) use windmill_common::BASE_URL;
use windmill_common::{
    utils::GIT_VERSION,
    INSTANCE_NAME,
};

use crate::scim_oss::has_scim_token;
use windmill_common::error::AppError;

mod ai;
mod apps;
pub mod args;
mod audit;
pub mod auth;
#[cfg(all(feature = "private", feature = "parquet"))]
pub mod azure_proxy_ee;
mod azure_proxy_oss;
#[cfg(feature = "bedrock")]
mod bedrock;
mod capture;
mod concurrency_groups;
mod db;

mod drafts;
#[cfg(feature = "private")]
pub mod ee;
pub mod ee_oss;
pub mod embeddings;
mod favorite;
pub mod flows;
mod folder_history;
mod folders;
mod granular_acls;
mod group_history;
mod groups;
mod health;
#[cfg(feature = "private")]
pub mod indexer_ee;
mod indexer_oss;
#[cfg(feature = "private")]
mod inkeep_ee;
mod inkeep_oss;
mod integration;
mod live_migrations;
#[cfg(all(feature = "private", feature = "parquet"))]
pub mod s3_proxy_ee;
mod s3_proxy_oss;
mod workspace_dependencies;

mod approvals;
#[cfg(all(feature = "enterprise", feature = "private"))]
pub mod apps_ee;
#[cfg(feature = "enterprise")]
mod apps_oss;
#[cfg(all(feature = "enterprise", feature = "private"))]
pub mod git_sync_ee;
#[cfg(feature = "enterprise")]
mod git_sync_oss;
#[cfg(all(feature = "parquet", feature = "private"))]
pub mod job_helpers_ee;
#[cfg(feature = "parquet")]
mod job_helpers_oss;
pub mod job_metrics;
pub mod jobs;
pub mod jobs_export;
#[cfg(all(feature = "oauth2", feature = "private"))]
pub mod oauth2_ee;
#[cfg(feature = "oauth2")]
pub mod oauth2_oss;
#[cfg(feature = "private")]
pub mod oidc_ee;
mod oidc_oss;
mod raw_apps;
mod resources;
#[cfg(feature = "private")]
pub mod saml_ee;
mod saml_oss;
#[cfg(feature = "private")]
pub mod scim_ee;
mod scim_oss;
mod scripts;
mod secret_backend_ext;
mod service_logs;
mod slack_approvals;
#[cfg(all(feature = "smtp", feature = "private"))]
pub mod smtp_server_ee;
#[cfg(feature = "smtp")]
mod smtp_server_oss;
#[cfg(feature = "private")]
pub mod teams_approvals_ee;
mod teams_approvals_oss;

#[cfg(feature = "native_trigger")]
pub mod native_triggers;
mod public_app_layer;
mod public_app_rate_limit;
mod static_assets;
#[cfg(all(feature = "stripe", feature = "enterprise", feature = "private"))]
pub mod stripe_ee;
#[cfg(all(feature = "stripe", feature = "enterprise"))]
mod stripe_oss;
#[cfg(feature = "private")]
pub mod teams_cache_ee;
mod teams_cache_oss;
#[cfg(feature = "private")]
pub mod teams_ee;
mod teams_oss;
mod token;
mod tracing_init;
pub mod triggers;
mod users;
#[cfg(feature = "private")]
pub mod users_ee;
mod users_oss;
mod utils;
mod variables;
pub mod webhook_util;
mod workspaces;
#[cfg(feature = "private")]
pub mod workspaces_ee;
mod workspaces_export;

#[cfg(feature = "mcp")]
mod mcp_tools;

#[cfg(feature = "mcp")]
mod mcp;
#[cfg(all(feature = "mcp", feature = "private"))]
mod mcp_oauth_ee;
#[cfg(feature = "mcp")]
mod mcp_oauth_oss;

pub use apps::EditApp;
pub const DEFAULT_BODY_LIMIT: usize = 2097152 * 100; // 200MB

lazy_static::lazy_static! {

    pub static ref REQUEST_SIZE_LIMIT: Arc<RwLock<usize>> = Arc::new(RwLock::new(DEFAULT_BODY_LIMIT));

    pub static ref SCIM_TOKEN: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));
    pub static ref SAML_METADATA: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));


    // COOKIE_DOMAIN and IS_SECURE are now in windmill_common::utils
}

pub use windmill_common::utils::HTTP_CLIENT_PERMISSIVE as HTTP_CLIENT;

pub use windmill_common::utils::{COOKIE_DOMAIN, IS_SECURE};

#[cfg(feature = "oauth2")]
pub use windmill_oauth::OAUTH_CLIENTS;

#[cfg(feature = "oauth2")]
lazy_static::lazy_static! {
    pub static ref SLACK_SIGNING_SECRET: Option<SlackVerifier> = std::env::var("SLACK_SIGNING_SECRET")
        .ok()
        .map(|x| SlackVerifier::new(x).unwrap());

}

// Compliance with cloud events spec.
pub async fn add_webhook_allowed_origin(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    if req.method() == http::Method::OPTIONS {
        if let Some(webhook_request_origin) = req.headers().get("Webhook-Request-Origin") {
            let webhook_request_origin = webhook_request_origin.clone();
            let mut response = next.run(req).await;

            response
                .headers_mut()
                .insert("Webhook-Allowed-Origin", webhook_request_origin);
            response
                .headers_mut()
                .insert("Webhook-Allowed-Rate", HeaderValue::from_static("*"));
            return response;
        }
    }
    next.run(req).await
}

#[cfg(not(feature = "tantivy"))]
type IndexReader = ();

#[cfg(not(feature = "tantivy"))]
type ServiceLogIndexReader = ();

#[cfg(feature = "tantivy")]
type IndexReader = windmill_indexer::completed_runs_oss::IndexReader;
#[cfg(feature = "tantivy")]
type ServiceLogIndexReader = windmill_indexer::service_logs_oss::ServiceLogIndexReader;

pub async fn run_server(
    db: DB,
    job_index_reader: Option<IndexReader>,
    log_index_reader: Option<ServiceLogIndexReader>,
    addr: SocketAddr,
    mut killpill_rx: tokio::sync::broadcast::Receiver<()>,
    port_tx: tokio::sync::oneshot::Sender<String>,
    server_mode: bool,
    mcp_mode: bool,
    _base_internal_url: String,
    name: Option<String>,
) -> anyhow::Result<()> {
    let user_db = UserDB::new(db.clone());

    for x in [HUB_CACHE_DIR] {
        DirBuilder::new()
            .recursive(true)
            .create(x)
            .expect("could not create initial server dir");
    }

    #[cfg(feature = "enterprise")]
    let ext_jwks = ExternalJwks::load().await;
    let auth_cache = Arc::new(crate::auth::AuthCache::new(
        db.clone(),
        std::env::var("SUPERADMIN_SECRET").ok(),
        #[cfg(feature = "enterprise")]
        ext_jwks,
    ));
    let argon2 = Arc::new(Argon2::default());

    // Initialize debug signing key for debugger authentication
    windmill_api_debug::init_debug_signing_key().await;

    let disable_response_logs = std::env::var("DISABLE_RESPONSE_LOGS")
        .ok()
        .map(|x| x == "true")
        .unwrap_or(false);

    let middleware_stack = ServiceBuilder::new()
        .layer(Extension(db.clone()))
        .layer(Extension(user_db.clone()))
        .layer(Extension(auth_cache.clone()))
        .layer(Extension(job_index_reader))
        .layer(Extension(log_index_reader))
        // .layer(Extension(index_writer))
        .layer(CookieManagerLayer::new())
        .layer(Extension(WebhookShared::new(
            killpill_rx.resubscribe(),
            db.clone(),
        )))
        .layer(DefaultBodyLimit::max(
            REQUEST_SIZE_LIMIT.read().await.clone(),
        ));

    let cors = CorsLayer::new()
        .allow_methods([http::Method::GET, http::Method::POST])
        .allow_headers([http::header::CONTENT_TYPE, http::header::AUTHORIZATION])
        .allow_origin(Any);

    let sp_extension = Arc::new(saml_oss::build_sp_extension().await?);

    if server_mode {
        #[cfg(feature = "embedding")]
        load_embeddings_db(&db);

        #[cfg(feature = "cloud")]
        if *CLOUD_HOSTED {
            windmill_queue::init_usage_buffer(db.clone());
        }

        let mut start_smtp_server = false;
        if let Some(smtp_settings) =
            load_value_from_global_settings(&db, EMAIL_DOMAIN_SETTING).await?
        {
            if smtp_settings.as_str().unwrap_or("") != "" {
                start_smtp_server = true;
            }
        }
        if !start_smtp_server {
            tracing::info!("SMTP server not started because email domain is not set");
        } else {
            #[cfg(feature = "smtp")]
            {
                let smtp_server = Arc::new(SmtpServer {
                    db: db.clone(),
                    user_db: user_db.clone(),
                    auth_cache: auth_cache.clone(),
                    base_internal_url: _base_internal_url.clone(),
                });
                if let Err(err) = smtp_server.start_listener_thread(addr).await {
                    tracing::error!("Error starting SMTP server: {err:#}");
                }
            }
            #[cfg(not(feature = "smtp"))]
            {
                tracing::info!("SMTP server not started because SMTP feature is not enabled");
            }
        }
    }

    let job_helpers_service = {
        #[cfg(feature = "parquet")]
        {
            job_helpers_oss::workspaced_service()
        }

        #[cfg(not(feature = "parquet"))]
        {
            Router::new()
        }
    };

    // Initialize HTTP trigger refresh loop
    #[cfg(feature = "http_trigger")]
    {
        let http_killpill_rx = killpill_rx.resubscribe();
        triggers::http::refresh_routers_loop(&db, http_killpill_rx).await;
    }

    let triggers_service = triggers::generate_trigger_routers();

    if !*CLOUD_HOSTED && server_mode && !mcp_mode {
        start_all_listeners(db.clone(), &killpill_rx);
    }

    if server_mode {
        health::start_health_check_loop(db.clone(), killpill_rx.resubscribe());
    }

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .context("binding main windmill server")?;
    let port = listener.local_addr().map(|x| x.port()).unwrap_or(8000);
    let ip = listener
        .local_addr()
        .map(|x| x.ip().to_string())
        .unwrap_or("localhost".to_string());

    // Setup MCP server
    #[allow(unused_variables)]
    let (mcp_router, mcp_cancellation_token) = {
        #[cfg(feature = "mcp")]
        if server_mode || mcp_mode {
            use mcp::add_www_authenticate_header;
            let (mcp_router, mcp_cancellation_token) =
                setup_mcp_server(db.clone(), user_db, _base_internal_url.clone()).await?;
            // Apply middleware: auth check inside WWW-Authenticate wrapper so 401s get the header
            let mcp_router = mcp_router
                .route_layer(from_extractor::<ApiAuthed>())
                .layer(axum::middleware::from_fn(add_www_authenticate_header))
                .layer(axum::middleware::from_fn(extract_and_store_workspace_id));
            (mcp_router, Some(mcp_cancellation_token))
        } else {
            (Router::new(), None)
        }

        #[cfg(not(feature = "mcp"))]
        (Router::new(), Option::<()>::None)
    };

    let mcp_list_tools_service = {
        #[cfg(feature = "mcp")]
        {
            mcp::list_tools_service()
        }

        #[cfg(not(feature = "mcp"))]
        {
            Router::new()
        }
    };

    #[cfg(feature = "agent_worker_server")]
    let (agent_workers_router, agent_workers_bg_processor, agent_workers_job_completed_tx) =
        if server_mode {
            windmill_api_agent_workers::workspaced_service(db.clone(), _base_internal_url.clone())
        } else {
            (Router::new(), vec![], None)
        };

    #[cfg(feature = "agent_worker_server")]
    let agent_cache = Arc::new(AgentCache::new());

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
                        .nest("/assets", windmill_api_assets::workspaced_service())
                        .nest("/audit", audit::workspaced_service())
                        .nest("/capture", capture::workspaced_service())
                        .nest(
                            "/concurrency_groups",
                            concurrency_groups::workspaced_service(),
                        )
                        .nest("/embeddings", embeddings::workspaced_service())
                        .nest("/drafts", drafts::workspaced_service())
                        .nest("/favorites", favorite::workspaced_service())
                        .nest("/flows", flows::workspaced_service())
                        .nest(
                            "/workspace_dependencies",
                            workspace_dependencies::workspaced_service(),
                        )
                        .nest(
                            "/flow_conversations",
                            windmill_api_flow_conversations::workspaced_service(),
                        )
                        .nest("/folders", folders::workspaced_service())
                        .nest("/folders_history", folder_history::workspaced_service())
                        .nest("/groups", groups::workspaced_service())
                        .nest("/groups_history", group_history::workspaced_service())
                        .nest("/inputs", windmill_api_inputs::workspaced_service())
                        .nest("/job_metrics", job_metrics::workspaced_service())
                        .nest("/job_helpers", job_helpers_service)
                        .nest("/jobs", jobs::workspaced_service())
                        .nest("/debug", windmill_api_debug::workspaced_service())
                        .nest("/native_triggers", {
                            #[cfg(feature = "native_trigger")]
                            {
                                native_triggers::handler::generate_native_trigger_routers().merge(
                                    native_triggers::workspace_integrations::workspaced_service(),
                                )
                            }
                            #[cfg(not(feature = "native_trigger"))]
                            {
                                axum::Router::new()
                            }
                        })
                        .nest("/oauth", {
                            #[cfg(feature = "oauth2")]
                            {
                                oauth2_oss::workspaced_service()
                            }

                            #[cfg(not(feature = "oauth2"))]
                            Router::new()
                        })
                        .nest("/mcp/oauth/server", {
                            #[cfg(feature = "mcp")]
                            {
                                // Only /approve requires authentication (called by frontend)
                                mcp::oauth_server::workspaced_authed_service()
                            }

                            #[cfg(not(feature = "mcp"))]
                            Router::new()
                        })
                        .nest("/ai", ai::workspaced_service())
                        .nest("/npm_proxy", windmill_api_npm_proxy::workspaced_service())
                        .nest("/raw_apps", raw_apps::workspaced_service())
                        .nest("/resources", resources::workspaced_service())
                        .nest("/schedules", windmill_api_schedule::workspaced_service())
                        .nest("/scripts", scripts::workspaced_service())
                        .nest(
                            "/users",
                            users::workspaced_service().layer(Extension(argon2.clone())),
                        )
                        .nest("/variables", variables::workspaced_service())
                        .nest("/workers", windmill_api_workers::workspaced_service())
                        .nest("/workspaces", workspaces::workspaced_service())
                        .nest("/oidc", oidc_oss::workspaced_service())
                        .nest("/openapi", {
                            #[cfg(feature = "http_trigger")]
                            {
                                windmill_api_openapi::openapi_service()
                            }

                            #[cfg(not(feature = "http_trigger"))]
                            {
                                Router::new()
                            }
                        })
                        .merge(triggers_service),
                )
                .nest("/workspaces", workspaces::global_service())
                .nest(
                    "/users",
                    users::global_service().layer(Extension(argon2.clone())),
                )
                .nest("/settings", windmill_api_settings::global_service())
                .nest("/workers", windmill_api_workers::global_service())
                .nest("/service_logs", service_logs::global_service())
                .nest("/configs", windmill_api_configs::global_service())
                .nest("/scripts", scripts::global_service())
                .nest("/integrations", integration::global_service())
                .nest("/groups", groups::global_service())
                .nest("/flows", flows::global_service())
                .nest("/apps", apps::global_service().layer(cors.clone()))
                .nest("/schedules", windmill_api_schedule::global_service())
                .nest("/embeddings", embeddings::global_service())
                .nest("/ai", ai::global_service())
                .nest("/inkeep", inkeep_oss::global_service())
                .nest("/mcp/w/:workspace_id/list_tools", mcp_list_tools_service)
                .nest("/health/detailed", health::detailed_service())
                .route_layer(from_extractor::<ApiAuthed>())
                .route_layer(from_extractor::<users::Tokened>())
                // Workspace-scoped OAuth endpoints that don't require authentication
                // (authorize and token are called by MCP client before user is authenticated)
                .nest("/w/:workspace_id/mcp/oauth/server", {
                    #[cfg(feature = "mcp")]
                    {
                        mcp::oauth_server::workspaced_unauthed_service()
                    }

                    #[cfg(not(feature = "mcp"))]
                    Router::new()
                })
                .nest("/jobs", jobs::global_root_service())
                .nest(
                    "/srch/w/:workspace_id/index",
                    indexer_oss::workspaced_service(),
                )
                .nest("/srch/index", indexer_oss::global_service())
                .nest("/oidc", oidc_oss::global_service())
                .nest("/debug", windmill_api_debug::global_service())
                .nest(
                    "/saml",
                    saml_oss::global_service().layer(Extension(Arc::clone(&sp_extension))),
                )
                .nest(
                    "/scim",
                    scim_oss::global_service()
                        .route_layer(axum::middleware::from_fn(has_scim_token)),
                )
                .nest("/tokens", token::global_service())
                .nest("/concurrency_groups", concurrency_groups::global_service())
                .nest("/scripts_u", scripts::global_unauthed_service())
                .nest("/apps_u", {
                    #[cfg(feature = "enterprise")]
                    {
                        apps_oss::global_unauthed_service()
                    }

                    #[cfg(not(feature = "enterprise"))]
                    {
                        Router::new()
                    }
                })
                .nest(
                    "/w/:workspace_id/apps_u",
                    apps::unauthed_service()
                        .layer(from_extractor::<OptAuthed>())
                        .layer(cors.clone()),
                )
                .layer(from_extractor::<OptAuthed>())
                // Deprecated, here for backwards compatibility: user should use /mcp/w/:workspace_id/mcp instead
                .nest("/mcp/w/:workspace_id/sse", mcp_router.clone())
                .nest("/mcp/w/:workspace_id/mcp", mcp_router)
                .nest("/agent_workers", {
                    #[cfg(feature = "agent_worker_server")]
                    {
                        if let Some(agent_workers_job_completed_tx) =
                            agent_workers_job_completed_tx.clone()
                        {
                            windmill_api_agent_workers::global_service(agent_workers_job_completed_tx)
                                .layer(Extension(agent_cache.clone()))
                        } else {
                            Router::new()
                        }
                    }
                    #[cfg(not(feature = "agent_worker_server"))]
                    {
                        Router::new()
                    }
                })
                .nest("/w/:workspace_id/agent_workers", {
                    #[cfg(feature = "agent_worker_server")]
                    {
                        agent_workers_router.layer(Extension(agent_cache.clone()))
                    }
                    #[cfg(not(feature = "agent_worker_server"))]
                    {
                        Router::new()
                    }
                })
                .nest(
                    "/w/:workspace_id/jobs_u",
                    jobs::workspace_unauthed_service().layer(cors.clone()),
                )
                .route("/slack", post(slack_approvals::slack_app_callback_handler))
                .nest("/teams", {
                    #[cfg(feature = "enterprise")]
                    {
                        teams_oss::teams_service()
                    }

                    #[cfg(not(feature = "enterprise"))]
                    {
                        Router::new()
                    }
                })
                .route(
                    "/w/:workspace_id/jobs/slack_approval/:job_id",
                    get(slack_approvals::request_slack_approval),
                )
                .route(
                    "/w/:workspace_id/jobs/teams_approval/:job_id",
                    get(teams_approvals_oss::request_teams_approval),
                )
                .nest("/w/:workspace_id/github_app", {
                    #[cfg(feature = "enterprise")]
                    {
                        git_sync_oss::workspaced_service()
                    }

                    #[cfg(not(feature = "enterprise"))]
                    Router::new()
                })
                .nest("/github_app", {
                    #[cfg(feature = "enterprise")]
                    {
                        git_sync_oss::global_service()
                    }

                    #[cfg(not(feature = "enterprise"))]
                    Router::new()
                })
                .nest(
                    "/w/:workspace_id/resources_u",
                    public_service().layer(cors.clone()),
                )
                .nest(
                    "/w/:workspace_id/capture_u",
                    capture::workspaced_unauthed_service().layer(cors.clone()),
                )
                .nest("/w/:workspace_id/s3_proxy", {
                    s3_proxy_oss::workspaced_unauthed_service()
                })
                .nest(
                    "/auth",
                    users::make_unauthed_service().layer(Extension(argon2)),
                )
                .nest("/oauth", {
                    #[cfg(feature = "oauth2")]
                    {
                        oauth2_oss::global_service().layer(Extension(Arc::clone(&sp_extension)))
                    }

                    #[cfg(not(feature = "oauth2"))]
                    Router::new()
                })
                .nest("/mcp/oauth", {
                    #[cfg(feature = "mcp")]
                    {
                        mcp_oauth_oss::global_service()
                    }

                    #[cfg(not(feature = "mcp"))]
                    Router::new()
                })
                .nest("/mcp/oauth/server", {
                    #[cfg(feature = "mcp")]
                    {
                        mcp::oauth_server::global_service().layer(cors.clone())
                    }

                    #[cfg(not(feature = "mcp"))]
                    Router::new()
                })
                .nest("/r", {
                    #[cfg(feature = "http_trigger")]
                    {
                        triggers::http::handler::http_route_trigger_handler()
                    }

                    #[cfg(not(feature = "http_trigger"))]
                    {
                        Router::new()
                    }
                })
                .nest("/gcp/w/:workspace_id", {
                    #[cfg(all(
                        feature = "enterprise",
                        feature = "gcp_trigger",
                        feature = "private"
                    ))]
                    {
                        triggers::gcp::handler_oss::gcp_push_route_handler()
                    }
                    #[cfg(not(all(
                        feature = "enterprise",
                        feature = "gcp_trigger",
                        feature = "private"
                    )))]
                    {
                        Router::new()
                    }
                })
                .route("/version", get(git_v))
                .nest("/health/status", health::status_service())
                .route("/min_keep_alive_version", get(min_keep_alive_version))
                .route("/uptodate", get(is_up_to_date))
                .route("/ee_license", get(ee_license))
                .route("/openapi.yaml", get(openapi))
                .route("/openapi.json", get(openapi_json)),
        )
        // Clients must use workspace-scoped OAuth metadata at:
        // /.well-known/oauth-authorization-server/api/w/:workspace_id/mcp/oauth/server
        // This is discovered via /.well-known/oauth-protected-resource?workspace_id=...
        .route(
            "/.well-known/oauth-authorization-server/api/w/:workspace_id/mcp/oauth/server",
            {
                #[cfg(feature = "mcp")]
                {
                    get(mcp::oauth_server::workspaced_oauth_metadata)
                }
                #[cfg(not(feature = "mcp"))]
                {
                    get(|| async { axum::http::StatusCode::NOT_FOUND })
                }
            },
        )
        // RFC 9728 path-based discovery: /.well-known/oauth-protected-resource/api/mcp/w/:workspace_id/mcp
        .route(
            "/.well-known/oauth-protected-resource/api/mcp/w/:workspace_id/mcp",
            {
                #[cfg(feature = "mcp")]
                {
                    get(mcp::oauth_server::protected_resource_metadata_by_path)
                }
                #[cfg(not(feature = "mcp"))]
                {
                    get(|| async { axum::http::StatusCode::NOT_FOUND })
                }
            },
        )
        // JWKS endpoint for HashiCorp Vault JWT authentication (must be outside /api prefix)
        .route("/.well-known/jwks.json", get(windmill_api_settings::get_jwks))
        .fallback(static_assets::static_handler)
        .layer(middleware_stack);

    let app = if disable_response_logs {
        app
    } else {
        app.layer(
            TraceLayer::new_for_http()
                .on_response(MyOnResponse {})
                .make_span_with(MyMakeSpan {})
                .on_request(())
                .on_failure(MyOnFailure {}),
        )
    };

    let app = if let Some(domain) = public_app_layer::PUBLIC_APP_DOMAIN.as_ref() {
        tracing::info!("Public app domain filter enabled for domain: {}", domain);
        app.layer(axum::middleware::from_fn(
            public_app_layer::public_app_domain_filter,
        ))
    } else {
        app
    };

    let app = app.layer(CatchPanicLayer::custom(|err| {
        tracing::error!("panic in handler, returning 500: {:?}", err);
        Response::builder()
            .status(http::StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from("Internal Server Error"))
            .unwrap()
    }));

    if let Some(name) = name.as_ref() {
        tracing::info!("server starting for name={name}");
    }
    let server = axum::serve(listener, app.into_make_service()).tcp_nodelay(!server_mode);

    tracing::info!(
        instance = %*INSTANCE_NAME,
        "server started on port={} and addr={} {}",
        port,
        ip,
        name.map(|x| format!("name={x}")).unwrap_or_default()
    );

    if let Err(e) = port_tx.send(format!("http://localhost:{}", port)) {
        tracing::error!("Failed to send port: {e:#}");
        return Err(anyhow::anyhow!("Failed to send port, exiting early: {e:#}"));
    }

    let server = server.with_graceful_shutdown(async move {
        killpill_rx.recv().await.ok();
        #[cfg(feature = "agent_worker_server")]
        if let Some(agent_workers_job_completed_tx) = agent_workers_job_completed_tx {
            if let Err(e) = agent_workers_job_completed_tx.kill().await {
                tracing::error!("Error killing agent workers: {e:#}");
            }
        }
        tracing::info!("Graceful shutdown of server");

        #[cfg(feature = "mcp")]
        if let Some(mcp_cancellation_token) = mcp_cancellation_token {
            mcp_cancellation_token.cancel();
            tracing::info!("MCP server shutdown");
        }
    });

    server.await?;
    #[cfg(feature = "agent_worker_server")]
    for (i, bg_processor) in agent_workers_bg_processor.into_iter().enumerate() {
        tracing::info!("server off. shutting down agent worker bg processor {i}");
        bg_processor.await?;
        tracing::info!("agent worker bg processor {i} shut down");
    }
    Ok(())
}

async fn is_up_to_date() -> Result<String, AppError> {
    let error_reading_version = || anyhow::anyhow!("Error reading latest released version");
    let version = HTTP_CLIENT
        .get("https://api.github.com/repos/windmill-labs/windmill/releases/latest")
        .timeout(Duration::from_secs(10))
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

async fn min_keep_alive_version() -> Json<serde_json::Value> {
    let worker = windmill_common::min_version::MIN_KEEP_ALIVE_VERSION;
    let agent = windmill_common::min_version::AGENT_MIN_KEEP_ALIVE_VERSION;
    Json(serde_json::json!({
        "worker": format!("{}.{}.{}", worker.0, worker.1, worker.2),
        "agent": format!("{}.{}.{}", agent.0, agent.1, agent.2)
    }))
}

#[cfg(not(feature = "enterprise"))]
async fn ee_license() -> &'static str {
    ""
}

#[cfg(feature = "enterprise")]
async fn ee_license() -> String {
    use windmill_common::ee_oss::{LICENSE_KEY_ID, LICENSE_KEY_VALID};

    if *LICENSE_KEY_VALID.read().await {
        LICENSE_KEY_ID.read().await.clone()
    } else {
        "".to_string()
    }
}

async fn openapi() -> Response {
    Response::builder()
        .header("content-type", "application/yaml")
        .body(Body::from(include_str!("../openapi-deref.yaml")))
        .unwrap()
}

async fn openapi_json() -> Response {
    Response::builder()
        .header("content-type", "application/json")
        .body(Body::from(include_str!("../openapi-deref.json")))
        .unwrap()
}

pub async fn migrate_db(
    db: &DB,
    killpill_rx: tokio::sync::broadcast::Receiver<()>,
) -> anyhow::Result<Option<JoinHandle<()>>> {
    db::migrate(db, killpill_rx)
        .await
        .map_err(|e| anyhow::anyhow!("Error migrating db: {e:#}"))
}

pub async fn wait_for_db_migrations(
    db: &DB,
    killpill_rx: tokio::sync::broadcast::Receiver<()>,
) -> anyhow::Result<()> {
    db::wait_for_migrations(db, killpill_rx)
        .await
        .map_err(|e| anyhow::anyhow!("Error waiting for db migrations: {e:#}"))
}
