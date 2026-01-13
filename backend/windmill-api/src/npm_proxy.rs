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
    global_settings::{load_value_from_global_settings, NPM_CONFIG_REGISTRY_SETTING},
};

use crate::{db::ApiAuthed, HTTP_CLIENT};

#[derive(Deserialize)]
struct ProxyQuery {
    tag: Option<String>,
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
        .route("/metadata/:package", get(get_package_metadata))
        .route("/resolve/:package", get(resolve_package_version))
        .route("/filetree/:package/:version", get(get_package_filetree))
        .route("/file/:package/:version/*filepath", get(get_package_file))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
}

/// Get package metadata (versions and tags) from the private registry
async fn get_package_metadata(
    authed: ApiAuthed,
    Path(package): Path<String>,
    Extension(db): Extension<sqlx::Pool<sqlx::Postgres>>,
) -> JsonResult<PackageVersions> {
    let npm_registry = get_npm_registry(&db).await?;

    if npm_registry.is_none() {
        return Err(Error::BadRequest(
            "No private npm registry configured".to_string(),
        ));
    }

    let registry_url = npm_registry.unwrap();
    let package_url = format_registry_url(&registry_url, &package, None, None);

    tracing::info!("Fetching package metadata from: {}", package_url);

    let response = HTTP_CLIENT
        .get(&package_url)
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

    // Extract versions and dist-tags from the package metadata
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
    authed: ApiAuthed,
    Path(package): Path<String>,
    Query(query): Query<ProxyQuery>,
    Extension(db): Extension<sqlx::Pool<sqlx::Postgres>>,
) -> JsonResult<PackageVersion> {
    let npm_registry = get_npm_registry(&db).await?;

    if npm_registry.is_none() {
        return Err(Error::BadRequest(
            "No private npm registry configured".to_string(),
        ));
    }

    let registry_url = npm_registry.unwrap();
    let reference = query.tag.unwrap_or_else(|| "latest".to_string());
    let package_url = format_registry_url(&registry_url, &package, None, None);

    tracing::info!("Resolving package version from: {}", package_url);

    let response = HTTP_CLIENT
        .get(&package_url)
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
    authed: ApiAuthed,
    Path((package, version)): Path<(String, String)>,
    Extension(db): Extension<sqlx::Pool<sqlx::Postgres>>,
) -> JsonResult<PackageFiletree> {
    let npm_registry = get_npm_registry(&db).await?;

    if npm_registry.is_none() {
        return Err(Error::BadRequest(
            "No private npm registry configured".to_string(),
        ));
    }

    let registry_url = npm_registry.unwrap();
    let package_url = format_registry_url(&registry_url, &package, None, None);

    tracing::info!("Fetching package filetree from: {}", package_url);

    let response = HTTP_CLIENT
        .get(&package_url)
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

    // Get the tarball URL for this version
    let tarball_url = package_json
        .get("versions")
        .and_then(|v| v.get(&version))
        .and_then(|v| v.get("dist"))
        .and_then(|d| d.get("tarball"))
        .and_then(|t| t.as_str())
        .ok_or_else(|| Error::NotFound(format!("Tarball not found for {}@{}", package, version)))?;

    // Download and extract tarball to get file list
    let tarball_response = HTTP_CLIENT
        .get(tarball_url)
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

    Ok(Json(PackageFiletree {
        default: main,
        files,
    }))
}

/// Get a specific file from a package version
async fn get_package_file(
    authed: ApiAuthed,
    Path((package, version, filepath)): Path<(String, String, String)>,
    Extension(db): Extension<sqlx::Pool<sqlx::Postgres>>,
) -> Result<String> {
    let npm_registry = get_npm_registry(&db).await?;

    if npm_registry.is_none() {
        return Err(Error::BadRequest(
            "No private npm registry configured".to_string(),
        ));
    }

    let registry_url = npm_registry.unwrap();
    let package_url = format_registry_url(&registry_url, &package, None, None);

    tracing::info!("Fetching package file from: {}", package_url);

    let response = HTTP_CLIENT
        .get(&package_url)
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

    // Get the tarball URL for this version
    let tarball_url = package_json
        .get("versions")
        .and_then(|v| v.get(&version))
        .and_then(|v| v.get("dist"))
        .and_then(|d| d.get("tarball"))
        .and_then(|t| t.as_str())
        .ok_or_else(|| Error::NotFound(format!("Tarball not found for {}@{}", package, version)))?;

    // Download tarball
    let tarball_response = HTTP_CLIENT
        .get(tarball_url)
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

/// Get the npm registry URL from global settings
async fn get_npm_registry(db: &sqlx::Pool<sqlx::Postgres>) -> Result<Option<String>> {
    let registry = load_value_from_global_settings(db, NPM_CONFIG_REGISTRY_SETTING)
        .await?
        .and_then(|v| v.as_str().map(|s| s.to_string()));

    Ok(registry)
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
            format!("{}/{}/{}/-/{}-{}/{}", registry_base, package_path, v, package, v, f)
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
            files.push(FileEntry {
                name: format!("/{}", stripped),
            });
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
                entry
                    .read_to_string(&mut content)
                    .map_err(|e| Error::InternalErr(format!("Failed to read file content: {}", e)))?;
                return Ok(content);
            }
        }
    }

    Err(Error::NotFound(format!("File {} not found in tarball", target_file)))
}
