/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

pub mod asset_dispatch;
#[cfg(feature = "private")]
pub mod cascade_ee;
pub mod cascade_oss;
#[cfg(feature = "private")]
pub use cascade_ee as cascade;
#[cfg(not(feature = "private"))]
pub use cascade_oss as cascade;
#[cfg(feature = "private")]
pub mod ducklake_maintenance_ee;
pub mod ducklake_maintenance_oss;
#[cfg(feature = "private")]
pub use ducklake_maintenance_ee as ducklake_maintenance;
#[cfg(not(feature = "private"))]
pub use ducklake_maintenance_oss as ducklake_maintenance;
#[cfg(feature = "private")]
pub mod freshness_watchdog_ee;
pub mod freshness_watchdog_oss;
#[cfg(feature = "private")]
pub use freshness_watchdog_ee as freshness_watchdog;
#[cfg(not(feature = "private"))]
pub use freshness_watchdog_oss as freshness_watchdog;
pub mod jobs;
#[cfg(feature = "private")]
pub mod jobs_ee;
pub mod jobs_oss;
pub mod schedule;
pub use jobs::*;
pub mod flow_status;
pub mod tags;
pub mod workspace_fairness;
#[cfg(feature = "private")]
pub mod workspace_fairness_ee;

#[cfg(feature = "cloud")]
pub mod cloud_usage;
#[cfg(feature = "cloud")]
pub use cloud_usage::init_usage_buffer;
