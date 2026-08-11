/*
 * This file provides a proxy endpoint for npm package requests
 * to support private registries in the frontend ATA (Automatic Type Acquisition)
 */

use axum::{
    body::Body,
    extract::{Path, Query},
    http::header,
    response::{IntoResponse, Response},
    routing::get,
    Extension, Json, Router,
};
use quick_cache::{sync::Cache, Weighter};
use serde::{Deserialize, Serialize};
use sqlx::types::JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
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

#[derive(Serialize)]
struct NpmProxyConfig {
    registry_configured: bool,
}

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/config", get(get_config))
        // Use wildcards for package names to support scoped packages like @scope/package
        .route("/metadata/{*package}", get(get_package_metadata))
        .route("/resolve/{*package}", get(resolve_package_version))
        .route("/filetree/{*package_version}", get(get_package_filetree))
        .route("/file/{*package_version_filepath}", get(get_package_file))
        .route("/tarball/{*package_version}", get(get_package_tarball))
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

/// npm's abbreviated packument: same `dist-tags` and `versions` keys, same
/// `versions[v].dist`, without the per-version prose that makes a full document run to
/// tens of megabytes. Registries that do not implement it answer with the full document.
const ABBREVIATED_PACKUMENT: &str = "application/vnd.npm.install-v1+json";

/// An install resolves and then downloads every package, so without a cache each
/// dependency of the app being edited costs several packument round trips.
const PACKAGE_JSON_CACHE_TTL: Duration = Duration::from_secs(60);
/// Bounded by bytes rather than documents: packument size varies by orders of magnitude
/// between packages, so a count-based bound puts no ceiling on resident memory.
const PACKAGE_JSON_CACHE_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Clone)]
struct CachedPackageJson {
    document: Arc<JsonValue>,
    /// Encoded length of the registry response, standing in for the parsed tree's size.
    bytes: u64,
    fetched_at: Instant,
}

/// Type acquisition asks for a package's file tree and then for each `.d.ts` in it, all
/// out of the same archive, so without this one package costs a download and an inflate
/// per file rather than per package.
const TARBALL_CACHE_TTL: Duration = Duration::from_secs(300);
const TARBALL_CACHE_BYTES: u64 = 128 * 1024 * 1024;

/// A tarball is inflated entirely into memory, and its transfer size bounds neither how
/// far it expands nor how many entries it holds, so extraction is capped as it goes: the
/// cache's own budget only governs what is kept, which is too late.
const MAX_EXTRACTED_BYTES: u64 = 64 * 1024 * 1024;
const MAX_EXTRACTED_ENTRIES: usize = 20_000;
/// Per-entry map and vector overhead, so an archive of many empty files cannot sit in the
/// cache at near-zero weight.
const CACHE_ENTRY_OVERHEAD: u64 = 64;

/// A package tarball, inflated once. Paths have the `package/` prefix stripped.
struct ExtractedTarball {
    /// In archive order, `/`-prefixed, for the file tree listing.
    names: Vec<String>,
    contents: HashMap<String, Vec<u8>>,
}

impl ExtractedTarball {
    fn retained_bytes(&self) -> u64 {
        let names: u64 = self.names.iter().map(|n| n.len() as u64).sum();
        let contents: u64 = self
            .contents
            .iter()
            .map(|(path, content)| (path.len() + content.len()) as u64 + CACHE_ENTRY_OVERHEAD)
            .sum();
        names + contents
    }
}

#[derive(Clone)]
struct CachedTarball {
    extracted: Arc<ExtractedTarball>,
    bytes: u64,
    fetched_at: Instant,
}

#[derive(Clone)]
struct CacheWeighter;

impl Weighter<(String, String), CachedPackageJson> for CacheWeighter {
    fn weight(&self, _key: &(String, String), cached: &CachedPackageJson) -> u64 {
        cached.bytes.max(1)
    }
}

impl Weighter<(String, String, String), CachedTarball> for CacheWeighter {
    fn weight(&self, _key: &(String, String, String), cached: &CachedTarball) -> u64 {
        cached.bytes.max(1)
    }
}

/// Single-sharded on purpose. quick_cache divides the weight budget across shards and
/// declines an item heavier than one shard's share of it, so a sharded cache silently
/// refuses exactly the packages that cost the most to fetch and inflate again — the very
/// regression these caches exist to prevent. They are consulted a handful of times per
/// editor session and hold only map operations under the lock, so the concurrency a
/// single shard gives up is not worth that.
fn byte_bounded_cache<Key, Val>(items: usize, bytes: u64) -> Cache<Key, Val, CacheWeighter>
where
    Key: Eq + std::hash::Hash,
    Val: Clone,
    CacheWeighter: Weighter<Key, Val>,
{
    let options = quick_cache::OptionsBuilder::new()
        .shards(1)
        .estimated_items_capacity(items)
        .weight_capacity(bytes)
        .build()
        .expect("every cache option is set");
    Cache::with_options(
        options,
        CacheWeighter,
        Default::default(),
        Default::default(),
    )
}

lazy_static::lazy_static! {
    static ref PACKAGE_JSON_CACHE: Cache<(String, String), CachedPackageJson, CacheWeighter> =
        byte_bounded_cache(500, PACKAGE_JSON_CACHE_BYTES);
    static ref TARBALL_CACHE: Cache<(String, String, String), CachedTarball, CacheWeighter> =
        byte_bounded_cache(200, TARBALL_CACHE_BYTES);
}

/// Fetch a package's registry document, together with the registry it came from so
/// callers can validate tarball URLs against it.
async fn fetch_package_json(
    db: &sqlx::Pool<sqlx::Postgres>,
    package: &str,
) -> Result<(Arc<JsonValue>, String, Option<String>)> {
    let (registry_url, auth_token) = get_npm_registry(db)
        .await?
        .ok_or_else(|| Error::BadRequest("No private npm registry configured".to_string()))?;

    let cache_key = (registry_url.clone(), package.to_string());
    if let Some(cached) = PACKAGE_JSON_CACHE.get(&cache_key) {
        if cached.fetched_at.elapsed() < PACKAGE_JSON_CACHE_TTL {
            return Ok((cached.document, registry_url, auth_token));
        }
    }

    let package_url = format_registry_url(&registry_url, package, None, None);

    tracing::info!("Fetching package metadata from: {}", package_url);

    let response = build_registry_request(&package_url, &auth_token, &registry_url)?
        .header(header::ACCEPT, ABBREVIATED_PACKUMENT)
        .send()
        .await
        .map_err(|e| Error::InternalErr(format!("Failed to fetch package metadata: {}", e)))?;

    if !response.status().is_success() {
        return Err(Error::NotFound(format!(
            "Package {} not found in private registry",
            package
        )));
    }

    let body = response
        .bytes()
        .await
        .map_err(|e| Error::InternalErr(format!("Failed to read package metadata: {}", e)))?;
    let package_json: Arc<JsonValue> = Arc::new(
        serde_json::from_slice(&body)
            .map_err(|e| Error::InternalErr(format!("Failed to parse package metadata: {}", e)))?,
    );

    PACKAGE_JSON_CACHE.insert(
        cache_key,
        CachedPackageJson {
            document: package_json.clone(),
            bytes: body.len() as u64,
            fetched_at: Instant::now(),
        },
    );

    Ok((package_json, registry_url, auth_token))
}

/// Open the tarball of a resolved package version on the private registry
async fn tarball_response(
    package_json: &JsonValue,
    package: &str,
    version: &str,
    registry_url: &str,
    auth_token: &Option<String>,
) -> Result<reqwest::Response> {
    let tarball_url = package_json
        .get("versions")
        .and_then(|v| v.get(version))
        .and_then(|v| v.get("dist"))
        .and_then(|d| d.get("tarball"))
        .and_then(|t| t.as_str())
        .ok_or_else(|| Error::NotFound(format!("Tarball not found for {}@{}", package, version)))?;

    let response = build_registry_request(tarball_url, auth_token, registry_url)?
        .send()
        .await
        .map_err(|e| Error::InternalErr(format!("Failed to download tarball: {}", e)))?;

    if !response.status().is_success() {
        return Err(Error::NotFound(format!(
            "Failed to download tarball for {}@{}",
            package, version
        )));
    }

    Ok(response)
}

/// Download the tarball of a resolved package version, for the handlers that unpack it here
async fn fetch_tarball(
    package_json: &JsonValue,
    package: &str,
    version: &str,
    registry_url: &str,
    auth_token: &Option<String>,
) -> Result<axum::body::Bytes> {
    tarball_response(package_json, package, version, registry_url, auth_token)
        .await?
        .bytes()
        .await
        .map_err(|e| Error::InternalErr(format!("Failed to read tarball: {}", e)))
}

/// Report whether a private registry is configured, so clients that otherwise hit the
/// public npm CDNs (the raw app editor's in-browser installer) know to route through here.
async fn get_config(
    _authed: ApiAuthed,
    Path(_w_id): Path<String>,
    Extension(db): Extension<sqlx::Pool<sqlx::Postgres>>,
) -> JsonResult<NpmProxyConfig> {
    Ok(Json(NpmProxyConfig {
        registry_configured: get_npm_registry(&db).await?.is_some(),
    }))
}

/// Get package metadata (versions and tags) from the private registry
async fn get_package_metadata(
    _authed: ApiAuthed,
    Path((_w_id, package_path)): Path<(String, StripPath)>,
    Extension(db): Extension<sqlx::Pool<sqlx::Postgres>>,
) -> JsonResult<PackageVersions> {
    let package = parse_package_name(package_path.to_path());
    let (package_json, _, _) = fetch_package_json(&db, &package).await?;

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

/// Resolve a package tag/version/range reference to a specific version
async fn resolve_package_version(
    _authed: ApiAuthed,
    Path((_w_id, package_path)): Path<(String, StripPath)>,
    Query(query): Query<ProxyQuery>,
    Extension(db): Extension<sqlx::Pool<sqlx::Postgres>>,
) -> JsonResult<PackageVersion> {
    let package = parse_package_name(package_path.to_path());
    let reference = query.tag.unwrap_or_else(|| "latest".to_string());
    let (package_json, _, _) = fetch_package_json(&db, &package).await?;

    Ok(Json(PackageVersion {
        version: resolve_version_spec(&package_json, &reference),
    }))
}

/// Resolve an npm version spec against a registry document: a dist-tag, an exact
/// version, or a semver range as it appears in a package.json dependency.
fn resolve_version_spec(package_json: &JsonValue, spec: &str) -> Option<String> {
    let spec = spec.trim();

    if let Some(version) = package_json
        .get("dist-tags")
        .and_then(|v| v.get(spec))
        .and_then(|v| v.as_str())
    {
        return Some(version.to_string());
    }

    let versions = package_json.get("versions").and_then(|v| v.as_object())?;
    // A spec that pins one version, `v` prefix and all, is exact to npm. It has to be
    // recognised here rather than left to the range path, where `1.2.3` reads as `^1.2.3`.
    let pinned = strip_version_prefix(spec);
    if versions.contains_key(pinned) {
        return Some(pinned.to_string());
    }
    if pinned.parse::<semver::Version>().is_ok() {
        return None;
    }

    let requirements = parse_npm_range(spec)?;
    versions
        .keys()
        .filter_map(|raw| semver::Version::parse(raw).ok().map(|parsed| (parsed, raw)))
        .filter(|(parsed, _)| requirements.iter().any(|req| req.matches(parsed)))
        .max_by(|(a, _), (b, _)| a.cmp(b))
        .map(|(_, raw)| raw.clone())
}

/// npm ranges OR comparator sets together with `||`; the semver crate takes one set per
/// `VersionReq`. Sets it cannot parse are dropped, so an unsupported alternative narrows
/// the match rather than failing the whole range.
fn parse_npm_range(spec: &str) -> Option<Vec<semver::VersionReq>> {
    let requirements = spec
        .split("||")
        .filter_map(|set| semver::VersionReq::parse(&normalize_comparator_set(set)).ok())
        .collect::<Vec<_>>();

    (!requirements.is_empty()).then_some(requirements)
}

/// Rewrite one npm comparator set into the crate's syntax. npm additionally accepts an
/// operator detached from its version (`>= 1.0.0`), a `v` prefix, hyphen ranges, and
/// AND-ed comparators separated by spaces rather than commas; and it reads a bare partial
/// as an x-range (`1.2` is `1.2.x`) where the crate reads it as a caret.
fn normalize_comparator_set(set: &str) -> String {
    let comparators = split_comparators(set);
    match comparators.as_slice() {
        [] => "*".to_string(),
        [low, hyphen, high] if hyphen == "-" => format!(">={},<={}", low, high),
        _ => comparators
            .iter()
            .map(|comparator| widen_bare_partial(comparator))
            .collect::<Vec<_>>()
            .join(","),
    }
}

/// Split on whitespace, keeping an operator attached to the version it applies to.
fn split_comparators(set: &str) -> Vec<String> {
    let mut comparators: Vec<String> = Vec::new();
    for token in set.split_whitespace() {
        match comparators.last_mut() {
            Some(pending) if is_operator(pending) && token != "-" => {
                pending.push_str(strip_version_prefix(token))
            }
            _ => comparators.push(if is_operator(token) {
                token.to_string()
            } else {
                strip_operator_and_version_prefix(token)
            }),
        }
    }
    comparators
}

fn is_operator(token: &str) -> bool {
    !token.is_empty() && token.chars().all(|c| "><=^~".contains(c))
}

/// npm accepts a `v` in front of a version (`v1.2.3`, `^v1.2.3`); the semver crate does not.
fn strip_version_prefix(version: &str) -> &str {
    version
        .strip_prefix('v')
        .filter(|rest| rest.starts_with(|c: char| c.is_ascii_digit()))
        .unwrap_or(version)
}

fn strip_operator_and_version_prefix(comparator: &str) -> String {
    let operator_len = comparator
        .find(|c: char| !"><=^~".contains(c))
        .unwrap_or(comparator.len());
    let (operator, version) = comparator.split_at(operator_len);
    format!("{}{}", operator, strip_version_prefix(version))
}

/// `1.2` bounds npm to the `1.2.x` line; the crate would read it as `^1.2`.
fn widen_bare_partial(comparator: &str) -> String {
    let components = comparator.split('.').collect::<Vec<_>>();
    if components.len() < 3
        && components
            .iter()
            .all(|c| !c.is_empty() && c.chars().all(|c| c.is_ascii_digit()))
    {
        format!("{}.*", comparator)
    } else {
        comparator.to_string()
    }
}

/// Get the file tree for a specific package version
async fn get_package_filetree(
    _authed: ApiAuthed,
    Path((_w_id, package_version_path)): Path<(String, StripPath)>,
    Extension(db): Extension<sqlx::Pool<sqlx::Postgres>>,
) -> JsonResult<PackageFiletree> {
    let (package, version) = parse_package_and_version(package_version_path.to_path())?;
    let (package_json, registry_url, auth_token) = fetch_package_json(&db, &package).await?;
    let extracted = extracted_tarball(
        &package_json,
        &package,
        &version,
        &registry_url,
        &auth_token,
    )
    .await?;

    // The entry point comes from the packaged manifest rather than the registry document,
    // which carries no `main` in its abbreviated form.
    let main = extracted
        .contents
        .get("package.json")
        .and_then(|manifest| serde_json::from_slice::<JsonValue>(manifest).ok())
        .and_then(|manifest| {
            manifest
                .get("main")
                .and_then(|m| m.as_str())
                .map(String::from)
        })
        .unwrap_or_else(|| "index.js".to_string());

    let files = extracted
        .names
        .iter()
        .map(|name| FileEntry { name: name.clone() })
        .collect();

    Ok(Json(PackageFiletree { default: main, files }))
}

/// Get a specific file from a package version
async fn get_package_file(
    _authed: ApiAuthed,
    Path((_w_id, full_path)): Path<(String, StripPath)>,
    Extension(db): Extension<sqlx::Pool<sqlx::Postgres>>,
) -> Result<String> {
    let (package, version, filepath) = parse_package_version_and_file(full_path.to_path())?;
    let (package_json, registry_url, auth_token) = fetch_package_json(&db, &package).await?;
    let extracted = extracted_tarball(
        &package_json,
        &package,
        &version,
        &registry_url,
        &auth_token,
    )
    .await?;

    let content = extracted
        .contents
        .get(filepath.trim_start_matches('/'))
        .ok_or_else(|| Error::NotFound(format!("File {} not found in tarball", filepath)))?;

    String::from_utf8(content.clone())
        .map_err(|_| Error::BadRequest(format!("File {} is not valid UTF-8", filepath)))
}

/// Stream a package version's tarball, so in-browser installers can unpack it
/// themselves instead of reaching the public registry directly
async fn get_package_tarball(
    _authed: ApiAuthed,
    Path((_w_id, package_version_path)): Path<(String, StripPath)>,
    Extension(db): Extension<sqlx::Pool<sqlx::Postgres>>,
) -> Result<Response> {
    let (package, version) = parse_package_and_version(package_version_path.to_path())?;
    let (package_json, registry_url, auth_token) = fetch_package_json(&db, &package).await?;
    let response = tarball_response(
        &package_json,
        &package,
        &version,
        &registry_url,
        &auth_token,
    )
    .await?;

    Ok((
        [
            (header::CONTENT_TYPE, "application/octet-stream"),
            // A published version's tarball never changes, and the response is
            // registry-credentialed, so it is the viewer's cache to keep.
            (
                header::CACHE_CONTROL,
                "private, max-age=31536000, immutable",
            ),
        ],
        Body::from_stream(response.bytes_stream()),
    )
        .into_response())
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

/// Inflate a tarball in one pass over the archive
fn extract_tarball(tarball_bytes: &[u8]) -> Result<ExtractedTarball> {
    extract_tarball_within(tarball_bytes, MAX_EXTRACTED_BYTES, MAX_EXTRACTED_ENTRIES)
}

fn extract_tarball_within(
    tarball_bytes: &[u8],
    max_bytes: u64,
    max_entries: usize,
) -> Result<ExtractedTarball> {
    use flate2::read::GzDecoder;
    use std::io::Read;
    use tar::Archive;

    let gz = GzDecoder::new(tarball_bytes);
    let mut archive = Archive::new(gz);

    let mut names = Vec::new();
    let mut contents = HashMap::new();
    let mut extracted_bytes = 0u64;

    for entry in archive
        .entries()
        .map_err(|e| Error::InternalErr(format!("Failed to read tarball entries: {}", e)))?
    {
        let mut entry = entry
            .map_err(|e| Error::InternalErr(format!("Failed to read tarball entry: {}", e)))?;
        let path = entry
            .path()
            .map_err(|e| Error::InternalErr(format!("Failed to read entry path: {}", e)))?;

        // Remove the package/ prefix that npm tarballs have
        let path_str = path.to_string_lossy().to_string();
        if let Some(stripped) = path_str.strip_prefix("package/") {
            if names.len() >= max_entries {
                return Err(Error::BadRequest(format!(
                    "Package archive holds more than {} files",
                    max_entries
                )));
            }
            let stripped = stripped.to_string();
            names.push(format!("/{}", stripped));

            // Read through a cap rather than sizing from the header, which an archive is
            // free to lie about, and one byte past it so the overrun is detectable.
            let headroom = max_bytes - extracted_bytes;
            let mut content = Vec::new();
            if (&mut entry)
                .take(headroom + 1)
                .read_to_end(&mut content)
                .is_ok()
            {
                if content.len() as u64 > headroom {
                    return Err(Error::BadRequest(format!(
                        "Package archive inflates past {} bytes",
                        max_bytes
                    )));
                }
                extracted_bytes += content.len() as u64;
                contents.insert(stripped, content);
            }
        }
    }

    Ok(ExtractedTarball { names, contents })
}

/// Download and inflate a package version's tarball, reusing a recent extraction
async fn extracted_tarball(
    package_json: &JsonValue,
    package: &str,
    version: &str,
    registry_url: &str,
    auth_token: &Option<String>,
) -> Result<Arc<ExtractedTarball>> {
    let cache_key = (
        registry_url.to_string(),
        package.to_string(),
        version.to_string(),
    );
    if let Some(cached) = TARBALL_CACHE.get(&cache_key) {
        if cached.fetched_at.elapsed() < TARBALL_CACHE_TTL {
            return Ok(cached.extracted);
        }
    }

    let tarball_bytes =
        fetch_tarball(package_json, package, version, registry_url, auth_token).await?;
    let extracted = Arc::new(extract_tarball(&tarball_bytes)?);

    TARBALL_CACHE.insert(
        cache_key,
        CachedTarball {
            extracted: extracted.clone(),
            bytes: extracted.retained_bytes(),
            fetched_at: Instant::now(),
        },
    );

    Ok(extracted)
}

#[cfg(test)]
mod tests {
    use super::{
        byte_bounded_cache, extract_tarball_within, resolve_version_spec, CachedTarball,
        ExtractedTarball, MAX_EXTRACTED_BYTES, TARBALL_CACHE_BYTES,
    };
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Instant;

    fn packument() -> serde_json::Value {
        serde_json::json!({
            "dist-tags": { "latest": "19.2.0", "next": "20.0.0-rc.1" },
            "versions": {
                "18.3.1": {}, "19.0.0": {}, "19.1.5": {}, "19.2.0": {}, "20.0.0-rc.1": {}
            }
        })
    }

    #[test]
    fn resolves_tags_exact_versions_and_ranges() {
        let p = packument();
        let resolve = |spec: &str| resolve_version_spec(&p, spec);

        assert_eq!(resolve("latest").as_deref(), Some("19.2.0"));
        assert_eq!(resolve("19.0.0").as_deref(), Some("19.0.0"));
        // Ranges are what package.json dependencies actually hold, and each of these
        // forms means something different to npm than to the semver crate's parser
        assert_eq!(resolve("^19.0.0").as_deref(), Some("19.2.0"));
        assert_eq!(resolve("~19.0.0").as_deref(), Some("19.0.0"));
        assert_eq!(resolve(">=18 <19").as_deref(), Some("18.3.1"));
        assert_eq!(resolve("^18.0.0 || ^19.0.0").as_deref(), Some("19.2.0"));
        assert_eq!(resolve("*").as_deref(), Some("19.2.0"));
        assert_eq!(resolve("19.x").as_deref(), Some("19.2.0"));
        assert_eq!(resolve("18.3.1 - 19.1.5").as_deref(), Some("19.1.5"));
        assert_eq!(resolve(">= 18 < 19").as_deref(), Some("18.3.1"));
        assert_eq!(resolve("^v19.0.0").as_deref(), Some("19.2.0"));
        // A `v` prefix does not turn a pinned version into a range
        assert_eq!(resolve("v19.0.0").as_deref(), Some("19.0.0"));
        assert_eq!(resolve("v19.0.1"), None);
        // npm reads a bare partial as an x-range, the crate as a caret
        assert_eq!(resolve("19.1").as_deref(), Some("19.1.5"));
        assert_eq!(resolve("19").as_deref(), Some("19.2.0"));
        // A pinned version the registry does not carry must not widen to a caret range
        assert_eq!(resolve("19.0.1"), None);
        assert_eq!(resolve("^21.0.0"), None);
    }

    fn tarball(entries: &[(&str, usize)]) -> Vec<u8> {
        let encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        let mut builder = tar::Builder::new(encoder);
        for (path, size) in entries {
            let content = vec![b'x'; *size];
            let mut header = tar::Header::new_gnu();
            header.set_size(content.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder
                .append_data(&mut header, path, content.as_slice())
                .unwrap();
        }
        builder.into_inner().unwrap().finish().unwrap()
    }

    #[test]
    fn extraction_is_capped_before_the_archive_is_held_in_memory() {
        let archive = tarball(&[("package/a.js", 32), ("package/b.js", 32)]);

        let extracted = extract_tarball_within(&archive, 1024, 16).unwrap();
        assert_eq!(extracted.names, vec!["/a.js", "/b.js"]);
        assert!(extracted.retained_bytes() > 64);

        // A tarball's transfer size bounds neither of these
        assert!(extract_tarball_within(&archive, 40, 16).is_err());
        assert!(extract_tarball_within(&archive, 1024, 1).is_err());
    }

    /// A cache that splits its budget across shards refuses anything heavier than one
    /// shard's share, silently declining the packages worth caching most.
    #[test]
    fn the_largest_extraction_allowed_still_fits_the_cache() {
        let cache = byte_bounded_cache(200, TARBALL_CACHE_BYTES);
        let key = ("registry".to_string(), "big".to_string(), "1.0.0".to_string());

        cache.insert(
            key.clone(),
            CachedTarball {
                extracted: Arc::new(ExtractedTarball { names: vec![], contents: HashMap::new() }),
                bytes: MAX_EXTRACTED_BYTES,
                fetched_at: Instant::now(),
            },
        );

        assert!(cache.get(&key).is_some());
    }
}
