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

use reqwest::Result;

pub async fn get_ip() -> Result<String> {
    reqwest::ClientBuilder::new()
        .timeout(Duration::from_secs(3))
        .build()?
        .get("https://hub.windmill.dev/getip")
        .send()
        .await?
        .error_for_status()?
        .text()
        .await
}
