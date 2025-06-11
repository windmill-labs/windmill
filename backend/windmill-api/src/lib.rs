/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use crate::db::ApiAuthed;
#[cfg(feature = "enterprise")]
use crate::ee_oss::ExternalJwks;
#[cfg(feature = "embedding")]
use crate::embeddings::load_embeddings_db;
#[cfg(feature = "oauth2")]
use crate::oauth2_oss::AllClients;
#[cfg(feature = "oauth2")]
use crate::oauth2_oss::SlackVerifier;
#[cfg(feature = "smtp")]
use crate::smtp_server_oss::SmtpServer;

#[cfg(feature = "mcp")]
use crate::mcp::{extract_and_store_workspace_id, setup_mcp_server, shutdown_mcp_server};
use crate::tracing_init::MyOnFailure;
use crate::{
    tracing_init::{MyMakeSpan, MyOnResponse},
    users::OptAuthed,
    webhook_util::WebhookShared,
};
#[cfg(feature = "agent_worker_server")]
use agent_workers_oss::AgentCache;

use anyhow::Context;
use argon2::Argon2;
use axum::extract::DefaultBodyLimit;
use axum::{middleware::from_extractor, routing::get, routing::post, Extension, Router};
use db::DB;
use http::HeaderValue;
use reqwest::Client;
#[cfg(feature = "oauth2")]
use std::collections::HashMap;
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
use windmill_common::{utils::GIT_VERSION, BASE_URL, INSTANCE_NAME};

use crate::scim_oss::has_scim_token;
use windmill_common::error::AppError;

#[cfg(all(feature = "agent_worker_server", feature = "private"))]
pub mod agent_workers_ee;
#[cfg(feature = "agent_worker_server")]
mod agent_workers_oss;
mod ai;
mod apps;
pub mod args;
mod audit;
pub mod auth;
mod capture;
mod concurrency_groups;
mod configs;
mod db;
mod drafts;
#[cfg(feature = "private")]
pub mod ee;
pub mod ee_oss;
pub mod embeddings;
mod favorite;
mod flows;
mod folders;
mod granular_acls;
mod groups;
#[cfg(feature = "http_trigger")]
mod http_trigger_args;
#[cfg(feature = "http_trigger")]
mod http_trigger_auth;
#[cfg(feature = "http_trigger")]
pub mod http_triggers;
#[cfg(feature = "private")]
pub mod indexer_ee;
mod indexer_oss;
#[cfg(feature = "private")]
mod inkeep_ee;
mod inkeep_oss;
mod inputs;
mod integration;
#[cfg(feature = "postgres_trigger")]
mod postgres_triggers;

mod approvals;
#[cfg(all(feature = "enterprise", feature = "private"))]
pub mod apps_ee;
#[cfg(feature = "enterprise")]
mod apps_oss;
#[cfg(all(feature = "enterprise", feature = "gcp_trigger", feature = "private"))]
pub mod gcp_triggers_ee;
#[cfg(all(feature = "enterprise", feature = "gcp_trigger"))]
mod gcp_triggers_oss;
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
#[cfg(all(feature = "enterprise", feature = "kafka", feature = "private"))]
pub mod kafka_triggers_ee;
#[cfg(all(feature = "enterprise", feature = "kafka"))]
mod kafka_triggers_oss;
#[cfg(feature = "mqtt_trigger")]
mod mqtt_triggers;
#[cfg(all(feature = "enterprise", feature = "nats", feature = "private"))]
pub mod nats_triggers_ee;
#[cfg(all(feature = "enterprise", feature = "nats"))]
mod nats_triggers_oss;
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
mod schedule;
#[cfg(feature = "private")]
pub mod scim_ee;
mod scim_oss;
mod scripts;
mod service_logs;
mod settings;
mod slack_approvals;
#[cfg(all(feature = "smtp", feature = "private"))]
pub mod smtp_server_ee;
#[cfg(feature = "smtp")]
mod smtp_server_oss;
#[cfg(all(feature = "enterprise", feature = "sqs_trigger", feature = "private"))]
pub mod sqs_triggers_ee;
#[cfg(all(feature = "enterprise", feature = "sqs_trigger"))]
mod sqs_triggers_oss;
#[cfg(feature = "private")]
pub mod teams_approvals_ee;
mod teams_approvals_oss;
mod trigger_helpers;

mod static_assets;
#[cfg(all(feature = "stripe", feature = "enterprise", feature = "private"))]
pub mod stripe_ee;
#[cfg(all(feature = "stripe", feature = "enterprise"))]
mod stripe_oss;
#[cfg(feature = "private")]
pub mod teams_ee;
mod teams_oss;
mod tracing_init;
mod triggers;
mod users;
#[cfg(feature = "private")]
pub mod users_ee;
mod users_oss;
mod utils;
mod variables;
pub mod webhook_util;
#[cfg(feature = "websocket")]
mod websocket_triggers;
mod workers;
mod workspaces;
#[cfg(feature = "private")]
pub mod workspaces_ee;
mod workspaces_export;
mod workspaces_extra;
mod workspaces_oss;

#[cfg(feature = "mcp")]
mod mcp;

pub const DEFAULT_BODY_LIMIT: usize = 2097152 * 100; // 200MB

lazy_static::lazy_static! {

    pub static ref REQUEST_SIZE_LIMIT: Arc<RwLock<usize>> = Arc::new(RwLock::new(DEFAULT_BODY_LIMIT));

    pub static ref SCIM_TOKEN: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));
    pub static ref SAML_METADATA: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));


    pub static ref COOKIE_DOMAIN: Option<String> = std::env::var("COOKIE_DOMAIN").ok();

    pub static ref IS_SECURE: Arc<RwLock<bool>> = Arc::new(RwLock::new(false));

    pub static ref HTTP_CLIENT: Client = reqwest::ClientBuilder::new()
        .user_agent("windmill/beta")
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .danger_accept_invalid_certs(std::env::var("ACCEPT_INVALID_CERTS").is_ok())
        .build().unwrap();


}

#[cfg(feature = "oauth2")]
lazy_static::lazy_static! {
    pub static ref OAUTH_CLIENTS: Arc<RwLock<AllClients>> = Arc::new(RwLock::new(AllClients {
        logins: HashMap::new(),
        connects: HashMap::new(),
        slack: None
    }));


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
                    user_db: user_db,
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

    let kafka_triggers_service = {
        #[cfg(all(feature = "enterprise", feature = "kafka"))]
        {
            kafka_triggers_oss::workspaced_service()
        }

        #[cfg(not(all(feature = "enterprise", feature = "kafka")))]
        {
            Router::new()
        }
    };

    let nats_triggers_service = {
        #[cfg(all(feature = "enterprise", feature = "nats"))]
        {
            nats_triggers_oss::workspaced_service()
        }

        #[cfg(not(all(feature = "enterprise", feature = "nats")))]
        {
            Router::new()
        }
    };

    let mqtt_triggers_service = {
        #[cfg(all(feature = "mqtt_trigger"))]
        {
            mqtt_triggers::workspaced_service()
        }

        #[cfg(not(feature = "mqtt_trigger"))]
        {
            Router::new()
        }
    };

    let gcp_triggers_service = {
        #[cfg(all(feature = "enterprise", feature = "gcp_trigger"))]
        {
            gcp_triggers_oss::workspaced_service()
        }

        #[cfg(not(all(feature = "enterprise", feature = "gcp_trigger")))]
        {
            Router::new()
        }
    };

    let sqs_triggers_service = {
        #[cfg(all(feature = "enterprise", feature = "sqs_trigger"))]
        {
            sqs_triggers_oss::workspaced_service()
        }

        #[cfg(not(all(feature = "enterprise", feature = "sqs_trigger")))]
        {
            Router::new()
        }
    };

    let websocket_triggers_service = {
        #[cfg(feature = "websocket")]
        {
            websocket_triggers::workspaced_service()
        }

        #[cfg(not(feature = "websocket"))]
        Router::new()
    };

    let http_triggers_service = {
        #[cfg(feature = "http_trigger")]
        {
            http_triggers::workspaced_service()
        }

        #[cfg(not(feature = "http_trigger"))]
        Router::new()
    };

    #[cfg(feature = "http_trigger")]
    {
        let http_killpill_rx = killpill_rx.resubscribe();
        http_triggers::refresh_routers_loop(&db, http_killpill_rx).await;
    }

    let postgres_triggers_service = {
        #[cfg(feature = "postgres_trigger")]
        {
            postgres_triggers::workspaced_service()
        }

        #[cfg(not(feature = "postgres_trigger"))]
        Router::new()
    };

    if !*CLOUD_HOSTED && server_mode && !mcp_mode {
        #[cfg(feature = "websocket")]
        {
            let ws_killpill_rx = killpill_rx.resubscribe();
            websocket_triggers::start_websockets(db.clone(), ws_killpill_rx);
        }

        #[cfg(all(feature = "enterprise", feature = "kafka"))]
        {
            let kafka_killpill_rx = killpill_rx.resubscribe();
            kafka_triggers_oss::start_kafka_consumers(db.clone(), kafka_killpill_rx);
        }

        #[cfg(all(feature = "enterprise", feature = "nats"))]
        {
            let nats_killpill_rx = killpill_rx.resubscribe();
            nats_triggers_oss::start_nats_consumers(db.clone(), nats_killpill_rx);
        }

        #[cfg(feature = "postgres_trigger")]
        {
            let db_killpill_rx = killpill_rx.resubscribe();
            postgres_triggers::start_database(db.clone(), db_killpill_rx);
        }

        #[cfg(feature = "mqtt_trigger")]
        {
            let mqtt_killpill_rx = killpill_rx.resubscribe();
            mqtt_triggers::start_mqtt_consumer(db.clone(), mqtt_killpill_rx);
        }

        #[cfg(all(feature = "enterprise", feature = "sqs_trigger"))]
        {
            let sqs_killpill_rx = killpill_rx.resubscribe();
            sqs_triggers_oss::start_sqs(db.clone(), sqs_killpill_rx);
        }

        #[cfg(all(feature = "enterprise", feature = "gcp_trigger"))]
        {
            let gcp_killpill_rx = killpill_rx.resubscribe();
            gcp_triggers_oss::start_consuming_gcp_pubsub_event(db.clone(), gcp_killpill_rx);
        }
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
    let (mcp_router, mcp_session_manager) = {
        #[cfg(feature = "mcp")]
        if server_mode || mcp_mode {
            let (mcp_router, mcp_session_manager) = setup_mcp_server().await?;
            // #[cfg(feature = "mcp")]
            // let mcp_main_ct = mcp_sse_server.config.ct.clone(); // Token to signal shutdown *to* MCP
            // #[cfg(feature = "mcp")]
            // let mcp_service_ct = mcp_sse_server.with_service(McpRunner::new); // Token to wait for MCP *service* shutdown
            (mcp_router, Some(mcp_session_manager))
        } else {
            (Router::new(), None)
        }

        #[cfg(not(feature = "mcp"))]
        (Router::new(), None)
    };

    #[cfg(feature = "agent_worker_server")]
    let (agent_workers_router, agent_workers_bg_processor, agent_workers_killpill_tx) =
        if server_mode {
            agent_workers_oss::workspaced_service(db.clone(), _base_internal_url.clone())
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
                        .nest("/folders", folders::workspaced_service())
                        .nest("/groups", groups::workspaced_service())
                        .nest("/inputs", inputs::workspaced_service())
                        .nest("/job_metrics", job_metrics::workspaced_service())
                        .nest("/job_helpers", job_helpers_service)
                        .nest("/jobs", jobs::workspaced_service())
                        .nest("/oauth", {
                            #[cfg(feature = "oauth2")]
                            {
                                oauth2_oss::workspaced_service()
                            }

                            #[cfg(not(feature = "oauth2"))]
                            Router::new()
                        })
                        .nest("/ai", ai::workspaced_service())
                        .nest("/raw_apps", raw_apps::workspaced_service())
                        .nest("/resources", resources::workspaced_service())
                        .nest("/schedules", schedule::workspaced_service())
                        .nest("/scripts", scripts::workspaced_service())
                        .nest(
                            "/users",
                            users::workspaced_service().layer(Extension(argon2.clone())),
                        )
                        .nest("/variables", variables::workspaced_service())
                        .nest("/workspaces", workspaces::workspaced_service())
                        .nest("/oidc", oidc_oss::workspaced_service())
                        .nest("/http_triggers", http_triggers_service)
                        .nest("/websocket_triggers", websocket_triggers_service)
                        .nest("/kafka_triggers", kafka_triggers_service)
                        .nest("/nats_triggers", nats_triggers_service)
                        .nest("/mqtt_triggers", mqtt_triggers_service)
                        .nest("/sqs_triggers", sqs_triggers_service)
                        .nest("/gcp_triggers", gcp_triggers_service)
                        .nest("/postgres_triggers", postgres_triggers_service)
                        .nest(
                            "/mcp",
                            mcp_router.clone().route_layer(axum::middleware::from_fn(
                                extract_and_store_workspace_id,
                            )),
                        ),
                )
                .nest("/workspaces", workspaces::global_service())
                .nest(
                    "/users",
                    users::global_service().layer(Extension(argon2.clone())),
                )
                .nest("/settings", settings::global_service())
                .nest("/workers", workers::global_service())
                .nest("/service_logs", service_logs::global_service())
                .nest("/configs", configs::global_service())
                .nest("/scripts", scripts::global_service())
                .nest("/integrations", integration::global_service())
                .nest("/groups", groups::global_service())
                .nest("/flows", flows::global_service())
                .nest("/apps", apps::global_service().layer(cors.clone()))
                .nest("/schedules", schedule::global_service())
                .nest("/embeddings", embeddings::global_service())
                .nest("/ai", ai::global_service())
                .nest("/inkeep", inkeep_oss::global_service())
                .route_layer(from_extractor::<ApiAuthed>())
                .route_layer(from_extractor::<users::Tokened>())
                .nest("/jobs", jobs::global_root_service())
                .nest(
                    "/srch/w/:workspace_id/index",
                    indexer_oss::workspaced_service(),
                )
                .nest("/srch/index", indexer_oss::global_service())
                .nest("/oidc", oidc_oss::global_service())
                .nest(
                    "/saml",
                    saml_oss::global_service().layer(Extension(Arc::clone(&sp_extension))),
                )
                .nest(
                    "/scim",
                    scim_oss::global_service()
                        .route_layer(axum::middleware::from_fn(has_scim_token)),
                )
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
                // DEPRECATED new path is /api/w/:workspace_id/mcp
                .nest(
                    "/mcp/w/:workspace_id",
                    mcp_router
                        .route_layer(axum::middleware::from_fn(extract_and_store_workspace_id)),
                )
                .nest("/agent_workers", {
                    #[cfg(feature = "agent_worker_server")]
                    {
                        agent_workers_oss::global_service().layer(Extension(agent_cache.clone()))
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
                    resources::public_service().layer(cors.clone()),
                )
                .nest(
                    "/w/:workspace_id/capture_u",
                    capture::workspaced_unauthed_service().layer(cors.clone()),
                )
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
                .nest(
                    "/r",
                    {
                        #[cfg(feature = "http_trigger")]
                        {
                            http_triggers::routes_global_service()
                        }

                        #[cfg(not(feature = "http_trigger"))]
                        {
                            Router::new()
                        }
                    }
                    .layer(from_extractor::<OptAuthed>()),
                )
                .nest(
                    "/gcp/w/:workspace_id",
                    {
                        #[cfg(all(feature = "enterprise", feature = "gcp_trigger"))]
                        {
                            gcp_triggers_oss::gcp_push_route_handler()
                        }
                        #[cfg(not(all(feature = "enterprise", feature = "gcp_trigger")))]
                        {
                            Router::new()
                        }
                    }
                    .layer(from_extractor::<OptAuthed>()),
                )
                .route("/version", get(git_v))
                .route("/uptodate", get(is_up_to_date))
                .route("/ee_license", get(ee_license))
                .route("/openapi.yaml", get(openapi))
                .route("/openapi.json", get(openapi_json)),
        )
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

    let server = axum::serve(listener, app.into_make_service());

    tracing::info!(
        instance = %*INSTANCE_NAME,
        "server started on port={} and addr={}",
        port,
        ip
    );

    port_tx
        .send(format!("http://localhost:{}", port))
        .expect("Failed to send port");

    let server = server.with_graceful_shutdown(async move {
        killpill_rx.recv().await.ok();
        #[cfg(feature = "agent_worker_server")]
        if let Some(agent_workers_killpill_tx) = agent_workers_killpill_tx {
            if let Err(e) = agent_workers_killpill_tx.kill().await {
                tracing::error!("Error killing agent workers: {e:#}");
            }
        }
        tracing::info!("Graceful shutdown of server");

        #[cfg(feature = "mcp")]
        if let Some(mcp_session_manager) = mcp_session_manager {
            shutdown_mcp_server(mcp_session_manager).await;
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

async fn openapi() -> &'static str {
    include_str!("../openapi-deref.yaml")
}

async fn openapi_json() -> &'static str {
    include_str!("../openapi-deref.json")
}

pub async fn migrate_db(db: &DB) -> anyhow::Result<Option<JoinHandle<()>>> {
    db::migrate(db)
        .await
        .map_err(|e| anyhow::anyhow!("Error migrating db: {e:#}"))
}
