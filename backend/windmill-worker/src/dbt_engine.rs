//! Provisioning the three dbt engines on a worker.
//!
//! `dbt-core-1x` and `dbt-core-2x` are Apache 2.0 and may be baked into the
//! images; the Fusion engine is **never bundled**. Its license grants only a
//! non-transferable, non-sublicensable redistribution right and forbids
//! interposing on Provider-to-End-User communication, which is exactly what
//! shipping it inside a job runner would do. The mitigation is that the user's
//! own instance fetches it from dbt Labs on first use, so Windmill never
//! redistributes it (docs/dbt-runtime.md, decision 1).
//!
//! Everything lands in a worker-global cache keyed by engine and version, so
//! the fetch happens once per worker rather than once per job.

use std::path::{Path, PathBuf};
use std::process::Stdio;

use tokio::process::Command;
use uuid::Uuid;
use windmill_common::error::{self, Error};
use windmill_common::worker::{write_file, Connection, ROOT_CACHE_NOMOUNT_DIR};
use windmill_parser_yaml::DbtEngine;
use windmill_queue::append_logs;

use crate::dbt_executor::digest;
use crate::dbt_profiles::DbtAdapter;

lazy_static::lazy_static! {
    pub static ref DBT_CACHE_DIR: String = format!("{}dbt", *ROOT_CACHE_NOMOUNT_DIR);
    /// Where the full images bake the Apache-2.0 engines. A persistent image
    /// path, unlike the runtime caches, which are a fresh volume at start.
    static ref DBT_BUNDLED_DIR: String =
        std::env::var("DBT_BUNDLED_DIR").unwrap_or_else(|_| "/usr/local/dbt".to_string());
    static ref UV_PATH: String =
        std::env::var("UV_PATH").unwrap_or_else(|_| "/usr/local/bin/uv".to_string());
    /// Pinned so a worker's engine does not drift under running projects. Both
    /// are overridable per instance for upgrades without a release.
    static ref DBT_CORE_1X_VERSION: String =
        std::env::var("DBT_CORE_1X_VERSION").unwrap_or_else(|_| "1.12.0".to_string());
    static ref DBT_CORE_2X_VERSION: String =
        std::env::var("DBT_CORE_2X_VERSION").unwrap_or_else(|_| "2.0.0-alpha.5".to_string());
    static ref DBT_PYTHON_VERSION: String =
        std::env::var("DBT_PYTHON_VERSION").unwrap_or_else(|_| "3.12".to_string());
    /// Where the Fusion engine is fetched from. Never a Windmill-hosted mirror:
    /// the point of runtime fetch is that the binary comes from dbt Labs.
    static ref DBT_FUSION_INSTALL_URL: String = std::env::var("DBT_FUSION_INSTALL_URL")
        .unwrap_or_else(|_| "https://public.cdn.getdbt.com/fs/install/install.sh".to_string());
}

/// Bound on engine provisioning. Generous — a cold `uv pip install` of dbt-core
/// plus an adapter is minutes — but finite, unlike a stalled index.
const PROVISION_TIMEOUT_SECS: u64 = 1800;

pub struct ProvisionedEngine {
    /// Absolute path of the dbt binary to invoke.
    pub bin: PathBuf,
    pub version: String,
    pub engine: DbtEngine,
    /// The adapter version this venv resolved, for dbt-core 1.x where the
    /// adapter is a separate package that versions independently of core.
    /// `None` for the Rust engines, which ship their adapters in the binary.
    pub adapter_version: Option<String>,
}

/// Ensure the engine is present on this worker and return how to invoke it.
pub async fn provision_engine(
    engine: DbtEngine,
    adapter: DbtAdapter,
    // The version the lockfile pinned at deploy. Honoring it is what makes the
    // lockfile a lockfile: without it a script silently changes dbt version
    // when the instance upgrades or lands on a differently configured worker.
    // `None` for a deploy (which is what writes the pin) and for a script whose
    // lock predates it.
    pinned_version: Option<&str>,
    pinned_adapter_version: Option<&str>,
    job_id: &Uuid,
    w_id: &str,
    conn: &Connection,
) -> error::Result<ProvisionedEngine> {
    tokio::fs::create_dir_all(&*DBT_CACHE_DIR).await.ok();
    match engine {
        DbtEngine::DbtCore1x => {
            provision_core_1x(
                adapter,
                pinned_version,
                pinned_adapter_version,
                job_id,
                w_id,
                conn,
            )
            .await
        }
        DbtEngine::DbtCore2x => provision_core_2x(pinned_version, job_id, w_id, conn).await,
        DbtEngine::Fusion => provision_fusion(pinned_version, job_id, w_id, conn).await,
    }
}

/// A uv venv per (dbt version, adapter): the adapter is a separate pip package
/// and installing every adapter into one venv would make their transitive
/// dependency sets fight.
async fn provision_core_1x(
    adapter: DbtAdapter,
    pinned_version: Option<&str>,
    pinned_adapter_version: Option<&str>,
    job_id: &Uuid,
    w_id: &str,
    conn: &Connection,
) -> error::Result<ProvisionedEngine> {
    let version = pinned_version
        .map(str::to_string)
        .unwrap_or_else(|| DBT_CORE_1X_VERSION.clone());
    // The adapter is in the cache key: pinning core alone would let a rebuilt
    // cache resolve a newer adapter than the deploy did, which changes runtime
    // behavior under a lockfile that claims to prevent exactly that.
    let adapter_spec = match pinned_adapter_version {
        Some(v) => format!("{}=={v}", adapter.pip_package()),
        None => adapter.pip_package().to_string(),
    };
    let dir = PathBuf::from(&*DBT_CACHE_DIR)
        .join(format!("core1x-{version}-{}", digest(&adapter_spec)));
    let bin = dir.join("bin").join("dbt");
    if bin.exists() {
        let adapter_version = installed_adapter_version(&dir, adapter).await;
        return Ok(ProvisionedEngine {
            bin,
            version,
            engine: DbtEngine::DbtCore1x,
            adapter_version,
        });
    }

    append_logs(
        job_id,
        w_id,
        format!(
            "\nProvisioning dbt-core {version} with {}...\n",
            adapter.pip_package()
        ),
        conn,
    )
    .await;
    // Build beside the target and rename: two jobs racing on the same worker
    // must not observe a half-installed venv through the `bin.exists()` check.
    // `--relocatable` is what makes that rename safe — without it uv bakes the
    // staging path into every entry point's shebang and the moved venv's `dbt`
    // fails with ENOENT.
    let staging = staging_path(&dir, job_id);
    tokio::fs::remove_dir_all(&staging).await.ok();
    run_tool(
        Command::new(UV_PATH.as_str())
            .args([
                "venv",
                "--relocatable",
                "--python",
                DBT_PYTHON_VERSION.as_str(),
            ])
            .arg(&staging),
        "uv venv",
    )
    .await?;
    run_tool(
        Command::new(UV_PATH.as_str())
            .env("VIRTUAL_ENV", &staging)
            .args(["pip", "install", &format!("dbt-core=={version}"), &adapter_spec]),
        "uv pip install",
    )
    .await?;
    match tokio::fs::rename(&staging, &dir).await {
        Ok(()) => {}
        // Lost the race: the winner's venv is equivalent, so use it.
        Err(_) if bin.exists() => {
            tokio::fs::remove_dir_all(&staging).await.ok();
        }
        Err(e) => return Err(Error::internal_err(format!("installing dbt-core: {e}"))),
    }
    let adapter_version = installed_adapter_version(&dir, adapter).await;
    Ok(ProvisionedEngine {
        bin,
        version,
        engine: DbtEngine::DbtCore1x,
        adapter_version,
    })
}

/// The adapter version a venv actually resolved, read from its dist-info so a
/// deploy can lock it and later runs can ask for the same one.
async fn installed_adapter_version(dir: &Path, adapter: DbtAdapter) -> Option<String> {
    let prefix = format!("{}-", adapter.pip_package().replace('-', "_"));
    let mut entries = tokio::fs::read_dir(dir.join("lib")).await.ok()?;
    while let Ok(Some(py)) = entries.next_entry().await {
        let mut pkgs = tokio::fs::read_dir(py.path().join("site-packages"))
            .await
            .ok()?;
        while let Ok(Some(e)) = pkgs.next_entry().await {
            let name = e.file_name().to_string_lossy().to_string();
            if let Some(rest) = name.strip_prefix(&prefix) {
                if let Some(v) = rest.strip_suffix(".dist-info") {
                    return Some(v.to_string());
                }
            }
        }
    }
    None
}

async fn provision_core_2x(
    pinned_version: Option<&str>,
    job_id: &Uuid,
    w_id: &str,
    conn: &Connection,
) -> error::Result<ProvisionedEngine> {
    let version = pinned_version
        .map(str::to_string)
        .unwrap_or_else(|| DBT_CORE_2X_VERSION.clone());
    // The full images bake this engine in; only a slim image pays the fetch.
    let bundled = PathBuf::from(&*DBT_BUNDLED_DIR)
        .join(format!("core2x-{version}"))
        .join("dbt-sa-cli");
    if bundled.exists() {
        return Ok(ProvisionedEngine {
            bin: bundled,
            version,
            engine: DbtEngine::DbtCore2x,
            adapter_version: None,
        });
    }
    let dir = PathBuf::from(&*DBT_CACHE_DIR).join(format!("core2x-{version}"));
    let bin = dir.join("dbt-sa-cli");
    if bin.exists() {
        return Ok(ProvisionedEngine {
            bin,
            version,
            engine: DbtEngine::DbtCore2x,
            adapter_version: None,
        });
    }
    let target = format!("{}-unknown-linux-gnu", std::env::consts::ARCH);
    let url = format!(
        "https://github.com/dbt-labs/dbt-core/releases/download/v{version}/dbt-core-{version}-{target}.tar.gz"
    );
    append_logs(
        job_id,
        w_id,
        format!("\nFetching dbt-core {version}...\n"),
        conn,
    )
    .await;
    fetch_and_extract(&url, &dir, "dbt-sa-cli", job_id).await?;
    Ok(ProvisionedEngine {
        bin,
        version,
        engine: DbtEngine::DbtCore2x,
        adapter_version: None,
    })
}

/// Fusion is fetched from dbt Labs at runtime and cached — see the module docs
/// for why it must never be baked into an image.
async fn provision_fusion(
    pinned_version: Option<&str>,
    job_id: &Uuid,
    w_id: &str,
    conn: &Connection,
) -> error::Result<ProvisionedEngine> {
    // Version-keyed like the other two: one shared `fusion` directory means a
    // run landing on a clean worker fetches whatever is current rather than
    // what the deploy locked.
    let dir = PathBuf::from(&*DBT_CACHE_DIR).join(match pinned_version {
        Some(v) => format!("fusion-{v}"),
        None => "fusion".to_string(),
    });
    // The installer places the binary directly in the directory `--to` names.
    let bin = dir.join("dbt");
    if bin.exists() {
        return Ok(ProvisionedEngine {
            bin,
            version: fusion_version(&dir).await,
            engine: DbtEngine::Fusion,
            adapter_version: None,
        });
    }
    append_logs(
        job_id,
        w_id,
        "\nFetching the dbt Fusion engine from dbt Labs (not bundled with Windmill; \
         subject to the dbt Fusion engine license agreement)...\n"
            .to_string(),
        conn,
    )
    .await;
    let script = windmill_common::utils::HTTP_CLIENT
        .get(&*DBT_FUSION_INSTALL_URL)
        .send()
        .await
        .map_err(|e| Error::internal_err(format!("fetching the Fusion installer: {e}")))?
        .error_for_status()
        .map_err(|e| Error::internal_err(format!("fetching the Fusion installer: {e}")))?
        .text()
        .await
        .map_err(|e| Error::internal_err(format!("fetching the Fusion installer: {e}")))?;
    // Install into a per-job sibling and rename, like the other two engines:
    // pointing the installer straight at the shared cache lets a second job
    // observe `bin/dbt` and execute it while the first is still writing.
    let staging = staging_path(&dir, job_id);
    tokio::fs::remove_dir_all(&staging).await.ok();
    let tmp = std::env::temp_dir().join(format!("wm-fusion-install-{job_id}.sh"));
    write_file(
        tmp.parent().unwrap().to_str().unwrap(),
        tmp.file_name().unwrap().to_str().unwrap(),
        &script,
    )?;
    // `--to` and `--version` are the installer's own flags (`install.sh
    // --help`); it takes no positional arguments and ignores unknown ones, so
    // an approximation here fails by silently installing the latest release
    // into the user's $HOME instead of the cache.
    let mut install = Command::new("sh");
    install.arg(&tmp).arg("--to").arg(&staging);
    if let Some(v) = pinned_version {
        install.args(["--version", v]);
    } else {
        install.arg("--update");
    }
    run_tool(&mut install, "fusion install").await?;
    tokio::fs::remove_file(&tmp).await.ok();
    if !staging.join("dbt").exists() {
        tokio::fs::remove_dir_all(&staging).await.ok();
        return Err(Error::internal_err(
            "the Fusion installer did not produce a dbt binary".to_string(),
        ));
    }
    if tokio::fs::rename(&staging, &dir).await.is_err() {
        tokio::fs::remove_dir_all(&staging).await.ok();
        if !bin.exists() {
            return Err(Error::internal_err(
                "could not install the Fusion engine".to_string(),
            ));
        }
    }
    Ok(ProvisionedEngine {
        bin,
        version: fusion_version(&dir).await,
        engine: DbtEngine::Fusion,
        adapter_version: None,
    })
}

/// From the binary itself: the installer writes no version file, and a pinned
/// directory name would only echo back what was asked for rather than what was
/// installed.
async fn fusion_version(dir: &Path) -> String {
    Command::new(dir.join("dbt"))
        .arg("--version")
        .output()
        .await
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| {
            String::from_utf8_lossy(&o.stdout)
                .split_whitespace()
                .nth(1)
                .map(|v| v.to_string())
        })
        .unwrap_or_else(|| "unknown".to_string())
}

/// A sibling of `dir` to build in before renaming into place. Appended to the
/// whole file name rather than via `with_extension`, which would eat everything
/// after the version's last dot.
fn staging_path(dir: &Path, job_id: &Uuid) -> PathBuf {
    let name = dir.file_name().unwrap_or_default().to_string_lossy();
    dir.with_file_name(format!("{name}.staging-{job_id}"))
}

async fn fetch_and_extract(
    url: &str,
    dir: &Path,
    expected_bin: &str,
    job_id: &Uuid,
) -> error::Result<()> {
    let bytes = windmill_common::utils::HTTP_CLIENT
        .get(url)
        .send()
        .await
        .map_err(|e| Error::internal_err(format!("fetching {url}: {e}")))?
        .error_for_status()
        .map_err(|e| Error::internal_err(format!("fetching {url}: {e}")))?
        .bytes()
        .await
        .map_err(|e| Error::internal_err(format!("fetching {url}: {e}")))?;
    let tarball = std::env::temp_dir().join(format!("wm-dbt-{job_id}.tar.gz"));
    tokio::fs::write(&tarball, &bytes)
        .await
        .map_err(|e| Error::internal_err(format!("writing {url}: {e}")))?;
    let staging = staging_path(dir, job_id);
    tokio::fs::create_dir_all(&staging)
        .await
        .map_err(|e| Error::internal_err(format!("creating {staging:?}: {e}")))?;
    run_tool(
        Command::new("tar")
            .arg("xzf")
            .arg(&tarball)
            .arg("-C")
            .arg(&staging)
            // The release tarballs wrap the binary in a versioned directory.
            .args(["--strip-components", "1"]),
        "tar",
    )
    .await?;
    tokio::fs::remove_file(&tarball).await.ok();
    if !staging.join(expected_bin).exists() {
        return Err(Error::internal_err(format!(
            "{url} did not contain the expected `{expected_bin}` binary"
        )));
    }
    if tokio::fs::rename(&staging, dir).await.is_err() {
        tokio::fs::remove_dir_all(&staging).await.ok();
        if !dir.join(expected_bin).exists() {
            return Err(Error::internal_err(format!("could not install {url}")));
        }
    }
    Ok(())
}

/// Run a provisioning command to completion. These are worker-level setup steps
/// with no job to attribute progress to, so they are not routed through
/// `handle_child`; failures surface with the tool's own stderr.
async fn run_tool(cmd: &mut Command, name: &str) -> error::Result<()> {
    // Bounded: these fetch from PyPI, GitHub or dbt Labs, and an unreachable
    // endpoint would otherwise hold the worker slot indefinitely.
    let out = tokio::time::timeout(
        std::time::Duration::from_secs(PROVISION_TIMEOUT_SECS),
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).output(),
    )
    .await
    .map_err(|_| {
        Error::ExecutionErr(format!("{name} did not finish within {PROVISION_TIMEOUT_SECS}s"))
    })?
    .map_err(|e| Error::internal_err(format!("{name} could not be started: {e}")))?;
    if !out.status.success() {
        return Err(Error::internal_err(format!(
            "{name} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        )));
    }
    Ok(())
}
