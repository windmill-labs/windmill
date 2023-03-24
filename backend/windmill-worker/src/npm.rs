/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::fs::{self};

// TEMPORARY CODE NEEDED TO REFRESH NPM PACKAGE INFO REGISTRIES BECAUSE DENO DOES NOT DO IT FOR NOW

use anyhow::Result;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use windmill_common::error::to_anyhow;
use windmill_queue::HTTP_CLIENT;

use crate::DENO_CACHE_DIR;

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct NpmName {
    pub name: String,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct NpmPackageInfo {
    pub name: String,
    pub versions: HashMap<String, NpmPackageVersionInfo>,
    #[serde(rename = "dist-tags")]
    pub dist_tags: HashMap<String, String>,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct NpmPeerDependencyMeta {
    #[serde(default)]
    optional: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(untagged)]
pub enum NpmPackageVersionBinEntry {
    String(String),
    Map(HashMap<String, String>),
}

#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NpmPackageVersionInfo {
    pub version: String,
    pub dist: NpmPackageVersionDistInfo,
    pub bin: Option<NpmPackageVersionBinEntry>,
    // Bare specifier to version (ex. `"typescript": "^3.0.1") or possibly
    // package and version (ex. `"typescript-3.0.1": "npm:typescript@3.0.1"`).
    #[serde(default)]
    pub dependencies: HashMap<String, String>,
    #[serde(default)]
    pub peer_dependencies: HashMap<String, String>,
    #[serde(default)]
    pub peer_dependencies_meta: HashMap<String, NpmPeerDependencyMeta>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NpmPackageVersionDistInfo {
    /// URL to the tarball.
    pub tarball: String,
    shasum: String,
    integrity: Option<String>,
}

const FIFTEEN_MINUTES: u64 = 60 * 60 * 15;
pub async fn refresh_registries() -> Result<()> {
    let root = format!("{DENO_CACHE_DIR}/npm");
    let metadata = fs::metadata(&root);
    let root_exists = metadata.is_ok();
    tracing::info!(
        "Refreshing registries in {:#?}",
        metadata
            .and_then(|x| x.modified())
            .map_err(to_anyhow)
            .and_then(|x| x
                .elapsed()
                .map(|x| x.as_secs() > FIFTEEN_MINUTES)
                .map_err(to_anyhow))
            .unwrap_or(true)
    );
    if !root_exists {
        return Ok(());
    }
    let r = fs::read_dir(root)
        .unwrap()
        .map_ok(|registry| {
            tracing::info!("Refreshing registry {}", registry.path().display());
            let rpath = registry.path();
            let registry_path = rpath.to_str().unwrap();
            let registry = registry
                .path()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string();
            fs::read_dir(registry_path)
                .unwrap()
                .map_ok(|package_info| {
                    let pinfo = package_info.path();
                    let package = pinfo.file_name().unwrap().to_str().unwrap();
                    let registry = registry.clone();
                    let registry_file = format!("{registry_path}/{package}/registry.json");
                    replace_registry(registry_file, registry)
                })
                .flatten()
                .collect::<Vec<_>>()
                .into_iter()
        })
        .flatten()
        .into_iter()
        .flatten();

    //this is important to parallelize all registry fetching
    futures::future::join_all(r).await;

    Ok(())
}

async fn replace_registry(registry_file: String, registry: String) -> anyhow::Result<()> {
    let rf_str = tokio::fs::read_to_string(&registry_file).await?;
    let name = serde_json::from_str::<NpmName>(&rf_str)?.name;
    let r = HTTP_CLIENT
        .get(format!("https://{registry}/{name}"))
        .header("Accept", "application/json")
        .send()
        .await?
        .json::<NpmPackageInfo>()
        .await?;

    tokio::fs::write(registry_file, serde_json::to_string(&r)?).await?;

    Ok(())
}
