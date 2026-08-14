/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Used to determine the internet address that connections from workers will appear to come from.
//!
//! For users writing scripts to access their infrastructure with firewalls requiring incoming
//! connections to be from whitelisted IP addresses.

use crate::utils::configure_client;
use std::sync::OnceLock;
use std::time::Duration;

/// Reported once the lookup has definitively failed, so that a worker reclaiming another's
/// `worker_ping` row stops advertising an address nothing has confirmed. Matches the column
/// default, and the frontend filters it out of the addresses it offers for whitelisting.
pub const UNKNOWN_IP: &str = "NO IP";

/// `worker_ping.ip` is `VARCHAR(50)`, and the initial ping is a hard failure for the worker, so
/// an overlong value must be rejected as config rather than reaching the insert.
const MAX_IP_LEN: usize = 50;

lazy_static::lazy_static! {
    /// Skips the hub lookup entirely, for deployments that already know their egress address or
    /// whose egress is blocked.
    static ref WORKER_EXTERNAL_IP: Option<String> = std::env::var("WORKER_EXTERNAL_IP")
        .ok()
        .filter(|ip| !ip.is_empty())
        .filter(|ip| {
            let ok = ip.len() <= MAX_IP_LEN;
            if !ok {
                tracing::error!("WORKER_EXTERNAL_IP is longer than {MAX_IP_LEN} chars, ignoring it");
            }
            ok
        });
}

static EXTERNAL_IP: OnceLock<String> = OnceLock::new();

/// The external IP of this process, [`UNKNOWN_IP`] once the lookup has failed, or `None` while it
/// is still in flight.
pub fn cached_ip() -> Option<&'static str> {
    EXTERNAL_IP.get().map(String::as_str)
}

/// Resolves the external IP into the process-wide cache without blocking the caller. The value is
/// informational, and behind a firewall the lookup burns its whole 5s connect timeout on every
/// process start, so nothing on the worker startup path may wait on it.
pub fn resolve_ip_in_background() {
    if let Some(ip) = WORKER_EXTERNAL_IP.as_ref() {
        let _ = EXTERNAL_IP.set(ip.clone());
        return;
    }
    tokio::spawn(async {
        let ip = get_ip()
            .await
            .map(|ip| {
                if ip.len() > MAX_IP_LEN {
                    tracing::error!("external IP lookup returned an overlong value, ignoring it");
                    UNKNOWN_IP.to_string()
                } else {
                    ip
                }
            })
            .unwrap_or_else(|e| {
                tracing::warn!(
                    error = e.to_string(),
                    "failed to get external IP, workers of this process will report no IP"
                );
                UNKNOWN_IP.to_string()
            });
        let _ = EXTERNAL_IP.set(ip);
    });
}

pub async fn get_ip() -> anyhow::Result<String> {
    tokio::select! {
        biased;
        _ = tokio::time::sleep(Duration::from_secs(10)) => {
            return Err(anyhow::anyhow!("Expected to get ip under 10s"))
        },
        ip = configure_client(reqwest::ClientBuilder::new()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(5)))
        .build()?
        .get("https://hub.windmill.dev/getip")
        .send() => Ok(ip?
            .error_for_status()?
            .text().await?),
    }
}
