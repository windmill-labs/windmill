use crate::error::{self, Error};
use semver::Version;
use std::sync::Arc;
use tokio::sync::RwLock;

// ============ Feature Definitions ============

pub const MIN_VERSION_SUPPORTS_SYNC_JOBS_DEBOUNCING: VC = vc(1, 602, 0, "Sync jobs debouncing");
pub const MIN_VERSION_SUPPORTS_DEBOUNCING_V2: VC = vc(1, 597, 0, "Debouncing V2");
pub const MIN_VERSION_IS_AT_LEAST_1_595: VC = vc(1, 595, 0, "Flow status separate table");
pub const MIN_VERSION_SUPPORTS_RUNNABLE_SETTINGS_V0: VC = vc(1, 592, 0, "Runnable settings V0");
pub const MIN_VERSION_SUPPORTS_V0_WORKSPACE_DEPENDENCIES: VC =
    vc(1, 587, 0, "Workspace dependencies");
pub const MIN_VERSION_SUPPORTS_DEBOUNCING: VC = vc(1, 566, 0, "Debouncing");
pub const MIN_VERSION_IS_AT_LEAST_1_440: VC = vc(1, 440, 0, "Flow node value on pull");
pub const MIN_VERSION_IS_AT_LEAST_1_432: VC = vc(1, 432, 0, "Flow script job kind");
pub const MIN_VERSION_IS_AT_LEAST_1_427: VC = vc(1, 427, 0, "Flow version lite table");

// TODO: Currently only shows warning in frontend. In the future,
// workers below this version should be terminated automatically.

/// Minimum version workers must have to stay connected.
/// Served via: GET /api/min_keep_alive_version (returns { worker, agent })
/// Also used by vc() for compile-time checks.
pub const MIN_KEEP_ALIVE_VERSION: (u64, u64, u64) = (1, 420, 0);

/// Minimum version agent workers must have to stay connected.
/// Served via: GET /api/min_keep_alive_version (returns { worker, agent })
pub const AGENT_MIN_KEEP_ALIVE_VERSION: (u64, u64, u64) = (1, 0, 0);

// Compile-time check: MIN_KEEP_ALIVE_VERSION must lag at least 50 minor versions behind current,
// AGENT_MIN_KEEP_ALIVE_VERSION must lag at least 100 minor versions behind current.
// NOTE: These version lags are constants and should NEVER be changed. If this check
// fails, wait until enough versions have passed rather than reducing the lag requirement.
// Skip check if GIT_VERSION is "unknown-version" (no git tags available during build)
const _: () = assert!(
    !const_str::contains!(crate::utils::GIT_VERSION, ".")
        || (const_str::parse!(const_str::split!(crate::utils::GIT_VERSION, ".")[1], u64)
            - MIN_KEEP_ALIVE_VERSION.1
            >= 50
            && const_str::parse!(const_str::split!(crate::utils::GIT_VERSION, ".")[1], u64)
                - AGENT_MIN_KEEP_ALIVE_VERSION.1
                >= 100)
);

// ============ Implementation ============
lazy_static::lazy_static! {
    // Global minimum version across all workers (for feature flags)
    pub static ref MIN_VERSION: Arc<RwLock<Version>> = Arc::new(RwLock::new(Version::new(0, 0, 0)));
}

/// Creates a VersionConstraint with compile-time assertion that version > MIN_KEEP_ALIVE_VERSION.
/// When MIN_KEEP_ALIVE_VERSION is raised, obsolete constraints will fail to compile.
pub const fn vc(major: u64, minor: u64, patch: u64, name: &'static str) -> VersionConstraint {
    let is_greater = major > MIN_KEEP_ALIVE_VERSION.0
        || (major == MIN_KEEP_ALIVE_VERSION.0 && minor > MIN_KEEP_ALIVE_VERSION.1)
        || (major == MIN_KEEP_ALIVE_VERSION.0
            && minor == MIN_KEEP_ALIVE_VERSION.1
            && patch > MIN_KEEP_ALIVE_VERSION.2);
    assert!(
        is_greater,
        "Feature version must be > MIN_KEEP_ALIVE_VERSION. Remove this obsolete constraint."
    );
    VersionConstraint { available_since: Version::new(major, minor, patch), name }
}

pub type VC = VersionConstraint;

#[derive(Clone)]
pub struct VersionConstraint {
    available_since: Version,
    name: &'static str,
}

impl VersionConstraint {
    pub fn version(&self) -> &Version {
        &self.available_since
    }

    pub async fn met(&self) -> bool {
        let min = MIN_VERSION.read().await;
        // If MIN_VERSION is 0.0.0, it hasn't been set yet - assume met
        if *min == Version::new(0, 0, 0) {
            tracing::warn!(
                "MIN_VERSION not set yet, assuming feature '{}' is met",
                self.name
            );
            return true;
        }
        &self.available_since <= &*min
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

use crate::utils::{GIT_SEM_VERSION, GIT_VERSION};
use crate::worker::Connection;

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
                .filter_map(|x| Version::parse(if x.starts_with('v') { &x[1..] } else { x }).ok())
                .min()
        }
        Connection::Http(client) => Some(
            client
                .get::<String>("/api/agent_workers/get_min_version")
                .await
                .map(|v| Version::parse(&v))??,
        ),
    };

    Ok(fetched.unwrap_or_else(|| GIT_SEM_VERSION.clone()))
}

/// Updates MIN_VERSION and optionally checks min keep-alive version for workers.
/// If `_worker_mode` is true, checks min keep-alive version and sends critical alerts.
/// If `initial_load` is true, skips the min keep-alive check (server may not be ready).
pub async fn update_min_version(
    conn: &Connection,
    _worker_mode: bool,
    _worker_names: Vec<String>,
    _initial_load: bool,
) {
    // Update MIN_VERSION
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

    // Workers check min keep-alive version and send critical alerts
    // Skip on initial_load since the server may not be ready yet
    #[cfg(all(feature = "enterprise", feature = "private"))]
    if _worker_mode && !_initial_load {
        let min_keep_alive_version: Option<Version> = match conn {
            Connection::Sql(db) => {
                match crate::global_settings::load_value_from_global_settings(
                    db,
                    crate::global_settings::MIN_KEEP_ALIVE_VERSION_SETTING,
                )
                .await
                {
                    Ok(Some(v)) => v.as_str().and_then(|s| Version::parse(s).ok()),
                    Ok(None) => None,
                    Err(e) => {
                        tracing::error!(
                            "Failed to load min keep-alive version from global_settings: {:#?}",
                            e
                        );
                        None
                    }
                }
            }
            Connection::Http(client) => {
                match client
                    .get::<serde_json::Value>("/api/min_keep_alive_version")
                    .await
                {
                    Ok(resp) => resp
                        .get("agent")
                        .and_then(|v| v.as_str())
                        .and_then(|v| Version::parse(v).ok()),
                    Err(e) => {
                        tracing::error!("Failed to fetch min keep-alive version: {:#?}", e);
                        None
                    }
                }
            }
        };

        if let Some(min_keep_alive) = min_keep_alive_version {
            let current = GIT_SEM_VERSION.clone();
            match conn {
                Connection::Sql(db) => {
                    for worker_name in &_worker_names {
                        crate::ee::simple_alert_helper(
                            format!("Worker {worker_name} version {current} is below minimum keep-alive version {min_keep_alive}. Upgrade immediately."),
                            format!("Worker {worker_name} version {current} is now at or above minimum keep-alive version {min_keep_alive}."),
                            &format!("worker-below-min-keep-alive-{worker_name}"),
                            || current < min_keep_alive,
                            Some("admins"),
                            db,
                        ).await;
                    }
                }
                Connection::Http(_) => {
                    if current < min_keep_alive {
                        tracing::warn!(
                            "Agent worker version {current} is below minimum keep-alive version {min_keep_alive}. Upgrade immediately."
                        );
                    }
                }
            }
        }
    }
}

/// Stores the min keep-alive version in global_settings.
/// Called by server on startup, NOT by workers.
pub async fn store_min_keep_alive_version(db: &sqlx::Pool<sqlx::Postgres>) {
    let version = format!(
        "{}.{}.{}",
        MIN_KEEP_ALIVE_VERSION.0, MIN_KEEP_ALIVE_VERSION.1, MIN_KEEP_ALIVE_VERSION.2
    );
    if let Err(e) = crate::global_settings::set_value_in_global_settings(
        db,
        crate::global_settings::MIN_KEEP_ALIVE_VERSION_SETTING,
        serde_json::json!(version),
    )
    .await
    {
        tracing::error!(
            "Failed to store min keep-alive version in global_settings: {:#?}",
            e
        );
    }
}
