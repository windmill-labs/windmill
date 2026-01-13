// Re-export from EE when private feature is enabled
#[cfg(feature = "private")]
pub use crate::mcp_oauth_ee::*;

// OSS stub implementations when private feature is not enabled
#[cfg(not(feature = "private"))]
mod oss_impl {
    use axum::{
        extract::Query,
        response::{Html, Redirect},
        routing::{get, post},
        Json, Router,
    };
    use serde::{Deserialize, Serialize};
    use windmill_common::error::{self, JsonResult};

    /// Global routes for MCP OAuth (OSS stub - returns errors)
    pub fn global_service() -> Router {
        Router::new()
            .route("/discover", post(discover_mcp_oauth))
            .route("/start", get(start_mcp_oauth))
            .route("/callback", get(mcp_oauth_callback))
            .route("/client-metadata.json", get(get_client_metadata))
    }

    #[derive(Serialize)]
    pub struct ClientMetadata {
        pub client_name: &'static str,
        pub redirect_uris: Vec<String>,
        pub grant_types: Vec<&'static str>,
        pub response_types: Vec<&'static str>,
        pub token_endpoint_auth_method: &'static str,
    }

    pub async fn get_client_metadata() -> Json<ClientMetadata> {
        Json(ClientMetadata {
            client_name: "Windmill",
            redirect_uris: vec![],
            grant_types: vec!["authorization_code", "refresh_token"],
            response_types: vec!["code"],
            token_endpoint_auth_method: "none",
        })
    }

    #[derive(Deserialize)]
    pub struct DiscoverRequest {
        pub mcp_server_url: String,
    }

    #[derive(Serialize)]
    pub struct DiscoverResponse {
        pub scopes_supported: Option<Vec<String>>,
        pub authorization_endpoint: String,
        pub token_endpoint: String,
        pub registration_endpoint: Option<String>,
        pub supports_dynamic_registration: bool,
    }

    pub async fn discover_mcp_oauth(
        Json(_req): Json<DiscoverRequest>,
    ) -> JsonResult<DiscoverResponse> {
        Err(error::Error::BadRequest(
            "Not implemented in Windmill's Open Source repository".to_string(),
        ))
    }

    #[derive(Deserialize)]
    pub struct StartPopupParams {
        pub mcp_server_url: String,
        #[serde(default)]
        pub scopes: Option<String>,
    }

    pub async fn start_mcp_oauth(
        Query(_params): Query<StartPopupParams>,
    ) -> Result<Redirect, error::Error> {
        Err(error::Error::BadRequest(
            "Not implemented in Windmill's Open Source repository".to_string(),
        ))
    }

    #[derive(Deserialize)]
    pub struct CallbackParams {
        pub code: String,
        pub state: String,
    }

    pub async fn mcp_oauth_callback(
        Query(_params): Query<CallbackParams>,
    ) -> Result<Html<String>, error::Error> {
        let html = r#"<!DOCTYPE html>
<html>
<head><title>MCP OAuth Error</title></head>
<body>
    <script>
        if (window.opener) {
            window.opener.postMessage({
                type: 'MCP_ERROR',
                error: 'Not implemented in Windmill's Open Source repository'
            }, window.location.origin);
        }
        window.close();
    </script>
    <p>Not implemented in Windmill's Open Source repository</p>
</body>
</html>"#;
        Ok(Html(html.to_string()))
    }
}

#[cfg(not(feature = "private"))]
pub use oss_impl::*;
