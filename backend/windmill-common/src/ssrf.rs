use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use crate::error::Error;

pub const ALLOW_PRIVATE_MCP_SERVER_URLS_ENV: &str = "ALLOW_PRIVATE_MCP_SERVER_URLS";

pub const ALLOW_PRIVATE_SAML_METADATA_URLS_ENV: &str = "ALLOW_PRIVATE_SAML_METADATA_URLS";

/// Why a URL failed SSRF validation.
///
/// The distinction matters for callers that gate private endpoints behind a
/// flag (e.g. `ALLOW_PRIVATE_AI_BASE_URLS`): a "set this env var" hint is only
/// actionable for [`SsrfValidationError::Private`]. Surfacing that hint for a
/// malformed URL or bad scheme sends users down the wrong path (see #9171).
#[derive(Debug)]
pub enum SsrfValidationError {
    /// The URL could not be parsed (e.g. missing `http://` scheme).
    InvalidUrl(String),
    /// Scheme is not `http`/`https`.
    DisallowedScheme(String),
    /// No host in the URL.
    MissingHost,
    /// DNS resolution failed for the host.
    ResolutionFailed { host: String, source: String },
    /// Host did not resolve to any address.
    NoAddresses(String),
    /// The URL targets (or resolves to) a private/internal address. `resolved`
    /// is true when the host was a DNS name that resolved to a private IP.
    Private { resolved: bool },
}

impl std::fmt::Display for SsrfValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SsrfValidationError::InvalidUrl(e) => write!(f, "Invalid URL: {e}"),
            SsrfValidationError::DisallowedScheme(s) => write!(
                f,
                "URL scheme '{s}' is not allowed, only http and https are permitted"
            ),
            SsrfValidationError::MissingHost => write!(f, "URL must have a host"),
            SsrfValidationError::ResolutionFailed { host, source } => {
                write!(f, "Failed to resolve host '{host}': {source}")
            }
            SsrfValidationError::NoAddresses(host) => {
                write!(f, "Host '{host}' did not resolve to any addresses")
            }
            SsrfValidationError::Private { resolved: false } => {
                write!(f, "URL targets a private/internal IP address")
            }
            SsrfValidationError::Private { resolved: true } => {
                write!(f, "URL resolves to a private/internal IP address")
            }
        }
    }
}

// Enables `?` from `validate_url_for_ssrf` in functions returning
// `anyhow::Result` (e.g. the EE SAML metadata loader).
impl std::error::Error for SsrfValidationError {}

/// A URL that passed SSRF validation, carrying the exact addresses its host
/// resolved to so the eventual connect targets the SAME address that was
/// checked.
///
/// Validation resolves the host once and verifies every address is public; it
/// then hands those addresses back instead of discarding them. Callers pin them
/// onto their client — [`apply_dns_pinning`](ValidatedTarget::apply_dns_pinning)
/// for reqwest, or [`pinned_addrs`](ValidatedTarget::pinned_addrs) for a raw TCP
/// connect — so a DNS rebinder cannot answer a public IP at check-time and an
/// internal one (e.g. 169.254.169.254) at connect-time. Without pinning the
/// check and the connect resolve independently and the guard is a TOCTOU no-op.
///
/// `addrs` is empty when the host was an IP literal (there is nothing to rebind)
/// or when an `ALLOW_PRIVATE_*` override skipped resolution entirely; pinning is
/// then a no-op and the caller connects normally.
///
/// Limitation: pinning governs only *direct* connections. When a deployment
/// configures an outbound egress proxy (`HTTP_PROXY`/`HTTPS_PROXY`), the proxy
/// resolves the target host itself and the pin does not reach it — a property of
/// proxy-based egress shared by every app-side SSRF guard, not specific to this
/// one. The public/private pre-check still runs; closing the proxy hop would
/// require the proxy to resolve, which it owns.
#[derive(Debug, Clone)]
pub struct ValidatedTarget {
    /// The URL host, exactly as reqwest keys its DNS override on.
    pub host: String,
    /// Public addresses the host resolved to, to pin at connect time.
    pub addrs: Vec<SocketAddr>,
}

impl ValidatedTarget {
    /// A target with nothing to pin: an IP-literal host (no rebinding possible)
    /// or a host whose SSRF check was skipped by an `ALLOW_PRIVATE_*` override.
    fn unpinned(host: &str) -> Self {
        ValidatedTarget { host: host.to_string(), addrs: Vec::new() }
    }

    /// Pin the validated addresses onto a reqwest client builder so connect-time
    /// resolution cannot diverge from what was checked. No-op when there is
    /// nothing to pin (IP-literal host, or a skipped `ALLOW_PRIVATE_*` check).
    pub fn apply_dns_pinning(&self, builder: reqwest::ClientBuilder) -> reqwest::ClientBuilder {
        if self.addrs.is_empty() {
            builder
        } else {
            builder.resolve_to_addrs(&self.host, &self.addrs)
        }
    }

    /// The validated addresses to connect to, for callers that pin by opening
    /// the socket themselves (e.g. the WebSocket trigger's raw TCP connect)
    /// rather than through reqwest. Empty means "nothing to pin, connect
    /// normally".
    pub fn pinned_addrs(&self) -> &[SocketAddr] {
        &self.addrs
    }
}

impl From<SsrfValidationError> for Error {
    fn from(e: SsrfValidationError) -> Self {
        Error::BadRequest(e.to_string())
    }
}

/// Validates that a URL is safe to fetch server-side (not targeting private/internal networks).
///
/// Checks:
/// 1. Scheme must be http or https
/// 2. Host must be present and not a private/loopback/link-local IP
/// 3. The host is resolved and every address verified public
///
/// Returns the resolved addresses as a [`ValidatedTarget`] so the caller can pin
/// them onto the client that actually connects. Validating here and re-resolving
/// at connect time is a TOCTOU no-op against a DNS rebinder — the check only
/// closes the hole if the connect targets the SAME address this resolved.
pub async fn validate_url_for_ssrf(url: &str) -> Result<ValidatedTarget, SsrfValidationError> {
    let parsed =
        url::Url::parse(url).map_err(|e| SsrfValidationError::InvalidUrl(e.to_string()))?;

    // 1. Scheme check
    match parsed.scheme() {
        "http" | "https" => {}
        scheme => {
            return Err(SsrfValidationError::DisallowedScheme(scheme.to_string()));
        }
    }

    // 2. Host check
    let host = parsed.host_str().ok_or(SsrfValidationError::MissingHost)?;

    // 3. If the host is an IP literal, check it directly. There is nothing to
    // rebind (reqwest connects straight to the literal), so no addresses to pin.
    if let Ok(ip) = host.parse::<IpAddr>() {
        if is_private_ip(&ip) {
            return Err(SsrfValidationError::Private { resolved: false });
        }
        return Ok(ValidatedTarget::unpinned(host));
    }

    // 4. DNS resolution check — resolve the hostname and verify all IPs are public
    let port = parsed.port().unwrap_or(match parsed.scheme() {
        "https" => 443,
        _ => 80,
    });
    let resolve_target = format!("{host}:{port}");
    let addrs: Vec<std::net::SocketAddr> = tokio::net::lookup_host(&resolve_target)
        .await
        .map_err(|e| SsrfValidationError::ResolutionFailed {
            host: host.to_string(),
            source: e.to_string(),
        })?
        .collect();

    if addrs.is_empty() {
        return Err(SsrfValidationError::NoAddresses(host.to_string()));
    }

    for addr in &addrs {
        if is_private_ip(&addr.ip()) {
            return Err(SsrfValidationError::Private { resolved: true });
        }
    }

    Ok(ValidatedTarget { host: host.to_string(), addrs })
}

pub fn allow_private_mcp_server_urls() -> bool {
    std::env::var(ALLOW_PRIVATE_MCP_SERVER_URLS_ENV)
        .ok()
        .is_some_and(|v| v == "true" || v == "1")
}

pub fn allow_private_saml_metadata_urls() -> bool {
    std::env::var(ALLOW_PRIVATE_SAML_METADATA_URLS_ENV)
        .ok()
        .is_some_and(|v| v == "true" || v == "1")
}

pub async fn validate_saml_metadata_url(url: &str) -> Result<ValidatedTarget, SsrfValidationError> {
    let parsed =
        url::Url::parse(url).map_err(|e| SsrfValidationError::InvalidUrl(e.to_string()))?;

    match parsed.scheme() {
        "http" | "https" => {}
        scheme => return Err(SsrfValidationError::DisallowedScheme(scheme.to_string())),
    }

    let host = parsed.host_str().ok_or(SsrfValidationError::MissingHost)?;

    if allow_private_saml_metadata_urls() {
        return Ok(ValidatedTarget::unpinned(host));
    }

    validate_url_for_ssrf(url).await
}

pub async fn validate_mcp_server_url(url: &str) -> Result<ValidatedTarget, SsrfValidationError> {
    let parsed =
        url::Url::parse(url).map_err(|e| SsrfValidationError::InvalidUrl(e.to_string()))?;

    match parsed.scheme() {
        "http" | "https" => {}
        scheme => return Err(SsrfValidationError::DisallowedScheme(scheme.to_string())),
    }

    let host = parsed.host_str().ok_or(SsrfValidationError::MissingHost)?;

    if allow_private_mcp_server_urls() {
        return Ok(ValidatedTarget::unpinned(host));
    }

    validate_url_for_ssrf(url).await
}

/// Validate an MCP-related URL and return the [`ValidatedTarget`] so the caller
/// can pin the connect: the OAuth registration/discovery/token requests carry
/// secrets, so they must target the validated address (see
/// `windmill_mcp::oauth::no_redirect_http_client_pinned`). Callers that only
/// pre-validate (no adjacent connect) can discard the target.
pub async fn validate_mcp_server_url_for_bad_request(
    url: &str,
    label: &str,
) -> Result<ValidatedTarget, Error> {
    validate_mcp_server_url(url).await.map_err(|e| {
        Error::BadRequest(format!(
            "{label} is not allowed: {}",
            mcp_ssrf_error_message(&e)
        ))
    })
}

pub fn mcp_ssrf_error_message(e: &SsrfValidationError) -> String {
    match e {
        SsrfValidationError::Private { .. } => format!(
            "{e}. If you need to use private/internal MCP server URLs, \
             set the {ALLOW_PRIVATE_MCP_SERVER_URLS_ENV}=true environment variable"
        ),
        _ => e.to_string(),
    }
}

pub fn saml_ssrf_error_message(e: &SsrfValidationError) -> String {
    match e {
        SsrfValidationError::Private { .. } => format!(
            "{e}. If you need to use private/internal SAML metadata URLs, \
             set the {ALLOW_PRIVATE_SAML_METADATA_URLS_ENV}=true environment variable"
        ),
        _ => e.to_string(),
    }
}

fn is_private_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(ipv4) => is_private_ipv4(ipv4),
        IpAddr::V6(ipv6) => is_private_ipv6(ipv6),
    }
}

fn is_private_ipv4(ip: &Ipv4Addr) -> bool {
    ip.is_loopback()              // 127.0.0.0/8
        || ip.is_private()        // 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16
        || ip.is_link_local()     // 169.254.0.0/16 (AWS IMDS lives here)
        || ip.is_broadcast()      // 255.255.255.255
        || ip.is_unspecified()    // 0.0.0.0
        || ip.is_documentation()  // 192.0.2.0/24, 198.51.100.0/24, 203.0.113.0/24
        // CGNAT / shared address space
        || (ip.octets()[0] == 100 && (ip.octets()[1] & 0xC0) == 64) // 100.64.0.0/10
}

fn is_private_ipv6(ip: &Ipv6Addr) -> bool {
    ip.is_loopback()           // ::1
        || ip.is_unspecified() // ::
        // Unique local addresses (fc00::/7)
        || (ip.segments()[0] & 0xfe00) == 0xfc00
        // Link-local (fe80::/10)
        || (ip.segments()[0] & 0xffc0) == 0xfe80
        // IPv4-mapped addresses — check the embedded IPv4
        || match ip.to_ipv4_mapped() {
            Some(ipv4) => is_private_ipv4(&ipv4),
            None => false,
        }
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    struct PrivateMcpServerUrlsEnvGuard {
        previous: Option<String>,
    }

    impl PrivateMcpServerUrlsEnvGuard {
        fn set(value: Option<&str>) -> Self {
            let previous = std::env::var(ALLOW_PRIVATE_MCP_SERVER_URLS_ENV).ok();
            match value {
                Some(value) => std::env::set_var(ALLOW_PRIVATE_MCP_SERVER_URLS_ENV, value),
                None => std::env::remove_var(ALLOW_PRIVATE_MCP_SERVER_URLS_ENV),
            }
            Self { previous }
        }
    }

    impl Drop for PrivateMcpServerUrlsEnvGuard {
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => std::env::set_var(ALLOW_PRIVATE_MCP_SERVER_URLS_ENV, value),
                None => std::env::remove_var(ALLOW_PRIVATE_MCP_SERVER_URLS_ENV),
            }
        }
    }

    struct PrivateSamlMetadataUrlsEnvGuard {
        previous: Option<String>,
    }

    impl PrivateSamlMetadataUrlsEnvGuard {
        fn set(value: Option<&str>) -> Self {
            let previous = std::env::var(ALLOW_PRIVATE_SAML_METADATA_URLS_ENV).ok();
            match value {
                Some(value) => std::env::set_var(ALLOW_PRIVATE_SAML_METADATA_URLS_ENV, value),
                None => std::env::remove_var(ALLOW_PRIVATE_SAML_METADATA_URLS_ENV),
            }
            Self { previous }
        }
    }

    impl Drop for PrivateSamlMetadataUrlsEnvGuard {
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => std::env::set_var(ALLOW_PRIVATE_SAML_METADATA_URLS_ENV, value),
                None => std::env::remove_var(ALLOW_PRIVATE_SAML_METADATA_URLS_ENV),
            }
        }
    }

    #[test]
    fn test_private_ipv4() {
        assert!(is_private_ipv4(&"127.0.0.1".parse().unwrap()));
        assert!(is_private_ipv4(&"10.0.0.1".parse().unwrap()));
        assert!(is_private_ipv4(&"172.16.0.1".parse().unwrap()));
        assert!(is_private_ipv4(&"192.168.1.1".parse().unwrap()));
        assert!(is_private_ipv4(&"169.254.169.254".parse().unwrap()));
        assert!(is_private_ipv4(&"0.0.0.0".parse().unwrap()));
        assert!(is_private_ipv4(&"100.64.0.1".parse().unwrap()));
        assert!(!is_private_ipv4(&"8.8.8.8".parse().unwrap()));
        assert!(!is_private_ipv4(&"1.1.1.1".parse().unwrap()));
    }

    #[test]
    fn test_private_ipv6() {
        assert!(is_private_ipv6(&"::1".parse().unwrap()));
        assert!(is_private_ipv6(&"::".parse().unwrap()));
        assert!(is_private_ipv6(&"fc00::1".parse().unwrap()));
        assert!(is_private_ipv6(&"fd00::1".parse().unwrap()));
        assert!(is_private_ipv6(&"fe80::1".parse().unwrap()));
        // IPv4-mapped loopback
        assert!(is_private_ipv6(&"::ffff:127.0.0.1".parse().unwrap()));
        assert!(!is_private_ipv6(&"2001:4860:4860::8888".parse().unwrap()));
    }

    #[tokio::test]
    async fn test_validate_url_blocks_private() {
        assert!(validate_url_for_ssrf("http://127.0.0.1/foo").await.is_err());
        assert!(validate_url_for_ssrf("http://10.0.0.1/foo").await.is_err());
        assert!(
            validate_url_for_ssrf("http://169.254.169.254/latest/meta-data")
                .await
                .is_err()
        );
        assert!(validate_url_for_ssrf("http://[::1]/foo").await.is_err());
        assert!(validate_url_for_ssrf("ftp://example.com/foo")
            .await
            .is_err());
        assert!(validate_url_for_ssrf("file:///etc/passwd").await.is_err());
        assert!(validate_url_for_ssrf("not-a-url").await.is_err());
    }

    #[tokio::test]
    async fn test_validate_url_allows_public() {
        // This resolves to a public IP
        assert!(validate_url_for_ssrf("https://google.com").await.is_ok());
    }

    /// An IP-literal host has nothing to rebind — reqwest connects straight to
    /// the literal — so the target pins no addresses.
    #[tokio::test]
    async fn validate_url_ip_literal_pins_nothing() {
        let target = validate_url_for_ssrf("http://8.8.8.8:1234/x")
            .await
            .unwrap();
        assert_eq!(target.host, "8.8.8.8");
        assert!(target.pinned_addrs().is_empty());
    }

    /// Regression for the DNS-rebinding TOCTOU: the guard must surface the exact
    /// public addresses it validated so the caller can pin the connect to the
    /// SAME address. If
    /// this returned nothing, the connect would re-resolve and a rebinder could
    /// swap in an internal IP after the check.
    #[tokio::test]
    async fn validate_url_surfaces_resolved_addrs_for_pinning() {
        let target = validate_url_for_ssrf("https://google.com").await.unwrap();
        assert_eq!(target.host, "google.com");
        assert!(!target.pinned_addrs().is_empty());
        assert!(target
            .pinned_addrs()
            .iter()
            .all(|a| !is_private_ip(&a.ip())));
        // The pin applies cleanly onto a reqwest builder.
        let _ = target.apply_dns_pinning(reqwest::ClientBuilder::new());
    }

    /// Regression for #9171: a malformed base URL (missing scheme) must report
    /// `InvalidUrl`/`DisallowedScheme`, not `Private` — only `Private` gets the
    /// "set ALLOW_PRIVATE_AI_BASE_URLS" hint, which is misleading for a typo'd
    /// URL and sent the issue reporter down the wrong path.
    #[tokio::test]
    async fn test_error_variants_are_discriminated() {
        // No scheme and no colon → the exact "relative URL without a base"
        // error from the issue.
        assert!(matches!(
            validate_url_for_ssrf("api.example.com/v1").await,
            Err(SsrfValidationError::InvalidUrl(_))
        ));
        // `localhost:11434/v1` parses with `localhost` as the scheme — a very
        // common Ollama misconfiguration.
        assert!(matches!(
            validate_url_for_ssrf("localhost:11434/v1").await,
            Err(SsrfValidationError::DisallowedScheme(s)) if s == "localhost"
        ));
        assert!(matches!(
            validate_url_for_ssrf("ftp://example.com/foo").await,
            Err(SsrfValidationError::DisallowedScheme(_))
        ));
        assert!(matches!(
            validate_url_for_ssrf("http://127.0.0.1/foo").await,
            Err(SsrfValidationError::Private { resolved: false })
        ));
    }

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
    async fn private_mcp_error_message_includes_env_hint_only_for_private_urls() {
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

    #[tokio::test]
    async fn allow_private_saml_metadata_urls_defaults_to_false() {
        let _lock = TEST_ENV_LOCK.lock().await;
        let _guard = PrivateSamlMetadataUrlsEnvGuard::set(None);
        assert!(!allow_private_saml_metadata_urls());
    }

    #[tokio::test]
    async fn allow_private_saml_metadata_urls_honors_true_and_one() {
        let _lock = TEST_ENV_LOCK.lock().await;

        let _guard = PrivateSamlMetadataUrlsEnvGuard::set(Some("true"));
        assert!(allow_private_saml_metadata_urls());

        let _guard = PrivateSamlMetadataUrlsEnvGuard::set(Some("1"));
        assert!(allow_private_saml_metadata_urls());

        let _guard = PrivateSamlMetadataUrlsEnvGuard::set(Some("false"));
        assert!(!allow_private_saml_metadata_urls());
    }

    #[tokio::test]
    async fn validate_saml_metadata_url_blocks_private_by_default() {
        let _lock = TEST_ENV_LOCK.lock().await;
        let _guard = PrivateSamlMetadataUrlsEnvGuard::set(None);

        assert!(matches!(
            validate_saml_metadata_url("http://127.0.0.1/metadata").await,
            Err(SsrfValidationError::Private { resolved: false })
        ));
    }

    #[tokio::test]
    async fn validate_saml_metadata_url_allows_private_when_env_is_enabled() {
        let _lock = TEST_ENV_LOCK.lock().await;
        let _guard = PrivateSamlMetadataUrlsEnvGuard::set(Some("true"));

        assert!(validate_saml_metadata_url("http://127.0.0.1/metadata")
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn validate_saml_metadata_url_allows_private_when_env_is_one() {
        let _lock = TEST_ENV_LOCK.lock().await;
        let _guard = PrivateSamlMetadataUrlsEnvGuard::set(Some("1"));

        assert!(validate_saml_metadata_url("http://10.0.0.1/metadata")
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn validate_saml_metadata_url_keeps_syntax_guards_when_private_urls_are_allowed() {
        let _lock = TEST_ENV_LOCK.lock().await;
        let _guard = PrivateSamlMetadataUrlsEnvGuard::set(Some("true"));

        assert!(matches!(
            validate_saml_metadata_url("ftp://example.com/metadata").await,
            Err(SsrfValidationError::DisallowedScheme(_))
        ));
        assert!(matches!(
            validate_saml_metadata_url("not-a-url").await,
            Err(SsrfValidationError::InvalidUrl(_))
        ));
    }

    #[tokio::test]
    async fn saml_ssrf_error_message_includes_env_hint_only_for_private_urls() {
        let private_error = validate_url_for_ssrf("http://127.0.0.1/metadata")
            .await
            .unwrap_err();
        assert!(saml_ssrf_error_message(&private_error)
            .contains("ALLOW_PRIVATE_SAML_METADATA_URLS=true"));

        let invalid_error = validate_url_for_ssrf("ftp://example.com/metadata")
            .await
            .unwrap_err();
        assert!(
            !saml_ssrf_error_message(&invalid_error).contains(ALLOW_PRIVATE_SAML_METADATA_URLS_ENV)
        );
    }
}
