use crate::db::ApiAuthed;
use axum::{body::Bytes, response::IntoResponse, routing::post, Router};
use http::{HeaderMap, StatusCode};
use reqwest::Client;
use tracing::{error, info, warn};
use windmill_common::ee::WINDMILL_CUSTOMER_SERVICE_BASE_URL;
use windmill_common::error::{to_anyhow, Error, Result};

lazy_static::lazy_static! {
    static ref HTTP_CLIENT: Client = reqwest::ClientBuilder::new()
        .timeout(std::time::Duration::from_secs(60 * 5))
        .user_agent("windmill/beta")
        .build().unwrap();
}

pub fn global_service() -> Router {
    Router::new().route("/", post(send_inkeep_request))
}

pub async fn send_inkeep_request(
    authed: ApiAuthed,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let request_url = format!("{}/inkeep", WINDMILL_CUSTOMER_SERVICE_BASE_URL.as_str());

    // Ensure Content-Type is set to application/json
    let mut forwarded_headers = headers.clone();
    forwarded_headers.insert(
        http::header::CONTENT_TYPE,
        "application/json".parse().unwrap(),
    );

    match HTTP_CLIENT
        .post(&request_url)
        .headers(forwarded_headers)
        .body(body)
        .send()
        .await
    {
        Ok(response) => {
            let status = response.status();
            let mut response_headers = response.headers().clone();

            match response.bytes().await {
                Ok(response_body) => {
                    // Remove problematic headers that axum should handle
                    response_headers.remove("transfer-encoding");
                    response_headers.remove("content-length");

                    (status, response_headers, response_body)
                }
                Err(e) => {
                    error!("Failed to read response body: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        HeaderMap::new(),
                        Bytes::from(r#"{"error": "Failed to read response body"}"#),
                    )
                }
            }
        }
        Err(e) => {
            error!("Failed to send request to inkeep service: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                HeaderMap::new(),
                Bytes::from(r#"{"error": "Failed to connect to inkeep service"}"#),
            )
        }
    }
}
