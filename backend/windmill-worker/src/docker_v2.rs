//! Sandboxed container runtime: run a container as a sandboxed subprogram of the job.
//!
//! Unlike the legacy `# docker` (dind/daemon) path, this has no daemon and no Docker
//! API. It splits *pull* from *run*:
//!
//! 1. **pull/extract** (podman, rootless): materialize the image's root filesystem
//!    into `{job_dir}/rootfs` and read its OCI config (Env/Cmd/Entrypoint/WorkingDir).
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
    pub static ref PODMAN_PATH: String =
        std::env::var("PODMAN_PATH").unwrap_or_else(|_| "podman".to_string());
}

/// Guards against overlapping cache-eviction passes across concurrent jobs.
static EVICTION_RUNNING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// podman pull policy from the `sandbox_image_pull_policy` instance setting. `newer`
/// (the default when unset/invalid) re-pulls only when the registry digest changed —
/// one cheap manifest check per job, no transfer if unchanged — so moving tags like
/// `:latest` don't go stale. `missing` is fastest (tags can go stale); `always`
/// re-checks every job.
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

/// If the `sandbox_registry_auth` instance setting holds a docker/podman `auth.json`
/// blob, write it to a per-job authfile (0600, removed with the job) and return its
/// path to pass to `podman --authfile`. Returns `None` when unset.
async fn write_auth_file(job_dir: &str) -> Result<Option<String>, Error> {
    let auth = SANDBOX_REGISTRY_AUTH.read().await.clone();
    let Some(auth) = auth.filter(|a| !a.trim().is_empty()) else {
        return Ok(None);
    };
    let path = format!("{job_dir}/registry_auth.json");
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
    Ok(Some(path))
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

async fn podman(args: &[&str]) -> Result<std::process::Output, Error> {
    Command::new(PODMAN_PATH.as_str())
        .args(args)
        .output()
        .await
        .map_err(|e| Error::ExecutionErr(format!("failed to run podman {}: {e}", args.join(" "))))
}

/// Pull (if needed) and unpack `image` into `{job_dir}/rootfs`, returning its OCI
/// config. Uses podman rootless: `create` (auto-pulls) + `export | tar -x`, with the
/// config read from the resulting container (== image config, no command override).
async fn extract_image(image: &str, job_dir: &str) -> Result<OciConfig, Error> {
    let rootfs = format!("{job_dir}/rootfs");
    tokio::fs::create_dir_all(&rootfs).await?;

    // `podman create` (no command) pulls the image per the configured policy and
    // records the image's own Cmd/Entrypoint, which we then read back from the
    // container config. `--` guards against an `image` ref that starts with `-` being
    // parsed as a flag (e.g. `--authfile=...`) — the ref is attacker-controlled in
    // the untrusted case.
    let pull = format!("--pull={}", pull_policy().await);
    let mut create_args = vec!["create", &pull];
    let authfile = write_auth_file(job_dir).await?;
    if let Some(authfile) = authfile.as_deref() {
        create_args.push("--authfile");
        create_args.push(authfile);
    }
    create_args.push("--");
    create_args.push(image);
    let created = podman(&create_args).await?;
    if !created.status.success() {
        return Err(Error::ExecutionErr(format!(
            "failed to pull/create image {image}: {}",
            String::from_utf8_lossy(&created.stderr)
        )));
    }
    let container_id = String::from_utf8_lossy(&created.stdout).trim().to_string();

    // Always clean up the container, even on a later failure.
    let result = extract_created(image, &container_id, &rootfs).await;
    let _ = podman(&["rm", "-f", &container_id]).await;
    result
}

async fn extract_created(
    image: &str,
    container_id: &str,
    rootfs: &str,
) -> Result<OciConfig, Error> {
    // Reject oversized images before paying the (large) extraction cost.
    enforce_image_size_limit(image).await?;

    let inspected = podman(&["inspect", container_id, "--format", "{{json .Config}}"]).await?;
    if !inspected.status.success() {
        return Err(Error::ExecutionErr(format!(
            "failed to inspect image {image}: {}",
            String::from_utf8_lossy(&inspected.stderr)
        )));
    }
    let config: OciConfig = serde_json::from_slice(&inspected.stdout)
        .map_err(|e| Error::ExecutionErr(format!("failed to parse image {image} config: {e}")))?;

    // Flatten the image's layers into a rootfs directory. Go through a tar on disk
    // (in the job dir, cleaned up with the job) rather than a shell pipe. Extracted
    // as the worker user, so the rootfs is owned by the worker user — which the
    // single-uid jail maps to uid 0 inside.
    let tar_path = format!("{rootfs}.tar");
    let exported = podman(&["export", container_id, "--output", &tar_path]).await?;
    if !exported.status.success() {
        let _ = tokio::fs::remove_file(&tar_path).await;
        return Err(Error::ExecutionErr(format!(
            "failed to export image {image}: {}",
            String::from_utf8_lossy(&exported.stderr)
        )));
    }
    let untar = Command::new("tar")
        .args(["-xf", &tar_path, "-C", rootfs])
        .output()
        .await
        .map_err(|e| Error::ExecutionErr(format!("failed to run tar: {e}")))?;
    let _ = tokio::fs::remove_file(&tar_path).await;
    if !untar.status.success() {
        return Err(Error::ExecutionErr(format!(
            "failed to unpack image {image}: {}",
            String::from_utf8_lossy(&untar.stderr)
        )));
    }

    Ok(config)
}

/// Reject the image if its on-disk (uncompressed) size exceeds
/// `SANDBOX_IMAGE_MAX_SIZE_MB`. No-op when the limit is 0 (unset).
async fn enforce_image_size_limit(image: &str) -> Result<(), Error> {
    let max = max_image_size_mb().await;
    if max == 0 {
        return Ok(());
    }
    let out = podman(&["image", "inspect", image, "--format", "{{.Size}}"]).await?;
    if !out.status.success() {
        // Don't silently bypass the guard — surface it so an operator can see the
        // size limit isn't being enforced for this image.
        tracing::warn!(
            "sandbox image size guard: `podman image inspect {image}` failed, not \
            enforcing SANDBOX_IMAGE_MAX_SIZE_MB: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        return Ok(());
    }
    let bytes: u64 = String::from_utf8_lossy(&out.stdout)
        .trim()
        .parse()
        .unwrap_or(0);
    let mb = bytes / 1_000_000;
    if mb > max {
        return Err(Error::ExecutionErr(format!(
            "image {image} is {mb} MB, over the SANDBOX_IMAGE_MAX_SIZE_MB limit of {max} MB"
        )));
    }
    Ok(())
}

#[derive(Deserialize)]
struct PodmanImage {
    #[serde(rename = "Id")]
    id: String,
    #[serde(rename = "Size")]
    size: u64,
    #[serde(rename = "Created")]
    created: i64,
}

/// Best-effort eviction: while the summed size of podman's images exceeds
/// `SANDBOX_IMAGE_CACHE_MAX_MB`, remove the oldest (by created time, an LRU proxy).
/// No-op when the limit is 0 (unset). Skipped if another pass is already running.
/// Images currently backing a container (e.g. a concurrent job mid-extract) fail
/// `rmi` and stop the pass, so in-use images are never removed.
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
    loop {
        let Ok(out) = podman(&["images", "--format", "json"]).await else {
            break;
        };
        if !out.status.success() {
            break;
        }
        let mut imgs: Vec<PodmanImage> = serde_json::from_slice(&out.stdout).unwrap_or_default();
        let total: u64 = imgs.iter().map(|i| i.size).sum();
        if total <= max_bytes || imgs.is_empty() {
            break;
        }
        imgs.sort_by_key(|i| i.created);
        let victim = imgs[0].id.clone();
        match podman(&["rmi", &victim]).await {
            Ok(rm) if rm.status.success() => {
                tracing::info!("sandbox image cache eviction: removed {victim}");
            }
            Ok(rm) => {
                tracing::warn!(
                    "sandbox image cache eviction: cannot remove {victim} (in use?): {}",
                    String::from_utf8_lossy(&rm.stderr)
                );
                break;
            }
            Err(_) => break,
        }
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

    // Best-effort: keep podman's image store under its size cap (overlaps the run).
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
        write_file(
            &rootfs,
            ".windmill_docker_main.sh",
            &format!("set -e\n{content}"),
        )?;
        let mut v = vec![
            "/bin/sh".to_string(),
            "/.windmill_docker_main.sh".to_string(),
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
    use super::{proto_str, registry_qualified, render_envars};

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
