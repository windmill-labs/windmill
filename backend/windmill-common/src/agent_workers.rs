use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct QueueInitJob {
    pub content: String,
}

use lazy_static::lazy_static;
use std::time::Duration;

use reqwest_middleware::ClientBuilder;
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};

use crate::{jwt::decode_without_verify, utils::{AGENT_JWT_PREFIX, AGENT_TOKEN}, worker::HttpClient};

lazy_static! {
    pub static ref BASE_INTERNAL_URL: String =
        std::env::var("BASE_INTERNAL_URL").unwrap_or("http://localhost:8080".to_string());
    pub static ref DECODED_AGENT_TOKEN: Option<AgentAuth> = {
        if AGENT_TOKEN.is_empty() {
            None
        } else {
            decode_without_verify::<AgentAuth>(AGENT_TOKEN.trim_start_matches(AGENT_JWT_PREFIX))
                .ok()
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

pub fn build_agent_http_client(worker_suffix: &str) -> HttpClient {
    let client = ClientBuilder::new(
        reqwest::Client::builder()
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Duration::from_secs(60))
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
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
                    AGENT_TOKEN.trim_start_matches(AGENT_JWT_PREFIX),
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
    HttpClient(client)
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
