use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct QueueInitJob {
    pub content: String,
}

use lazy_static::lazy_static;
use std::time::Duration;

use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};

lazy_static! {
    pub static ref BASE_INTERNAL_URL: String =
        std::env::var("BASE_INTERNAL_URL").unwrap_or("http://localhost:8080".to_string());
    pub static ref AGENT_TOKEN: String = std::env::var("AGENT_TOKEN").unwrap_or_default();
}

pub const AGENT_JWT_PREFIX: &str = "jwt_agent_";
pub fn build_agent_http_client(worker_suffix: &str) -> ClientWithMiddleware {
    ClientBuilder::new(
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
    .build()
}

// #[derive(Serialize, Deserialize)]
// pub struct PullJobRequest {
//     pub worker_name: String,
// }
