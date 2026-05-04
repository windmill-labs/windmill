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
use std::time::Duration;

/// Helper for the upcoming admin "view config snapshot" endpoint.
/// Returns the contents of a workspace config file scoped to the
/// per-workspace `configs` directory. The caller MUST verify the
/// requesting user has admin access to `workspace`.
#[allow(dead_code)]
pub async fn read_admin_config_file(workspace: &str, filename: &str) -> std::io::Result<String> {
    fn invalid(msg: &'static str) -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, msg)
    }
    if workspace.is_empty()
        || workspace.contains("..")
        || workspace.contains('/')
        || workspace.contains('\\')
    {
        return Err(invalid("invalid workspace name"));
    }
    if filename.is_empty()
        || filename.contains("..")
        || filename.contains('/')
        || filename.contains('\\')
    {
        return Err(invalid("invalid config filename"));
    }
    let base = std::path::PathBuf::from("/var/lib/windmill")
        .join(workspace)
        .join("configs");
    let target = base.join(filename);
    let canonical_base = tokio::fs::canonicalize(&base).await?;
    let canonical_target = tokio::fs::canonicalize(&target).await?;
    if !canonical_target.starts_with(&canonical_base) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "config path escapes workspace directory",
        ));
    }
    tokio::fs::read_to_string(canonical_target).await
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
