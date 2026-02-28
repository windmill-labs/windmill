/*
 * This file provides a proxy endpoint for npm package requests
 * to support private registries in the frontend ATA (Automatic Type Acquisition)
 */

use axum::{
    extract::{Path, Query},
    routing::get,
    Extension, Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::types::JsonValue;
use std::collections::HashMap;
use tower_http::cors::{Any, CorsLayer};
use windmill_common::{
    error::{Error, JsonResult, Result},
    global_settings::{
        load_value_from_global_settings, NPMRC_SETTING, NPM_CONFIG_REGISTRY_SETTING,
    },
    utils::{parse_npmrc_registry, StripPath},
};

use windmill_api_auth::ApiAuthed;
use windmill_common::utils::HTTP_CLIENT_PERMISSIVE as HTTP_CLIENT;

#[derive(Deserialize)]
struct ProxyQuery {
    tag: Option<String>,
}

/// Parse a scoped package path like "@scope/name" or "name" from a wildcard path
fn parse_package_name(path: &str) -> String {
    // Remove leading slash if present
    path.trim_start_matches('/').to_string()
}

/// Parse package and version from a path like "@scope/name/1.0.0" or "name/1.0.0"
fn parse_package_and_version(path: &str) -> Result<(String, String)> {
    let path = path.trim_start_matches('/');

    if path.starts_with('@') {
        // Scoped package: @scope/name/version
        let parts: Vec<&str> = path.splitn(3, '/').collect();
        if parts.len() < 3 {
            return Err(Error::BadRequest(
                "Invalid scoped package path, expected @scope/name/version".to_string(),
            ));
        }
        let package = format!("{}/{}", parts[0], parts[1]);
        let version = parts[2].to_string();
        Ok((package, version))
    } else {
        // Regular package: name/version
        let parts: Vec<&str> = path.splitn(2, '/').collect();
        if parts.len() < 2 {
            return Err(Error::BadRequest(
                "Invalid package path, expected name/version".to_string(),
            ));
        }
        Ok((parts[0].to_string(), parts[1].to_string()))
    }
}

/// Parse package, version, and filepath from a path like "@scope/name/1.0.0/index.d.ts"
fn parse_package_version_and_file(path: &str) -> Result<(String, String, String)> {
    let path = path.trim_start_matches('/');

    if path.starts_with('@') {
        // Scoped package: @scope/name/version/filepath
        let parts: Vec<&str> = path.splitn(4, '/').collect();
        if parts.len() < 4 {
            return Err(Error::BadRequest(
                "Invalid scoped package file path, expected @scope/name/version/filepath"
                    .to_string(),
            ));
        }
        let package = format!("{}/{}", parts[0], parts[1]);
        let version = parts[2].to_string();
        let filepath = parts[3].to_string();
        Ok((package, version, filepath))
    } else {
        // Regular package: name/version/filepath
        let parts: Vec<&str> = path.splitn(3, '/').collect();
        if parts.len() < 3 {
            return Err(Error::BadRequest(
                "Invalid package file path, expected name/version/filepath".to_string(),
            ));
        }
        Ok((
            parts[0].to_string(),
            parts[1].to_string(),
            parts[2].to_string(),
        ))
    }
}

#[derive(Serialize)]
struct PackageVersions {
    tags: HashMap<String, String>,
    versions: Vec<String>,
}

#[derive(Serialize)]
struct PackageVersion {
    version: Option<String>,
}

#[derive(Serialize)]
struct PackageFiletree {
    default: String,
    files: Vec<FileEntry>,
}

#[derive(Serialize)]
struct FileEntry {
    name: String,
}

pub fn workspaced_service() -> Router {
    Router::new()
        // Use wildcards for package names to support scoped packages like @scope/package
        .route("/metadata/*package", get(get_package_metadata))
        .route("/resolve/*package", get(resolve_package_version))
        .route("/filetree/*package_version", get(get_package_filetree))
        .route("/file/*package_version_filepath", get(get_package_file))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
}

fn build_registry_request(
    url: &str,
    auth_token: &Option<String>,
    registry_base_url: &str,
) -> Result<reqwest::RequestBuilder> {
    let parsed_url =
        url::Url::parse(url).map_err(|e| Error::BadRequest(format!("Invalid URL: {}", e)))?;
    let parsed_base = url::Url::parse(registry_base_url)
        .map_err(|e| Error::BadRequest(format!("Invalid registry URL: {}", e)))?;

    if parsed_url.host_str() != parsed_base.host_str() {
        return Err(Error::BadRequest(format!(
            "Tarball URL host '{}' does not match registry host '{}'",
            parsed_url.host_str().unwrap_or("unknown"),
            parsed_base.host_str().unwrap_or("unknown"),
        )));
    }

    let mut req = HTTP_CLIENT.get(url);
    if let Some(token) = auth_token {
        req = req.bearer_auth(token);
    }
    Ok(req)
}

/// Get package metadata (versions and tags) from the private registry
async fn get_package_metadata(
    _authed: ApiAuthed,
    Path((_w_id, package_path)): Path<(String, StripPath)>,
    Extension(db): Extension<sqlx::Pool<sqlx::Postgres>>,
) -> JsonResult<PackageVersions> {
    let package = parse_package_name(package_path.to_path());
    let (registry_url, auth_token) = get_npm_registry(&db)
        .await?
        .ok_or_else(|| Error::BadRequest("No private npm registry configured".to_string()))?;
    let package_url = format_registry_url(&registry_url, &package, None, None);

    tracing::info!("Fetching package metadata from: {}", package_url);

    let response = build_registry_request(&package_url, &auth_token, &registry_url)?
        .send()
        .await
        .map_err(|e| Error::InternalErr(format!("Failed to fetch package metadata: {}", e)))?;

    if !response.status().is_success() {
        return Err(Error::NotFound(format!(
            "Package {} not found in private registry",
            package
        )));
    }

    let package_json: JsonValue = response
        .json()
        .await
        .map_err(|e| Error::InternalErr(format!("Failed to parse package metadata: {}", e)))?;

    let mut versions = Vec::new();
    let mut tags = HashMap::new();

    if let Some(versions_obj) = package_json.get("versions").and_then(|v| v.as_object()) {
        versions = versions_obj.keys().cloned().collect();
    }

    if let Some(tags_obj) = package_json.get("dist-tags").and_then(|v| v.as_object()) {
        for (tag, version) in tags_obj {
            if let Some(version_str) = version.as_str() {
                tags.insert(tag.clone(), version_str.to_string());
            }
        }
    }

    Ok(Json(PackageVersions { tags, versions }))
}

/// Resolve a package tag/version reference to a specific version
async fn resolve_package_version(
    _authed: ApiAuthed,
    Path((_w_id, package_path)): Path<(String, StripPath)>,
    Query(query): Query<ProxyQuery>,
    Extension(db): Extension<sqlx::Pool<sqlx::Postgres>>,
) -> JsonResult<PackageVersion> {
    let package = parse_package_name(package_path.to_path());
    let (registry_url, auth_token) = get_npm_registry(&db)
        .await?
        .ok_or_else(|| Error::BadRequest("No private npm registry configured".to_string()))?;
    let reference = query.tag.unwrap_or_else(|| "latest".to_string());
    let package_url = format_registry_url(&registry_url, &package, None, None);

    tracing::info!("Resolving package version from: {}", package_url);

    let response = build_registry_request(&package_url, &auth_token, &registry_url)?
        .send()
        .await
        .map_err(|e| Error::InternalErr(format!("Failed to fetch package metadata: {}", e)))?;

    if !response.status().is_success() {
        return Err(Error::NotFound(format!(
            "Package {} not found in private registry",
            package
        )));
    }

    let package_json: JsonValue = response
        .json()
        .await
        .map_err(|e| Error::InternalErr(format!("Failed to parse package metadata: {}", e)))?;

    // Try to resolve the reference as a tag first
    let version = if let Some(tags) = package_json.get("dist-tags").and_then(|v| v.as_object()) {
        if let Some(version) = tags.get(&reference).and_then(|v| v.as_str()) {
            Some(version.to_string())
        } else {
            // If not a tag, check if it's a valid version
            if let Some(versions) = package_json.get("versions").and_then(|v| v.as_object()) {
                if versions.contains_key(&reference) {
                    Some(reference)
                } else {
                    None
                }
            } else {
                None
            }
        }
    } else {
        None
    };

    Ok(Json(PackageVersion { version }))
}

/// Get the file tree for a specific package version
async fn get_package_filetree(
    _authed: ApiAuthed,
    Path((_w_id, package_version_path)): Path<(String, StripPath)>,
    Extension(db): Extension<sqlx::Pool<sqlx::Postgres>>,
) -> JsonResult<PackageFiletree> {
    let (package, version) = parse_package_and_version(package_version_path.to_path())?;
    let (registry_url, auth_token) = get_npm_registry(&db)
        .await?
        .ok_or_else(|| Error::BadRequest("No private npm registry configured".to_string()))?;
    let package_url = format_registry_url(&registry_url, &package, None, None);

    tracing::info!("Fetching package filetree from: {}", package_url);

    let response = build_registry_request(&package_url, &auth_token, &registry_url)?
        .send()
        .await
        .map_err(|e| Error::InternalErr(format!("Failed to fetch package metadata: {}", e)))?;

    if !response.status().is_success() {
        return Err(Error::NotFound(format!(
            "Package {} not found in private registry",
            package
        )));
    }

    let package_json: JsonValue = response
        .json()
        .await
        .map_err(|e| Error::InternalErr(format!("Failed to parse package metadata: {}", e)))?;

    let tarball_url = package_json
        .get("versions")
        .and_then(|v| v.get(&version))
        .and_then(|v| v.get("dist"))
        .and_then(|d| d.get("tarball"))
        .and_then(|t| t.as_str())
        .ok_or_else(|| Error::NotFound(format!("Tarball not found for {}@{}", package, version)))?;

    let tarball_response = build_registry_request(tarball_url, &auth_token, &registry_url)?
        .send()
        .await
        .map_err(|e| Error::InternalErr(format!("Failed to download tarball: {}", e)))?;

    if !tarball_response.status().is_success() {
        return Err(Error::NotFound(format!(
            "Failed to download tarball for {}@{}",
            package, version
        )));
    }

    let tarball_bytes = tarball_response
        .bytes()
        .await
        .map_err(|e| Error::InternalErr(format!("Failed to read tarball: {}", e)))?;

    // Extract file list from tarball
    let files = extract_tarball_files(&tarball_bytes)?;

    // Find the main entry point
    let main = package_json
        .get("versions")
        .and_then(|v| v.get(&version))
        .and_then(|v| v.get("main"))
        .and_then(|m| m.as_str())
        .unwrap_or("index.js")
        .to_string();

    Ok(Json(PackageFiletree { default: main, files }))
}

/// Get a specific file from a package version
async fn get_package_file(
    _authed: ApiAuthed,
    Path((_w_id, full_path)): Path<(String, StripPath)>,
    Extension(db): Extension<sqlx::Pool<sqlx::Postgres>>,
) -> Result<String> {
    let (package, version, filepath) = parse_package_version_and_file(full_path.to_path())?;
    let (registry_url, auth_token) = get_npm_registry(&db)
        .await?
        .ok_or_else(|| Error::BadRequest("No private npm registry configured".to_string()))?;
    let package_url = format_registry_url(&registry_url, &package, None, None);

    tracing::info!("Fetching package file from: {}", package_url);

    let response = build_registry_request(&package_url, &auth_token, &registry_url)?
        .send()
        .await
        .map_err(|e| Error::InternalErr(format!("Failed to fetch package metadata: {}", e)))?;

    if !response.status().is_success() {
        return Err(Error::NotFound(format!(
            "Package {} not found in private registry",
            package
        )));
    }

    let package_json: JsonValue = response
        .json()
        .await
        .map_err(|e| Error::InternalErr(format!("Failed to parse package metadata: {}", e)))?;

    let tarball_url = package_json
        .get("versions")
        .and_then(|v| v.get(&version))
        .and_then(|v| v.get("dist"))
        .and_then(|d| d.get("tarball"))
        .and_then(|t| t.as_str())
        .ok_or_else(|| Error::NotFound(format!("Tarball not found for {}@{}", package, version)))?;

    let tarball_response = build_registry_request(tarball_url, &auth_token, &registry_url)?
        .send()
        .await
        .map_err(|e| Error::InternalErr(format!("Failed to download tarball: {}", e)))?;

    if !tarball_response.status().is_success() {
        return Err(Error::NotFound(format!(
            "Failed to download tarball for {}@{}",
            package, version
        )));
    }

    let tarball_bytes = tarball_response
        .bytes()
        .await
        .map_err(|e| Error::InternalErr(format!("Failed to read tarball: {}", e)))?;

    // Extract the specific file from the tarball
    let file_content = extract_file_from_tarball(&tarball_bytes, &filepath)?;

    Ok(file_content)
}

/// Get the npm registry URL and optional auth token from global settings.
/// Checks the `npmrc` setting first, then falls back to `npm_config_registry`.
async fn get_npm_registry(
    db: &sqlx::Pool<sqlx::Postgres>,
) -> Result<Option<(String, Option<String>)>> {
    let npmrc = load_value_from_global_settings(db, NPMRC_SETTING)
        .await?
        .and_then(|v| v.as_str().map(|s| s.to_string()));

    if let Some(ref npmrc_content) = npmrc {
        if let Some(parsed) = parse_npmrc_registry(npmrc_content) {
            return Ok(Some(parsed));
        }
    }

    let registry = load_value_from_global_settings(db, NPM_CONFIG_REGISTRY_SETTING)
        .await?
        .and_then(|v| v.as_str().map(|s| s.to_string()));

    if let Some(ref s) = registry {
        let (url, token) = if s.contains(":_authToken=") {
            let parts: Vec<&str> = s.split(":_authToken=").collect();
            let url = parts[0].to_string();
            let token = parts.get(1).map(|t| t.to_string());
            (url, token)
        } else {
            (s.clone(), None)
        };
        return Ok(Some((url, token)));
    }

    Ok(None)
}

/// Format a registry URL for a package
fn format_registry_url(
    registry_base: &str,
    package: &str,
    version: Option<&str>,
    file: Option<&str>,
) -> String {
    let registry_base = registry_base.trim_end_matches('/');

    // Handle scoped packages (e.g., @types/node)
    let package_path = if package.starts_with('@') {
        // Scoped packages need to be URL encoded properly
        package.replace('/', "%2F")
    } else {
        package.to_string()
    };

    match (version, file) {
        (Some(v), Some(f)) => {
            format!(
                "{}/{}/{}/-/{}-{}/{}",
                registry_base, package_path, v, package, v, f
            )
        }
        (Some(v), None) => {
            format!("{}/{}/{}", registry_base, package_path, v)
        }
        _ => {
            format!("{}/{}", registry_base, package_path)
        }
    }
}

/// Extract file list from a tarball
fn extract_tarball_files(tarball_bytes: &[u8]) -> Result<Vec<FileEntry>> {
    use flate2::read::GzDecoder;
    use tar::Archive;

    let gz = GzDecoder::new(tarball_bytes);
    let mut archive = Archive::new(gz);

    let mut files = Vec::new();

    for entry in archive
        .entries()
        .map_err(|e| Error::InternalErr(format!("Failed to read tarball entries: {}", e)))?
    {
        let entry = entry
            .map_err(|e| Error::InternalErr(format!("Failed to read tarball entry: {}", e)))?;
        let path = entry
            .path()
            .map_err(|e| Error::InternalErr(format!("Failed to read entry path: {}", e)))?;

        // Remove the package/ prefix that npm tarballs have
        let path_str = path.to_string_lossy().to_string();
        if let Some(stripped) = path_str.strip_prefix("package/") {
            files.push(FileEntry { name: format!("/{}", stripped) });
        }
    }

    Ok(files)
}

/// Extract a specific file from a tarball
fn extract_file_from_tarball(tarball_bytes: &[u8], target_file: &str) -> Result<String> {
    use flate2::read::GzDecoder;
    use std::io::Read;
    use tar::Archive;

    let gz = GzDecoder::new(tarball_bytes);
    let mut archive = Archive::new(gz);

    // Normalize the target file path
    let target = target_file.trim_start_matches('/');

    for entry in archive
        .entries()
        .map_err(|e| Error::InternalErr(format!("Failed to read tarball entries: {}", e)))?
    {
        let mut entry = entry
            .map_err(|e| Error::InternalErr(format!("Failed to read tarball entry: {}", e)))?;
        let path = entry
            .path()
            .map_err(|e| Error::InternalErr(format!("Failed to read entry path: {}", e)))?;

        let path_str = path.to_string_lossy().to_string();

        // Check if this is the file we're looking for
        if let Some(stripped) = path_str.strip_prefix("package/") {
            if stripped == target {
                let mut content = String::new();
                entry.read_to_string(&mut content).map_err(|e| {
                    Error::InternalErr(format!("Failed to read file content: {}", e))
                })?;
                return Ok(content);
            }
        }
    }

    Err(Error::NotFound(format!(
        "File {} not found in tarball",
        target_file
    )))
}
