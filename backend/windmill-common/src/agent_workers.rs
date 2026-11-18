use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct QueueInitJob {
    pub content: String,
}

use lazy_static::lazy_static;
use std::{path::PathBuf, time::Duration};

use reqwest_middleware::ClientBuilder;
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};

use crate::{jwt::decode_without_verify, utils::configure_client, worker::HttpClient};

/// How an agent proves its identity to the server.
#[derive(Clone)]
pub enum AgentCredential {
    /// `AGENT_TOKEN`: an agent token a superadmin minted.
    Token(String),
    /// `AGENT_IDENTITY_TOKEN_FILE`: a workload identity token the platform keeps rotating on
    /// disk, re-read on every request.
    IdentityTokenFile(PathBuf),
}

/// Configuration required for agent mode: a credential and the server's URL.
#[derive(Clone)]
pub struct AgentConfig {
    pub credential: AgentCredential,
    pub base_internal_url: String,
}

impl AgentConfig {
    pub fn from_env() -> Result<Self, AgentConfigError> {
        let credential = match (
            std::env::var("AGENT_TOKEN").ok(),
            std::env::var(AGENT_IDENTITY_TOKEN_FILE_ENV).ok(),
        ) {
            (Some(_), Some(_)) => return Err(AgentConfigError::BothCredentials),
            (Some(token), None) => AgentCredential::Token(token),
            (None, Some(path)) => AgentCredential::IdentityTokenFile(PathBuf::from(path)),
            (None, None) => return Err(AgentConfigError::MissingAgentToken),
        };
        let base_internal_url = std::env::var("BASE_INTERNAL_URL")
            .map_err(|_| AgentConfigError::MissingBaseInternalUrl)?;
        Ok(Self { credential, base_internal_url })
    }

    pub fn build_http_client(&self, worker_suffix: &str) -> HttpClient {
        match &self.credential {
            AgentCredential::Token(token) => {
                build_agent_http_client(worker_suffix, token, &self.base_internal_url)
            }
            AgentCredential::IdentityTokenFile(path) => {
                build_agent_http_client_inner(&self.base_internal_url, None, |builder| {
                    builder.with(IdentityTokenAuth {
                        path: path.clone(),
                        worker_suffix: worker_suffix.to_string(),
                    })
                })
            }
        }
    }

    /// With a workload identity token the worker group and tags are whatever rule the server
    /// matched, so they are asked for before anything reads [`DECODED_AGENT_TOKEN`] (and
    /// through it `WORKER_GROUP`). A no-op with a static agent token, which carries them.
    pub async fn resolve_workload_identity(&self, worker_suffix: &str) -> anyhow::Result<()> {
        if let AgentCredential::IdentityTokenFile(_) = self.credential {
            let auth: AgentAuth = self
                .build_http_client(worker_suffix)
                .get("/api/agent_workers/whoami")
                .await?;
            let _ = AGENT_WORKLOAD_IDENTITY_AUTH.set(auth);
        }
        Ok(())
    }
}

pub const AGENT_IDENTITY_TOKEN_FILE_ENV: &str = "AGENT_IDENTITY_TOKEN_FILE";

#[derive(Debug)]
pub enum AgentConfigError {
    MissingAgentToken,
    BothCredentials,
    MissingBaseInternalUrl,
}

impl std::fmt::Display for AgentConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentConfigError::MissingAgentToken => {
                write!(
                    f,
                    "AGENT_TOKEN or AGENT_IDENTITY_TOKEN_FILE environment variable is required for agent mode"
                )
            }
            AgentConfigError::BothCredentials => {
                write!(
                    f,
                    "AGENT_TOKEN and AGENT_IDENTITY_TOKEN_FILE are mutually exclusive, set only one"
                )
            }
            AgentConfigError::MissingBaseInternalUrl => {
                write!(
                    f,
                    "BASE_INTERNAL_URL environment variable is not set but required for agent mode"
                )
            }
        }
    }
}

impl std::error::Error for AgentConfigError {}

/// Sets the bearer from the identity token file on each request, so a token the platform
/// rotated on disk is picked up without a restart.
struct IdentityTokenAuth {
    path: PathBuf,
    worker_suffix: String,
}

#[async_trait::async_trait]
impl reqwest_middleware::Middleware for IdentityTokenAuth {
    async fn handle(
        &self,
        mut req: reqwest::Request,
        extensions: &mut http::Extensions,
        next: reqwest_middleware::Next<'_>,
    ) -> reqwest_middleware::Result<reqwest::Response> {
        let token = tokio::fs::read_to_string(&self.path).await.map_err(|e| {
            reqwest_middleware::Error::middleware(std::io::Error::new(
                e.kind(),
                format!(
                    "reading {AGENT_IDENTITY_TOKEN_FILE_ENV} {}: {e}",
                    self.path.display()
                ),
            ))
        })?;
        let header = format!(
            "Bearer {AGENT_WORKLOAD_IDENTITY_PREFIX}{}{AGENT_WORKLOAD_IDENTITY_SEPARATOR}{}",
            self.worker_suffix,
            token.trim()
        )
        .parse()
        .map_err(reqwest_middleware::Error::middleware)?;
        req.headers_mut()
            .insert(reqwest::header::AUTHORIZATION, header);
        next.run(req, extensions).await
    }
}

lazy_static! {
    pub static ref DECODED_AGENT_TOKEN: Option<AgentAuth> = {
        let agent_token = std::env::var("AGENT_TOKEN");
        if let Ok(token) = agent_token {
            decode_without_verify::<AgentAuth>(token.trim_start_matches(AGENT_JWT_PREFIX)).ok()
        } else {
            AGENT_WORKLOAD_IDENTITY_AUTH.get().cloned()
        }
    };
}

static AGENT_WORKLOAD_IDENTITY_AUTH: std::sync::OnceLock<AgentAuth> = std::sync::OnceLock::new();

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AgentAuth {
    pub worker_group: String,
    pub suffix: Option<String>,
    pub tags: Vec<String>,
    pub exp: Option<usize>,
}

pub const AGENT_JWT_PREFIX: &str = "jwt_agent_";
/// `wid_agent_<worker suffix>:<workload identity JWT>`. A worker suffix may contain `_` (a
/// `WORKER_SUFFIX` label) and a JWT may contain `_` (base64url) but neither contains `:`.
pub const AGENT_WORKLOAD_IDENTITY_PREFIX: &str = "wid_agent_";
pub const AGENT_WORKLOAD_IDENTITY_SEPARATOR: char = ':';

pub fn build_agent_http_client(
    worker_suffix: &str,
    agent_token: &str,
    base_internal_url: &str,
) -> HttpClient {
    let token = format!(
        "{}{}_{}",
        AGENT_JWT_PREFIX,
        worker_suffix,
        agent_token.trim_start_matches(AGENT_JWT_PREFIX)
    );
    build_agent_http_client_inner(base_internal_url, Some(token), |builder| builder)
}

fn build_agent_http_client_inner(
    base_internal_url: &str,
    static_token: Option<String>,
    with_auth: impl FnOnce(ClientBuilder) -> ClientBuilder,
) -> HttpClient {
    let client = ClientBuilder::new(
        configure_client(
            reqwest::Client::builder()
                .pool_max_idle_per_host(10)
                .pool_idle_timeout(Duration::from_secs(60))
                .connect_timeout(Duration::from_secs(10))
                .timeout(Duration::from_secs(30)),
        )
        .default_headers({
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert(
                "User-Agent",                          // Replace with your desired header name
                "Windmill-Agent/1.0".parse().unwrap(), // Replace with your desired header value
            );
            if let Some(token) = static_token {
                headers.insert(
                    "Authorization",
                    format!("Bearer {}", token).parse().unwrap(),
                );
            }
            headers
        })
        .build()
        .expect("Failed to create HTTP client"),
    )
    .with(RetryTransientMiddleware::new_with_policy(
        ExponentialBackoff::builder().build_with_max_retries(5),
    ));

    HttpClient {
        client: with_auth(client).build(),
        base_internal_url: base_internal_url.to_string(),
    }
}

#[derive(Deserialize, Serialize)]
pub struct PingJobStatus {
    pub mem_peak: Option<i32>,
    pub current_mem: Option<i32>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct PingJobStatusResponse {
    pub canceled_by: Option<String>,
    pub canceled_reason: Option<String>,
    pub already_completed: bool,
}

// #[derive(Serialize, Deserialize)]
// pub struct PullJobRequest {
//     pub worker_name: String,
// }
