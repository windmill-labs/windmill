/*
 * Author: Windmill Labs, Inc
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use axum::{
    extract::OriginalUri,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
    body::Body,
};
use crate::{db::ApiAuthed, scopes::check_scopes_for_route};
use windmill_common::error::Error;

/// Middleware to check JWT token scopes against API routes
pub async fn check_scopes_middleware(
    original_uri: OriginalUri,
    authed: ApiAuthed,
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // Skip scope checking for certain conditions
    if should_skip_scope_check(&authed, &original_uri) {
        return Ok(next.run(request).await);
    }

    let path = original_uri.path();
    let method = request.method().as_str();

    // Check if scopes allow access to this route
    match check_scopes_for_route(authed.scopes.as_deref(), path, method) {
        Ok(()) => Ok(next.run(request).await),
        Err(Error::BadRequest(msg)) => {
            tracing::warn!(
                "Scope check failed for user {} on {} {}: {}", 
                authed.email, 
                method, 
                path, 
                msg
            );
            Err(StatusCode::FORBIDDEN)
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Determine if scope checking should be skipped for this request
fn should_skip_scope_check(authed: &ApiAuthed, original_uri: &OriginalUri) -> bool {
    // Skip for superadmins and admins
    if authed.is_admin {
        return true;
    }

    // Skip if no scopes are defined (backward compatibility)
    if authed.scopes.is_none() {
        return true;
    }

    let path = original_uri.path();

    // Skip for certain paths that should always be accessible
    let always_accessible_paths = [
        "/api/version",
        "/api/uptodate", 
        "/api/ee_license",
        "/api/openapi.yaml",
        "/api/openapi.json",
        "/api/w/", // Let individual endpoints handle their own scope checks if needed
    ];

    for accessible_path in &always_accessible_paths {
        if path.starts_with(accessible_path) {
            return true;
        }
    }

    // Skip for unauthed endpoints
    if path.contains("_u/") || path.contains("/auth/") {
        return true;
    }

    false
}

/// Enhanced scope checking function with better error handling
pub fn check_enhanced_scopes(
    authed: &ApiAuthed,
    required_scope: &str,
    resource_path: Option<&str>,
) -> Result<(), Error> {
    // Skip for superadmins and admins
    if authed.is_admin {
        return Ok(());
    }

    // Skip if no scopes are defined (backward compatibility)
    let scopes = match &authed.scopes {
        Some(s) if !s.is_empty() => s,
        _ => return Ok(()),
    };

    // Check for wildcard access
    if scopes.contains(&"*".to_string()) {
        return Ok(());
    }

    // Check for exact scope match
    if scopes.contains(&required_scope.to_string()) {
        return Ok(());
    }

    // Check for resource-specific scope if provided
    if let Some(resource) = resource_path {
        let resource_scope = format!("{}:{}", required_scope, resource);
        if scopes.contains(&resource_scope) {
            return Ok(());
        }

        // Check for wildcard resource scope
        let wildcard_scope = format!("{}:*", required_scope);
        if scopes.contains(&wildcard_scope) {
            return Ok(());
        }

        // Check for folder-based scope (f/folder/*)
        if resource.starts_with("f/") {
            let parts: Vec<&str> = resource.split('/').collect();
            if parts.len() >= 2 {
                let folder_scope = format!("{}:f/{}/*", required_scope, parts[1]);
                if scopes.contains(&folder_scope) {
                    return Ok(());
                }
            }
        }
    }

    // Check for hierarchical action permissions
    // e.g., "admin" scope includes "read", "write", "delete", "execute"
    let scope_parts: Vec<&str> = required_scope.split(':').collect();
    if scope_parts.len() >= 2 {
        let domain = scope_parts[0];
        let action = scope_parts[1];

        // Check for admin access to this domain
        let admin_scope = format!("{}:admin", domain);
        if scopes.contains(&admin_scope) {
            return Ok(());
        }

        // Check if write access includes read
        if action == "read" {
            let write_scope = format!("{}:write", domain);
            if scopes.contains(&write_scope) {
                return Ok(());
            }
        }
    }

    Err(Error::BadRequest(format!(
        "Access denied. Missing required scope: {} (available scopes: {:?})", 
        required_scope,
        scopes
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::ApiAuthed;

    fn create_test_authed(scopes: Option<Vec<String>>) -> ApiAuthed {
        ApiAuthed {
            email: "test@example.com".to_string(),
            username: "testuser".to_string(),
            is_admin: false,
            is_operator: false,
            groups: vec![],
            folders: vec![],
            scopes,
            username_override: None,
        }
    }

    #[test]
    fn test_wildcard_scope() {
        let authed = create_test_authed(Some(vec!["*".to_string()]));
        assert!(check_enhanced_scopes(&authed, "jobs:read", None).is_ok());
        assert!(check_enhanced_scopes(&authed, "scripts:execute", Some("test_script")).is_ok());
    }

    #[test]
    fn test_exact_scope_match() {
        let authed = create_test_authed(Some(vec!["jobs:read".to_string()]));
        assert!(check_enhanced_scopes(&authed, "jobs:read", None).is_ok());
        assert!(check_enhanced_scopes(&authed, "jobs:write", None).is_err());
    }

    #[test]
    fn test_hierarchical_permissions() {
        let authed = create_test_authed(Some(vec!["jobs:admin".to_string()]));
        assert!(check_enhanced_scopes(&authed, "jobs:read", None).is_ok());
        assert!(check_enhanced_scopes(&authed, "jobs:write", None).is_ok());
        assert!(check_enhanced_scopes(&authed, "jobs:delete", None).is_ok());
        assert!(check_enhanced_scopes(&authed, "jobs:execute", None).is_ok());
    }

    #[test]
    fn test_write_includes_read() {
        let authed = create_test_authed(Some(vec!["scripts:write".to_string()]));
        assert!(check_enhanced_scopes(&authed, "scripts:read", None).is_ok());
        assert!(check_enhanced_scopes(&authed, "scripts:write", None).is_ok());
        assert!(check_enhanced_scopes(&authed, "scripts:delete", None).is_err());
    }

    #[test]
    fn test_resource_specific_scope() {
        let authed = create_test_authed(Some(vec!["scripts:execute:f/folder/*".to_string()]));
        assert!(check_enhanced_scopes(&authed, "scripts:execute", Some("f/folder/test_script")).is_ok());
        assert!(check_enhanced_scopes(&authed, "scripts:execute", Some("f/other/test_script")).is_err());
    }

    #[test]
    fn test_admin_bypass() {
        let mut authed = create_test_authed(Some(vec![]));
        authed.is_admin = true;
        assert!(check_enhanced_scopes(&authed, "jobs:read", None).is_ok());
        assert!(check_enhanced_scopes(&authed, "jobs:delete", None).is_ok());
    }

    #[test]
    fn test_no_scopes_backward_compatibility() {
        let authed = create_test_authed(None);
        assert!(check_enhanced_scopes(&authed, "jobs:read", None).is_ok());
    }
}