/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use axum::body::Body;
use axum::response::Response;
use http::StatusCode;

lazy_static::lazy_static! {
    pub static ref PUBLIC_APP_DOMAIN: Option<String> = std::env::var("PUBLIC_APP_DOMAIN").ok();
}

/// Middleware to restrict public app domain to whitelisted routes
pub async fn public_app_domain_filter(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    if let Some(public_domain) = PUBLIC_APP_DOMAIN.as_ref() {
        if let Some(host) = req.headers().get(http::header::HOST) {
            if let Ok(host_str) = host.to_str() {
                // Extract hostname without port
                let hostname = host_str.split(':').next().unwrap_or(host_str);

                if hostname == public_domain {
                    let path = req.uri().path();

                    // Check if route is whitelisted
                    let is_whitelisted = is_public_route_whitelisted(path);

                    if !is_whitelisted {
                        tracing::warn!(
                            "Rejected request to {} on public app domain {}",
                            path,
                            public_domain
                        );
                        return Response::builder()
                            .status(StatusCode::FORBIDDEN)
                            .body(Body::from(
                                "Access forbidden: route not allowed on public app domain",
                            ))
                            .unwrap();
                    }
                }
            }
        }
    }
    next.run(req).await
}

fn is_public_route_whitelisted(path: &str) -> bool {
    // Whitelisted route patterns for public app domain
    let whitelist = [
        // Public app routes
        "/api/ee_license",
        "/api/w/*/users/whoami",
        "/api/w/*/apps_u/*",
        "/api/w/*/resources/list",
        "/api/w/*/jobs_u/getupdate_sse/*",
        "/api/auth/login",
        "/api/w/*/folders/listnames",
        "/api/w/*/resources/exists/*",
        "/api/w/*/resources/type/get/*",
        "/api/w/*/resources/type/listnames",
        "/api/oauth/login/*",
        "/api/oauth/connect/*",
        "/oauth/callback/*",
        "/user/login_callback/*",
        "/api/workspaces/users",
        "/api/users/whoami",
        "/api/apps_u/*",
        "/api/oauth/list_connects",
        "/api/oauth/list_logins",
        "/public/*",
        "/a/*",
        "/api/oauth/get_connect/*",
        "/Inter-Variable.woff2",
    ];

    for pattern in &whitelist {
        if matches_pattern(path, pattern) {
            return true;
        }
    }

    false
}

fn matches_pattern(path: &str, pattern: &str) -> bool {
    // Handle exact match
    if path == pattern {
        return true;
    }

    // Handle wildcard patterns
    if !pattern.contains('*') {
        return false;
    }

    let parts: Vec<&str> = pattern.split('*').collect();

    // Start from the beginning - path must start with first part
    if !path.starts_with(parts[0]) {
        return false;
    }

    let mut remaining = &path[parts[0].len()..];

    // Check each part in between wildcards
    for i in 1..parts.len() - 1 {
        let part = parts[i];
        if let Some(pos) = remaining.find(part) {
            remaining = &remaining[pos + part.len()..];
        } else {
            return false;
        }
    }

    // Check the last part
    let last = parts[parts.len() - 1];
    if last.is_empty() {
        // Wildcard at the end, already matched
        return true;
    } else {
        // Must end with the last part
        return remaining.ends_with(last);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_pattern_exact() {
        assert!(matches_pattern("/api/version", "/api/version"));
        assert!(!matches_pattern("/api/versions", "/api/version"));
    }

    #[test]
    fn test_matches_pattern_wildcard_middle() {
        assert!(matches_pattern(
            "/api/w/my-workspace/apps_u/my-app",
            "/api/w/*/apps_u/*"
        ));
        assert!(matches_pattern(
            "/api/w/workspace123/jobs_u/job456",
            "/api/w/*/jobs_u/*"
        ));
        assert!(!matches_pattern(
            "/api/w/workspace/apps/my-app",
            "/api/w/*/apps_u/*"
        ));
    }

    #[test]
    fn test_matches_pattern_wildcard_end() {
        assert!(matches_pattern("/api/r/some/path", "/api/r/*"));
        assert!(matches_pattern("/api/auth/login", "/api/auth/*"));
        assert!(matches_pattern("/public/asset.css", "/public/*"));
        assert!(!matches_pattern("/api/admin/users", "/api/r/*"));
    }

    #[test]
    fn test_is_public_route_whitelisted() {
        // Whitelisted routes based on current whitelist
        assert!(is_public_route_whitelisted("/api/ee_license"));
        assert!(is_public_route_whitelisted(
            "/api/w/my-workspace/users/whoami"
        ));
        assert!(is_public_route_whitelisted(
            "/api/w/my-workspace/apps_u/my-app"
        ));
        assert!(is_public_route_whitelisted(
            "/api/w/workspace/resources/list"
        ));

        // Non-whitelisted routes
        assert!(!is_public_route_whitelisted("/api/version"));
        assert!(!is_public_route_whitelisted("/api/uptodate"));
        assert!(!is_public_route_whitelisted("/api/w/workspace/scripts"));
        assert!(!is_public_route_whitelisted("/api/w/workspace/flows"));
        assert!(!is_public_route_whitelisted(
            "/api/w/workspace/apps/private"
        ));
        assert!(!is_public_route_whitelisted("/api/admin/settings"));
        assert!(!is_public_route_whitelisted("/api/users"));
    }

    #[test]
    fn test_matches_pattern_complex_multiple_wildcards() {
        // Test pattern with two wildcards like "/api/w/*/foo/*"
        assert!(matches_pattern(
            "/api/w/workspace1/s3_proxy/file.txt",
            "/api/w/*/s3_proxy/*"
        ));
        assert!(matches_pattern(
            "/api/w/my-workspace/s3_proxy/path/to/file.jpg",
            "/api/w/*/s3_proxy/*"
        ));
        assert!(matches_pattern(
            "/api/w/test/apps_u/myapp",
            "/api/w/*/apps_u/*"
        ));

        // Should not match if middle part is different
        assert!(!matches_pattern(
            "/api/w/workspace1/other/file.txt",
            "/api/w/*/s3_proxy/*"
        ));
        assert!(!matches_pattern(
            "/api/w/workspace/apps/myapp",
            "/api/w/*/apps_u/*"
        ));

        // Edge cases
        assert!(matches_pattern(
            "/api/w/ws/s3_proxy/a",
            "/api/w/*/s3_proxy/*"
        ));

        // Path must have content after the last wildcard segment
        assert!(matches_pattern(
            "/api/w/ws/s3_proxy/",
            "/api/w/*/s3_proxy/*"
        ));
    }
}
