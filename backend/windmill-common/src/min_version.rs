use crate::error::{self, Error};
use semver::Version;
use std::sync::Arc;
use tokio::sync::RwLock;

// ============ Feature Definitions ============

pub const MIN_VERSION_SUPPORTS_SYNC_JOBS_DEBOUNCING: VC = vc(1, 602, 0, "Sync jobs debouncing");
pub const MIN_VERSION_SUPPORTS_DEBOUNCING_V2: VC = vc(1, 597, 0, "Debouncing V2");
pub const MIN_VERSION_IS_AT_LEAST_1_595: VC = vc(1, 595, 0, "Flow status separate table");
pub const MIN_VERSION_SUPPORTS_RUNNABLE_SETTINGS_V0: VC = vc(1, 592, 0, "Runnable settings V0");
pub const MIN_VERSION_SUPPORTS_V0_WORKSPACE_DEPENDENCIES: VC = vc(1, 587, 0, "Workspace dependencies");
pub const MIN_VERSION_SUPPORTS_DEBOUNCING: VC = vc(1, 566, 0, "Debouncing");
pub const MIN_VERSION_IS_AT_LEAST_1_461: VC = vc(1, 461, 0, "V2 job tables");
pub const MIN_VERSION_IS_AT_LEAST_1_440: VC = vc(1, 440, 0, "Flow node value on pull");
pub const MIN_VERSION_IS_AT_LEAST_1_432: VC = vc(1, 432, 0, "Flow script job kind");
pub const MIN_VERSION_IS_AT_LEAST_1_427: VC = vc(1, 427, 0, "Flow version lite table");

// NOTE: Workers must NOT use LOCAL_MIN_KEEP_ALIVE_VERSION directly.
// They should call the server API: GET /api/settings/min_keep_alive_version
//
// TODO: Currently only shows warning in frontend. In the future,
// workers below this version should be terminated automatically.

/// Authoritative min keep-alive version defined in this codebase.
/// Served via: GET /api/settings/min_keep_alive_version
/// Also used by vc() for compile-time checks.
pub const LOCAL_MIN_KEEP_ALIVE_VERSION: (u64, u64, u64) = (1, 0, 0);

// ============ Implementation ============
lazy_static::lazy_static! {
    // Global minimum version across all workers (for feature flags)
    pub static ref MIN_VERSION: Arc<RwLock<Version>> = Arc::new(RwLock::new(Version::new(0, 0, 0)));

    /// Min keep-alive version fetched from server by workers.
    pub static ref REMOTE_MIN_KEEP_ALIVE_VERSION: Arc<RwLock<Version>> = Arc::new(RwLock::new(Version::new(0, 0, 0)));
}

/// Creates a VersionConstraint with compile-time assertion that version > LOCAL_MIN_KEEP_ALIVE_VERSION.
/// When LOCAL_MIN_KEEP_ALIVE_VERSION is raised, obsolete constraints will fail to compile.
pub const fn vc(major: u64, minor: u64, patch: u64, name: &'static str) -> VersionConstraint {
    let is_greater = major > LOCAL_MIN_KEEP_ALIVE_VERSION.0
        || (major == LOCAL_MIN_KEEP_ALIVE_VERSION.0 && minor > LOCAL_MIN_KEEP_ALIVE_VERSION.1)
        || (major == LOCAL_MIN_KEEP_ALIVE_VERSION.0 && minor == LOCAL_MIN_KEEP_ALIVE_VERSION.1 && patch > LOCAL_MIN_KEEP_ALIVE_VERSION.2);
    assert!(
        is_greater,
        "Feature version must be > LOCAL_MIN_KEEP_ALIVE_VERSION. Remove this obsolete constraint."
    );
    VersionConstraint { available_since: Version::new(major, minor, patch), name }
}

pub type VC = VersionConstraint;

#[derive(Clone)]
pub struct VersionConstraint {
    pub available_since: Version,
    pub name: &'static str,
}

impl VersionConstraint {
    pub async fn met(&self) -> bool {
        &*MIN_VERSION.read().await <= &self.available_since
    }

    pub async fn assert(&self) -> error::Result<()> {
        if self.met().await {
            Ok(())
        } else {
            Err(Error::WorkersAreBehind {
                feature: self.name.to_string(),
                min_version: self.available_since.to_string(),
            })
        }
    }
}


// ============ Version Management ============

use crate::worker::{Connection, HttpClient};
use crate::utils::{GIT_SEM_VERSION, GIT_VERSION};

/// Fetches the minimum version across all workers.
// TODO: consider using HTTP for everything instead of Connection enum
pub async fn get_min_version(conn: &Connection) -> error::Result<Version> {
    let fetched = match conn {
        Connection::Sql(pool) => {
            let pings = sqlx::query_scalar!(
                "SELECT wm_version FROM worker_ping WHERE wm_version != $1 AND ping_at > now() - interval '5 minutes'",
                GIT_VERSION
            ).fetch_all(pool).await?;

            pings
                .iter()
                .filter(|x| !x.is_empty())
                .filter_map(|x| {
                    Version::parse(if x.starts_with('v') { &x[1..] } else { x }).ok()
                })
                .min()
        }
        Connection::Http(client) => {
            Some(
                client
                    .get::<String>("/api/agent_workers/get_min_version")
                    .await
                    .map(|v| Version::parse(&v))??,
            )
        }
    };

    Ok(fetched.unwrap_or_else(|| GIT_SEM_VERSION.clone()))
}

/// Server-side: Fetches and updates MIN_VERSION only.
pub async fn update_min_version(conn: &Connection) {
    match get_min_version(conn).await {
        Ok(ref mut v) => {
            let cur = GIT_SEM_VERSION.clone();
            if v != &cur {
                tracing::info!("Minimal worker version: {v}");
            }
            v.pre = semver::Prerelease::EMPTY;
            v.build = semver::BuildMetadata::EMPTY;
            *MIN_VERSION.write().await = v.clone();
        }
        Err(e) => tracing::error!("Failed to fetch min version: {:#?}", e),
    }
}

/// Worker-side: Fetches and updates MIN_VERSION and REMOTE_MIN_KEEP_ALIVE_VERSION.
pub async fn handle_min_versions(conn: &Connection, client: &HttpClient) {
    update_min_version(conn).await;

    // Update REMOTE_MIN_KEEP_ALIVE_VERSION
    match client.get::<String>("/api/settings/min_keep_alive_version").await {
        Ok(v) => match Version::parse(&v) {
            Ok(v) => *REMOTE_MIN_KEEP_ALIVE_VERSION.write().await = v,
            Err(e) => tracing::error!("Failed to parse min keep-alive version: {:#?}", e),
        },
        Err(e) => tracing::error!("Failed to fetch min keep-alive version: {:#?}", e),
    }
}
