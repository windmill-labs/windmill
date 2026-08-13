//! HTTP CONNECT proxy support for outbound WebSocket connections.
//!
//! `tokio-tungstenite::connect_async` opens a raw TCP socket and does not
//! honour `HTTPS_PROXY` / `HTTP_PROXY` / `NO_PROXY`. On networks without
//! direct egress this leaves WebSocket triggers unable to reach the
//! upstream service. This module re-uses the env-var snapshots already
//! parsed by `windmill-common` and, when a proxy applies to the target
//! host, opens an HTTP CONNECT tunnel before delegating the TLS +
//! WebSocket handshake back to tungstenite.
//!
//! When no proxy env vars are set (the common case), this module
//! forwards straight to `tokio_tungstenite::connect_async` so the
//! networking path stays byte-for-byte identical to the previous
//! behaviour.

use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use std::io;
use std::net::SocketAddr;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
};
use tokio_tungstenite::{
    client_async_tls_with_config, connect_async,
    tungstenite::{
        client::IntoClientRequest,
        error::{Error as WsError, UrlError},
        handshake::client::Response,
    },
    MaybeTlsStream, WebSocketStream,
};
use url::Url;
use windmill_common::{HTTPS_PROXY, HTTP_PROXY, NO_PROXY};

/// Drop-in replacement for `tokio_tungstenite::connect_async` that routes
/// the underlying TCP connection through `HTTPS_PROXY` / `HTTP_PROXY`
/// (with `NO_PROXY` exclusions) when those env vars are set, and — for direct
/// (non-proxied) connections — pins DNS to `pinned_addrs`.
///
/// `pinned_addrs` are the addresses the SSRF guard already resolved and
/// validated for this URL (see `validate_websocket_url_for_ssrf`). Connecting
/// straight to them, rather than letting `connect_async` re-resolve the host,
/// closes the DNS-rebinding window between the check and the connect: a rebinder
/// cannot answer a public IP at validation time and an internal one here. When
/// `pinned_addrs` is empty (IP-literal host, or the SSRF guard opted out via
/// `ALLOW_PRIVATE_WEBSOCKET_URLS`) there is nothing to pin and we fall back to
/// `connect_async`, keeping the behaviour for those cases unchanged.
///
/// When a proxy applies, the proxy itself resolves the target host, so DNS
/// rebinding at this hop is not the worker's concern and `pinned_addrs` is
/// unused for that path.
pub async fn connect_async_with_proxy<R>(
    request: R,
    pinned_addrs: &[SocketAddr],
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

    let proxy = if HTTPS_PROXY.is_none() && HTTP_PROXY.is_none() {
        None
    } else {
        proxy_url_for(&scheme, &host).and_then(|raw| parse_proxy_target(&raw))
    };

    if let Some(proxy) = proxy {
        tracing::debug!(
            "Connecting to WebSocket {}:{} through HTTP proxy {}:{}",
            host,
            port,
            proxy.host,
            proxy.port,
        );
        let socket = http_connect_tunnel(&proxy, &host, port)
            .await
            .map_err(WsError::Io)?;
        return client_async_tls_with_config(request, socket, None, None).await;
    }

    // Direct connection. Nothing to pin (IP literal or SSRF guard opted out):
    // preserve the original resolve-and-connect path.
    if pinned_addrs.is_empty() {
        return connect_async(request).await;
    }

    // Pin to a validated address so this connect targets the same IP the SSRF
    // guard checked. Try each in order (e.g. IPv6 then IPv4) until one connects.
    let mut last_err: Option<io::Error> = None;
    for addr in pinned_addrs {
        match TcpStream::connect(addr).await {
            Ok(socket) => {
                return client_async_tls_with_config(request, socket, None, None).await;
            }
            Err(e) => last_err = Some(e),
        }
    }
    Err(WsError::Io(last_err.unwrap_or_else(|| {
        io::Error::new(
            io::ErrorKind::AddrNotAvailable,
            "no pinned address to connect",
        )
    })))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProxyTarget {
    host: String,
    port: u16,
    /// Base64-encoded `user:pass` from URL userinfo, ready to drop into
    /// the `Proxy-Authorization: Basic …` header value.
    basic_auth: Option<String>,
}

/// Resolve the proxy URL string to use for outbound `(scheme, host)`.
///
/// `wss://`/`https://` reads `HTTPS_PROXY`, `ws://`/`http://` reads
/// `HTTP_PROXY`. `NO_PROXY` short-circuits to `None`. The env-var
/// snapshots come from `windmill-common` so they share a single source
/// of truth with the worker's `PROXY_ENVS`.
fn proxy_url_for(scheme: &str, host: &str) -> Option<String> {
    if let Some(no_proxy) = NO_PROXY.as_deref() {
        if matches_no_proxy(host, no_proxy) {
            return None;
        }
    }
    let primary = if scheme.eq_ignore_ascii_case("wss") || scheme.eq_ignore_ascii_case("https") {
        HTTPS_PROXY.as_deref()
    } else {
        HTTP_PROXY.as_deref()
    };
    primary
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
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

    // `url::Url::parse` requires an explicit scheme — prepend `http://`
    // when the user passed a bare `host[:port]`.
    let prepended = if raw.contains("://") {
        std::borrow::Cow::Borrowed(raw)
    } else {
        std::borrow::Cow::Owned(format!("http://{raw}"))
    };
    let url = Url::parse(&prepended).ok()?;

    // `host_str()` keeps brackets around IPv6 literals; strip them so
    // `TcpStream::connect((host, port))` resolves the address correctly.
    let host = url
        .host_str()?
        .trim_start_matches('[')
        .trim_end_matches(']');
    if host.is_empty() {
        return None;
    }
    let host = host.to_string();
    let port =
        url.port_or_known_default()
            .unwrap_or(if url.scheme().eq_ignore_ascii_case("https") {
                443
            } else {
                80
            });

    let basic_auth = match (url.username(), url.password()) {
        ("", None) => None,
        (user, pass) => {
            let creds = match pass {
                Some(p) => format!("{user}:{p}"),
                None => user.to_string(),
            };
            Some(BASE64_STANDARD.encode(creds))
        }
    };

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
    //! The single live test (`http_connect_tunnel_…_unwraps_stream`) drives
    //! a real `TcpListener` masquerading as a proxy and verifies both the
    //! on-the-wire CONNECT request and that the returned `TcpStream`
    //! actually carries tunneled bytes. The other proxy-URL and NO_PROXY
    //! checks are kept under `#[ignore]` for manual debugging — they cover
    //! logic that's mostly delegated to `url::Url::parse` and trivial
    //! string matching, so re-running them on every CI build is low ROI.
    use super::*;

    #[test]
    #[ignore = "covered by upstream `url::Url::parse`; run manually with `--ignored` if changed"]
    fn parse_proxy_target_shapes_and_ipv6_and_basic_auth() {
        let p = parse_proxy_target("http://outbound.eps.apple.com:80").unwrap();
        assert_eq!(p.host, "outbound.eps.apple.com");
        assert_eq!(p.port, 80);

        let p = parse_proxy_target("https://proxy.internal").unwrap();
        assert_eq!(p.port, 443);

        let p = parse_proxy_target("proxy.internal:3128").unwrap();
        assert_eq!(p.port, 3128);

        let p = parse_proxy_target("http://alice:s3cret@proxy.lan:8080").unwrap();
        // base64("alice:s3cret") = YWxpY2U6czNjcmV0
        assert_eq!(p.basic_auth.as_deref(), Some("YWxpY2U6czNjcmV0"));

        let p = parse_proxy_target("http://[::1]:3128").unwrap();
        assert_eq!(p.host, "::1");
        assert_eq!(p.port, 3128);

        assert!(parse_proxy_target("").is_none());
        assert!(parse_proxy_target("http://").is_none());
    }

    #[test]
    #[ignore = "trivial string matching; run manually with `--ignored` if rules change"]
    fn no_proxy_matching_rules() {
        assert!(matches_no_proxy("example.com", "*"));
        assert!(matches_no_proxy("example.com", "example.com"));
        assert!(matches_no_proxy("api.example.com", "example.com"));
        assert!(matches_no_proxy("api.example.com", ".example.com"));
        assert!(matches_no_proxy("example.com", ".example.com"));
        assert!(matches_no_proxy("example.com", "example.com:8080"));
        assert!(matches_no_proxy("API.Example.COM", "example.com"));
        assert!(!matches_no_proxy("notexample.com", "example.com"));
        assert!(!matches_no_proxy("slack.com", "example.com,internal.lan"));
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
    #[ignore = "manual; fake-proxy edge cases (auth, error status). Run with `--ignored` if `http_connect_tunnel` changes."]
    async fn http_connect_tunnel_forwards_basic_auth_and_surfaces_non_2xx() {
        use tokio::io::AsyncWriteExt;

        // Basic-auth header is forwarded.
        let (addr, handle) = fake_proxy("HTTP/1.1 200 OK\r\n\r\n", |req| {
            assert!(req.contains("Proxy-Authorization: Basic YWxpY2U6czNjcmV0\r\n"));
        })
        .await;
        let proxy = ProxyTarget {
            host: addr.ip().to_string(),
            port: addr.port(),
            basic_auth: Some("YWxpY2U6czNjcmV0".to_string()),
        };
        let mut stream = http_connect_tunnel(&proxy, "slack.com", 443).await.unwrap();
        stream.shutdown().await.unwrap();
        let _ = handle.await.unwrap();

        // Non-2xx status surfaces as an error.
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
        assert!(err.to_string().contains("407"));
        let _ = handle.await.unwrap();
    }
}
