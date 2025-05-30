use crate::db::DB;
use axum::{body::Bytes, extract::Extension, response::IntoResponse, routing::post, Router};
use http::{HeaderMap, StatusCode};
use reqwest::Client;
use tracing::error;
use windmill_common::ee::{LICENSE_KEY, WINDMILL_CUSTOMER_SERVICE_BASE_URL};
use windmill_common::utils::get_uid;

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
    Extension(db): Extension<DB>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let request_url = format!("{}/inkeep", WINDMILL_CUSTOMER_SERVICE_BASE_URL.as_str());
    let license_key = LICENSE_KEY.read().await.clone();
    let uid = get_uid(&db).await.unwrap();

    // Ensure Content-Type is set to application/json
    let mut forwarded_headers = headers.clone();
    forwarded_headers.insert(
        http::header::CONTENT_TYPE,
        "application/json".parse().unwrap(),
    );
    forwarded_headers.insert(
        http::header::AUTHORIZATION,
        format!("Bearer {}", license_key).parse().unwrap(),
    );
    forwarded_headers.insert("X-Uid", uid.parse().unwrap());

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
                    tracing::error!("Failed to read response body: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        HeaderMap::new(),
                        Bytes::from(r#"{"error": "Failed to read response body"}"#),
                    )
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to send request to inkeep service: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                HeaderMap::new(),
                Bytes::from(r#"{"error": "Failed to connect to inkeep service"}"#),
            )
        }
    }
}
