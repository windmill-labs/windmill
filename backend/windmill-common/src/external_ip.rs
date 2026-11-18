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

/// No address has ever been established for the row. Matches the `worker_ping.ip` column default,
/// and doubles as what an agent sends while its lookup is in flight, since a server that predates
/// the lookup being asynchronous rejects an initial ping carrying nothing.
pub const UNKNOWN_IP: &str = "NO IP";

/// The lookup ran and could not produce an address. Distinct from [`UNKNOWN_IP`] because it tells
/// an operator the difference between "never asked" and "asked, and this instance cannot reach the
/// hub", which is the actionable one. Both are filtered out of the addresses the frontend offers
/// for whitelisting.
pub const UNRETRIEVABLE_IP: &str = "unretrievable IP";

/// `worker_ping.ip` is `VARCHAR(50)`, and a failed initial ping takes the worker down, so an
/// overlong value must not reach the insert.
const MAX_IP_LEN: usize = 50;

static EXTERNAL_IP: OnceLock<String> = OnceLock::new();

/// The external IP of this process, [`UNRETRIEVABLE_IP`] once the lookup has failed, or `None`
/// while it is still in flight.
pub fn cached_ip() -> Option<&'static str> {
    EXTERNAL_IP.get().map(String::as_str)
}

/// Resolves the external IP into the process-wide cache without blocking the caller. The value is
/// informational, and behind a firewall the lookup burns its whole 5s connect timeout on every
/// process start, so nothing on the worker startup path may wait on it.
pub fn resolve_ip_in_background() {
    tokio::spawn(async {
        let ip = get_ip()
            .await
            .map(|ip| {
                if ip.len() > MAX_IP_LEN {
                    tracing::error!("external IP lookup returned an overlong value, ignoring it");
                    UNRETRIEVABLE_IP.to_string()
                } else {
                    ip
                }
            })
            .unwrap_or_else(|e| {
                tracing::warn!(
                    error = e.to_string(),
                    "failed to get external IP, workers of this process will report it as unretrievable"
                );
                UNRETRIEVABLE_IP.to_string()
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
