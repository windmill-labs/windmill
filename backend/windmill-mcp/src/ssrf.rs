use windmill_common::ssrf::SsrfValidationError;

pub(crate) const ALLOW_PRIVATE_MCP_SERVER_URLS_ENV: &str = "ALLOW_PRIVATE_MCP_SERVER_URLS";

pub(crate) fn allow_private_mcp_server_urls() -> bool {
    std::env::var(ALLOW_PRIVATE_MCP_SERVER_URLS_ENV)
        .ok()
        .is_some_and(|v| v == "true" || v == "1")
}

pub(crate) async fn validate_mcp_server_url(url: &str) -> Result<(), SsrfValidationError> {
    let parsed =
        reqwest::Url::parse(url).map_err(|e| SsrfValidationError::InvalidUrl(e.to_string()))?;

    match parsed.scheme() {
        "http" | "https" => {}
        scheme => return Err(SsrfValidationError::DisallowedScheme(scheme.to_string())),
    }

    parsed.host_str().ok_or(SsrfValidationError::MissingHost)?;

    if allow_private_mcp_server_urls() {
        return Ok(());
    }

    windmill_common::ssrf::validate_url_for_ssrf(url).await
}

#[cfg(feature = "auth")]
pub(crate) async fn validate_mcp_server_url_for_bad_request(
    url: &str,
    label: &str,
) -> Result<(), windmill_common::error::Error> {
    validate_mcp_server_url(url).await.map_err(|e| {
        windmill_common::error::Error::BadRequest(format!(
            "{label} is not allowed: {}",
            mcp_ssrf_error_message(&e)
        ))
    })
}

pub(crate) fn mcp_ssrf_error_message(e: &SsrfValidationError) -> String {
    match e {
        SsrfValidationError::Private { .. } => format!(
            "{e}. If you need to use private/internal MCP server URLs, \
             set the {ALLOW_PRIVATE_MCP_SERVER_URLS_ENV}=true environment variable"
        ),
        _ => e.to_string(),
    }
}

#[cfg(test)]
pub(crate) static TEST_ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[cfg(test)]
pub(crate) struct PrivateMcpServerUrlsEnvGuard {
    previous: Option<String>,
}

#[cfg(test)]
impl PrivateMcpServerUrlsEnvGuard {
    pub(crate) fn set(value: Option<&str>) -> Self {
        let previous = std::env::var(ALLOW_PRIVATE_MCP_SERVER_URLS_ENV).ok();
        match value {
            Some(value) => std::env::set_var(ALLOW_PRIVATE_MCP_SERVER_URLS_ENV, value),
            None => std::env::remove_var(ALLOW_PRIVATE_MCP_SERVER_URLS_ENV),
        }
        Self { previous }
    }
}

#[cfg(test)]
impl Drop for PrivateMcpServerUrlsEnvGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => std::env::set_var(ALLOW_PRIVATE_MCP_SERVER_URLS_ENV, value),
            None => std::env::remove_var(ALLOW_PRIVATE_MCP_SERVER_URLS_ENV),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn validate_mcp_server_url_blocks_private_by_default() {
        let _lock = TEST_ENV_LOCK.lock().await;
        let _guard = PrivateMcpServerUrlsEnvGuard::set(None);

        assert!(matches!(
            validate_mcp_server_url("http://127.0.0.1/foo").await,
            Err(SsrfValidationError::Private { resolved: false })
        ));
    }

    #[tokio::test]
    async fn validate_mcp_server_url_allows_private_when_env_is_enabled() {
        let _lock = TEST_ENV_LOCK.lock().await;
        let _guard = PrivateMcpServerUrlsEnvGuard::set(Some("true"));

        assert!(validate_mcp_server_url("http://127.0.0.1/foo")
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn validate_mcp_server_url_allows_private_when_env_is_one() {
        let _lock = TEST_ENV_LOCK.lock().await;
        let _guard = PrivateMcpServerUrlsEnvGuard::set(Some("1"));

        assert!(validate_mcp_server_url("http://10.0.0.1/foo").await.is_ok());
    }

    #[tokio::test]
    async fn validate_mcp_server_url_keeps_syntax_guards_when_private_urls_are_allowed() {
        let _lock = TEST_ENV_LOCK.lock().await;
        let _guard = PrivateMcpServerUrlsEnvGuard::set(Some("true"));

        assert!(matches!(
            validate_mcp_server_url("localhost:11434/v1").await,
            Err(SsrfValidationError::DisallowedScheme(_))
        ));
        assert!(matches!(
            validate_mcp_server_url("file:///tmp/socket").await,
            Err(SsrfValidationError::DisallowedScheme(_))
        ));
    }

    #[tokio::test]
    async fn private_error_message_includes_env_hint_only_for_private_urls() {
        let _lock = TEST_ENV_LOCK.lock().await;
        let _guard = PrivateMcpServerUrlsEnvGuard::set(None);

        let private_error = validate_mcp_server_url("http://127.0.0.1/foo")
            .await
            .unwrap_err();
        assert!(
            mcp_ssrf_error_message(&private_error).contains("ALLOW_PRIVATE_MCP_SERVER_URLS=true")
        );

        let invalid_error = validate_mcp_server_url("localhost:11434/v1")
            .await
            .unwrap_err();
        assert!(!mcp_ssrf_error_message(&invalid_error).contains(ALLOW_PRIVATE_MCP_SERVER_URLS_ENV));
    }
}
