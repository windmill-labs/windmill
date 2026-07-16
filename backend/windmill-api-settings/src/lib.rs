/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2023
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::{
    collections::{BTreeSet, HashMap},
    time::Duration,
};

#[cfg(feature = "parquet")]
mod audit_logs_s3;
#[cfg(feature = "parquet")]
mod audit_logs_s3_backfill;
#[cfg(feature = "parquet")]
mod background_task;
#[cfg(feature = "private")]
mod ee;
pub mod ee_oss;
#[cfg(feature = "parquet")]
mod log_cleanup;
#[cfg(feature = "parquet")]
mod storage_usage;

use windmill_api_auth::{require_devops_role, require_super_admin, ApiAuthed};
use windmill_common::utils::HTTP_CLIENT_PERMISSIVE as HTTP_CLIENT;
use windmill_common::DB;

use ee_oss::validate_license_key;
use windmill_common::usernames::generate_instance_username_for_all_users;

#[cfg(feature = "enterprise")]
use axum::extract::Query;
use axum::{
    body::Body,
    extract::{Extension, Path},
    response::Response,
    routing::{get, post},
    Json, Router,
};
use serde_json::json;

use serde::{Deserialize, Serialize};
use windmill_ai::ai_cache::bump_instance_ai_config_revision;
#[cfg(feature = "enterprise")]
use windmill_common::ee_oss::{send_critical_alert, CriticalAlertKind, CriticalErrorChannel};
#[cfg(all(feature = "private", feature = "enterprise"))]
use windmill_common::secret_backend::{
    AwsSecretsManagerSettings, AzureKeyVaultSettings, SecretMigrationReport, VaultSettings,
};
use windmill_common::{
    ee_oss::{get_license_plan, LicensePlan},
    email_oss::send_email_plain_text,
    error::{self, JsonResult, Result},
    get_database_url,
    global_settings::{
        AI_CONFIG_SETTING, APP_WORKSPACED_ROUTE_SETTING, AUTOMATE_USERNAME_CREATION_SETTING,
        CRITICAL_ALERT_MUTE_UI_SETTING, DEFAULT_TAGS_WORKSPACES_SETTING, DISABLE_HUB_SETTING,
        EMAIL_DOMAIN_SETTING, ENV_SETTINGS, HTTP_ROUTE_WORKSPACED_ROUTE_SETTING,
        HUB_ACCESSIBLE_URL_SETTING, HUB_BASE_URL_SETTING, MAX_RETENTION_OVERRIDE_WORKSPACES,
        RETENTION_PERIOD_SECS_OVERRIDES_SETTING, RUFF_CONFIG_SETTING,
        WORKSPACE_FAIRNESS_DURATION_SECS_SETTING, WORKSPACE_FAIRNESS_ENABLED_SETTING,
        WORKSPACE_FAIRNESS_MAX_PERCENT_SETTING, WORKSPACE_FAIRNESS_MIN_TOTAL_SETTING,
        WS_BASE_URL_SETTING,
    },
    instance_config::{self, ApplyMode, InstanceConfig},
    server::Smtp,
};
use windmill_common::{error::to_anyhow, worker::CLOUD_HOSTED, PgDatabase};

/// Unauthenticated settings routes.
///
/// Used by the extra container (LSP service) to fetch non-sensitive instance
/// configuration like the shared ruff.toml content without needing to carry
/// a credential.
pub fn unauthed_service() -> Router {
    Router::new().route("/ruff_config", get(get_ruff_config_unauthed))
}

/// Public endpoint that returns the instance-level ruff config as plain text
/// TOML. Returns an empty body when unset.
///
/// This is intentionally unauthenticated: ruff config is lint/format policy,
/// not a credential, and the extra container needs to pull it from any
/// deployment topology (docker-compose, k8s, local dev) without the extra
/// burden of shared secrets.
async fn get_ruff_config_unauthed(Extension(db): Extension<DB>) -> error::Result<Response> {
    let value = sqlx::query_scalar!(
        "SELECT value FROM global_settings WHERE name = $1",
        RUFF_CONFIG_SETTING
    )
    .fetch_optional(&db)
    .await?;

    let body = value
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_default();

    Ok(Response::builder()
        .status(200)
        .header("content-type", "text/plain; charset=utf-8")
        .header("cache-control", "no-store")
        .body(Body::from(body))
        .unwrap())
}

pub fn global_service() -> Router {
    #[warn(unused_mut)]
    let r = Router::new()
        .route("/envs", get(get_local_settings))
        .route(
            "/global/{key}",
            post(set_global_setting).get(get_global_setting),
        )
        .route("/list_global", get(list_global_settings))
        .route(
            "/instance_config",
            get(get_instance_config).put(set_instance_config),
        )
        .route("/instance_config/yaml", get(get_instance_config_yaml))
        .route("/test_smtp", post(test_email))
        .route("/test_license_key", post(test_license_key))
        .route("/send_stats", post(send_stats))
        .route("/get_stats", get(get_stats))
        .route(
            "/latest_key_renewal_attempt",
            get(get_latest_key_renewal_attempt),
        )
        .route("/renew_license_key", post(renew_license_key))
        .route("/offline_license_status", get(get_offline_license_status))
        .route("/instance_hash", get(get_instance_hash))
        .route("/customer_portal", post(create_customer_portal_session))
        .route("/test_critical_channels", post(test_critical_channels))
        .route("/critical_alerts", get(get_critical_alerts))
        .route(
            "/critical_alerts/{id}/acknowledge",
            post(acknowledge_critical_alert),
        )
        .route(
            "/list_custom_instance_pg_databases",
            post(list_custom_instance_pg_databases),
        )
        .route(
            "/refresh_custom_instance_user_pwd",
            post(refresh_custom_instance_user_pwd),
        )
        .route(
            "/setup_custom_instance_pg_database/{name}",
            post(setup_custom_instance_pg_database),
        )
        .route(
            "/drop_custom_instance_pg_database/{name}",
            post(drop_custom_instance_pg_database),
        )
        .route(
            "/critical_alerts/acknowledge_all",
            post(acknowledge_all_critical_alerts),
        )
        .route(
            "/sync_cached_resource_types",
            post(sync_cached_resource_types),
        )
        .route(
            "/restart_worker_group/{worker_group}",
            post(restart_worker_group),
        );

    // Vault/Azure KV integration routes (EE only - requires both private and enterprise features)
    #[cfg(all(feature = "private", feature = "enterprise"))]
    let r = r
        .route("/test_secret_backend", post(test_secret_backend))
        .route("/migrate_secrets_to_vault", post(migrate_secrets_to_vault))
        .route(
            "/migrate_secrets_to_database",
            post(migrate_secrets_to_database),
        )
        .route("/test_azure_kv_backend", post(test_azure_kv_backend))
        .route(
            "/migrate_secrets_to_azure_kv",
            post(migrate_secrets_to_azure_kv),
        )
        .route(
            "/migrate_secrets_from_azure_kv",
            post(migrate_secrets_from_azure_kv),
        )
        .route("/test_aws_sm_backend", post(test_aws_sm_backend))
        .route(
            "/migrate_secrets_to_aws_sm",
            post(migrate_secrets_to_aws_sm),
        )
        .route(
            "/migrate_secrets_from_aws_sm",
            post(migrate_secrets_from_aws_sm),
        );

    #[cfg(feature = "parquet")]
    {
        return r
            .route("/test_object_storage_config", post(test_s3_bucket))
            .route(
                "/object_storage_usage",
                get(get_object_storage_usage).post(compute_object_storage_usage),
            )
            .route("/run_log_cleanup", post(run_log_cleanup))
            .route("/log_cleanup_status", get(log_cleanup_status))
            .route("/audit_logs_s3_status", get(audit_logs_s3_status))
            .route("/audit_logs_s3_backfill", post(run_audit_logs_s3_backfill))
            .route(
                "/audit_logs_s3_backfill_status",
                get(audit_logs_s3_backfill_status),
            );
    }

    #[cfg(not(feature = "parquet"))]
    {
        return r;
    }
}

#[derive(Deserialize)]
pub struct TestEmail {
    pub to: String,
    pub smtp: Smtp,
}

pub async fn test_email(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Json(test_email): Json<TestEmail>,
) -> error::Result<String> {
    require_super_admin(&db, &authed).await?;
    let smtp = test_email.smtp;
    let to = test_email.to;

    let client_timeout = Duration::from_secs(3);
    send_email_plain_text(
        "Test email from Windmill",
        "Test email content",
        vec![to],
        smtp,
        Some(client_timeout),
    )
    .await?;

    Ok("Sent test email".to_string())
}

#[cfg(feature = "parquet")]
use windmill_object_store::ObjectSettings;

#[cfg(feature = "parquet")]
use windmill_object_store::build_object_store_from_settings;

#[cfg(feature = "parquet")]
pub async fn test_s3_bucket(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Json(test_s3_bucket): Json<ObjectSettings>,
) -> error::Result<String> {
    use bytes::Bytes;
    use futures::StreamExt;

    // The probe executes on the API server itself. On multi-tenant Cloud that is a shared control
    // plane, so we constrain untrusted callers to remove the SSRF / credential-exfiltration /
    // local-filesystem surface (see validate_object_storage_test). On self-hosted instances the
    // object store usually lives on the local/private network and all authenticated users are
    // trusted, so testing there stays unrestricted. Super admins keep the unrestricted path too.
    let is_super_admin = windmill_api_auth::is_super_admin_authed(&db, &authed).await?;
    let restrict = !is_super_admin && *CLOUD_HOSTED;
    if restrict {
        validate_object_storage_test(&test_s3_bucket).await?;
    }

    let client = build_object_store_from_settings(test_s3_bucket, Some(&db))
        .await?
        .store;

    let run = async {
        let mut list = client.list(Some(
            &windmill_object_store::object_store_reexports::Path::from("".to_string()),
        ));
        let first_file = list.next().await;
        if first_file.is_some() {
            if let Err(e) = first_file.as_ref().unwrap() {
                tracing::error!("error listing bucket: {e:#}");
                error::Error::internal_err(format!("Failed to list files in blob storage: {e:#}"));
            }
            tracing::info!("Listed files: {:?}", first_file.unwrap());
        } else {
            tracing::info!("No files in blob storage");
        }

        let path = windmill_object_store::object_store_reexports::Path::from(format!(
            "/test-s3-bucket-{uuid}",
            uuid = uuid::Uuid::new_v4()
        ));
        tracing::info!("Testing blob storage at path: {path}");
        client
            .put(
                &path,
                windmill_object_store::object_store_reexports::PutPayload::from_static(b"hello"),
            )
            .await
            .map_err(|e| anyhow::anyhow!("error writing file to {path}: {e:#}"))?;
        let content = client
            .get(&path)
            .await
            .map_err(to_anyhow)?
            .bytes()
            .await
            .map_err(to_anyhow)?;
        if content != Bytes::from_static(b"hello") {
            return Err(error::Error::internal_err(
                "Failed to read back from blob storage".to_string(),
            ));
        }
        client.delete(&path).await.map_err(to_anyhow)?;
        Ok::<String, error::Error>("Tested blob storage successfully".to_string())
    };

    if restrict {
        // The object-store client is built with timeouts disabled, so a malicious endpoint could
        // otherwise hold the API server connection open indefinitely.
        tokio::time::timeout(Duration::from_secs(15), run)
            .await
            .map_err(|_| {
                error::Error::internal_err("Object storage connectivity test timed out".to_string())
            })?
    } else {
        run.await
    }
}

// Hardening for the object-storage connectivity test by an untrusted (non-super-admin) caller on
// Cloud. The probe runs on the shared API server, so without these constraints an authenticated
// user could coerce the server into connecting to arbitrary internal endpoints (SSRF), signing
// requests with the instance role (credential exfiltration), or reading/writing the server's local
// disk (filesystem object store).
#[cfg(feature = "parquet")]
async fn validate_object_storage_test(settings: &ObjectSettings) -> error::Result<()> {
    fn non_empty(opt: &Option<String>) -> bool {
        opt.as_ref().is_some_and(|s| !s.is_empty())
    }

    // Reject backends that rely on the server's identity or local filesystem, require explicit
    // credentials for the rest (so the server never falls back to its own ambient credentials), and
    // resolve the host the client will actually connect to. We derive the *effective* endpoint here
    // — mirroring build_*_from_settings: the region/account-derived default and the virtual-hosted
    // bucket prefix — rather than only validating a caller-supplied `endpoint`, so caller-controlled
    // `region`/`account_name`/`bucket` cannot smuggle an internal host past the check (e.g. an empty
    // endpoint with region = "@169.254.169.254/" otherwise resolves to the cloud metadata service).
    let effective_endpoint: Option<String> = match settings {
        ObjectSettings::Filesystem(_) => {
            return Err(error::Error::NotAuthorized(
                "Testing a local filesystem object store requires a super admin".to_string(),
            ));
        }
        ObjectSettings::AwsOidc(_) => {
            return Err(error::Error::NotAuthorized(
                "Testing OIDC-based object storage requires a super admin".to_string(),
            ));
        }
        ObjectSettings::S3(s3) => {
            if !(non_empty(&s3.access_key) && non_empty(&s3.secret_key)) {
                return Err(error::Error::NotAuthorized(
                    "Testing S3 storage without explicit credentials requires a super admin"
                        .to_string(),
                ));
            }
            let region = s3
                .region
                .clone()
                .filter(|r| !r.is_empty())
                .or_else(|| std::env::var("AWS_REGION").ok().filter(|r| !r.is_empty()))
                .unwrap_or_else(|| "us-east-1".to_string());
            let raw_endpoint = s3
                .endpoint
                .clone()
                .filter(|e| !e.is_empty())
                .or_else(|| std::env::var("S3_ENDPOINT").ok().filter(|e| !e.is_empty()))
                .unwrap_or_else(|| format!("s3.{region}.amazonaws.com"));
            Some(windmill_object_store::render_endpoint(
                raw_endpoint,
                !s3.allow_http.unwrap_or(true),
                s3.port,
                s3.path_style,
                s3.bucket.clone().unwrap_or_default(),
            ))
        }
        ObjectSettings::Azure(azure) => {
            if !non_empty(&azure.access_key) {
                return Err(error::Error::NotAuthorized(
                    "Testing Azure storage without an explicit access key requires a super admin"
                        .to_string(),
                ));
            }
            Some(
                azure
                    .endpoint
                    .clone()
                    .filter(|e| !e.is_empty())
                    .unwrap_or_else(|| format!("{}.blob.core.windows.net", azure.account_name)),
            )
        }
        ObjectSettings::Gcs(gcs) => {
            // Mirror `build_gcs_client`'s blank-key check (shared predicate): a blank/`{}` key falls
            // back to the instance's ambient credentials there, so it must be rejected here too —
            // otherwise an untrusted caller could probe with the server's identity (the very
            // SSRF/credential-exfil this function guards against).
            if windmill_object_store::gcs_service_account_key_is_blank(&gcs.service_account_key) {
                return Err(error::Error::NotAuthorized(
                    "Testing GCS storage without a service account key requires a super admin"
                        .to_string(),
                ));
            }
            // The service-account-key JSON can override the data-plane URL (`gcs_base_url`) and the
            // OAuth token endpoint (`token_uri`); the GCS client connects to whatever they point at.
            // Validate every http(s) URL embedded in the key. When none override it, the host stays
            // the public storage.googleapis.com, so no further check is needed.
            if let Ok(serde_json::Value::Object(map)) =
                serde_json::from_str::<serde_json::Value>(&gcs.service_account_key)
            {
                for value in map.values() {
                    if let Some(url) = value.as_str() {
                        // Match how the URL parser reads the value: leading whitespace/control is
                        // ignored and the scheme is case-insensitive.
                        let url =
                            url.trim_start_matches(|c: char| c.is_whitespace() || c.is_control());
                        if strip_http_scheme(url).is_some() {
                            validate_public_endpoint(url).await?;
                        }
                    }
                }
            }
            None
        }
    };

    // Block non-public network targets (internal services, cloud metadata, loopback, ...).
    if let Some(endpoint) = effective_endpoint {
        validate_public_endpoint(&endpoint).await?;
    }
    Ok(())
}

#[cfg(feature = "parquet")]
async fn validate_public_endpoint(endpoint: &str) -> error::Result<()> {
    let host = extract_host(endpoint).ok_or_else(|| {
        error::Error::BadRequest(format!("Invalid object storage endpoint: {endpoint}"))
    })?;

    let addrs: Vec<std::net::SocketAddr> = tokio::net::lookup_host((host.as_str(), 443u16))
        .await
        .map_err(|e| {
            error::Error::BadRequest(format!(
                "Could not resolve object storage endpoint '{host}': {e}"
            ))
        })?
        .collect();

    if addrs.is_empty() {
        return Err(error::Error::BadRequest(format!(
            "Could not resolve object storage endpoint '{host}'"
        )));
    }

    // Reject if any resolved address is non-public, which also defeats the simplest DNS-rebinding
    // attempts (a name resolving to both a public and a private address).
    for addr in addrs {
        if is_forbidden_ip(addr.ip()) {
            return Err(error::Error::NotAuthorized(
                "Testing object storage at a private, loopback, or link-local endpoint requires a super admin"
                    .to_string(),
            ));
        }
    }
    Ok(())
}

// Strip a leading `http://`/`https://` scheme case-insensitively (URL schemes are
// case-insensitive), returning the remainder when one was present.
#[cfg(feature = "parquet")]
fn strip_http_scheme(s: &str) -> Option<&str> {
    for scheme in ["https://", "http://"] {
        let b = scheme.as_bytes();
        if s.len() >= b.len() && s.as_bytes()[..b.len()].eq_ignore_ascii_case(b) {
            return Some(&s[b.len()..]);
        }
    }
    None
}

#[cfg(feature = "parquet")]
fn extract_host(endpoint: &str) -> Option<String> {
    let mut s = endpoint.trim();
    if let Some(rest) = strip_http_scheme(s) {
        s = rest;
    }
    s = s.split(['/', '?', '#', '\\']).next().unwrap_or(s);
    if let Some((_, rest)) = s.rsplit_once('@') {
        s = rest;
    }
    let host = if let Some(rest) = s.strip_prefix('[') {
        // IPv6 literal, e.g. [::1]:9000
        rest.split(']').next().unwrap_or(rest)
    } else {
        // host or host:port
        s.split(':').next().unwrap_or(s)
    }
    .trim();
    if host.is_empty() {
        None
    } else {
        Some(host.to_string())
    }
}

#[cfg(feature = "parquet")]
fn is_forbidden_ip(ip: std::net::IpAddr) -> bool {
    use std::net::{IpAddr, Ipv4Addr};
    match ip {
        IpAddr::V4(v4) => {
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local() // 169.254.0.0/16, incl. the cloud metadata endpoint
                || v4.is_unspecified()
                || v4.is_broadcast()
                || v4.is_documentation()
                || v4.is_multicast()
                || v4.octets()[0] == 0 // 0.0.0.0/8
                || (v4.octets()[0] == 100 && (v4.octets()[1] & 0xc0) == 64) // 100.64.0.0/10 CGNAT
        }
        IpAddr::V6(v6) => {
            // Any IPv4 embedded in an IPv6 address (IPv4-mapped ::ffff:0:0/96, IPv4-compatible
            // ::/96, or NAT64 64:ff9b::/96) is re-checked against the IPv4 rules, so e.g.
            // 64:ff9b::169.254.169.254 cannot route to the metadata endpoint in a NAT64 network.
            let seg = v6.segments();
            let is_v4_compatible = seg[0..6] == [0, 0, 0, 0, 0, 0];
            let is_nat64 = seg[0] == 0x0064 && seg[1] == 0xff9b && seg[2..6] == [0, 0, 0, 0];
            if let Some(v4) = v6.to_ipv4_mapped() {
                return is_forbidden_ip(IpAddr::V4(v4));
            }
            if is_v4_compatible || is_nat64 {
                let embedded = Ipv4Addr::new(
                    (seg[6] >> 8) as u8,
                    (seg[6] & 0xff) as u8,
                    (seg[7] >> 8) as u8,
                    (seg[7] & 0xff) as u8,
                );
                if is_forbidden_ip(IpAddr::V4(embedded)) {
                    return true;
                }
            }
            v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_multicast()
                || (seg[0] & 0xfe00) == 0xfc00 // fc00::/7 unique local
                || (seg[0] & 0xffc0) == 0xfe80 // fe80::/10 link-local
        }
    }
}

#[cfg(feature = "parquet")]
async fn get_object_storage_usage(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
) -> error::JsonResult<Option<storage_usage::StorageUsageProgress>> {
    require_super_admin(&db, &authed).await?;
    Ok(Json(storage_usage::get_status(&db).await?))
}

#[cfg(feature = "parquet")]
async fn compute_object_storage_usage(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
) -> error::Result<axum::http::StatusCode> {
    require_super_admin(&db, &authed).await?;
    storage_usage::try_start(&db).await?;
    storage_usage::spawn_compute(db.clone());
    Ok(axum::http::StatusCode::ACCEPTED)
}

#[cfg(feature = "parquet")]
async fn run_log_cleanup(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
) -> error::Result<axum::http::StatusCode> {
    require_super_admin(&db, &authed).await?;
    log_cleanup::try_start(&db).await?;
    log_cleanup::spawn_cleanup(db.clone());
    Ok(axum::http::StatusCode::ACCEPTED)
}

#[cfg(feature = "parquet")]
async fn log_cleanup_status(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
) -> error::JsonResult<Option<log_cleanup::LogCleanupProgress>> {
    require_super_admin(&db, &authed).await?;
    Ok(Json(log_cleanup::get_status(&db).await?))
}

#[cfg(feature = "parquet")]
async fn audit_logs_s3_status(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
) -> error::JsonResult<Option<audit_logs_s3::AuditLogsS3ExportStatus>> {
    require_super_admin(&db, &authed).await?;
    Ok(Json(audit_logs_s3::get_status(&db).await?))
}

#[cfg(feature = "parquet")]
async fn run_audit_logs_s3_backfill(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Json(req): Json<audit_logs_s3_backfill::BackfillRequest>,
) -> error::Result<axum::http::StatusCode> {
    require_super_admin(&db, &authed).await?;
    if !matches!(get_license_plan().await, LicensePlan::Enterprise) {
        return Err(error::Error::BadRequest(
            "Audit log export to object storage is an Enterprise feature".to_string(),
        ));
    }
    audit_logs_s3_backfill::try_start(&db, req.from, req.to).await?;
    audit_logs_s3_backfill::spawn_backfill(db.clone(), req.from, req.to);
    Ok(axum::http::StatusCode::ACCEPTED)
}

#[cfg(feature = "parquet")]
async fn audit_logs_s3_backfill_status(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
) -> error::JsonResult<Option<audit_logs_s3_backfill::AuditBackfillProgress>> {
    require_super_admin(&db, &authed).await?;
    Ok(Json(audit_logs_s3_backfill::get_status(&db).await?))
}

#[derive(Deserialize)]
pub struct TestKey {
    pub license_key: String,
}

pub async fn test_license_key(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Json(TestKey { license_key }): Json<TestKey>,
) -> error::Result<String> {
    require_super_admin(&db, &authed).await?;
    let (_, expired, _offline_meta) = validate_license_key(license_key, Some(&db)).await?;

    if expired {
        Err(error::Error::BadRequest("Expired license key".to_string()))
    } else {
        Ok("Valid license key".to_string())
    }
}

#[derive(serde::Serialize)]
pub struct InstanceHash {
    pub instance_hash: Option<String>,
}

/// Returns the live cap status for an offline license, or `null` when no
/// offline license is loaded. Used by the superadmin settings panel.
pub async fn get_offline_license_status(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
) -> error::JsonResult<Option<windmill_common::ee_oss::OfflineCapStatus>> {
    require_super_admin(&db, &authed).await?;

    let offline = (**windmill_common::ee_oss::LICENSE_OFFLINE_METADATA.load()).clone();
    let is_offline = matches!(&offline, Some(m) if m.is_offline());

    if !is_offline {
        return Ok(Json(None));
    }

    #[cfg(feature = "enterprise")]
    let cap = windmill_common::ee_oss::enforce_offline_caps(&db)
        .await
        .map_err(|e| error::Error::internal_err(format!("enforce_offline_caps: {e:#}")))?;
    #[cfg(not(feature = "enterprise"))]
    let cap: Option<windmill_common::ee_oss::OfflineCapStatus> = None;

    Ok(Json(cap))
}

/// Returns the per-instance binding hash that goes into offline license keys.
/// Admin invokes via `curl` with their personal token when requesting a key
/// from support.
pub async fn get_instance_hash(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
) -> error::JsonResult<InstanceHash> {
    require_super_admin(&db, &authed).await?;
    #[cfg(feature = "enterprise")]
    let hash = windmill_common::ee_oss::compute_instance_hash(&db)
        .await
        .map_err(|e| error::Error::internal_err(format!("compute_instance_hash: {e:#}")))?;
    #[cfg(not(feature = "enterprise"))]
    let hash: Option<String> = None;
    Ok(Json(InstanceHash { instance_hash: hash }))
}

pub async fn get_local_settings(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
) -> error::JsonResult<serde_json::Value> {
    require_super_admin(&db, &authed).await?;

    let mut settings = serde_json::Map::new();
    for key in ENV_SETTINGS.iter() {
        if let Some(value) = std::env::var(key).ok() {
            settings.insert(key.to_string(), serde_json::Value::String(value));
        }
    }
    Ok(Json(serde_json::Value::Object(settings)))
}

#[derive(serde::Deserialize)]
pub struct Value {
    pub value: Option<serde_json::Value>,
}

pub async fn delete_global_setting(db: &DB, key: &str) -> error::Result<()> {
    // ducklake_user_pg_pwd and ducklake_settings were old names stored as standalone global settings.
    // Leave them for backward compatibility (CLI will try to delete them if not present in the yaml)
    if key == "ducklake_user_pg_pwd"
        || key == "ducklake_settings"
        || key == "custom_instance_pg_databases"
    {
        tracing::error!("Tried to unset global setting {}, ignored", key);
        return Ok(());
    }
    sqlx::query!("DELETE FROM global_settings WHERE name = $1", key,)
        .execute(db)
        .await?;
    tracing::info!("Unset global setting {}", key);
    Ok(())
}
/// Returns true when `key` is one of the workspace-fairness settings whose
/// writes must be gated to cloud only.
fn is_workspace_fairness_setting(key: &str) -> bool {
    matches!(
        key,
        WORKSPACE_FAIRNESS_ENABLED_SETTING
            | WORKSPACE_FAIRNESS_MAX_PERCENT_SETTING
            | WORKSPACE_FAIRNESS_DURATION_SECS_SETTING
            | WORKSPACE_FAIRNESS_MIN_TOTAL_SETTING
    )
}

/// Enterprise gate for the workspace-fairness settings. Workspace fairness is
/// only useful on multi-tenant clusters where one workspace can starve other
/// workspaces sharing the same worker pool, and the feature is licensed as
/// part of Enterprise. Non-EE installs are rejected at write time; the runtime
/// dispatch additionally honours the `WORKSPACE_FAIRNESS_ENABLED` toggle.
async fn workspace_fairness_settings_allowed() -> bool {
    matches!(get_license_plan().await, LicensePlan::Enterprise)
}

pub async fn set_global_setting(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Path(key): Path<String>,
    Json(value): Json<Value>,
) -> error::Result<()> {
    require_super_admin(&db, &authed).await?;
    set_global_setting_internal(&db, key, value.value.unwrap_or(serde_json::Value::Null)).await
}

pub async fn set_global_setting_internal(
    db: &DB,
    key: String,
    value: serde_json::Value,
) -> error::Result<()> {
    let should_bump_instance_ai_revision = key == AI_CONFIG_SETTING;
    let value = if key == "retention_period_secs" {
        instance_config::clamp_retention_period(value)
    } else {
        value
    };

    // EE gate for workspace-fairness settings. Workspace fairness only matters
    // on multi-tenant clusters; it is licensed as an Enterprise feature so the
    // setter rejects writes from non-EE builds. Disabling/clearing writes are
    // *always* allowed regardless of license, so an admin who downgrades from
    // EE (or imports a row from a cloned EE DB) can always turn the cap off:
    //   - `Null` / empty-string  → row delete
    //   - `Bool(false)` on `workspace_fairness_enabled` → explicit disable
    // Without the `Bool(false)` carve-out, a stale `enabled=true` row from a
    // downgrade would be impossible to flip off through the normal API/UI
    // and the runtime path (which only checks the toggle) would keep
    // throttling.
    let is_clearing_value = matches!(&value, serde_json::Value::Null)
        || matches!(&value, serde_json::Value::String(s) if s.trim().is_empty())
        || (key == WORKSPACE_FAIRNESS_ENABLED_SETTING
            && matches!(&value, serde_json::Value::Bool(false)));
    if is_workspace_fairness_setting(&key)
        && !is_clearing_value
        && !workspace_fairness_settings_allowed().await
    {
        return Err(error::Error::BadRequest(format!(
            "{} requires an Enterprise license",
            key
        )));
    }

    run_setting_pre_write_hook(db, &key, &value).await?;

    match value {
        serde_json::Value::Null => {
            if instance_config::PROTECTED_SETTINGS.contains(&key.as_str()) {
                return Err(error::Error::BadRequest(format!(
                    "{key} is a protected setting and cannot be deleted"
                )));
            }
            delete_global_setting(db, &key).await?;
        }
        serde_json::Value::String(ref x) if x.trim().is_empty() => {
            if instance_config::PROTECTED_SETTINGS.contains(&key.as_str()) {
                return Err(error::Error::BadRequest(format!(
                    "{key} is a protected setting and cannot be set to empty"
                )));
            }
            delete_global_setting(db, &key).await?;
        }
        v => {
            sqlx::query!(
                 "INSERT INTO global_settings (name, value) VALUES ($1, $2) ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value, updated_at = now()",
                 key,
                 v
             )
             .execute(db)
             .await?;
            tracing::info!(
                "Set global setting {} to {}",
                key,
                instance_config::format_setting_value(&key, &v)
            );
        }
    };

    if should_bump_instance_ai_revision {
        bump_instance_ai_config_revision();
    }

    Ok(())
}

/// Run side-effect hooks for specific settings before writing to DB.
/// Extracted from `set_global_setting_internal` for reuse by the bulk endpoint.
async fn run_setting_pre_write_hook(
    db: &DB,
    key: &str,
    value: &serde_json::Value,
) -> error::Result<()> {
    match key {
        AUTOMATE_USERNAME_CREATION_SETTING => {
            if value.as_bool().unwrap_or(false) {
                generate_instance_username_for_all_users(db)
                    .await
                    .map_err(|err| {
                        error::Error::internal_err(format!(
                            "Failed to generate instance wide usernames: {}",
                            err
                        ))
                    })?;
            } else {
                // Disabling is only allowed before any instance-wide username has been
                // assigned. Once usernames exist they are globally unique and are baked
                // into stored `u/<username>` identities (schedules, triggers, drafts,
                // and non-member superadmin ownership). Disabling would drop back to
                // workspace-local username uniqueness, letting a member reuse an
                // existing instance username and silently take over those identities —
                // so the setting is effectively one-way once derivation has taken
                // effect. Re-saving `false` on an already-disabled instance is a no-op
                // and stays allowed (guarded by the current-value check).
                let currently_enabled = sqlx::query_scalar!(
                    "SELECT value FROM global_settings WHERE name = $1",
                    AUTOMATE_USERNAME_CREATION_SETTING
                )
                .fetch_optional(db)
                .await?
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
                if currently_enabled {
                    let usernames_exist = sqlx::query_scalar!(
                        "SELECT EXISTS(SELECT 1 FROM password WHERE username IS NOT NULL)"
                    )
                    .fetch_one(db)
                    .await?
                    .unwrap_or(false);
                    if usernames_exist {
                        return Err(error::Error::BadRequest(
                            "automate_username_creation cannot be disabled once instance-wide usernames have been assigned: existing u/<username> identities (schedules, triggers, drafts, superadmin ownership) rely on those usernames staying stable and globally unique.".to_string(),
                        ));
                    }
                }
            }
        }
        CRITICAL_ALERT_MUTE_UI_SETTING => {
            if value.as_bool().unwrap_or(false) {
                sqlx::query!("UPDATE alerts SET acknowledged = true")
                    .execute(db)
                    .await?;
            }
        }
        APP_WORKSPACED_ROUTE_SETTING => {
            let serde_json::Value::Bool(workspaced_route) = value else {
                return Err(error::Error::BadRequest(format!(
                    "{} setting Expected to be boolean",
                    APP_WORKSPACED_ROUTE_SETTING
                )));
            };

            // Cloud always scopes app custom paths by workspace_id (see
            // `custom_path_exists` in apps.rs), so duplicates across workspaces
            // are expected and this setting has no runtime effect on cloud.
            if !*workspaced_route && !*CLOUD_HOSTED {
                #[derive(Debug, Deserialize, Serialize)]
                #[allow(unused)]
                struct DuplicateApp {
                    custom_path: Option<String>,
                    path: String,
                }
                let duplicate_app = sqlx::query_as!(
                    DuplicateApp,
                    r#"
                        SELECT
                            path,
                            custom_path
                        FROM
                            app
                        WHERE
                            custom_path IN (
                                SELECT
                                    custom_path
                                FROM
                                    app
                                GROUP
                                    BY custom_path
                                HAVING COUNT(*) > 1
                            )
                        ORDER BY custom_path
                        "#
                )
                .fetch_all(db)
                .await?;

                if !duplicate_app.is_empty() {
                    tracing::error!(
                        "Cannot disable {} setting as duplicate app with custom path were found: {:?}",
                        APP_WORKSPACED_ROUTE_SETTING,
                        &duplicate_app
                    );

                    #[derive(Serialize)]
                    struct ErrorResponse {
                        error: String,
                        details: Vec<DuplicateApp>,
                    }

                    let error_response = ErrorResponse {
                        error: "Duplicate custom paths detected".to_string(),
                        details: duplicate_app,
                    };

                    return Err(error::Error::JsonErr(
                        serde_json::to_value(error_response).unwrap(),
                    ));
                }
            }
        }
        HTTP_ROUTE_WORKSPACED_ROUTE_SETTING => {
            let serde_json::Value::Bool(workspaced_route) = value else {
                return Err(error::Error::BadRequest(format!(
                    "{} setting expected to be boolean",
                    HTTP_ROUTE_WORKSPACED_ROUTE_SETTING
                )));
            };

            // Cloud always scopes routes by workspace_id (see
            // `route_path_key_exists` in windmill-trigger-http), so duplicates
            // across workspaces are expected and this setting has no runtime
            // effect on cloud.
            if !*workspaced_route && !*CLOUD_HOSTED {
                #[derive(Debug, Deserialize, Serialize)]
                #[allow(unused)]
                struct DuplicateRoute {
                    route_path: String,
                    workspace_id: String,
                    http_method: String,
                }
                let duplicate_routes = sqlx::query_as!(
                    DuplicateRoute,
                    r#"
                        SELECT
                            route_path,
                            workspace_id,
                            http_method::TEXT AS "http_method!"
                        FROM
                            http_trigger
                        WHERE
                            workspaced_route IS FALSE
                            AND route_path_key IN (
                                SELECT
                                    route_path_key
                                FROM
                                    http_trigger
                                WHERE
                                    workspaced_route IS FALSE
                                GROUP BY
                                    route_path_key, http_method
                                HAVING COUNT(*) > 1
                            )
                        ORDER BY route_path_key
                    "#
                )
                .fetch_all(db)
                .await?;

                if !duplicate_routes.is_empty() {
                    tracing::error!(
                        "Cannot disable {} setting as duplicate http routes were found: {:?}",
                        HTTP_ROUTE_WORKSPACED_ROUTE_SETTING,
                        &duplicate_routes
                    );

                    #[derive(Serialize)]
                    struct ErrorResponse {
                        error: String,
                        details: Vec<DuplicateRoute>,
                    }

                    let error_response = ErrorResponse {
                        error: "Duplicate HTTP route paths detected".to_string(),
                        details: duplicate_routes,
                    };

                    return Err(error::Error::JsonErr(
                        serde_json::to_value(error_response).unwrap(),
                    ));
                }
            }
        }
        RETENTION_PERIOD_SECS_OVERRIDES_SETTING => {
            // Reject a malformed map at write time so it can never be persisted. A persisted bad
            // value (negative or non-integer) would fail to parse on the next server start and,
            // because the loader fails closed (skips cleanup until a known-good value is read),
            // silently disable ALL job-retention cleanup indefinitely. This shape check must stay in
            // sync with `parse_retention_overrides` in backend/src/monitor.rs.
            match value {
                // Clearing (delete row) is handled by the caller; allow it through.
                serde_json::Value::Null => {}
                serde_json::Value::String(s) if s.trim().is_empty() => {}
                serde_json::Value::Object(map) => {
                    if map.len() > MAX_RETENTION_OVERRIDE_WORKSPACES {
                        return Err(error::Error::BadRequest(format!(
                            "retention_period_secs_overrides: at most {MAX_RETENTION_OVERRIDE_WORKSPACES} per-workspace overrides are allowed, got {}",
                            map.len()
                        )));
                    }
                    for (ws, v) in map {
                        if !v.as_i64().is_some_and(|secs| secs >= 0) {
                            return Err(error::Error::BadRequest(format!(
                                "retention_period_secs_overrides: override for '{ws}' must be a non-negative integer number of seconds, got {v}"
                            )));
                        }
                    }
                }
                _ => {
                    return Err(error::Error::BadRequest(
                        "retention_period_secs_overrides must be a JSON object of {workspace_id: seconds}".to_string(),
                    ));
                }
            }
        }
        _ => {}
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Bulk instance config endpoints
// ---------------------------------------------------------------------------

async fn get_instance_config(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
) -> JsonResult<InstanceConfig> {
    require_super_admin(&db, &authed).await?;
    let config = InstanceConfig::from_db(&db)
        .await
        .map_err(|e| error::Error::internal_err(e.to_string()))?;
    Ok(Json(config))
}

async fn get_instance_config_yaml(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
) -> error::Result<Response> {
    require_super_admin(&db, &authed).await?;
    let config = InstanceConfig::from_db(&db)
        .await
        .map_err(|e| error::Error::internal_err(e.to_string()))?;
    let yaml = config
        .to_sorted_yaml()
        .map_err(|e| error::Error::internal_err(e))?;
    Response::builder()
        .header("content-type", "application/yaml")
        .body(Body::from(yaml))
        .map_err(|e| error::Error::internal_err(e.to_string()))
}

async fn set_instance_config(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Json(desired): Json<InstanceConfig>,
) -> error::Result<()> {
    require_super_admin(&db, &authed).await?;

    let current = InstanceConfig::from_db(&db)
        .await
        .map_err(|e| error::Error::internal_err(e.to_string()))?;

    let desired_map = desired.global_settings.to_settings_map();
    if !desired_map.is_empty() {
        let current_map = current.global_settings.to_settings_map();
        let settings_diff =
            instance_config::diff_global_settings(&current_map, &desired_map, ApplyMode::Merge);
        let ai_config_changed = settings_diff
            .upserts
            .iter()
            .any(|(key, _)| key == AI_CONFIG_SETTING);

        // Mirror the per-key EE gate in `set_global_setting_internal`. Without
        // this, the bulk endpoint would let a non-EE superadmin persist
        // `workspace_fairness_*` rows even though the per-key API rejects them.
        // Only block *non-disabling* upserts; deletes are allowed everywhere
        // (already filtered into `settings_diff.removals`) and a
        // `workspace_fairness_enabled=false` upsert is treated as a disable,
        // so a downgraded instance can always turn the cap off via the bulk
        // YAML endpoint too.
        let upserts_touch_fairness_non_disable = settings_diff.upserts.iter().any(|(k, v)| {
            if !is_workspace_fairness_setting(k) {
                return false;
            }
            !(k == WORKSPACE_FAIRNESS_ENABLED_SETTING
                && matches!(v, serde_json::Value::Bool(false)))
        });
        if upserts_touch_fairness_non_disable && !workspace_fairness_settings_allowed().await {
            return Err(error::Error::BadRequest(
                "Workspace fairness settings require an Enterprise license".to_string(),
            ));
        }

        for (key, value) in &settings_diff.upserts {
            run_setting_pre_write_hook(&db, key, value).await?;
        }

        instance_config::apply_settings_diff(&db, &settings_diff)
            .await
            .map_err(|e| error::Error::internal_err(e.to_string()))?;

        if ai_config_changed {
            bump_instance_ai_config_revision();
        }
    }

    if !desired.worker_configs.is_empty() {
        let current_wc: std::collections::BTreeMap<String, serde_json::Value> = current
            .worker_configs
            .iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    serde_json::to_value(v).expect("WorkerGroupConfig serialization cannot fail"),
                )
            })
            .collect();
        let desired_wc: std::collections::BTreeMap<String, serde_json::Value> = desired
            .worker_configs
            .iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    serde_json::to_value(v).expect("WorkerGroupConfig serialization cannot fail"),
                )
            })
            .collect();
        let configs_diff =
            instance_config::diff_worker_configs(&current_wc, &desired_wc, ApplyMode::Merge);
        instance_config::apply_configs_diff(&db, &configs_diff)
            .await
            .map_err(|e| error::Error::internal_err(e.to_string()))?;
    }

    Ok(())
}

pub async fn get_global_setting(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Path(key): Path<String>,
) -> JsonResult<serde_json::Value> {
    if !key.starts_with("default_error_handler_")
        && !key.starts_with("default_recovery_handler_")
        && !key.starts_with("default_success_handler_")
        && key != AUTOMATE_USERNAME_CREATION_SETTING
        && key != DEFAULT_TAGS_WORKSPACES_SETTING
        && key != HUB_BASE_URL_SETTING
        && key != HUB_ACCESSIBLE_URL_SETTING
        && key != DISABLE_HUB_SETTING
        && key != EMAIL_DOMAIN_SETTING
        && key != APP_WORKSPACED_ROUTE_SETTING
        && key != HTTP_ROUTE_WORKSPACED_ROUTE_SETTING
        && key != WS_BASE_URL_SETTING
    {
        require_super_admin(&db, &authed).await?;
    }
    let value = sqlx::query!("SELECT value FROM global_settings WHERE name = $1", key)
        .fetch_optional(&db)
        .await?
        .map(|x| x.value);

    Ok(Json(value.unwrap_or_else(|| serde_json::Value::Null)))
}

#[cfg(feature = "enterprise")]
#[derive(Deserialize, serde::Serialize)]
struct GlobalSetting {
    name: String,
    value: serde_json::Value,
}

#[cfg(feature = "enterprise")]
async fn list_global_settings(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
) -> JsonResult<Vec<GlobalSetting>> {
    require_super_admin(&db, &authed).await?;
    let settings = sqlx::query_as!(GlobalSetting, "SELECT name, value FROM global_settings")
        .fetch_all(&db)
        .await?;

    Ok(Json(settings))
}

#[cfg(not(feature = "enterprise"))]
async fn list_global_settings() -> JsonResult<String> {
    return Err(error::Error::BadRequest(
        "Listing global settings not available on community edition".to_string(),
    ));
}

pub async fn send_stats(Extension(db): Extension<DB>, authed: ApiAuthed) -> Result<String> {
    require_super_admin(&db, &authed).await?;
    windmill_common::stats_oss::send_stats(
        &HTTP_CLIENT,
        &db,
        windmill_common::stats_oss::SendStatsReason::Manual,
        false,
    )
    .await?;

    Ok("Sent stats".to_string())
}

async fn restart_worker_group(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Path(worker_group): Path<String>,
) -> error::Result<String> {
    require_devops_role(&db, &authed).await?;

    sqlx::query!(
        "INSERT INTO notify_event (channel, payload) VALUES ('restart_worker_group', $1)",
        worker_group
    )
    .execute(&db)
    .await?;

    Ok(format!(
        "Restart signal sent to worker group '{worker_group}'"
    ))
}

#[derive(serde::Serialize)]
pub struct StatsDownload {
    pub signature: String,
    pub data: String,
}

#[cfg(feature = "enterprise")]
pub async fn get_stats(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
) -> error::JsonResult<StatsDownload> {
    require_super_admin(&db, &authed).await?;
    let stats = windmill_common::stats_oss::get_stats_payload(
        &db,
        &windmill_common::stats_oss::SendStatsReason::Manual,
        false,
    )
    .await?;
    let json =
        serde_json::to_string(&stats).map_err(|e| error::Error::InternalErr(e.to_string()))?;
    let signature = windmill_common::stats_oss::sign_stats(&json);
    Ok(axum::Json(StatsDownload { signature, data: json }))
}

#[cfg(not(feature = "enterprise"))]
pub async fn get_stats() -> error::JsonResult<StatsDownload> {
    Err(error::Error::BadRequest(
        "Downloading telemetry is only available on enterprise edition".to_string(),
    ))
}

#[derive(serde::Serialize)]
pub struct KeyRenewalAttempt {
    result: String,
    attempted_at: chrono::DateTime<chrono::Utc>,
}

pub async fn get_latest_key_renewal_attempt(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
) -> JsonResult<Option<KeyRenewalAttempt>> {
    require_super_admin(&db, &authed).await?;

    let last_attempt = sqlx::query!(
        "SELECT value, created_at FROM metrics WHERE id = $1 ORDER BY created_at DESC LIMIT 1",
        "license_key_renewal"
    )
    .fetch_optional(&db)
    .await?;

    match last_attempt {
        Some(last_attempt) => {
            let last_attempt_result = serde_json::from_value::<String>(last_attempt.value)
                .map_err(|e| {
                    error::Error::internal_err(format!("Failed to parse last attempt: {}", e))
                })?;
            Ok(Json(Some(KeyRenewalAttempt {
                result: last_attempt_result,
                attempted_at: last_attempt.created_at,
            })))
        }
        None => Ok(Json(None)),
    }
}

#[cfg(feature = "enterprise")]
#[derive(Deserialize)]
pub struct LicenseQuery {
    license_key: Option<String>,
}

#[cfg(not(feature = "enterprise"))]
pub async fn renew_license_key() -> Result<String> {
    return Err(error::Error::BadRequest(
        "License key renewal not available on community edition".to_string(),
    ));
}

#[cfg(feature = "enterprise")]
pub async fn renew_license_key(
    Extension(db): Extension<DB>,
    Query(LicenseQuery { license_key }): Query<LicenseQuery>,
    authed: ApiAuthed,
) -> Result<String> {
    require_super_admin(&db, &authed).await?;
    let result = windmill_common::ee_oss::renew_license_key(
        &HTTP_CLIENT,
        &db,
        license_key,
        windmill_common::ee_oss::RenewReason::Manual,
    )
    .await;

    if result != "success" {
        return Err(error::Error::BadRequest(format!(
            "Failed to renew license key: {}",
            if result == "Unauthorized" {
                "Invalid key".to_string()
            } else {
                result
            }
        )));
    } else {
        return Ok("Renewed license key".to_string());
    }
}

#[cfg(not(feature = "enterprise"))]
pub async fn create_customer_portal_session() -> Result<String> {
    return Err(error::Error::BadRequest(
        "Customer portal is not available on community edition".to_string(),
    ));
}

#[cfg(feature = "enterprise")]
pub async fn create_customer_portal_session(
    Query(LicenseQuery { license_key }): Query<LicenseQuery>,
) -> Result<String> {
    let url =
        windmill_common::ee_oss::create_customer_portal_session(&HTTP_CLIENT, license_key).await?;

    return Ok(url);
}

#[cfg(feature = "enterprise")]
pub async fn test_critical_channels(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Json(test_critical_channels): Json<Vec<CriticalErrorChannel>>,
) -> Result<String> {
    require_super_admin(&db, &authed).await?;

    #[cfg(feature = "enterprise")]
    send_critical_alert(
        "Test critical error".to_string(),
        &db,
        CriticalAlertKind::CriticalError,
        Some(test_critical_channels),
    )
    .await;
    Ok("Sent test critical error".to_string())
}

#[cfg(not(feature = "enterprise"))]
pub async fn test_critical_channels() -> Result<String> {
    Ok("Critical channels require EE".to_string())
}

#[cfg(feature = "enterprise")]
pub async fn get_critical_alerts(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Query(params): Query<windmill_alerting::AlertQueryParams>,
) -> JsonResult<serde_json::Value> {
    require_devops_role(&db, &authed).await?;

    windmill_alerting::get_critical_alerts(db, params, None).await
}

#[cfg(not(feature = "enterprise"))]
pub async fn get_critical_alerts() -> error::Error {
    error::Error::NotFound("Critical Alerts require EE".to_string())
}

#[cfg(feature = "enterprise")]
pub async fn acknowledge_critical_alert(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Path(id): Path<i32>,
) -> error::Result<String> {
    require_devops_role(&db, &authed).await?;
    windmill_alerting::acknowledge_critical_alert(db, None, id).await
}

#[cfg(not(feature = "enterprise"))]
pub async fn acknowledge_critical_alert() -> error::Error {
    error::Error::NotFound("Critical Alerts require EE".to_string())
}

#[cfg(feature = "enterprise")]
pub async fn acknowledge_all_critical_alerts(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
) -> error::Result<String> {
    require_super_admin(&db, &authed).await?;

    windmill_alerting::acknowledge_all_critical_alerts(db, None).await
}

#[cfg(not(feature = "enterprise"))]
pub async fn acknowledge_all_critical_alerts() -> error::Error {
    error::Error::NotFound("Critical Alerts require EE".to_string())
}

#[derive(Deserialize, Debug, Serialize)]
struct CustomInstanceDb {
    logs: CustomInstanceDbLogs, // (Step, Message)[]
    success: bool,
    error: Option<String>,
    tag: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    used_by_workspaces: Vec<String>,
}

#[derive(Deserialize, Debug, Serialize, Default)]
#[serde(default)]
struct CustomInstanceDbLogs {
    super_admin: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    database_credentials: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    valid_dbname: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    created_database: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    db_connect: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    grant_permissions: String,
}

async fn list_custom_instance_pg_databases(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
) -> JsonResult<HashMap<String, CustomInstanceDb>> {
    let result = sqlx::query_scalar!(
        r#"SELECT value->'databases' FROM global_settings WHERE name = 'custom_instance_pg_databases'"#,
    )
    .fetch_one(&db)
    .await?
    .ok_or_else(|| error::Error::ExecutionErr("Couldn't find custom_instance_pg_databases".to_string()))?;
    let mut result: HashMap<String, CustomInstanceDb> =
        serde_json::from_value(result).map_err(|e| {
            error::Error::ExecutionErr(format!(
                "couldn't parse custom_instance_pg_databases.databases : {}",
                e.to_string()
            ))
        })?;

    if windmill_api_auth::is_super_admin_authed(&db, &authed).await? {
        // Enrich each database with the list of workspaces referencing it through
        // either a ducklake catalog or a datatable database whose resource_type is
        // 'instance'. Not stored in DB to avoid drift.
        let usages = sqlx::query!(
            r#"
            SELECT ws.workspace_id AS "workspace_id!", entry->'catalog'->>'resource_path' AS dbname
            FROM workspace_settings ws
            CROSS JOIN LATERAL jsonb_each(
                CASE WHEN jsonb_typeof(ws.ducklake->'ducklakes') = 'object'
                    THEN ws.ducklake->'ducklakes'
                    ELSE '{}'::jsonb END
            ) AS dl(k, entry)
            WHERE entry->'catalog'->>'resource_type' = 'instance'
            AND entry->'catalog'->>'resource_path' IS NOT NULL
            UNION ALL
            SELECT ws.workspace_id AS "workspace_id!", entry->'database'->>'resource_path' AS dbname
            FROM workspace_settings ws
            CROSS JOIN LATERAL jsonb_each(
                CASE WHEN jsonb_typeof(ws.datatable->'datatables') = 'object'
                    THEN ws.datatable->'datatables'
                    ELSE '{}'::jsonb END
            ) AS dt(k, entry)
            WHERE entry->'database'->>'resource_type' = 'instance'
            AND entry->'database'->>'resource_path' IS NOT NULL
            "#,
        )
        .fetch_all(&db)
        .await?;

        let mut by_db: HashMap<String, BTreeSet<String>> = HashMap::new();
        for row in usages {
            if let Some(dbname) = row.dbname {
                by_db.entry(dbname).or_default().insert(row.workspace_id);
            }
        }
        for (dbname, entry) in result.iter_mut() {
            if let Some(workspaces) = by_db.remove(dbname) {
                entry.used_by_workspaces = workspaces.into_iter().collect();
            }
        }
    }

    return Ok(Json(result));
}

async fn refresh_custom_instance_user_pwd(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
) -> JsonResult<()> {
    require_super_admin(&db, &authed).await?;
    windmill_common::utils::refresh_custom_instance_user_pwd(&db).await?;
    Ok(Json(()))
}

#[derive(Deserialize)]
struct SetupCustomInstanceDbBody {
    tag: Option<String>,
}

async fn setup_custom_instance_pg_database(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(dbname): Path<String>,
    Json(body): Json<SetupCustomInstanceDbBody>,
) -> JsonResult<CustomInstanceDb> {
    let mut logs = CustomInstanceDbLogs::default();
    let result = setup_custom_instance_pg_database_inner(authed, &db, &dbname, &mut logs).await;
    let success = result.is_ok();
    let error = result.err().map(|e| e.to_string());
    let status =
        CustomInstanceDb { logs, success, error, tag: body.tag, used_by_workspaces: vec![] };
    let status_json = serde_json::to_value(&status).map_err(to_anyhow)?;
    // Save that the database was setup successfully
    sqlx::query!(
        r#"UPDATE global_settings SET value = jsonb_set(value, '{databases}', (COALESCE(value->'databases', '{}'::jsonb) || to_jsonb($1::json))) WHERE name = 'custom_instance_pg_databases'"#,
        json!({ dbname: status_json })
    ).execute(&db).await?;

    Ok(Json(status))
}

async fn setup_custom_instance_pg_database_inner(
    authed: ApiAuthed,
    db: &DB,
    dbname: &str,
    logs: &mut CustomInstanceDbLogs,
) -> Result<()> {
    require_super_admin(db, &authed).await?;
    logs.super_admin = "OK".to_string();
    let wmill_pg_creds = PgDatabase::parse_uri(&get_database_url().await?.as_str().await)?;
    logs.database_credentials = "OK".to_string();

    // Validate name to ensure it only contains alphanumeric characters
    // Prevents SQL injection on the instance database
    lazy_static::lazy_static! {
        // Must start with a letter, then alphanumeric/underscore/hyphen
        static ref VALID_NAME: regex::Regex = regex::Regex::new(r"^[a-zA-Z][a-zA-Z0-9_-]*$").unwrap();
    }
    let dbname = dbname.trim();
    if dbname.is_empty() {
        return Err(error::Error::BadRequest(
            "Database name cannot be empty".to_string(),
        ));
    }
    // PostgreSQL identifier limit is 63 bytes
    if dbname.len() > 63 {
        return Err(error::Error::BadRequest(
            "Database name cannot exceed 63 characters".to_string(),
        ));
    }
    if !VALID_NAME.is_match(dbname) {
        return Err(error::Error::BadRequest(
            "Database name must start with a letter and contain only alphanumeric characters, underscores, or hyphens".to_string(),
        ));
    }
    // Additional check: block PostgreSQL reserved/special names
    let lower = dbname.to_lowercase();
    if lower == "template0" || lower == "template1" || lower == "postgres" {
        return Err(error::Error::BadRequest(
            "Cannot use reserved PostgreSQL database names".to_string(),
        ));
    }
    if wmill_pg_creds
        .dbname
        .trim()
        .eq_ignore_ascii_case(dbname.trim())
    {
        return Err(error::Error::BadRequest(
            "Database name cannot be the same as the main database".to_string(),
        ));
    }
    logs.valid_dbname = "OK".to_string();

    let db_exists = sqlx::query_scalar!(
        "SELECT EXISTS (SELECT 1 FROM pg_catalog.pg_database WHERE datname = $1)",
        dbname
    )
    .fetch_one(db)
    .await?
    .unwrap_or(false);

    let pg_creds = PgDatabase { dbname: dbname.to_string(), ..wmill_pg_creds };

    logs.created_database = "SKIP".to_string();
    if !db_exists {
        // SAFETY: `dbname` has been validated by the VALID_NAME regex and length checks above (lines 1088–1120).
        sqlx::query(&format!("CREATE DATABASE \"{dbname}\""))
            .execute(db)
            .await?;
        logs.created_database = "OK".to_string();
    }

    // We have to connect to the newly created database as admin to grant permissions
    let (client, connection) = pg_creds.connect(Some(db)).await?;
    let join_handle = tokio::spawn(async move { connection.await });

    logs.db_connect = "OK".to_string();

    // SAFETY: `dbname` has been validated by the VALID_NAME regex and length checks above.
    client
        .batch_execute(&format!(
            "GRANT CONNECT ON DATABASE \"{dbname}\" TO custom_instance_user;
             GRANT USAGE ON SCHEMA public TO custom_instance_user;
             GRANT CREATE ON SCHEMA public TO custom_instance_user;
             GRANT CREATE ON DATABASE \"{dbname}\" TO custom_instance_user;
             ALTER DEFAULT PRIVILEGES IN SCHEMA public
                 GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO custom_instance_user;
             ALTER ROLE custom_instance_user CREATEROLE;"
        ))
        .await
        .map_err(|e| {
            error::Error::ExecutionErr(format!(
                "Failed to grant permissions to custom_instance_user: {}",
                e.to_string(),
            ))
        })?;

    if let Err(e) = client
        .batch_execute(&format!("ALTER ROLE custom_instance_user REPLICATION;"))
        .await
    {
        tracing::error!("Failed to grant replication permission to custom_instance_user: {e:#}");
    }

    logs.grant_permissions = "OK".to_string();

    drop(client); // /!\ Drop before joining to avoid deadlock
    join_handle
        .await
        .map_err(|e| error::Error::ExecutionErr(format!("join error: {}", e.to_string())))?
        .map_err(|e| {
            error::Error::ExecutionErr(format!("tokio_postgres error: {}", e.to_string()))
        })?;

    Ok(())
}

async fn drop_custom_instance_pg_database(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(dbname): Path<String>,
) -> Result<String> {
    require_super_admin(&db, &authed).await?;

    windmill_common::drop_custom_instance_database(&db, &dbname).await?;

    Ok(format!("Database '{}' dropped successfully", dbname))
}

// ============================================================================
// Secret Backend Settings (HashiCorp Vault Integration) - Enterprise Edition
// ============================================================================

/// Test connection to a secret backend (HashiCorp Vault)
///
/// This endpoint validates that the Vault settings are correct and that
/// Windmill can successfully authenticate and communicate with Vault.
///
/// This is an Enterprise Edition feature.
#[cfg(all(feature = "private", feature = "enterprise"))]
pub async fn test_secret_backend(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Json(settings): Json<VaultSettings>,
) -> Result<String> {
    require_super_admin(&db, &authed).await?;

    windmill_common::secret_backend::test_vault_connection(&settings, Some(&db)).await?;

    Ok("Successfully connected to HashiCorp Vault".to_string())
}

/// Migrate existing secrets from database to HashiCorp Vault
///
/// This endpoint reads all encrypted secrets from the database, decrypts them,
/// and stores them in HashiCorp Vault. The database values are NOT deleted
/// automatically to allow for rollback if needed.
///
/// This is an Enterprise Edition feature.
#[cfg(all(feature = "private", feature = "enterprise"))]
pub async fn migrate_secrets_to_vault(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Json(settings): Json<VaultSettings>,
) -> JsonResult<SecretMigrationReport> {
    require_super_admin(&db, &authed).await?;

    let report = windmill_common::secret_backend::migrate_secrets_to_vault(&db, &settings).await?;

    Ok(Json(report))
}

/// Migrate secrets from HashiCorp Vault back to database
///
/// This endpoint reads all secrets from HashiCorp Vault, encrypts them using
/// the workspace encryption keys, and stores them in the database. The Vault
/// values are NOT deleted automatically to allow for rollback if needed.
///
/// This is an Enterprise Edition feature.
#[cfg(all(feature = "private", feature = "enterprise"))]
pub async fn migrate_secrets_to_database(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Json(settings): Json<VaultSettings>,
) -> JsonResult<SecretMigrationReport> {
    require_super_admin(&db, &authed).await?;

    let report =
        windmill_common::secret_backend::migrate_secrets_to_database(&db, &settings).await?;

    Ok(Json(report))
}

/// Test connection to Azure Key Vault
///
/// This is an Enterprise Edition feature.
#[cfg(all(feature = "private", feature = "enterprise"))]
pub async fn test_azure_kv_backend(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Json(settings): Json<AzureKeyVaultSettings>,
) -> Result<String> {
    require_super_admin(&db, &authed).await?;

    windmill_common::secret_backend::test_azure_kv_connection(&settings).await?;

    Ok("Successfully connected to Azure Key Vault".to_string())
}

/// Migrate existing secrets from database to Azure Key Vault
///
/// This is an Enterprise Edition feature.
#[cfg(all(feature = "private", feature = "enterprise"))]
pub async fn migrate_secrets_to_azure_kv(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Json(settings): Json<AzureKeyVaultSettings>,
) -> JsonResult<SecretMigrationReport> {
    require_super_admin(&db, &authed).await?;

    let report =
        windmill_common::secret_backend::migrate_secrets_to_azure_kv(&db, &settings).await?;

    Ok(Json(report))
}

/// Migrate secrets from Azure Key Vault back to database
///
/// This is an Enterprise Edition feature.
#[cfg(all(feature = "private", feature = "enterprise"))]
pub async fn migrate_secrets_from_azure_kv(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Json(settings): Json<AzureKeyVaultSettings>,
) -> JsonResult<SecretMigrationReport> {
    require_super_admin(&db, &authed).await?;

    let report =
        windmill_common::secret_backend::migrate_secrets_from_azure_kv(&db, &settings).await?;

    Ok(Json(report))
}

/// Test connection to AWS Secrets Manager
#[cfg(all(feature = "private", feature = "enterprise"))]
pub async fn test_aws_sm_backend(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Json(settings): Json<AwsSecretsManagerSettings>,
) -> Result<String> {
    require_super_admin(&db, &authed).await?;
    windmill_common::secret_backend::test_aws_sm_connection(&settings).await?;
    Ok("Successfully connected to AWS Secrets Manager".to_string())
}

/// Migrate existing secrets from database to AWS Secrets Manager
#[cfg(all(feature = "private", feature = "enterprise"))]
pub async fn migrate_secrets_to_aws_sm(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Json(settings): Json<AwsSecretsManagerSettings>,
) -> JsonResult<SecretMigrationReport> {
    require_super_admin(&db, &authed).await?;
    let report = windmill_common::secret_backend::migrate_secrets_to_aws_sm(&db, &settings).await?;
    Ok(Json(report))
}

/// Migrate secrets from AWS Secrets Manager back to database
#[cfg(all(feature = "private", feature = "enterprise"))]
pub async fn migrate_secrets_from_aws_sm(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Json(settings): Json<AwsSecretsManagerSettings>,
) -> JsonResult<SecretMigrationReport> {
    require_super_admin(&db, &authed).await?;
    let report =
        windmill_common::secret_backend::migrate_secrets_from_aws_sm(&db, &settings).await?;
    Ok(Json(report))
}

// ============================================================================
// JWKS Endpoint for Vault JWT Authentication
// ============================================================================

/// JSON Web Key Set response structure
#[derive(Serialize)]
pub struct JwksResponse {
    pub keys: Vec<serde_json::Value>,
}

/// Fallback JWKS endpoint used when OIDC support is not compiled in.
///
/// When built with `private` + `enterprise` + `openidconnect`, the route in
/// `windmill-api` dispatches to `oidc_oss::jwks` (re-exported from
/// `oidc_ee::jwks`) instead, which serves the actual public keys.
pub async fn get_jwks() -> JsonResult<JwksResponse> {
    Ok(Json(JwksResponse { keys: vec![] }))
}

#[derive(serde::Deserialize, serde::Serialize)]
struct CachedResourceType {
    #[allow(dead_code)]
    id: i64,
    name: String,
    schema: Option<serde_json::Value>,
    #[allow(dead_code)]
    app: String,
    description: Option<String>,
}

#[derive(serde::Deserialize)]
struct HubResourceTypeRaw {
    id: i64,
    name: String,
    schema: Option<String>,
    app: String,
    description: Option<String>,
}

async fn fetch_resource_types_from_hub() -> error::Result<Vec<CachedResourceType>> {
    let response = HTTP_CLIENT
        .get(format!(
            "{}/resource_types/list",
            windmill_common::DEFAULT_HUB_BASE_URL
        ))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| error::Error::InternalErr(format!("Failed to fetch from hub: {}", e)))?;

    if !response.status().is_success() {
        return Err(error::Error::InternalErr(format!(
            "Hub returned status {}",
            response.status()
        )));
    }

    let raw_types: Vec<HubResourceTypeRaw> = response
        .json()
        .await
        .map_err(|e| error::Error::InternalErr(format!("Failed to parse hub response: {}", e)))?;

    Ok(raw_types
        .into_iter()
        .filter_map(|rt| {
            let schema = match rt.schema {
                Some(s) => match serde_json::from_str(&s) {
                    Ok(v) => Some(v),
                    Err(_) => return None,
                },
                None => None,
            };
            Some(CachedResourceType {
                id: rt.id,
                name: rt.name,
                schema,
                app: rt.app,
                description: rt.description,
            })
        })
        .collect())
}

async fn sync_cached_resource_types(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
) -> error::Result<String> {
    require_super_admin(&db, &authed).await?;

    use windmill_common::worker::HUB_RT_CACHE_DIR;
    let cache_path = format!("{}/resource_types.json", *HUB_RT_CACHE_DIR);

    let cached_types = match tokio::fs::read_to_string(&cache_path).await {
        Ok(content) => serde_json::from_str::<Vec<CachedResourceType>>(&content).map_err(|e| {
            error::Error::InternalErr(format!("Failed to parse cached resource types: {}", e))
        })?,
        Err(_) => fetch_resource_types_from_hub().await?,
    };

    let mut synced_count = 0;

    for rt in &cached_types {
        let exists: Option<bool> = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM resource_type WHERE workspace_id = 'admins' AND name = $1 AND schema IS NOT DISTINCT FROM $2 AND description IS NOT DISTINCT FROM $3)",
            &rt.name,
            rt.schema.as_ref(),
            rt.description.as_deref(),
        )
        .fetch_one(&db)
        .await?;

        if exists.unwrap_or(false) {
            continue;
        }

        sqlx::query!(
            "INSERT INTO resource_type (workspace_id, name, schema, description, edited_at)
             VALUES ('admins', $1, $2, $3, now())
             ON CONFLICT (workspace_id, name) DO UPDATE
             SET schema = EXCLUDED.schema, description = EXCLUDED.description, edited_at = now()",
            &rt.name,
            rt.schema.as_ref(),
            rt.description.as_deref(),
        )
        .execute(&db)
        .await?;

        synced_count += 1;
    }

    Ok(format!(
        "Synced {} resource types ({} unchanged)",
        synced_count,
        cached_types.len() - synced_count
    ))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use windmill_common::instance_config::{GlobalSettings, InstanceConfig, WorkerGroupConfig};

    #[test]
    fn instance_config_yaml_round_trip() {
        let config = InstanceConfig {
            global_settings: GlobalSettings {
                base_url: Some("https://windmill.example.com".to_string()),
                retention_period_secs: Some(86400),
                expose_metrics: Some(true),
                ..Default::default()
            },
            worker_configs: BTreeMap::from([(
                "default".to_string(),
                WorkerGroupConfig {
                    worker_tags: Some(vec!["deno".to_string(), "python3".to_string()]),
                    init_bash: Some("apt-get update".to_string()),
                    ..Default::default()
                },
            )]),
        };

        let yaml = config.to_sorted_yaml().unwrap();

        // Verify key fields appear in the YAML output
        assert!(yaml.contains("base_url: https://windmill.example.com"));
        assert!(yaml.contains("retention_period_secs: 86400"));
        assert!(yaml.contains("expose_metrics: true"));
        assert!(yaml.contains("default:"));
        assert!(yaml.contains("- deno"));
        assert!(yaml.contains("- python3"));
        assert!(yaml.contains("init_bash: apt-get update"));

        // Round-trip back to struct
        let deserialized: InstanceConfig = serde_yml::from_str(&yaml).unwrap();
        assert_eq!(
            deserialized.global_settings.base_url.as_deref(),
            Some("https://windmill.example.com")
        );
        assert_eq!(
            deserialized.global_settings.retention_period_secs,
            Some(86400)
        );
        assert_eq!(deserialized.global_settings.expose_metrics, Some(true));
        let wc = &deserialized.worker_configs["default"];
        assert_eq!(
            wc.worker_tags.as_deref(),
            Some(["deno".to_string(), "python3".to_string()].as_slice())
        );
        assert_eq!(wc.init_bash.as_deref(), Some("apt-get update"));
    }

    #[test]
    fn sorted_yaml_global_settings_alphabetical() {
        let config = InstanceConfig {
            global_settings: GlobalSettings {
                retention_period_secs: Some(3600),
                base_url: Some("https://test.com".to_string()),
                expose_metrics: Some(true),
                email_domain: Some("example.com".to_string()),
                ..Default::default()
            },
            worker_configs: BTreeMap::new(),
        };

        let yaml = config.to_sorted_yaml().unwrap();

        // Keys must appear in alphabetical order
        let base_url_pos = yaml.find("base_url:").unwrap();
        let email_pos = yaml.find("email_domain:").unwrap();
        let expose_pos = yaml.find("expose_metrics:").unwrap();
        let retention_pos = yaml.find("retention_period_secs:").unwrap();

        assert!(
            base_url_pos < email_pos && email_pos < expose_pos && expose_pos < retention_pos,
            "global_settings keys should be alphabetically sorted, got yaml:\n{yaml}"
        );
    }

    #[test]
    fn sorted_yaml_worker_configs_default_and_native_first() {
        let config = InstanceConfig {
            global_settings: GlobalSettings::default(),
            worker_configs: BTreeMap::from([
                (
                    "gpu".to_string(),
                    WorkerGroupConfig {
                        init_bash: Some("echo gpu".to_string()),
                        ..Default::default()
                    },
                ),
                (
                    "native".to_string(),
                    WorkerGroupConfig {
                        init_bash: Some("echo native".to_string()),
                        ..Default::default()
                    },
                ),
                (
                    "default".to_string(),
                    WorkerGroupConfig {
                        init_bash: Some("echo default".to_string()),
                        ..Default::default()
                    },
                ),
                (
                    "alpha".to_string(),
                    WorkerGroupConfig {
                        init_bash: Some("echo alpha".to_string()),
                        ..Default::default()
                    },
                ),
            ]),
        };

        let yaml = config.to_sorted_yaml().unwrap();

        let default_pos = yaml.find("default:").unwrap();
        let native_pos = yaml.find("native:").unwrap();
        let alpha_pos = yaml.find("alpha:").unwrap();
        let gpu_pos = yaml.find("gpu:").unwrap();

        assert!(
            default_pos < native_pos
                && native_pos < alpha_pos
                && alpha_pos < gpu_pos,
            "worker_configs should have default, native first, then rest alphabetically, got yaml:\n{yaml}"
        );
    }

    #[test]
    fn sorted_yaml_roundtrips() {
        let config = InstanceConfig {
            global_settings: GlobalSettings {
                base_url: Some("https://rt.test".to_string()),
                retention_period_secs: Some(7200),
                expose_metrics: Some(false),
                ..Default::default()
            },
            worker_configs: BTreeMap::from([
                (
                    "default".to_string(),
                    WorkerGroupConfig {
                        worker_tags: Some(vec!["deno".to_string()]),
                        ..Default::default()
                    },
                ),
                (
                    "native".to_string(),
                    WorkerGroupConfig {
                        init_bash: Some("echo hi".to_string()),
                        ..Default::default()
                    },
                ),
            ]),
        };

        let yaml = config.to_sorted_yaml().unwrap();
        let deserialized: InstanceConfig = serde_yml::from_str(&yaml).unwrap();

        assert_eq!(
            deserialized.global_settings.base_url.as_deref(),
            Some("https://rt.test")
        );
        assert_eq!(
            deserialized.global_settings.retention_period_secs,
            Some(7200)
        );
        assert_eq!(deserialized.global_settings.expose_metrics, Some(false));
        assert_eq!(deserialized.worker_configs.len(), 2);
        assert_eq!(
            deserialized.worker_configs["default"]
                .worker_tags
                .as_deref(),
            Some(["deno".to_string()].as_slice())
        );
        assert_eq!(
            deserialized.worker_configs["native"].init_bash.as_deref(),
            Some("echo hi")
        );
    }
}

#[cfg(all(test, feature = "parquet"))]
mod object_storage_test_hardening {
    use super::{extract_host, is_forbidden_ip, validate_object_storage_test};
    use std::net::IpAddr;
    use windmill_object_store::ObjectSettings;

    // IP literals (not hostnames) keep validate_public_endpoint deterministic — `lookup_host`
    // parses them without any network round-trip.
    fn gcs_settings(gcs_base_url: &str) -> ObjectSettings {
        serde_json::from_value(serde_json::json!({
            "type": "Gcs",
            "bucket": "b",
            "serviceAccountKey": { "gcs_base_url": gcs_base_url, "client_email": "x@y.z" }
        }))
        .unwrap()
    }

    #[tokio::test]
    async fn rejects_gcs_internal_base_url() {
        // gcs_base_url in the service-account key must not smuggle an internal host past the check,
        // including via a mixed-case scheme (URL schemes are case-insensitive).
        for url in [
            "http://169.254.169.254",
            "HTTP://169.254.169.254",
            "Https://10.0.0.5",
        ] {
            assert!(
                validate_object_storage_test(&gcs_settings(url))
                    .await
                    .is_err(),
                "{url} should be rejected"
            );
        }
    }

    #[tokio::test]
    async fn allows_gcs_public_base_url() {
        assert!(
            validate_object_storage_test(&gcs_settings("https://8.8.8.8"))
                .await
                .is_ok()
        );
    }

    #[tokio::test]
    async fn rejects_gcs_blank_service_account_key() {
        // A blank key makes build_gcs_client fall back to the instance's ambient credentials, so an
        // untrusted caller must not be allowed to test with it. The `serviceAccountKey` field is
        // serialized via serde's `as_string` (`to_string` of the JSON value), so the settings UI's
        // "no key" empty object arrives as `"{}"` and a null as `"null"` — both must be rejected.
        for key in [serde_json::json!({}), serde_json::json!(null)] {
            let settings: ObjectSettings = serde_json::from_value(serde_json::json!({
                "type": "Gcs",
                "bucket": "b",
                "serviceAccountKey": key
            }))
            .unwrap();
            assert!(
                validate_object_storage_test(&settings).await.is_err(),
                "blank key {key:?} should be rejected"
            );
        }
    }

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    #[test]
    fn forbids_internal_ips() {
        for s in [
            "127.0.0.1",                // loopback
            "169.254.169.254",          // cloud metadata (link-local)
            "10.0.0.5",                 // private
            "172.16.3.4",               // private
            "192.168.1.10",             // private
            "0.0.0.0",                  // unspecified
            "100.64.0.1",               // CGNAT
            "::1",                      // IPv6 loopback
            "fe80::1",                  // IPv6 link-local
            "fc00::1",                  // IPv6 unique local
            "::ffff:127.0.0.1",         // IPv4-mapped loopback
            "::ffff:169.254.169.254",   // IPv4-mapped metadata
            "::169.254.169.254",        // IPv4-compatible metadata
            "64:ff9b::169.254.169.254", // NAT64-embedded metadata
            "64:ff9b::a9fe:a9fe",       // NAT64-embedded metadata (hex form)
        ] {
            assert!(is_forbidden_ip(ip(s)), "{s} should be forbidden");
        }
    }

    #[test]
    fn allows_public_ips() {
        for s in ["8.8.8.8", "1.1.1.1", "52.95.110.1", "2606:4700:4700::1111"] {
            assert!(!is_forbidden_ip(ip(s)), "{s} should be allowed");
        }
    }

    #[test]
    fn extracts_host_from_endpoint() {
        let cases = [
            ("s3.amazonaws.com", Some("s3.amazonaws.com")),
            ("https://minio.internal:9000", Some("minio.internal")),
            ("http://10.0.0.5:9000/bucket", Some("10.0.0.5")),
            ("user:pass@host.example:443", Some("host.example")),
            ("[::1]:9000", Some("::1")),
            ("https://[fe80::1]/x", Some("fe80::1")),
            ("", None),
            // Injection via region/bucket interpolation into the default endpoint string: the
            // userinfo `@` and the path `/` must not hide the real authority from the host check.
            (
                "https://s3.@169.254.169.254/.amazonaws.com",
                Some("169.254.169.254"),
            ),
            (
                "https://@169.254.169.254/mybucket.s3.amazonaws.com",
                Some("169.254.169.254"),
            ),
            ("s3.#@169.254.169.254/x.amazonaws.com", Some("s3.")),
            // Scheme is case-insensitive.
            ("HTTP://169.254.169.254", Some("169.254.169.254")),
        ];
        for (input, expected) in cases {
            assert_eq!(extract_host(input).as_deref(), expected, "input: {input}");
        }
    }
}
