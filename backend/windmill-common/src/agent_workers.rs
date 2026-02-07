use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct QueueInitJob {
    pub content: String,
}

use lazy_static::lazy_static;
use std::time::Duration;

use reqwest_middleware::ClientBuilder;
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};

use crate::{jwt::decode_without_verify, utils::configure_client, worker::HttpClient};

/// Configuration required for agent mode. Both fields are mandatory when running in agent mode.
#[derive(Clone)]
pub struct AgentConfig {
    pub agent_token: String,
    pub base_internal_url: String,
}

impl AgentConfig {
    pub fn from_env() -> Result<Self, AgentConfigError> {
        let agent_token =
            std::env::var("AGENT_TOKEN").map_err(|_| AgentConfigError::MissingAgentToken)?;
        let base_internal_url = std::env::var("BASE_INTERNAL_URL")
            .map_err(|_| AgentConfigError::MissingBaseInternalUrl)?;
        Ok(Self { agent_token, base_internal_url })
    }

    pub fn build_http_client(&self, worker_suffix: &str) -> HttpClient {
        build_agent_http_client(worker_suffix, &self.agent_token, &self.base_internal_url)
    }
}

#[derive(Debug)]
pub enum AgentConfigError {
    MissingAgentToken,
    MissingBaseInternalUrl,
}

impl std::fmt::Display for AgentConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentConfigError::MissingAgentToken => {
                write!(
                    f,
                    "AGENT_TOKEN environment variable is not set but required for agent mode"
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

lazy_static! {
    pub static ref DECODED_AGENT_TOKEN: Option<AgentAuth> = {
        let agent_token = std::env::var("AGENT_TOKEN");
        if let Ok(token) = agent_token {
            decode_without_verify::<AgentAuth>(token.trim_start_matches(AGENT_JWT_PREFIX)).ok()
        } else {
            None
        }
    };
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AgentAuth {
    pub worker_group: String,
    pub suffix: Option<String>,
    pub tags: Vec<String>,
    pub exp: Option<usize>,
}

pub const AGENT_JWT_PREFIX: &str = "jwt_agent_";

pub fn build_agent_http_client(
    worker_suffix: &str,
    agent_token: &str,
    base_internal_url: &str,
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
            let token = format!(
                "{}{}_{}",
                AGENT_JWT_PREFIX,
                worker_suffix,
                agent_token.trim_start_matches(AGENT_JWT_PREFIX)
            );
            headers.insert(
                "Authorization",
                format!("Bearer {}", token).parse().unwrap(),
            );
            headers
        })
        .build()
        .expect("Failed to create HTTP client"),
    )
    .with(RetryTransientMiddleware::new_with_policy(
        ExponentialBackoff::builder().build_with_max_retries(5),
    ))
    .build();

    HttpClient { client, base_internal_url: base_internal_url.to_string() }
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
