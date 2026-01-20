/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use crate::error::{self, Error};
use semver::Version;
use std::sync::Arc;
use tokio::sync::RwLock;

lazy_static::lazy_static! {
    // ============ Feature Definitions ============

    pub static ref MIN_VERSION_SUPPORTS_SYNC_JOBS_DEBOUNCING: VC = vc!((1, 602, 0), "Sync jobs debouncing");
    pub static ref MIN_VERSION_SUPPORTS_DEBOUNCING_V2: VC = vc!((1, 597, 0), "Debouncing V2");
    pub static ref MIN_VERSION_IS_AT_LEAST_1_595: VC = vc!((1, 595, 0), "Flow status separate table");
    pub static ref MIN_VERSION_SUPPORTS_RUNNABLE_SETTINGS_V0: VC = vc!((1, 592, 0), "Runnable settings V0");
    pub static ref MIN_VERSION_SUPPORTS_V0_WORKSPACE_DEPENDENCIES: VC = vc!((1, 587, 0), "Workspace dependencies");
    pub static ref MIN_VERSION_SUPPORTS_DEBOUNCING: VC = vc!((1, 566, 0), "Debouncing");
    pub static ref MIN_VERSION_IS_AT_LEAST_1_461: VC = vc!((1, 461, 0), "V2 job tables");
    pub static ref MIN_VERSION_IS_AT_LEAST_1_440: VC = vc!((1, 440, 0), "Flow node value on pull");
    pub static ref MIN_VERSION_IS_AT_LEAST_1_432: VC = vc!((1, 432, 0), "Flow script job kind");
    pub static ref MIN_VERSION_IS_AT_LEAST_1_427: VC = vc!((1, 427, 0), "Flow version lite table");

    // Global minimum version across all workers (for feature flags)
    pub static ref MIN_VERSION: Arc<RwLock<Version>> = Arc::new(RwLock::new(Version::new(0, 0, 0)));

    /// Min keep-alive version fetched from server by workers.
    pub static ref SERVER_MIN_KEEP_ALIVE_VERSION: Arc<RwLock<Version>> = Arc::new(RwLock::new(Version::new(0, 0, 0)));
}

// ============ Minimum Keep-Alive Version ============
//
// NOTE: Workers must NOT use LOCAL_MIN_KEEP_ALIVE_VERSION directly.
// They should call the server API: GET /api/settings/min_keep_alive_version
//
// TODO: Currently only shows warning in frontend. In the future,
// workers below this version should be terminated automatically.

/// Authoritative min keep-alive version defined in this codebase.
/// Served via: GET /api/settings/min_keep_alive_version
/// Also used by vc! macro for compile-time checks.
pub const LOCAL_MIN_KEEP_ALIVE_VERSION: Version = Version::new(0, 0, 0);

// ============ Implementation ============

pub type VC = VersionConstraint;

#[derive(Clone)]
pub struct VersionConstraint {
    pub available_since: Version,
    pub name: &'static str,
}

impl VersionConstraint {
    pub async fn is_met(&self) -> bool {
        !version_greater_than(*MIN_VERSION.read().await, self.available_since)
    }

    pub async fn assert(&self) -> error::Result<()> {
        if self.is_met().await {
            Ok(())
        } else {
            Err(Error::WorkersAreBehind {
                feature: self.name.to_string(),
                min_version: self.available_since.to_string(),
            })
        }
    }
}

/// Compares versions ignoring pre-release/build metadata. Returns true if a > b.
pub const fn version_greater_than(a: Version, b: Version) -> bool {
    a.major > b.major
        || (a.major == b.major && a.minor > b.minor)
        || (a.major == b.major && a.minor == b.minor && a.patch > b.patch)
}

/// Creates a VersionConstraint with compile-time assertion that version > LOCAL_MIN_KEEP_ALIVE_VERSION.
/// When LOCAL_MIN_KEEP_ALIVE_VERSION is raised, obsolete constraints will fail to compile.
macro_rules! vc {
    (($major:expr, $minor:expr, $patch:expr), $name:expr) => {{
        const V: Version = Version::new($major, $minor, $patch);
        const _: () = assert!(
            version_greater_than(V, LOCAL_MIN_KEEP_ALIVE_VERSION),
            "Feature version must be > LOCAL_MIN_KEEP_ALIVE_VERSION. Remove this obsolete constraint."
        );
        VersionConstraint {
            available_since: V,
            name: $name,
        }
    }};
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

/// Fetches and updates MIN_VERSION and SERVER_MIN_KEEP_ALIVE_VERSION.
pub async fn handle_min_versions(conn: &Connection, client: &HttpClient) {
    // Update MIN_VERSION
    match get_min_version(conn).await {
        Ok(v) => {
            let cur = GIT_SEM_VERSION.clone();
            if v != cur {
                tracing::info!("Minimal worker version: {v}");
            }
            *MIN_VERSION.write().await = v;
        }
        Err(e) => tracing::error!("Failed to fetch min version: {:#?}", e),
    }

    // Update SERVER_MIN_KEEP_ALIVE_VERSION
    match client.get::<String>("/api/settings/min_keep_alive_version").await {
        Ok(v) => match Version::parse(&v) {
            Ok(v) => *SERVER_MIN_KEEP_ALIVE_VERSION.write().await = v,
            Err(e) => tracing::error!("Failed to parse min keep-alive version: {:#?}", e),
        },
        Err(e) => tracing::error!("Failed to fetch min keep-alive version: {:#?}", e),
    }
}
