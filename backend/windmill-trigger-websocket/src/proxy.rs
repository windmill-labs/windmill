//! HTTP CONNECT proxy support for outbound WebSocket connections.
//!
//! `tokio-tungstenite::connect_async` opens a raw TCP socket and does not
//! honour `HTTPS_PROXY` / `HTTP_PROXY` / `NO_PROXY`. On networks without
//! direct egress this leaves WebSocket triggers unable to reach the
//! upstream service. This module replicates the env-var conventions used
//! by `curl` / `reqwest` for outbound HTTP and establishes an HTTP CONNECT
//! tunnel before delegating the TLS + WebSocket handshake back to
//! tungstenite.

use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use std::io;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
};
use tokio_tungstenite::{
    client_async_tls_with_config,
    tungstenite::{
        client::IntoClientRequest,
        error::{Error as WsError, UrlError},
        handshake::client::Response,
    },
    MaybeTlsStream, WebSocketStream,
};

/// Drop-in replacement for `tokio_tungstenite::connect_async` that routes
/// the underlying TCP connection through `HTTPS_PROXY` / `HTTP_PROXY`
/// (with `NO_PROXY` exclusions) when those env vars are set.
pub async fn connect_async_with_proxy<R>(
    request: R,
) -> Result<(WebSocketStream<MaybeTlsStream<TcpStream>>, Response), WsError>
where
    R: IntoClientRequest + Unpin,
{
    let request = request.into_client_request()?;
    let uri = request.uri().clone();
    let scheme = uri.scheme_str().unwrap_or_default().to_ascii_lowercase();
    let host = uri
        .host()
        .ok_or(WsError::Url(UrlError::NoHostName))?
        .to_string();
    let port = uri
        .port_u16()
        .or_else(|| match scheme.as_str() {
            "wss" => Some(443),
            "ws" => Some(80),
            _ => None,
        })
        .ok_or(WsError::Url(UrlError::UnsupportedUrlScheme))?;

    let proxy = proxy_url_for(&scheme, &host).and_then(|raw| parse_proxy_target(&raw));

    let socket = if let Some(proxy) = proxy {
        tracing::debug!(
            "Connecting to WebSocket {}:{} through HTTP proxy {}:{}",
            host,
            port,
            proxy.host,
            proxy.port,
        );
        http_connect_tunnel(&proxy, &host, port)
            .await
            .map_err(WsError::Io)?
    } else {
        TcpStream::connect((host.as_str(), port))
            .await
            .map_err(WsError::Io)?
    };

    client_async_tls_with_config(request, socket, None, None).await
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProxyTarget {
    host: String,
    port: u16,
    /// Base64-encoded `user:pass` from URL userinfo, ready to drop into
    /// the `Proxy-Authorization: Basic …` header value.
    basic_auth: Option<String>,
}

fn lookup_env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .or_else(|| std::env::var(name.to_lowercase()).ok())
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// Resolve the proxy URL string to use for outbound `(scheme, host)`.
///
/// `wss://`/`https://` reads `HTTPS_PROXY`, `ws://`/`http://` reads
/// `HTTP_PROXY`, falling back to `ALL_PROXY`. `NO_PROXY` short-circuits
/// to `None`. Lowercase variants are accepted as a fallback.
fn proxy_url_for(scheme: &str, host: &str) -> Option<String> {
    if let Some(no_proxy) = lookup_env("NO_PROXY") {
        if matches_no_proxy(host, &no_proxy) {
            return None;
        }
    }
    let primary = if scheme.eq_ignore_ascii_case("wss") || scheme.eq_ignore_ascii_case("https") {
        "HTTPS_PROXY"
    } else {
        "HTTP_PROXY"
    };
    lookup_env(primary).or_else(|| lookup_env("ALL_PROXY"))
}

/// Match a host against a `NO_PROXY` value (comma-separated list).
///
/// Supports the conventional rules used by `curl`/`reqwest`:
/// - `*` matches everything
/// - exact host match
/// - bare-domain entry (`example.com`) matches `example.com` and any
///   subdomain (`foo.example.com`)
/// - leading-dot entry (`.example.com`) is normalised to the bare form
///   (matches `example.com` and any subdomain), to match what `reqwest`
///   and most ops folks expect
/// - any `:port` suffix on entries is ignored
///
/// CIDR/IP-range matches are intentionally not supported.
fn matches_no_proxy(host: &str, no_proxy: &str) -> bool {
    let host = host.trim().trim_end_matches('.').to_ascii_lowercase();
    if host.is_empty() {
        return false;
    }
    for raw in no_proxy.split(',') {
        let entry = raw.trim().to_ascii_lowercase();
        if entry.is_empty() {
            continue;
        }
        if entry == "*" {
            return true;
        }
        let entry = entry.split(':').next().unwrap_or(&entry);
        let entry = entry.trim_end_matches('.');
        let bare = entry.trim_start_matches('.');
        if bare.is_empty() {
            continue;
        }
        if host == bare {
            return true;
        }
        if host.ends_with(&format!(".{}", bare)) {
            return true;
        }
    }
    false
}

/// Parse a proxy URL string into host/port and optional pre-encoded basic
/// auth. Accepts `host`, `host:port`, `scheme://host[:port]`, with an
/// optional `user[:pass]@` userinfo prefix. The scheme is used only to
/// pick a default port (`https` → 443, anything else → 80).
fn parse_proxy_target(raw: &str) -> Option<ProxyTarget> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }

    let (scheme, after_scheme) = match raw.split_once("://") {
        Some((s, r)) => (s.to_ascii_lowercase(), r),
        None => (String::from("http"), raw),
    };
    let authority = after_scheme
        .split(['/', '?', '#'])
        .next()
        .unwrap_or(after_scheme);

    let (userinfo, host_port) = match authority.rsplit_once('@') {
        Some((u, h)) => (Some(u), h),
        None => (None, authority),
    };

    let (host, port) = if let Some(rest) = host_port.strip_prefix('[') {
        let end = rest.find(']')?;
        let h = rest[..end].to_string();
        let after = &rest[end + 1..];
        let p = after.strip_prefix(':').and_then(|p| p.parse::<u16>().ok());
        (h, p)
    } else {
        match host_port.rsplit_once(':') {
            Some((h, p)) => match p.parse::<u16>() {
                Ok(p) => (h.to_string(), Some(p)),
                Err(_) => (host_port.to_string(), None),
            },
            None => (host_port.to_string(), None),
        }
    };

    if host.is_empty() {
        return None;
    }

    let port = port.unwrap_or(if scheme == "https" { 443 } else { 80 });
    let basic_auth = userinfo.map(|u| BASE64_STANDARD.encode(u));

    Some(ProxyTarget { host, port, basic_auth })
}

/// Open a TCP connection to `proxy` and ask it to tunnel to
/// `(target_host, target_port)` via HTTP CONNECT. Returns the raw socket
/// once the proxy has acknowledged with a 2xx response — subsequent bytes
/// belong to the tunneled connection.
async fn http_connect_tunnel(
    proxy: &ProxyTarget,
    target_host: &str,
    target_port: u16,
) -> io::Result<TcpStream> {
    let mut stream = TcpStream::connect((proxy.host.as_str(), proxy.port)).await?;

    let host_header = format!("{}:{}", target_host, target_port);
    let mut req = format!("CONNECT {h} HTTP/1.1\r\nHost: {h}\r\n", h = host_header,);
    if let Some(ref auth) = proxy.basic_auth {
        req.push_str("Proxy-Authorization: Basic ");
        req.push_str(auth);
        req.push_str("\r\n");
    }
    req.push_str("Proxy-Connection: keep-alive\r\n\r\n");

    stream.write_all(req.as_bytes()).await?;
    stream.flush().await?;

    let mut reader = BufReader::new(stream);
    let mut status_line = String::new();
    let n = reader.read_line(&mut status_line).await?;
    if n == 0 {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "HTTP proxy closed connection before sending CONNECT response",
        ));
    }

    let status_ok = status_line
        .split_whitespace()
        .nth(1)
        .map(|s| s == "200")
        .unwrap_or(false);

    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line).await?;
        if n == 0 || line == "\r\n" || line == "\n" {
            break;
        }
    }

    if !status_ok {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "HTTP proxy CONNECT to {} rejected: {}",
                host_header,
                status_line.trim_end()
            ),
        ));
    }

    // A conforming proxy stays silent after the CONNECT response until the
    // client speaks. If our read buffer is non-empty, the proxy spoke
    // first — handing the raw socket to TLS would silently drop those
    // bytes and break the handshake.
    if !reader.buffer().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "HTTP proxy sent unexpected bytes after CONNECT response",
        ));
    }

    Ok(reader.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_proxy_wildcard_matches_all() {
        assert!(matches_no_proxy("example.com", "*"));
        assert!(matches_no_proxy("any.host", " * "));
    }

    #[test]
    fn no_proxy_exact_and_subdomain() {
        assert!(matches_no_proxy("example.com", "example.com"));
        assert!(matches_no_proxy("api.example.com", "example.com"));
        assert!(matches_no_proxy("api.example.com", ".example.com"));
        assert!(matches_no_proxy("example.com", ".example.com"));
        assert!(!matches_no_proxy("notexample.com", "example.com"));
    }

    #[test]
    fn no_proxy_ignores_port_suffix_and_empty_entries() {
        assert!(matches_no_proxy("example.com", "example.com:8080"));
        assert!(matches_no_proxy(
            "api.example.com",
            ",  , example.com , other.com"
        ));
    }

    #[test]
    fn no_proxy_is_case_insensitive() {
        assert!(matches_no_proxy("API.Example.COM", "example.com"));
    }

    #[test]
    fn no_proxy_no_match_returns_false() {
        assert!(!matches_no_proxy("slack.com", "example.com,internal.lan"));
        assert!(!matches_no_proxy("slack.com", ""));
    }

    #[test]
    fn parse_proxy_target_handles_common_shapes() {
        let p = parse_proxy_target("http://outbound.eps.apple.com:80").unwrap();
        assert_eq!(p.host, "outbound.eps.apple.com");
        assert_eq!(p.port, 80);
        assert!(p.basic_auth.is_none());

        let p = parse_proxy_target("https://proxy.internal").unwrap();
        assert_eq!(p.host, "proxy.internal");
        assert_eq!(p.port, 443);

        let p = parse_proxy_target("proxy.internal").unwrap();
        assert_eq!(p.host, "proxy.internal");
        assert_eq!(p.port, 80);

        let p = parse_proxy_target("proxy.internal:3128").unwrap();
        assert_eq!(p.host, "proxy.internal");
        assert_eq!(p.port, 3128);
    }

    #[test]
    fn parse_proxy_target_extracts_basic_auth() {
        let p = parse_proxy_target("http://alice:s3cret@proxy.lan:8080").unwrap();
        assert_eq!(p.host, "proxy.lan");
        assert_eq!(p.port, 8080);
        // base64("alice:s3cret") = YWxpY2U6czNjcmV0
        assert_eq!(p.basic_auth.as_deref(), Some("YWxpY2U6czNjcmV0"));
    }

    #[test]
    fn parse_proxy_target_handles_ipv6() {
        let p = parse_proxy_target("http://[::1]:3128").unwrap();
        assert_eq!(p.host, "::1");
        assert_eq!(p.port, 3128);

        let p = parse_proxy_target("http://[::1]").unwrap();
        assert_eq!(p.host, "::1");
        assert_eq!(p.port, 80);
    }

    #[test]
    fn parse_proxy_target_rejects_empty() {
        assert!(parse_proxy_target("").is_none());
        assert!(parse_proxy_target("   ").is_none());
        assert!(parse_proxy_target("http://").is_none());
    }

    #[test]
    fn parse_proxy_target_strips_path() {
        let p = parse_proxy_target("http://proxy.lan:8080/whatever?x=1").unwrap();
        assert_eq!(p.host, "proxy.lan");
        assert_eq!(p.port, 8080);
    }

    /// Spin up a one-shot TCP listener acting as an HTTP proxy.
    /// Reads the CONNECT request, asserts on it via `validate`, then
    /// either replies `200 Connection Established` or the supplied
    /// `respond` string. Echoes any further client bytes back so the test
    /// can confirm the returned `TcpStream` carries the tunneled session.
    async fn fake_proxy<F>(
        respond: &'static str,
        validate: F,
    ) -> (std::net::SocketAddr, tokio::task::JoinHandle<Vec<u8>>)
    where
        F: FnOnce(&str) + Send + 'static,
    {
        use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = String::new();
            {
                let mut reader = BufReader::new(&mut socket);
                loop {
                    let mut line = String::new();
                    let n = reader.read_line(&mut line).await.unwrap();
                    request.push_str(&line);
                    if n == 0 || line == "\r\n" || line == "\n" {
                        break;
                    }
                }
            }
            validate(&request);
            socket.write_all(respond.as_bytes()).await.unwrap();
            socket.flush().await.unwrap();

            let mut tunneled = Vec::new();
            socket.read_to_end(&mut tunneled).await.unwrap();
            tunneled
        });
        (addr, handle)
    }

    #[tokio::test]
    async fn http_connect_tunnel_sends_well_formed_request_and_unwraps_stream() {
        use tokio::io::AsyncWriteExt;

        let (addr, handle) = fake_proxy(
            "HTTP/1.1 200 Connection Established\r\nProxy-Agent: test\r\n\r\n",
            |req| {
                assert!(
                    req.starts_with("CONNECT slack.com:443 HTTP/1.1\r\n"),
                    "got: {req:?}"
                );
                assert!(req.contains("Host: slack.com:443\r\n"));
                assert!(!req.contains("Proxy-Authorization"));
            },
        )
        .await;

        let proxy =
            ProxyTarget { host: addr.ip().to_string(), port: addr.port(), basic_auth: None };
        let mut stream = http_connect_tunnel(&proxy, "slack.com", 443).await.unwrap();
        stream.write_all(b"hello-tls").await.unwrap();
        stream.shutdown().await.unwrap();

        let tunneled = handle.await.unwrap();
        assert_eq!(tunneled, b"hello-tls");
    }

    #[tokio::test]
    async fn http_connect_tunnel_forwards_basic_auth_header() {
        use tokio::io::AsyncWriteExt;

        let (addr, handle) = fake_proxy("HTTP/1.1 200 OK\r\n\r\n", |req| {
            assert!(
                req.contains("Proxy-Authorization: Basic YWxpY2U6czNjcmV0\r\n"),
                "got: {req:?}"
            );
        })
        .await;

        let proxy = ProxyTarget {
            host: addr.ip().to_string(),
            port: addr.port(),
            basic_auth: Some("YWxpY2U6czNjcmV0".to_string()),
        };
        let mut stream = http_connect_tunnel(&proxy, "slack.com", 443).await.unwrap();
        // Close the client end so the fake proxy's `read_to_end` returns.
        stream.shutdown().await.unwrap();
        let _ = handle.await.unwrap();
    }

    #[tokio::test]
    async fn http_connect_tunnel_surfaces_non_2xx_status() {
        let (addr, handle) = fake_proxy(
            "HTTP/1.1 407 Proxy Authentication Required\r\nProxy-Authenticate: Basic\r\n\r\n",
            |_| {},
        )
        .await;
        let proxy =
            ProxyTarget { host: addr.ip().to_string(), port: addr.port(), basic_auth: None };
        let err = http_connect_tunnel(&proxy, "slack.com", 443)
            .await
            .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("407"), "expected 407 in error, got: {msg}");
        let _ = handle.await.unwrap();
    }
}
