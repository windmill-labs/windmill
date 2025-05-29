use axum::body::Bytes;
use reqwest::Client;
use windmill_common::ee::{EEServiceError, WINDMILL_CUSTOMER_SERVICE_BASE_URL};

lazy_static::lazy_static! {
    static ref HTTP_CLIENT: Client = reqwest::ClientBuilder::new()
        .timeout(std::time::Duration::from_secs(60 * 5))
        .user_agent("windmill/beta")
        .build().unwrap();

    static ref OPENAI_AZURE_BASE_PATH: Option<String> = std::env::var("OPENAI_AZURE_BASE_PATH").ok();

    pub static ref AI_REQUEST_CACHE: Cache<(String, AIProvider), ExpiringAIRequestConfig> = Cache::new(500);
}

pub async fn send_inkeep_request(body: Bytes) -> Result<()> {
    let request_url = format!("{}/inkeep", WINDMILL_CUSTOMER_SERVICE_BASE_URL.as_str());
    let response = http_client.post(request_url).body(body).send().await?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(EEServiceError::new("Failed to send inkeep request"))
    }
}
