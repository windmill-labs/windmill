//! Sandboxed container runtime: run a container as a sandboxed subprogram of the job.
//!
//! Unlike the legacy `# docker` (dind/daemon) path, this has no daemon and no Docker
//! API. It splits *pull* from *run*:
//!
//! 1. **pull/extract** (`crane`, no daemon/store/root): materialize the image's root
//!    filesystem into `{job_dir}/rootfs` and read its OCI config
//!    (Env/Cmd/Entrypoint/WorkingDir), via a digest-keyed rootfs cache.
//! 2. **run** (the job's own nsjail sandbox): execute the image command with the
//!    extracted rootfs bound in as the new root, so the container inherits exactly
//!    the job's confinement (filesystem mask, pid namespace, network, uid) and can't
//!    escape past what the job itself can reach.
//!
//! Selected by `# sandbox <image>` (a bare `# sandbox` keeps plain nsjail-bash;
//! `# docker` keeps the v1 daemon path). The script body runs inside the image via
//! `/bin/sh`; an empty body runs the image's ENTRYPOINT/CMD.

use std::process::Stdio;

use serde::Deserialize;
use serde_json::{json, value::RawValue};
use sqlx::types::Json;
use tokio::process::Command;

use windmill_common::{client::AuthedClient, scripts::ScriptLang};
use windmill_common::{
    error::Error,
    worker::{to_raw_value, write_file, Connection},
};

use windmill_queue::{append_logs, CanceledBy, MiniPulledJob};

use crate::{
    common::{
        build_args_map, get_reserved_variables, raw_to_string, resolve_nsjail_timeout,
        resolve_nsjail_tmp_mount_block, start_child_process, OccupancyMetrics, DEV_CONF_NSJAIL,
    },
    get_proxy_envs_for_lang,
    handle_child::handle_child,
    DISABLE_NUSER, NSJAIL_AVAILABLE, NSJAIL_PATH, SANDBOX_IMAGE_CACHE_MAX_MB,
    SANDBOX_IMAGE_DEFAULT_REGISTRY, SANDBOX_IMAGE_MAX_SIZE_MB, SANDBOX_IMAGE_PULL_POLICY,
    SANDBOX_REGISTRY_AUTH,
};

const NSJAIL_CONFIG_RUN_DOCKER_CONTENT: &str = include_str!("../nsjail/run.docker.config.proto");

const DEFAULT_PATH: &str = "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin";

lazy_static::lazy_static! {
    /// `crane` (google/go-containerregistry) — pulls + flattens an image to a rootfs
    /// without a daemon, store, root, or privileged container. We never *run* the
    /// image via crane (nsjail does the run), so a full container engine is overkill.
    pub static ref CRANE_PATH: String =
        std::env::var("CRANE_PATH").unwrap_or_else(|_| "crane".to_string());

    /// `linux/<arch>` for the worker, pinned on every crane call so multi-arch images
    /// resolve deterministically (and `crane manifest` returns a real manifest, not an
    /// index).
    static ref CRANE_PLATFORM: String = format!("linux/{}", match std::env::consts::ARCH {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        other => other,
    });

    /// Content-addressed cache of flattened rootfs tars, keyed by image digest. crane
    /// has no persistent store, so this is what gives cross-job dedup (and, since it's
    /// digest-keyed, automatic freshness when a moving tag changes).
    static ref ROOTFS_CACHE_DIR: String =
        format!("{}sandbox_rootfs", *windmill_common::worker::ROOT_CACHE_DIR);
}

/// Guards against overlapping cache-eviction passes across concurrent jobs.
static EVICTION_RUNNING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// `sandbox_image_pull_policy` instance setting. With the digest-keyed cache, `newer`
/// (default) re-resolves the digest each job (cheap manifest fetch) so moving tags
/// like `:latest` stay fresh while unchanged digests reuse the cache. `missing` skips
/// the registry when a digest is already cached for the ref; `never` only uses the
/// cache (errors if absent); `always` == `newer` here.
async fn pull_policy() -> String {
    let p = SANDBOX_IMAGE_PULL_POLICY.read().await.clone();
    match p.as_deref() {
        Some(p @ ("missing" | "newer" | "always" | "never")) => p.to_string(),
        _ => "newer".to_string(),
    }
}

/// `sandbox_image_max_size_mb` instance setting; 0 (or unset/non-positive) = no limit.
async fn max_image_size_mb() -> u64 {
    SANDBOX_IMAGE_MAX_SIZE_MB.read().await.unwrap_or(0).max(0) as u64
}

/// `sandbox_image_cache_max_mb` instance setting; 0 (or unset/non-positive) = unbounded.
async fn image_cache_max_mb() -> u64 {
    SANDBOX_IMAGE_CACHE_MAX_MB.read().await.unwrap_or(0).max(0) as u64
}

/// A ref is registry-qualified if the component before the first `/` looks like a
/// host (contains `.` or `:`, or is `localhost`). Bare repos (`alpine`,
/// `alpine:latest`, `myorg/img`) are unqualified and resolve against docker.io —
/// or the configured default registry.
fn registry_qualified(image: &str) -> bool {
    match image.split_once('/') {
        None => false,
        Some((first, _)) => first.contains('.') || first.contains(':') || first == "localhost",
    }
}

/// Prepend the `sandbox_image_default_registry` instance setting to unqualified image
/// refs (fully-qualified refs are left untouched).
async fn resolve_image_ref(image: &str) -> String {
    let registry = SANDBOX_IMAGE_DEFAULT_REGISTRY.read().await.clone();
    match registry {
        Some(registry) if !registry.trim().is_empty() && !registry_qualified(image) => {
            format!("{}/{}", registry.trim().trim_end_matches('/'), image)
        }
        _ => image.to_string(),
    }
}

/// If the `sandbox_registry_auth` instance setting holds a docker `auth.json` blob,
/// write it to a per-job `DOCKER_CONFIG` dir (`{job_dir}/.docker/config.json`, 0600,
/// removed with the job) and return the dir to pass to crane via `DOCKER_CONFIG`.
/// Returns `None` when unset. (docker `config.json` and podman `auth.json` share the
/// `{"auths": {...}}` schema, so the same blob works.)
async fn write_auth_dir(job_dir: &str) -> Result<Option<String>, Error> {
    let auth = SANDBOX_REGISTRY_AUTH.read().await.clone();
    let Some(auth) = auth.filter(|a| !a.trim().is_empty()) else {
        return Ok(None);
    };
    let dir = format!("{job_dir}/.docker");
    tokio::fs::create_dir_all(&dir).await?;
    let path = format!("{dir}/config.json");
    // Create 0600 from the start (registry credentials) — no world-readable window.
    #[cfg(unix)]
    {
        use tokio::io::AsyncWriteExt;
        let mut f = tokio::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&path)
            .await?;
        f.write_all(auth.as_bytes()).await?;
    }
    #[cfg(not(unix))]
    tokio::fs::write(&path, auth).await?;
    Ok(Some(dir))
}

/// The subset of an image's OCI config we apply to the run.
#[derive(Deserialize, Default, Debug)]
struct OciConfig {
    #[serde(default, rename = "Env")]
    env: Option<Vec<String>>,
    #[serde(default, rename = "Cmd")]
    cmd: Option<Vec<String>>,
    #[serde(default, rename = "Entrypoint")]
    entrypoint: Option<Vec<String>>,
    #[serde(default, rename = "WorkingDir")]
    working_dir: Option<String>,
}

/// Quote a string as a protobuf-text-format string literal for safe inclusion in
/// the nsjail config. Image-controlled values (mount srcs/dsts, symlink targets,
/// WorkingDir) flow into the config, so they MUST be escaped — an unescaped `"` or
/// newline would otherwise let a hostile image config inject arbitrary nsjail
/// directives and break out of the sandbox. Every byte is emitted as a printable
/// ASCII char or a valid protobuf escape (`\"`, `\\`, `\n`/`\r`/`\t`, or 3-digit
/// octal `\NNN` for control/non-ASCII bytes), so the result always parses.
fn proto_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for &b in s.as_bytes() {
        match b {
            b'"' => out.push_str("\\\""),
            b'\\' => out.push_str("\\\\"),
            b'\n' => out.push_str("\\n"),
            b'\r' => out.push_str("\\r"),
            b'\t' => out.push_str("\\t"),
            0x20..=0x7e => out.push(b as char),
            _ => out.push_str(&format!("\\{b:03o}")),
        }
    }
    out.push('"');
    out
}

/// Render container env vars as nsjail `envar:` directives (one per line). Each
/// `KEY=VALUE` is proto-escaped, so image-controlled keys/values can neither break
/// the config nor reach nsjail's own process environment.
fn render_envars(env: &[(String, String)]) -> String {
    env.iter()
        .map(|(k, v)| format!("envar: {}", proto_str(&format!("{k}={v}"))))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Run `crane` with the optional per-job `DOCKER_CONFIG` auth dir.
async fn crane(args: &[&str], auth_dir: Option<&str>) -> Result<std::process::Output, Error> {
    let mut cmd = Command::new(CRANE_PATH.as_str());
    cmd.args(args);
    if let Some(dir) = auth_dir {
        cmd.env("DOCKER_CONFIG", dir);
    }
    cmd.output()
        .await
        .map_err(|e| Error::ExecutionErr(format!("failed to run crane {}: {e}", args.join(" "))))
}

/// `crane config` output: the image config (Env/Cmd/Entrypoint/WorkingDir) is nested
/// under the top-level `config` key.
#[derive(Deserialize, Default)]
struct CraneConfig {
    #[serde(default)]
    config: OciConfig,
}

/// Filesystem-safe cache key for a digest (`sha256:ab..` -> `sha256_ab..`).
fn digest_key(digest: &str) -> String {
    digest.replace([':', '/'], "_")
}

/// Filesystem-safe, collision-resistant key for an image ref (the ref->digest file).
fn ref_key(image: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    image.hash(&mut h);
    let safe: String = image
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let safe = &safe[safe.len().saturating_sub(80)..];
    format!("{safe}_{:016x}", h.finish())
}

/// Resolve the image ref to a content digest, honoring the pull policy + a ref->digest
/// cache. `missing`/`never` reuse a cached digest without hitting the registry (`never`
/// errors if absent); `newer`/`always` always re-resolve via `crane digest`.
async fn resolve_digest(
    image: &str,
    policy: &str,
    auth_dir: Option<&str>,
) -> Result<String, Error> {
    let refs_dir = format!("{}/refs", *ROOTFS_CACHE_DIR);
    let ref_file = format!("{refs_dir}/{}", ref_key(image));

    if matches!(policy, "missing" | "never") {
        if let Ok(d) = tokio::fs::read_to_string(&ref_file).await {
            let d = d.trim().to_string();
            if !d.is_empty()
                && tokio::fs::metadata(format!("{}/{}.tar", *ROOTFS_CACHE_DIR, digest_key(&d)))
                    .await
                    .is_ok()
            {
                return Ok(d);
            }
        }
        if policy == "never" {
            return Err(Error::ExecutionErr(format!(
                "image {image} is not in the sandbox cache and SANDBOX_IMAGE_PULL_POLICY=never"
            )));
        }
    }

    let out = crane(&["digest", "--platform", &CRANE_PLATFORM, image], auth_dir).await?;
    if !out.status.success() {
        return Err(Error::ExecutionErr(format!(
            "failed to resolve image {image}: {}",
            String::from_utf8_lossy(&out.stderr)
        )));
    }
    let digest = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let _ = tokio::fs::create_dir_all(&refs_dir).await;
    // tmp+rename so a concurrent `missing`/`never` reader never sees a torn ref file.
    let ref_tmp = format!("{ref_file}.tmp.{}", digest_key(&digest));
    if tokio::fs::write(&ref_tmp, &digest).await.is_ok() {
        let _ = tokio::fs::rename(&ref_tmp, &ref_file).await;
    }
    Ok(digest)
}

/// Pull (if not cached) and unpack `image` into `{job_dir}/rootfs`, returning its OCI
/// config. Uses `crane export`/`config` (no daemon/store/root) with a content-addressed
/// rootfs+config cache keyed by digest for cross-job dedup.
async fn extract_image(image: &str, job_dir: &str) -> Result<OciConfig, Error> {
    let rootfs = format!("{job_dir}/rootfs");
    tokio::fs::create_dir_all(&rootfs).await?;
    tokio::fs::create_dir_all(&*ROOTFS_CACHE_DIR).await?;

    let auth_dir = write_auth_dir(job_dir).await?;
    let auth = auth_dir.as_deref();
    let digest = resolve_digest(image, &pull_policy().await, auth).await?;
    // Pin every subsequent fetch to the resolved digest, not the (mutable) tag, so the
    // content can't diverge from the digest we cache under if the tag moves mid-fetch.
    let pinned = format!("{}@{digest}", image.split('@').next().unwrap_or(image));
    let key = digest_key(&digest);
    let tar = format!("{}/{key}.tar", *ROOTFS_CACHE_DIR);
    let cfg = format!("{}/{key}.json", *ROOTFS_CACHE_DIR);
    let size_file = format!("{}/{key}.size", *ROOTFS_CACHE_DIR);
    let token = std::path::Path::new(job_dir)
        .file_name()
        .map(|x| x.to_string_lossy().into_owned())
        .unwrap_or_default();

    // Enforce the size cap on EVERY job (not just cache misses), using a cached size so
    // a cache reuse needs no registry call — lowering the limit rejects cached images too.
    enforce_image_size_limit(&pinned, &size_file, auth).await?;

    // Materialize the flattened rootfs. The cache tar can be evicted concurrently, so up
    // to two attempts: hardlink the cache tar into the job dir (pins the inode against
    // eviction) before extracting; if it vanished first, re-fetch.
    let job_tar = format!("{job_dir}/rootfs.tar");
    for attempt in 0..2 {
        if tokio::fs::metadata(&tar).await.is_err() {
            fetch_into_cache(&pinned, &tar, &cfg, &token, auth).await?;
        }
        let config = read_oci_config(&cfg).await;
        let _ = tokio::fs::remove_file(&job_tar).await;
        // Stage the cache tar into the job dir so concurrent eviction can't unlink it out
        // from under `tar -xf`. Prefer a hardlink (free), but the cache volume and the job
        // dir are usually on *different* filesystems in the shipped deployments (the cache
        // is its own volume/PVC) — there `hard_link` returns EXDEV, so fall back to a copy.
        // `copy` reads through the source inode, so an eviction mid-copy still completes.
        let staged = match tokio::fs::hard_link(&tar, &job_tar).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(e), // vanished — re-fetch
            Err(_) => tokio::fs::copy(&tar, &job_tar).await.map(|_| ()),
        };
        match staged {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound && attempt == 0 => {
                continue; // evicted between the check and the staging — re-fetch
            }
            Err(e) => return Err(Error::ExecutionErr(format!("failed to stage rootfs: {e}"))),
        }
        // Extract as the worker user (rootfs is worker-owned → uid 0 inside the jail).
        let untar = Command::new("tar")
            .args(["-xf", &job_tar, "-C", &rootfs])
            .output()
            .await
            .map_err(|e| Error::ExecutionErr(format!("failed to run tar: {e}")))?;
        let _ = tokio::fs::remove_file(&job_tar).await;
        if !untar.status.success() {
            return Err(Error::ExecutionErr(format!(
                "failed to unpack image {image}: {}",
                String::from_utf8_lossy(&untar.stderr)
            )));
        }
        return Ok(config);
    }
    Err(Error::ExecutionErr(format!(
        "failed to materialize rootfs for {image} (cache evicted twice)"
    )))
}

/// Fetch + flatten `pinned` (a `name@digest` ref) into the cache: export the rootfs tar
/// and write the OCI config sidecar, both via tmp+rename so concurrent readers never see
/// a torn file. The tar is published last (a present tar implies a present config).
async fn fetch_into_cache(
    pinned: &str,
    tar: &str,
    cfg: &str,
    token: &str,
    auth: Option<&str>,
) -> Result<(), Error> {
    let tar_tmp = format!("{tar}.tmp.{token}");
    let cfg_tmp = format!("{cfg}.tmp.{token}");
    let exported = crane(
        &["export", "--platform", &CRANE_PLATFORM, pinned, &tar_tmp],
        auth,
    )
    .await?;
    if !exported.status.success() {
        let _ = tokio::fs::remove_file(&tar_tmp).await;
        return Err(Error::ExecutionErr(format!(
            "failed to export image {pinned}: {}",
            String::from_utf8_lossy(&exported.stderr)
        )));
    }
    let config = crane(&["config", "--platform", &CRANE_PLATFORM, pinned], auth).await?;
    if !config.status.success() {
        let _ = tokio::fs::remove_file(&tar_tmp).await;
        return Err(Error::ExecutionErr(format!(
            "failed to read image {pinned} config: {}",
            String::from_utf8_lossy(&config.stderr)
        )));
    }
    let _ = tokio::fs::write(&cfg_tmp, &config.stdout).await;
    let _ = tokio::fs::rename(&cfg_tmp, cfg).await;
    tokio::fs::rename(&tar_tmp, tar).await?;
    Ok(())
}

/// Read the cached OCI config (Env/Cmd/Entrypoint/WorkingDir); tolerate a missing or torn
/// sidecar by falling back to defaults (the run still works off the body + image FS).
async fn read_oci_config(cfg: &str) -> OciConfig {
    match tokio::fs::read(cfg).await {
        Ok(bytes) => serde_json::from_slice::<CraneConfig>(&bytes)
            .map(|c| c.config)
            .unwrap_or_default(),
        Err(_) => OciConfig::default(),
    }
}

/// Manifest descriptor (`crane manifest`), for the pre-download size guard.
#[derive(Deserialize, Default)]
struct CraneDescriptor {
    #[serde(default)]
    size: u64,
}
#[derive(Deserialize, Default)]
struct CraneManifest {
    #[serde(default)]
    layers: Vec<CraneDescriptor>,
    #[serde(default)]
    config: CraneDescriptor,
}

/// Reject the image if its compressed download size exceeds `SANDBOX_IMAGE_MAX_SIZE_MB`.
/// Runs on EVERY job (so lowering the limit rejects already-cached images too); the size
/// is read from a `{digest}.size` sidecar when present (no registry call on cache reuse)
/// and otherwise fetched once via `crane manifest` (before any layer download) and cached.
/// No-op when the limit is 0 (unset).
async fn enforce_image_size_limit(
    pinned: &str,
    size_file: &str,
    auth_dir: Option<&str>,
) -> Result<(), Error> {
    let max = max_image_size_mb().await;
    if max == 0 {
        return Ok(());
    }
    let bytes = match tokio::fs::read_to_string(size_file)
        .await
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
    {
        Some(b) => b,
        None => {
            let out = crane(
                &["manifest", "--platform", &CRANE_PLATFORM, pinned],
                auth_dir,
            )
            .await?;
            if !out.status.success() {
                // Don't silently bypass the guard — surface it so an operator can see the
                // size limit isn't being enforced for this image.
                tracing::warn!(
                    "sandbox image size guard: `crane manifest {pinned}` failed, not enforcing \
                    SANDBOX_IMAGE_MAX_SIZE_MB: {}",
                    String::from_utf8_lossy(&out.stderr)
                );
                return Ok(());
            }
            let manifest: CraneManifest = match serde_json::from_slice(&out.stdout) {
                Ok(m) => m,
                Err(e) => {
                    tracing::warn!(
                        "sandbox image size guard: cannot parse `crane manifest` json: {e}"
                    );
                    return Ok(());
                }
            };
            let b = manifest.config.size + manifest.layers.iter().map(|l| l.size).sum::<u64>();
            let _ = tokio::fs::write(size_file, b.to_string()).await;
            b
        }
    };
    let mb = bytes / 1_000_000;
    if mb > max {
        return Err(Error::ExecutionErr(format!(
            "image {pinned} is {mb} MB (compressed), over the SANDBOX_IMAGE_MAX_SIZE_MB limit of {max} MB"
        )));
    }
    Ok(())
}

/// Best-effort eviction: while the cached rootfs tars exceed `SANDBOX_IMAGE_CACHE_MAX_MB`,
/// remove the oldest by mtime (creation order — tars are write-once, cache hits don't
/// touch mtime). No-op when the limit is 0 (unset). Skipped if another pass is already
/// running. The per-job extracted rootfs lives in the job dir (cleaned with the job), so
/// only the content-addressed tar+config+size cache is pruned. Also sweeps orphaned
/// `*.tmp.*` files left by a crashed mid-export.
async fn enforce_image_cache_limit() {
    use std::sync::atomic::Ordering;
    let max_mb = image_cache_max_mb().await;
    if max_mb == 0 {
        return;
    }
    if EVICTION_RUNNING
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }
    // Reset the guard on every exit path (incl. an early `break` or a panic), so a
    // stuck flag can never permanently disable eviction until a worker restart.
    struct ResetOnDrop;
    impl Drop for ResetOnDrop {
        fn drop(&mut self) {
            EVICTION_RUNNING.store(false, std::sync::atomic::Ordering::SeqCst);
        }
    }
    let _reset = ResetOnDrop;
    let max_bytes = max_mb.saturating_mul(1_000_000);

    // (path, size, mtime) for every cached rootfs tar; also sweep orphaned tmp files.
    async fn list_tars() -> Vec<(std::path::PathBuf, u64, std::time::SystemTime)> {
        let mut out = Vec::new();
        let Ok(mut rd) = tokio::fs::read_dir(&*ROOTFS_CACHE_DIR).await else {
            return out;
        };
        while let Ok(Some(e)) = rd.next_entry().await {
            let p = e.path();
            let name = e.file_name();
            let name = name.to_string_lossy();
            // Reclaim leftover `*.tmp.<token>` files from a crashed mid-export.
            if name.contains(".tmp.") {
                let _ = tokio::fs::remove_file(&p).await;
                continue;
            }
            if p.extension().and_then(|x| x.to_str()) != Some("tar") {
                continue;
            }
            if let Ok(m) = e.metadata().await {
                let mtime = m.modified().unwrap_or(std::time::UNIX_EPOCH);
                out.push((p, m.len(), mtime));
            }
        }
        out
    }

    loop {
        let mut tars = list_tars().await;
        let total: u64 = tars.iter().map(|(_, s, _)| *s).sum();
        if total <= max_bytes || tars.is_empty() {
            break;
        }
        tars.sort_by_key(|(_, _, mtime)| *mtime);
        let victim = tars[0].0.clone();
        if tokio::fs::remove_file(&victim).await.is_err() {
            break; // can't reclaim — stop rather than spin on the same victim
        }
        // Drop the sibling config + size sidecars too.
        let _ = tokio::fs::remove_file(victim.with_extension("json")).await;
        let _ = tokio::fs::remove_file(victim.with_extension("size")).await;
        tracing::info!("sandbox image cache eviction: removed {}", victim.display());
    }
    // `_reset` drops here and clears EVICTION_RUNNING.
}

/// Build the nsjail mount block that binds each top-level entry of the rootfs in
/// place. Binding the whole rootfs at `/` trips nsjail's read-only remount of its
/// base root in a rootless userns; per-entry binds avoid it. `proc`, `dev`, `tmp`
/// and `sys` are skipped — the profile provides them.
async fn generate_rootfs_mounts(rootfs: &str) -> Result<String, Error> {
    let mut block = String::new();
    let mut entries = tokio::fs::read_dir(rootfs).await?;
    while let Some(entry) = entries.next_entry().await? {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if matches!(name.as_ref(), "proc" | "dev" | "tmp" | "sys") {
            continue;
        }
        let src = proto_str(&format!("{rootfs}/{name}"));
        let dst = proto_str(&format!("/{name}"));
        let file_type = entry.file_type().await?;
        if file_type.is_symlink() {
            // Recreate top-level symlinks (e.g. usr-merged /bin -> usr/bin) as
            // symlinks in the jail. The target is image-controlled but only ever
            // *resolved inside the jail* (against the bound rootfs dirs / jail
            // pseudo-fs) — there is no host `/` in the jail for it to point at — and
            // it is escaped via proto_str, so it can neither escape nor inject config.
            let target = tokio::fs::read_link(entry.path())
                .await
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default();
            block.push_str(&format!(
                "mount {{\n  src: {}\n  dst: {dst}\n  is_symlink: true\n  mandatory: false\n}}\n",
                proto_str(&target),
            ));
        } else {
            block.push_str(&format!(
                "mount {{\n  src: {src}\n  dst: {dst}\n  is_bind: true\n  rw: true\n  mandatory: false\n}}\n",
            ));
        }
    }
    Ok(block)
}

#[tracing::instrument(level = "trace", skip_all)]
pub async fn handle_docker_v2_job(
    image: &str,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job: &MiniPulledJob,
    conn: &Connection,
    client: &AuthedClient,
    parent_runnable_path: Option<String>,
    content: &str,
    job_dir: &str,
    shared_mount: &str,
    base_internal_url: &str,
    worker_name: &str,
    occupancy_metrics: &mut OccupancyMetrics,
) -> Result<Box<RawValue>, Error> {
    // The sandboxed container runtime *is* nsjail, so it requires nsjail. (`# docker`
    // keeps the v1 dind path for non-sandboxed workers.)
    if NSJAIL_AVAILABLE.is_none() {
        return Err(Error::ExecutionErr(format!(
            "`# sandbox {image}` runs the image inside nsjail, which is not available on \
            this worker. Install nsjail, or use a bare `# docker` (dind) instead."
        )));
    }

    // Apply the default-registry instance setting to unqualified refs.
    let resolved_image = resolve_image_ref(image).await;
    let image = resolved_image.as_str();

    append_logs(
        &job.id,
        &job.workspace_id,
        format!("\n\n--- SANDBOXED CONTAINER (nsjail) ---\nextracting image {image}...\n"),
        conn,
    )
    .await;

    let config = extract_image(image, job_dir).await?;
    let rootfs = format!("{job_dir}/rootfs");

    // Best-effort: keep the cached rootfs tars under their size cap (overlaps the run).
    tokio::spawn(enforce_image_cache_limit());

    // Resolve the script args from the bash signature, like the bash executor.
    let args = build_args_map(job, client, conn).await?.map(Json);
    let job_args = if args.is_some() {
        args.as_ref()
    } else {
        job.args.as_ref()
    };
    let args_owned = windmill_parser_bash::parse_bash_sig(content)?
        .args
        .iter()
        .map(|arg| {
            job_args
                .and_then(|x| x.get(&arg.name).map(|x| raw_to_string(x.get())))
                .unwrap_or_else(String::new)
        })
        .collect::<Vec<String>>();

    // The body is everything that isn't a leading `#` annotation/comment line. With
    // a body we run it via the image's `/bin/sh`; without one we run the image's
    // ENTRYPOINT + CMD.
    let has_body = content
        .lines()
        .any(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'));

    let cmd_args: Vec<String> = if has_body {
        // Pass the body straight to `sh -c` rather than writing a script file into
        // the image-controlled rootfs: a malicious image could plant that path as a
        // symlink to a host file and capture the worker's write before nsjail starts
        // (sandbox-boundary bypass). `sh -c <body> sh <args...>` binds args as $1.. .
        let mut v = vec![
            "/bin/sh".to_string(),
            "-c".to_string(),
            format!("set -e\n{content}"),
            "sh".to_string(),
        ];
        v.extend(args_owned.iter().cloned());
        v
    } else {
        let mut v = config.entrypoint.clone().unwrap_or_default();
        v.extend(config.cmd.clone().unwrap_or_default());
        if v.is_empty() {
            return Err(Error::ExecutionErr(format!(
                "image {image} has no ENTRYPOINT/CMD and the script body is empty — \
                nothing to run"
            )));
        }
        v.extend(args_owned.iter().cloned());
        v
    };

    let working_dir = config
        .working_dir
        .as_deref()
        .filter(|w| !w.is_empty())
        .unwrap_or("/");

    // The image's OCI Env is attacker-controlled (BOTH keys and values), so it must
    // NOT enter the nsjail launcher's own process env: a hostile image could set
    // LD_PRELOAD / LD_LIBRARY_PATH / LD_AUDIT and have the dynamic loader run code in
    // the nsjail binary as the worker — outside the jail — before it sandboxes.
    // Deliver it to the *child only* via proto-escaped `envar:` directives.
    let mut container_env: Vec<(String, String)> = Vec::new();
    for kv in config.env.unwrap_or_default() {
        if let Some((k, v)) = kv.split_once('=') {
            container_env.push((k.to_string(), v.to_string()));
        }
    }
    if !container_env.iter().any(|(k, _)| k == "PATH") {
        container_env.push(("PATH".to_string(), DEFAULT_PATH.to_string()));
    }
    if !container_env.iter().any(|(k, _)| k == "HOME") {
        container_env.push(("HOME".to_string(), "/root".to_string()));
    }
    let envars = render_envars(&container_env);

    // Render the nsjail profile: dynamic per-entry rootfs binds + image WorkingDir.
    let nsjail_timeout = resolve_nsjail_timeout(conn, &job.workspace_id, job.id, job.timeout).await;
    let rootfs_mounts = generate_rootfs_mounts(&rootfs).await?;
    write_file(
        job_dir,
        "run.docker.config.proto",
        &NSJAIL_CONFIG_RUN_DOCKER_CONTENT
            .replace("{TIMEOUT}", &nsjail_timeout)
            .replace("{CLONE_NEWUSER}", &(!*DISABLE_NUSER).to_string())
            // proto_str-quoted: WorkingDir is image-controlled, must not break out
            // of the `cwd:` string and inject nsjail directives.
            .replace("{WORKDIR}", &proto_str(working_dir))
            .replace("{ROOTFS_MOUNTS}", &rootfs_mounts)
            .replace(
                "{TMP_MOUNT_BLOCK}",
                &resolve_nsjail_tmp_mount_block(job_dir).await,
            )
            // `# volume` mounts + same-worker shared folder (empty if none).
            .replace("{SHARED_MOUNT}", shared_mount)
            // Image env as `envar:` directives (child-only), so it never touches
            // nsjail's process env.
            .replace("{ENVARS}", &envars)
            .replace("#{DEV}", DEV_CONF_NSJAIL),
    )?;

    // nsjail's OWN process env: only windmill-trusted keys (reserved vars so
    // `wmill`/API calls work, + proxy). `keep_env: true` forwards these to the
    // child. The image env is NOT here — see container_env above.
    let mut reserved_variables =
        get_reserved_variables(job, &client.token, conn, parent_runnable_path).await?;
    reserved_variables.insert("RUST_LOG".to_string(), "info".to_string());
    reserved_variables.insert(
        "BASE_INTERNAL_URL".to_string(),
        base_internal_url.to_string(),
    );

    let proxy_envs = get_proxy_envs_for_lang(
        &ScriptLang::Bash,
        job.kind,
        &job.id,
        &job.workspace_id,
        conn,
    )
    .await?;

    let mut nsjail_run_args = vec!["--config", "run.docker.config.proto", "--"];
    nsjail_run_args.extend(cmd_args.iter().map(|s| s.as_str()));

    let mut nsjail_cmd = Command::new(NSJAIL_PATH.as_str());
    nsjail_cmd
        .current_dir(job_dir)
        .env_clear()
        .envs(reserved_variables)
        .envs(proxy_envs)
        .args(nsjail_run_args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let child = start_child_process(nsjail_cmd, NSJAIL_PATH.as_str(), false).await?;

    handle_child(
        &job.id,
        conn,
        mem_peak,
        canceled_by,
        child,
        true,
        worker_name,
        &job.workspace_id,
        "sandboxed container run",
        job.timeout,
        true,
        &mut Some(occupancy_metrics),
        None,
        None,
    )
    .await?;

    Ok(to_raw_value(&json!(format!(
        "sandboxed container ({image}) completed successfully"
    ))))
}

#[cfg(test)]
mod tests {
    use super::{digest_key, proto_str, ref_key, registry_qualified, render_envars};

    #[test]
    fn digest_key_is_filesystem_safe() {
        assert_eq!(digest_key("sha256:4d889c14e7d5"), "sha256_4d889c14e7d5");
        // No `:` or `/` survives (both would break the cache filename).
        let k = digest_key("sha256:ab/cd:ef");
        assert!(!k.contains(':') && !k.contains('/'));
    }

    #[test]
    fn ref_key_is_safe_and_stable() {
        // Deterministic for a given ref...
        assert_eq!(ref_key("ghcr.io/o/i:tag"), ref_key("ghcr.io/o/i:tag"));
        // ...distinguishes different refs...
        assert_ne!(ref_key("alpine:latest"), ref_key("alpine:edge"));
        // ...and is filesystem-safe (no `/` or `:`), incl. for multibyte refs (no panic
        // on the trailing-80 byte slice since every char maps to single-byte ASCII).
        for r in [
            "alpine",
            "ghcr.io/o/i:tag",
            "localhost:5000/r@sha256:ab",
            "rég/imagé:tag",
        ] {
            let k = ref_key(r);
            assert!(!k.contains('/') && !k.contains(':'));
        }
    }

    #[test]
    fn render_envars_emits_proto_directives() {
        // Image-controlled env (incl. loader vars) is rendered as `envar:` directives
        // — i.e. delivered to the child via the config, NOT nsjail's process env, so
        // it can never set LD_PRELOAD/etc. on the nsjail binary itself.
        let env = vec![
            ("PATH".to_string(), "/usr/bin".to_string()),
            ("LD_PRELOAD".to_string(), "rootfs/evil.so".to_string()),
        ];
        let out = render_envars(&env);
        assert_eq!(
            out,
            "envar: \"PATH=/usr/bin\"\nenvar: \"LD_PRELOAD=rootfs/evil.so\""
        );
        // A value trying to inject extra directives is escaped, not interpreted.
        let evil = vec![("X".to_string(), "v\"\nclone_newuser: false".to_string())];
        let line = render_envars(&evil);
        assert!(line.starts_with("envar: \""));
        assert!(!line.contains("\nclone_newuser"));
        assert!(line.contains("\\n"));
    }

    #[test]
    fn proto_str_escapes_injection() {
        // Normal paths are just wrapped in quotes.
        assert_eq!(proto_str("/app"), "\"/app\"");
        // A `"` is escaped so it cannot close the surrounding string and inject
        // subsequent nsjail directives — this is what the WorkingDir / mount-src
        // sandboxing fixes depend on.
        let malicious = "/x\"\nmount { src: \"/\" dst: \"/host\" is_bind: true }\n#";
        let escaped = proto_str(malicious);
        assert!(escaped.starts_with('"') && escaped.ends_with('"'));
        // No raw quote or newline survives inside the rendered literal.
        let inner = &escaped[1..escaped.len() - 1];
        assert!(!inner.contains('\n'));
        assert!(!inner.contains("\"") || inner.contains("\\\""));
        assert!(escaped.contains("\\\"")); // the inner quote is backslash-escaped
        assert!(escaped.contains("\\n")); // the newline is escaped
                                          // Control and non-ASCII bytes render as valid 3-digit octal escapes (never
                                          // a raw byte or an invalid `\u{..}` that nsjail's parser would reject).
        assert_eq!(proto_str("a\u{1b}b"), "\"a\\033b\""); // ESC (0x1b)
        assert_eq!(proto_str("é"), "\"\\303\\251\""); // UTF-8 bytes 0xc3 0xa9
    }

    #[test]
    fn registry_qualified_classifies_refs() {
        // Unqualified: bare repos (with/without tag) and docker.io org/repo.
        for img in ["alpine", "alpine:latest", "myorg/img", "myorg/img:1.2"] {
            assert!(!registry_qualified(img), "{img} should be unqualified");
        }
        // Qualified: the first path component is a host (has `.`/`:`) or localhost.
        for img in [
            "ghcr.io/org/img",
            "registry.example.com/img:tag",
            "localhost:5000/img",
            "localhost/img",
            "host:5000/a/b",
        ] {
            assert!(registry_qualified(img), "{img} should be qualified");
        }
    }
}
