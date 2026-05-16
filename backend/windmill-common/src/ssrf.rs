use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use crate::error::Error;

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
/// 3. DNS resolution is checked to prevent DNS rebinding to internal IPs
pub async fn validate_url_for_ssrf(url: &str) -> Result<(), SsrfValidationError> {
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

    // 3. If the host is an IP literal, check it directly
    if let Ok(ip) = host.parse::<IpAddr>() {
        if is_private_ip(&ip) {
            return Err(SsrfValidationError::Private { resolved: false });
        }
        return Ok(());
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

    Ok(())
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
}
