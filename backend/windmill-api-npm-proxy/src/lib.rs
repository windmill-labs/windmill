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
mod store;

use quick_cache::{sync::Cache, Weighter};
use store::PendingPackage;
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
/// between packages, so a count-based bound puts no ceiling on resident memory. The only
/// cache still held in memory: packuments are small, hot, and mutable as versions publish,
/// so a store that costs a round trip of its own would buy nothing.
const PACKAGE_JSON_CACHE_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Clone)]
struct CachedPackageJson {
    document: Arc<JsonValue>,
    /// Encoded length of the registry response, standing in for the parsed tree's size.
    bytes: u64,
    fetched_at: Instant,
}

/// Bounds on one archive's inflation. Both sit far above any real package (`next` unpacks
/// to ~184 MB across ~8.5k files), so they bound abuse rather than express a size policy:
/// a package is never refused for being large.
const MAX_INFLATED_BYTES: u64 = 1024 * 1024 * 1024;
const MAX_ENTRIES: usize = 100_000;
/// Ceiling on one entry kept. Past it the entry is read back on demand, which costs a walk
/// of the archive per read, so it sits above what real packages hold rather than tightly.
const RETAINED_ENTRY_BYTES: u64 = 4 * 1024 * 1024;
/// The manifest's own ceiling, above the entry one because it is the only file whose
/// absence changes an answer rather than the speed of one. It is also the one entry read
/// whole into the heap in a single `read_to_end`, from registry-supplied content, so it
/// stays close to what a real manifest is: kilobytes, with orders of magnitude of room.
const MANIFEST_BYTES: u64 = 8 * 1024 * 1024;
/// Ceiling on the entries one package keeps. Declarations past it are left to a read on
/// demand rather than refused, so this bounds the cache without bounding what is servable.
/// `aws-sdk` is the heaviest package known here at ~51 MB across 442 declarations.
const RETAINED_BYTES: u64 = 96 * 1024 * 1024;
/// Ceiling on the file list, counted apart from the entries so that filling one cannot trip
/// the other. Unlike the entries this one refuses when exceeded: a file tree missing entries
/// is a wrong answer, not a slower one. `next` lists ~8.5k paths for ~1 MB. This is the one
/// bound still paid in memory, since the list is built before it is written.
const RETAINED_PATH_BYTES: u64 = 32 * 1024 * 1024;
/// Charged per listed path on top of its bytes, for the `String` header and its allocation.
const NAME_OVERHEAD: u64 = 64;
/// What one file costs on disk however small it is, for the write accounting that decides
/// when to sweep early.
const DISK_BLOCK_BYTES: u64 = 4096;

/// npm roots its own tarballs at `package/`, but publishers vary — DefinitelyTyped roots
/// at the type name — so the leading component is dropped whatever it is. Matching
/// `package/` alone silently yields an empty file tree for those packages.
fn strip_archive_root(path: &str) -> Option<&str> {
    path.split_once('/')
        .map(|(_, rest)| rest)
        .filter(|rest| !rest.is_empty())
}

/// What a type request can ask for: `ata/index.ts` reads each dependency's manifest and
/// then every `.d.ts` the file tree lists. Retaining just those keeps a large package's
/// bundles, maps and licences out of memory — `next` lists 8.5k files, of which the
/// manifest is the 3748th, so a rule by size alone would spend the budget before reaching
/// what is actually read. Anything else stays available through a read on demand.
fn worth_retaining(path: &str) -> bool {
    path == MANIFEST || path.ends_with(".d.ts")
}

const MANIFEST: &str = "package.json";

/// What one read of an archive will hold on to. Parameterised so the bounds can be
/// exercised without building an archive the size of the production ones.
#[derive(Clone, Copy)]
struct Retention {
    max_entries: usize,
    entry_bytes: u64,
    manifest_bytes: u64,
    total_bytes: u64,
    path_bytes: u64,
}

impl Retention {
    const PRODUCTION: Self = Self {
        max_entries: MAX_ENTRIES,
        entry_bytes: RETAINED_ENTRY_BYTES,
        manifest_bytes: MANIFEST_BYTES,
        total_bytes: RETAINED_BYTES,
        path_bytes: RETAINED_PATH_BYTES,
    };
}

#[derive(Clone)]
struct CacheWeighter;

impl Weighter<(String, String), CachedPackageJson> for CacheWeighter {
    fn weight(&self, _key: &(String, String), cached: &CachedPackageJson) -> u64 {
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
    let document = fetch_package_json_from(&registry_url, &auth_token, package).await?;
    Ok((document, registry_url, auth_token))
}

/// The same fetch against a registry the caller has already resolved.
///
/// Anything that both stores under a registry-scoped key and reads the packument has to
/// take one snapshot of the setting and use it throughout: resolving twice lets the setting
/// change in between, and two registries on one host pass the tarball's host check, so the
/// second registry's files can be written under the first one's key. Cached content is
/// immutable, so that survives switching back.
async fn fetch_package_json_from(
    registry_url: &str,
    auth_token: &Option<String>,
    package: &str,
) -> Result<Arc<JsonValue>> {
    let registry_url = registry_url.to_string();
    let cache_key = (registry_url.clone(), package.to_string());
    if let Some(cached) = PACKAGE_JSON_CACHE.get(&cache_key) {
        if cached.fetched_at.elapsed() < PACKAGE_JSON_CACHE_TTL {
            return Ok(cached.document);
        }
    }

    let package_url = format_registry_url(&registry_url, package, None, None);

    tracing::info!("Fetching package metadata from: {}", package_url);

    let response = build_registry_request(&package_url, auth_token, &registry_url)?
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

    Ok(package_json)
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

    // The sweep can evict a package between the lookup and this read, so a missing manifest
    // is a miss to repopulate rather than an error. Bounded: the second attempt has just
    // written the directory it is reading.
    let mut manifest = None;
    for _ in 0..2 {
        manifest = match cached_package(&db, &package, &version).await? {
            PackageFiles::Disk(dir) => store::read_manifest(&dir).await,
            PackageFiles::Memory(_, manifest) => Some(manifest),
        };
        if manifest.is_some() {
            break;
        }
    }
    let manifest = manifest.ok_or_else(|| {
        Error::InternalErr(format!("Cached {}@{} has no manifest", package, version))
    })?;

    Ok(Json(PackageFiletree {
        default: manifest.main,
        files: manifest
            .names
            .into_iter()
            .map(|name| FileEntry { name })
            .collect(),
    }))
}

/// Get a specific file from a package version
async fn get_package_file(
    _authed: ApiAuthed,
    Path((_w_id, full_path)): Path<(String, StripPath)>,
    Extension(db): Extension<sqlx::Pool<sqlx::Postgres>>,
) -> Result<String> {
    let (package, version, filepath) = parse_package_version_and_file(full_path.to_path())?;
    let files = cached_package(&db, &package, &version).await?;

    let target = filepath.trim_start_matches('/').to_string();
    let kept = match &files {
        PackageFiles::Disk(dir) => store::read_file(dir, &target).await,
        PackageFiles::Memory(kept, _) => kept.get(&target).cloned(),
    };
    let content = match kept {
        Some(content) => content,
        // Not a file a type request asks for, so it was never kept: walk the archive for it
        None => {
            let (package_json, registry_url, auth_token) =
                fetch_package_json(&db, &package).await?;
            let archive =
                fetch_tarball(&package_json, &package, &version, &registry_url, &auth_token)
                    .await?;
            let path = target.clone();
            blocking(move || read_one_entry(&archive, &path))
                .await?
                .ok_or_else(|| {
                    Error::NotFound(format!("File {} not found in tarball", filepath))
                })?
        }
    };

    String::from_utf8(content)
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

/// Caps total inflation across a whole walk, entries skipped past included: a tar entry
/// can only be advanced over by decompressing it, so bounding the matched entry alone
/// leaves the work an archive can demand unbounded.
struct BoundedRead<R> {
    inner: R,
    remaining: u64,
}

impl<R: std::io::Read> std::io::Read for BoundedRead<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.remaining == 0 {
            return Err(std::io::Error::other(format!(
                "archive inflates past {} bytes",
                MAX_INFLATED_BYTES
            )));
        }
        let cap = buf.len().min(self.remaining as usize);
        let read = self.inner.read(&mut buf[..cap])?;
        self.remaining -= read as u64;
        Ok(read)
    }
}

/// Read a package archive once, writing the entries worth keeping straight into `pending`
/// and returning the manifest that describes them. Entries stream to disk one at a time, so
/// the peak here is one entry rather than the whole retained set. Blocking work: gunzip plus
/// a full walk, so callers hand it to a blocking thread.
fn extract_to(
    archive: &[u8],
    keep: &mut dyn FnMut(&str, &[u8]) -> Result<()>,
    retention: Retention,
) -> Result<store::Manifest> {
    use flate2::read::GzDecoder;
    use std::io::Read;
    use tar::Archive;

    let gz = BoundedRead { inner: GzDecoder::new(archive), remaining: MAX_INFLATED_BYTES };
    let mut tar = Archive::new(gz);

    let mut names = Vec::new();
    let mut main = None;
    // Counted apart: one overflow refuses, the other stops keeping entries, so sharing a
    // counter would let a package full of declarations be refused by the path check.
    let mut path_bytes = 0u64;
    let mut retained = 0u64;

    for entry in tar
        .entries()
        .map_err(|e| Error::InternalErr(format!("Failed to read tarball entries: {}", e)))?
    {
        let mut entry = entry
            .map_err(|e| Error::InternalErr(format!("Failed to read tarball entry: {}", e)))?;
        let path = entry
            .path()
            .map_err(|e| Error::InternalErr(format!("Failed to read entry path: {}", e)))?
            .to_string_lossy()
            .to_string();

        let Some(stripped) = strip_archive_root(&path).map(str::to_string) else {
            continue;
        };
        // Refused before either sink reaches it: an archive carrying a traversal is
        // malicious, so it must not degrade the way an unwritable cache does.
        if !store::is_safe_relative(&stripped) {
            return Err(Error::BadRequest(format!(
                "Package archive holds an unsafe path: {stripped}"
            )));
        }
        if names.len() >= retention.max_entries {
            return Err(Error::BadRequest(format!(
                "Package archive holds more than {} files",
                retention.max_entries
            )));
        }
        // Paths are held whatever the entry is, so they are charged like any other bytes:
        // an archive of long names would otherwise grow `names` without limit while
        // reporting almost no weight. This one has to refuse rather than skip: a file tree
        // missing entries is a wrong answer, not a slower one.
        path_bytes += stripped.len() as u64 + NAME_OVERHEAD;
        if path_bytes > retention.path_bytes {
            return Err(Error::BadRequest(format!(
                "Package archive holds more than {} bytes of file paths",
                retention.path_bytes
            )));
        }
        names.push(format!("/{}", stripped));

        if !worth_retaining(&stripped) {
            continue;
        }
        // The manifest has its own ceiling: skipping an oversized one for the entry cap
        // would default the entry point, which is a wrong answer rather than a slow one.
        let is_manifest = stripped == MANIFEST;
        let cap = if is_manifest { retention.manifest_bytes } else { retention.entry_bytes };
        // Read through the cap rather than sizing from the header, which an archive is free
        // to lie about, and one byte past it so an overrun is detectable.
        let mut content = Vec::new();
        (&mut entry)
            .take(cap + 1)
            .read_to_end(&mut content)
            .map_err(|e| Error::InternalErr(format!("Failed to read {}: {}", stripped, e)))?;
        if content.len() as u64 > cap {
            continue;
        }
        // The manifest is exempt from the budget: it is one small file, and defaulting the
        // entry point because a package's declarations came first is a wrong answer rather
        // than a slower one. `next` orders its manifest 3748th.
        if is_manifest {
            main = serde_json::from_slice::<JsonValue>(&content)
                .ok()
                .and_then(|m| m.get("main").and_then(|m| m.as_str()).map(String::from));
        } else {
            // Past the budget the entry simply is not kept: it stays reachable through a
            // read on demand, so stopping bounds the cache as well as refusing would while
            // leaving the package servable.
            let cost = (stripped.len() + content.len()) as u64;
            if retained + cost > retention.total_bytes {
                continue;
            }
            retained += cost;
        }
        keep(&stripped, &content)?;
    }

    Ok(store::Manifest { names, main: main.unwrap_or_else(|| "index.js".to_string()) })
}

/// Read one entry out of an archive, for the files too large to have been retained.
/// Blocking work, same as the full read.
fn read_one_entry(archive: &[u8], target: &str) -> Result<Option<Vec<u8>>> {
    use flate2::read::GzDecoder;
    use std::io::Read;
    use tar::Archive;

    let gz = BoundedRead { inner: GzDecoder::new(archive), remaining: MAX_INFLATED_BYTES };
    let mut tar = Archive::new(gz);

    for entry in tar
        .entries()
        .map_err(|e| Error::InternalErr(format!("Failed to read tarball entries: {}", e)))?
    {
        let mut entry = entry
            .map_err(|e| Error::InternalErr(format!("Failed to read tarball entry: {}", e)))?;
        let path = entry
            .path()
            .map_err(|e| Error::InternalErr(format!("Failed to read entry path: {}", e)))?
            .to_string_lossy()
            .to_string();

        if strip_archive_root(&path) == Some(target) {
            let mut content = Vec::new();
            entry
                .read_to_end(&mut content)
                .map_err(|e| Error::InternalErr(format!("Failed to read {}: {}", target, e)))?;
            return Ok(Some(content));
        }
    }

    Ok(None)
}

/// Where a package version's kept files are being served from. Disk is the intended
/// answer; `Memory` is what a cache that cannot be written degrades to, so an ENOSPC or a
/// read-only cache directory costs a repeated extraction rather than the endpoint.
enum PackageFiles {
    Disk(std::path::PathBuf),
    Memory(HashMap<String, Vec<u8>>, store::Manifest),
}

/// A package version's kept files, populating the cache if needed.
///
/// Local disk first, the instance object store on a local miss (which then fills disk), the
/// registry on a miss in both. A published version never changes, so a directory that is
/// already there is never revalidated.
async fn cached_package(
    db: &sqlx::Pool<sqlx::Postgres>,
    package: &str,
    version: &str,
) -> Result<PackageFiles> {
    // Resolving the registry is a settings read; the packument behind it is a round trip,
    // and a version already on disk needs neither, so it is fetched only on a miss.
    let (registry_url, auth_token) = get_npm_registry(db)
        .await?
        .ok_or_else(|| Error::BadRequest("No private npm registry configured".to_string()))?;

    let dir = store::package_dir(&registry_url, package, version);
    if store::has_manifest(&dir).await {
        store::touch(&dir);
        return Ok(PackageFiles::Disk(dir));
    }

    let key = store::object_key(&registry_url, package, version);
    match store::pull_from_object_store(&dir, &key).await {
        Ok(Some(unpacked)) => {
            store::sweep_if_due(unpacked);
            return Ok(PackageFiles::Disk(dir));
        }
        Ok(None) => {}
        // A cache that cannot be read is a reason to fetch, not to fail
        Err(e) => tracing::warn!("could not pull {key} from the object store: {e:?}"),
    }

    // The same snapshot the cache key came from, not a second read of the setting.
    let package_json = fetch_package_json_from(&registry_url, &auth_token, package).await?;
    let archive =
        fetch_tarball(&package_json, package, version, &registry_url, &auth_token).await?;

    let target = dir.clone();
    let extracted = blocking(move || {
        // The cache is an optimisation, so a disk that will not take it degrades to
        // extracting per request rather than failing one. The retained files are built in
        // memory only on that path: doing it always would put back the per-request heap
        // this module exists to remove.
        let mut written = 0u64;
        // Which sink failed, not which error it produced: an unreadable archive and a full
        // volume both surface as `InternalErr`, and only one of them is worth retrying
        // without the disk.
        let mut disk_failed = false;
        let to_disk = (|| {
            let pending = PendingPackage::new(&target).inspect_err(|_| disk_failed = true)?;
            let manifest = extract_to(
                &archive,
                &mut |path, content| {
                    // A block minimum, because the sweep measures blocks: a package of many
                    // tiny declarations would otherwise never trip the early sweep.
                    written += (content.len() as u64).max(DISK_BLOCK_BYTES);
                    pending.write_file(path, content).inspect_err(|_| disk_failed = true)
                },
                Retention::PRODUCTION,
            )?;
            pending.publish(&manifest).inspect_err(|_| disk_failed = true)
        })();

        match to_disk {
            Ok((published, manifest_bytes)) => Ok((
                PackageFiles::Disk(published),
                written + manifest_bytes.max(DISK_BLOCK_BYTES),
            )),
            // The archive is the problem, not the cache: extracting it again would fail the
            // same way, and reporting it as an unwritable cache misdirects whoever reads
            // the log to decide whether their volume is full.
            Err(e) if !disk_failed => Err(e),
            Err(e) => {
                tracing::warn!("npm proxy cache is not usable, serving without it: {e:?}");
                let mut kept = HashMap::new();
                let manifest = extract_to(
                    &archive,
                    &mut |path, content| {
                        kept.insert(path.to_string(), content.to_vec());
                        Ok(())
                    },
                    Retention::PRODUCTION,
                )?;
                Ok((PackageFiles::Memory(kept, manifest), 0))
            }
        }
    })
    .await?;

    match extracted {
        (PackageFiles::Disk(published), written) => {
            store::push_to_object_store(published.clone(), key);
            store::sweep_if_due(written);
            Ok(PackageFiles::Disk(published))
        }
        (files, _) => Ok(files),
    }
}

/// Inflating an archive is CPU-bound and can run for as long as the archive is large, so
/// it belongs on a blocking thread rather than a runtime worker serving other requests.
async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T> + Send + 'static,
) -> Result<T> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|e| Error::InternalErr(format!("Failed to read package archive: {}", e)))?
}

#[cfg(test)]
mod tests {
    use super::store::PendingPackage;
    use super::{
        byte_bounded_cache, extract_to, read_one_entry, resolve_version_spec, CachedPackageJson,
        Retention,
    };
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::Instant;
    use windmill_common::error::Result;

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

    const TINY: Retention = Retention {
        max_entries: 16,
        entry_bytes: 128,
        manifest_bytes: 4096,
        total_bytes: 4096,
        path_bytes: 4096,
    };

    /// Extract into a throwaway directory and report what landed where.
    fn extract(archive: &[u8], retention: Retention) -> Result<(super::store::Manifest, PathBuf)> {
        let dir = std::env::temp_dir()
            .join(format!("npm-proxy-test-{}", uuid::Uuid::new_v4()))
            .join("pkg")
            .join("1.0.0");
        let pending = PendingPackage::new(&dir)?;
        let manifest = extract_to(archive, &mut |path, content| pending.write_file(path, content), retention)?;
        let (published, _) = pending.publish(&manifest)?;
        Ok((manifest, published))
    }

    fn kept(dir: &PathBuf) -> Vec<String> {
        let files = dir.join("files");
        let mut out = Vec::new();
        let mut stack = vec![files.clone()];
        while let Some(current) = stack.pop() {
            for entry in std::fs::read_dir(&current).unwrap() {
                let entry = entry.unwrap();
                if entry.file_type().unwrap().is_dir() {
                    stack.push(entry.path());
                } else {
                    let path = entry.path();
                    out.push(
                        path.strip_prefix(&files).unwrap().to_string_lossy().to_string(),
                    );
                }
            }
        }
        out.sort();
        out
    }

    #[test]
    fn only_what_a_type_request_reads_is_retained() {
        // `next` orders its manifest 3748th, behind thousands of licences, and
        // DefinitelyTyped roots its archives at the type name rather than `package`
        let archive = tarball(&[
            ("express/dist/compiled/a/LICENSE", 64),
            ("express/dist/bundle.js", TINY.entry_bytes as usize + 1),
            ("express/package.json", 8),
            ("express/types/index.d.ts", 32),
        ]);

        let (manifest, dir) = extract(&archive, TINY).unwrap();

        assert_eq!(manifest.names.len(), 4);
        assert_eq!(kept(&dir), vec!["package.json", "types/index.d.ts"]);
        // what is not kept stays reachable by walking the archive
        assert_eq!(
            read_one_entry(&archive, "dist/bundle.js").unwrap().map(|c| c.len()),
            Some(TINY.entry_bytes as usize + 1)
        );
        assert_eq!(read_one_entry(&archive, "absent.js").unwrap(), None);

        // Refusing is for what would make an answer wrong, too many entries to list or too
        // many path bytes, never for a package merely being large
        assert!(extract(&archive, Retention { max_entries: 1, ..TINY }).is_err());

        // Filling the entry budget leaves the file list alone, and the manifest is exempt
        // from it, so a package is still served and still has an entry point
        let (manifest, dir) = extract(&archive, Retention { total_bytes: 8, ..TINY }).unwrap();
        assert_eq!(manifest.names.len(), 4);
        assert_eq!(kept(&dir), vec!["package.json"]);

        // A package whose declarations fill that budget is served too, rather than refused
        // by the path check for bytes the declarations spent
        let declarations: Vec<(String, usize)> = (0..20)
            .map(|i| (format!("pkg/t{}.d.ts", i), TINY.entry_bytes as usize - 1))
            .collect();
        let heavy = tarball(
            &declarations
                .iter()
                .map(|(path, size)| (path.as_str(), *size))
                .collect::<Vec<_>>(),
        );
        let (manifest, dir) =
            extract(&heavy, Retention { max_entries: 32, total_bytes: 1000, ..TINY }).unwrap();
        assert_eq!(manifest.names.len(), 20);
        assert!(kept(&dir).len() < 20, "the budget should have stopped retaining");
    }

    /// A manifest ordered behind a package's declarations must still set the entry point:
    /// defaulting it is a wrong answer, not a slower one.
    #[test]
    fn the_entry_point_survives_a_spent_budget() {
        let manifest_json = br#"{"main":"lib/index.js"}"#;
        let encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        let mut builder = tar::Builder::new(encoder);
        for (path, content) in [
            ("package/a.d.ts", &b"xxxxxxxxxxxxxxxx"[..]),
            ("package/package.json", &manifest_json[..]),
        ] {
            let mut header = tar::Header::new_gnu();
            header.set_size(content.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder.append_data(&mut header, path, content).unwrap();
        }
        let archive = builder.into_inner().unwrap().finish().unwrap();

        // the declaration ahead of it consumes the whole budget
        let (manifest, _) = extract(&archive, Retention { total_bytes: 20, ..TINY }).unwrap();
        assert_eq!(manifest.main, "lib/index.js");

        // and a manifest past the per-entry ceiling still sets it, since that ceiling is
        // about how much to keep rather than about which answer is right
        let (manifest, _) = extract(&archive, Retention { entry_bytes: 4, ..TINY }).unwrap();
        assert_eq!(manifest.main, "lib/index.js");

        // and a package that genuinely declares none still gets the default
        let bare = tarball(&[("package/index.js", 4)]);
        assert_eq!(extract(&bare, TINY).unwrap().0.main, "index.js");
    }

    /// The file list has to be complete for `/filetree` to be right, so an archive of long
    /// paths is refused rather than silently truncated.
    #[test]
    fn an_archive_of_long_paths_is_refused() {
        let long = format!("package/{}.js", "p".repeat(2000));
        let padded = tarball(&[(long.as_str(), 0)]);

        assert!(extract(&padded, Retention { path_bytes: 512, ..TINY }).is_err());
        assert!(extract(&padded, TINY).is_ok());
    }

    /// A cache that splits its budget across shards refuses anything heavier than one
    /// shard's share, silently declining the documents worth caching most.
    #[test]
    fn an_entry_near_the_whole_budget_is_kept() {
        let budget = 4096;
        let cache = byte_bounded_cache(200, budget);
        let key = ("registry".to_string(), "big".to_string());

        cache.insert(
            key.clone(),
            CachedPackageJson {
                document: Arc::new(serde_json::json!({})),
                bytes: budget / 2,
                fetched_at: Instant::now(),
            },
        );

        assert!(cache.get(&key).is_some());
    }
}
