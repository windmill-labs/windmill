use axum::{
    body::Bytes,
    response::{IntoResponse, Response},
    routing::post,
    Router,
};
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

pub async fn send_inkeep_request(headers: HeaderMap, body: Bytes) -> impl IntoResponse {
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
            let response_headers = response.headers().clone();

            match response.bytes().await {
                Ok(response_body) => {
                    info!("Response body size: {} bytes", response_body.len());

                    // Build proper axum Response
                    let mut response_builder = Response::builder().status(status);

                    // Filter out hop-by-hop headers that shouldn't be forwarded
                    let skip_headers = [
                        "connection",
                        "keep-alive",
                        "proxy-authenticate",
                        "proxy-authorization",
                        "te",
                        "trailers",
                        "transfer-encoding",
                        "upgrade",
                        "content-length", // Let axum handle this
                    ];

                    // Add safe headers from the upstream response
                    for (key, value) in response_headers.iter() {
                        let key_str = key.as_str().to_lowercase();
                        if !skip_headers.contains(&key_str.as_str()) {
                            response_builder = response_builder.header(key, value);
                        }
                    }

                    // Ensure we have content-type for JSON responses
                    response_builder = response_builder.header("content-type", "application/json");

                    match response_builder.body(axum::body::Body::from(response_body)) {
                        Ok(response) => {
                            info!("Successfully built response");
                            response
                        }
                        Err(e) => {
                            error!("Failed to build response: {}", e);
                            Response::builder()
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .header("content-type", "application/json")
                                .body(axum::body::Body::from(
                                    r#"{"error": "Failed to build response"}"#,
                                ))
                                .unwrap()
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to read response body: {}", e);
                    Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .header("content-type", "application/json")
                        .body(axum::body::Body::from(
                            r#"{"error": "Failed to read response body"}"#,
                        ))
                        .unwrap()
                }
            }
        }
        Err(e) => {
            error!("Failed to send request to inkeep service: {}", e);
            Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .header("content-type", "application/json")
                .body(axum::body::Body::from(
                    r#"{"error": "Failed to connect to inkeep service"}"#,
                ))
                .unwrap()
        }
    }
}
