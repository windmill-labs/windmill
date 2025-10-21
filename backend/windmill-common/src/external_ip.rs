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

use std::time::Duration;
use crate::utils::configure_client;

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
