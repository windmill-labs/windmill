//! Deciding whether a script deployment should also compile the script's binary ahead of
//! its first run, and pushing it to the instance object store.
//!
//! A deploy reaches this from one of two places — the dependency job that generated the
//! lock, or, when the caller supplied one (CLI, git-sync), the create-script handler that
//! never queued a dependency job — so the decision lives here rather than in either.

use std::collections::HashMap;

use serde_json::value::RawValue;
use windmill_common::{
    error::Result,
    jobs::JobPayload,
    min_version::MIN_VERSION_SUPPORTS_BINARY_PREBUILD,
    scripts::{ScriptHash, ScriptLang},
    worker::to_raw_value,
    DB,
};

/// Job arg marking a `dependencies` job as a build pass: it compiles the already-deployed
/// version instead of regenerating the lock. Set only by [`binary_prebuild_job`].
pub const BUILD_BINARY_ONLY_ARG: &str = "build_binary_only";

/// Whether a `dependencies` job is a build pass. Presence is not enough — an explicit
/// `false` must mean off.
pub fn is_build_binary_job(args: Option<&HashMap<String, Box<RawValue>>>) -> bool {
    args.and_then(|x| x.get(BUILD_BINARY_ONLY_ARG))
        .and_then(|v| serde_json::from_str::<bool>(v.get()).ok())
        .unwrap_or(false)
}

/// Languages whose compiled artifact lands in the instance object store, which is what
/// makes building it at deploy time spare *every* worker the first-run compile. Bun
/// bundles are already built inline by the dependency job.
fn supports_binary_prebuild(lang: ScriptLang) -> bool {
    matches!(lang, ScriptLang::Rust | ScriptLang::Go | ScriptLang::CSharp)
}

/// A build job ready to push, as returned by [`binary_prebuild_job`].
pub struct BinaryPrebuild {
    pub payload: JobPayload,
    pub args: HashMap<String, Box<RawValue>>,
    /// `None` means the script's language tag, where its dependency job already runs.
    pub tag: Option<String>,
}

/// `Some(..)` when the instance is configured to pre-build this script's binary.
///
/// The caller pushes it, so the build can ride whatever transaction the deploy already
/// holds — pushed independently, the build job could start before the script version it
/// builds is committed and fail to read it back.
pub async fn binary_prebuild_job(
    db: &DB,
    path: &str,
    hash: ScriptHash,
    language: ScriptLang,
    lock: Option<&str>,
) -> Result<Option<BinaryPrebuild>> {
    if !supports_binary_prebuild(language) {
        return Ok(None);
    }
    // The run path derives both the cache key and the build profile from the lock, so
    // there is nothing sound to build without one.
    if lock.is_none_or(|l| l.is_empty()) {
        return Ok(None);
    }
    let Some(tag) = windmill_common::global_settings::auto_build_binary_on_deploy(db).await? else {
        return Ok(None);
    };
    // Checked after the setting so the log only fires for someone who actually asked for
    // this — an enabled setting that silently does nothing is otherwise undiagnosable.
    if !MIN_VERSION_SUPPORTS_BINARY_PREBUILD.met().await {
        tracing::info!(
            "not building {path} at deploy time: some workers are older than {}",
            MIN_VERSION_SUPPORTS_BINARY_PREBUILD.version()
        );
        return Ok(None);
    }

    let mut args: HashMap<String, Box<RawValue>> = HashMap::new();
    args.insert(BUILD_BINARY_ONLY_ARG.to_string(), to_raw_value(&true));
    Ok(Some(BinaryPrebuild {
        payload: JobPayload::BuildBinary { path: path.to_string(), hash, language },
        args,
        tag,
    }))
}
