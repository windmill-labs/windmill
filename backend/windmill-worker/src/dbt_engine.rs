//! Provisioning the three dbt engines on a worker.
//!
//! No engine ships in a Windmill image. `dbt-core-1x` and `dbt-core-2x` are
//! Apache 2.0 and an operator may pre-stage either (`DBT_BUNDLED_DIR`); the
//! Fusion engine is **never bundled**. Its license grants only a
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
use crate::handle_child::{get_mem_peak, run_future_with_polling_update_job_poller, JobCtx};

lazy_static::lazy_static! {
    pub static ref DBT_CACHE_DIR: String = format!("{}dbt", *ROOT_CACHE_NOMOUNT_DIR);
    /// Adapters an operator vouches for beyond `PUBLISHED_ADAPTERS`, comma-separated — so a
    /// brand-new adapter needs an admin's decision, not a Windmill release.
    static ref DBT_EXTRA_ADAPTERS: Vec<String> = std::env::var("DBT_EXTRA_ADAPTERS")
        .unwrap_or_default()
        .split(',')
        .map(|a| a.trim().to_ascii_lowercase())
        .filter(|a| !a.is_empty())
        .collect();
    /// Where an operator may pre-stage an Apache-2.0 engine in a derived image.
    /// A persistent image path, unlike the runtime caches, which are a fresh
    /// volume at start — which is the whole reason it is a separate directory.
    static ref DBT_BUNDLED_DIR: String =
        std::env::var("DBT_BUNDLED_DIR").unwrap_or_else(|_| "/usr/local/dbt".to_string());
    static ref UV_PATH: String =
        std::env::var("UV_PATH").unwrap_or_else(|_| "/usr/local/bin/uv".to_string());
    /// Bounds on the dbt-core the 1.x engine resolves. A RANGE, not a pin: the
    /// adapter decides which core it can take, and several cap below the newest
    /// (dbt-oracle and dbt-databricks below 1.12), so pinning core independently
    /// makes those projects unprovisionable. The floor is the CLI this runtime
    /// invokes -- 1.7 rejects `--target` on `parse`, so resolving down to it
    /// produces a working venv that then fails on flags, which is worse than
    /// not resolving at all.
    ///
    /// Every static in this group is env-overridable per instance, so an
    /// operator can move an engine without waiting on a release.
    static ref DBT_CORE_1X_FLOOR: String =
        std::env::var("DBT_CORE_1X_FLOOR").unwrap_or_else(|_| "1.8".to_string());
    static ref DBT_CORE_1X_CEILING: String =
        std::env::var("DBT_CORE_1X_CEILING").unwrap_or_else(|_| "2.0.0".to_string());
    /// Reported as the engine version only when the installed `dist-info` cannot
    /// be read. It pins no install and is not in the cache key -- the resolved
    /// range is -- so it must not be mistaken for the version that gets used.
    static ref DBT_CORE_1X_VERSION: String =
        std::env::var("DBT_CORE_1X_VERSION").unwrap_or_else(|_| "1.12.0".to_string());
    /// Pinned, not ranged: the Rust engine ships its adapters in the binary, so
    /// there is no adapter resolution to accommodate.
    static ref DBT_CORE_2X_VERSION: String =
        std::env::var("DBT_CORE_2X_VERSION").unwrap_or_else(|_| "2.0.0-alpha.5".to_string());
    static ref DBT_PYTHON_VERSION: String =
        std::env::var("DBT_PYTHON_VERSION").unwrap_or_else(|_| "3.12".to_string());
    /// Where the Fusion engine is fetched from. Never a Windmill-hosted mirror:
    /// the point of runtime fetch is that the binary comes from dbt Labs.
    static ref DBT_FUSION_INSTALL_URL: String = std::env::var("DBT_FUSION_INSTALL_URL")
        .unwrap_or_else(|_| "https://public.cdn.getdbt.com/fs/install/install.sh".to_string());
}

pub struct ProvisionedEngine {
    /// Absolute path of the dbt binary to invoke.
    pub bin: PathBuf,
    /// The engine's own directory — the ONLY part of the cache a sandboxed job
    /// may see. Its siblings hold other workspaces' package trees, which are
    /// scoped by cache key, not by permissions.
    pub root: PathBuf,
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
    // What makes the lockfile a lockfile: without it a script silently changes dbt
    // version when the instance upgrades or lands on a different worker. `None` for
    // a deploy, which is what writes the pin.
    pinned_version: Option<&str>,
    pinned_adapter_version: Option<&str>,
    job_id: &Uuid,
    w_id: &str,
    conn: &Connection,
    ctx: &mut JobCtx<'_>,
) -> error::Result<ProvisionedEngine> {
    let pinned_version = checked_version(pinned_version, "engine_version")?;
    let pinned_adapter_version = checked_version(pinned_adapter_version, "adapter_version")?;
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
                ctx,
            )
            .await
        }
        DbtEngine::DbtCore2x => provision_core_2x(pinned_version, job_id, w_id, conn, ctx).await,
        DbtEngine::Fusion => provision_fusion(pinned_version, job_id, w_id, conn, ctx).await,
    }
}

/// A version out of the lockfile, which is CALLER DATA: a preview run submits
/// its own `lock` alongside its content, so nothing about this string has been
/// through a deploy.
///
/// It is interpolated into the engine cache path and into a pip requirement,
/// and provisioning runs on the host rather than inside the dbt jail — a
/// `../..` in it would download an archive and extract it anywhere the worker
/// can write. Accepted only as a plain version token, which every version any
/// of the three engines publishes already is.
fn checked_version<'a>(v: Option<&'a str>, field: &str) -> error::Result<Option<&'a str>> {
    if let Some(v) = v {
        let plain = v.len() <= 64
            && v.starts_with(|c: char| c.is_ascii_alphanumeric())
            && v.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '+'));
        if !plain {
            return Err(Error::BadRequest(format!(
                "the lockfile's `{field}` is not a version: expected a token of letters, digits, \
                 `.`, `-`, `_` or `+` starting with a letter or digit, got `{v}`"
            )));
        }
    }
    Ok(v)
}

/// dbt adapters whose package this worker installs unasked. A PUBLISHED-PACKAGE list, not a
/// capability one: an adapter absent from it still renders a profile and still runs under an
/// engine that already carries it. See `ensure_adapter_installable`.
const PUBLISHED_ADAPTERS: &[&str] = &[
    "athena", "clickhouse", "databricks", "decodable", "doris", "dremio", "duckdb", "exasol",
    "extrica", "fabric", "fabricspark", "firebolt", "glue", "greenplum", "hive", "ibmdb2",
    "impala", "materialize", "mysql", "oracle", "postgres", "redshift", "risingwave", "rockset",
    "singlestore", "snowflake", "spark", "sqlite", "sqlserver", "starrocks", "synapse", "teradata",
    "tidb", "trino", "vertica", "yellowbrick",
];

/// Refuse to install a package nobody vouched for. `dbt-<name>` comes from an adapter name a
/// SCRIPT AUTHOR chooses, `dbt-` is not a reserved PyPI prefix, and this install runs outside
/// the nsjail ordinary dependency installation uses — so an unbounded name would run a PEP
/// 517 backend as the worker. The admin decides what is trusted, via `DBT_EXTRA_ADAPTERS`.
fn ensure_adapter_installable(adapter: &DbtAdapter) -> error::Result<()> {
    let name = adapter.name();
    if adapter.known().is_some()
        || PUBLISHED_ADAPTERS.contains(&name)
        || DBT_EXTRA_ADAPTERS.iter().any(|a| a == name)
    {
        return Ok(());
    }
    Err(Error::BadRequest(format!(
        "`{name}` is not an adapter this instance installs: the dbt-core 1.x engine would have \
         to fetch `dbt-{name}` from PyPI, and `dbt-` is not a reserved name there. An admin adds \
         it to DBT_EXTRA_ADAPTERS, or use an engine that ships its adapters (`engine: fusion`)"
    )))
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
    ctx: &mut JobCtx<'_>,
) -> error::Result<ProvisionedEngine> {
    ensure_adapter_installable(&adapter)?;
    if adapter.pip_package().is_empty() {
        return Err(Error::BadRequest(format!(
            "the {} adapter has no dbt-core 1.x package: it exists only inside the Fusion \
             engine. Set `engine: fusion` to use it",
            adapter.name()
        )));
    }
    // A RANGE, not an `==`. A locked version still pins exactly, because that
    // one was resolved for this adapter in the first place.
    let version_spec = match pinned_version {
        Some(v) => format!("dbt-core=={v}"),
        None => format!(
            "dbt-core>={},<{}",
            DBT_CORE_1X_FLOOR.as_str(),
            DBT_CORE_1X_CEILING.as_str()
        ),
    };
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
    let dir = PathBuf::from(&*DBT_CACHE_DIR).join(format!(
        "core1x-{}-{}",
        digest(&version_spec),
        digest(&adapter_spec)
    ));
    let bin = dir.join("bin").join("dbt");
    if bin.exists() {
        let adapter_version = installed_adapter_version(&dir, adapter).await;
        return Ok(ProvisionedEngine {
            root: dir.clone(),
            bin,
            // What the resolver actually chose, so the lock pins the version
            // this adapter can take rather than the one we asked for.
            version: installed_package_version(&dir, "dbt_core")
                .await
                .unwrap_or(version),
            engine: DbtEngine::DbtCore1x,
            adapter_version,
        });
    }

    append_logs(
        job_id,
        w_id,
        format!(
            "\nProvisioning {version_spec} with {}...\n",
            adapter.pip_package()
        ),
        conn,
    )
    .await;
    // Build beside the target and rename, so two jobs racing on one worker cannot
    // observe a half-installed venv through `bin.exists()`. `--relocatable` makes
    // the rename safe: without it uv bakes the staging path into every shebang and
    // the moved venv's `dbt` fails with ENOENT.
    let staging_guard = Scratch::new(staging_path(&dir, job_id));
    let staging = staging_guard.path().to_path_buf();
    tokio::fs::remove_dir_all(&staging).await.ok();
    run_tool(
        Command::new(UV_PATH.as_str())
            // The interpreter goes in Windmill's own cache rather than the
            // worker user's home: a sandboxed job can only see the paths that
            // are mounted into it, and this is one of them.
            .env("UV_PYTHON_INSTALL_DIR", crate::PY_INSTALL_DIR.as_str())
            .args([
                "venv",
                "--relocatable",
                "--python",
                DBT_PYTHON_VERSION.as_str(),
            ])
            .arg(&staging),
        "uv venv",
        job_id,
        w_id,
        conn,
        ctx,
    )
    .await?;
    run_tool(
        Command::new(UV_PATH.as_str())
            .env("UV_PYTHON_INSTALL_DIR", crate::PY_INSTALL_DIR.as_str())
            .env("VIRTUAL_ENV", &staging)
            .args(["pip", "install", &version_spec, &adapter_spec]),
        "uv pip install",
        job_id,
        w_id,
        conn,
        ctx,
    )
    .await
    .map_err(|e| {
        // The common cause is an adapter with no release for this dbt-core
        // range: dbt-mysql, for one, has not shipped past `~=1.7`. uv reports
        // that as a resolver dump, which does not say what to do about it.
        Error::ExecutionErr(format!(
            "{e}\n\ninstalling the {} adapter: it must have a release compatible with \
             {version_spec}, which is the dbt-core CLI this engine invokes. If the adapter has \
             not kept up, use `engine: dbt-core-2x` or `engine: fusion`",
            adapter.name()
        ))
    })?;
    match tokio::fs::rename(&staging, &dir).await {
        Ok(()) => staging_guard.keep(),
        // Lost the race: the winner's venv is equivalent, so use it. The guard
        // removes ours.
        Err(_) if bin.exists() => {}
        Err(e) => return Err(Error::internal_err(format!("installing dbt-core: {e}"))),
    }
    let adapter_version = installed_adapter_version(&dir, adapter).await;
    let version = installed_package_version(&dir, "dbt_core")
        .await
        .unwrap_or(version);
    Ok(
        ProvisionedEngine {
            root: dir,
            bin,
            version,
            engine: DbtEngine::DbtCore1x,
            adapter_version,
        },
    )
}

/// The adapter version a venv actually resolved, read from its dist-info so a
/// deploy can lock it and later runs can ask for the same one.
async fn installed_adapter_version(dir: &Path, adapter: DbtAdapter) -> Option<String> {
    installed_package_version(dir, &adapter.pip_package().replace('-', "_")).await
}

/// The version of an installed distribution, read from its `.dist-info`.
async fn installed_package_version(dir: &Path, dist: &str) -> Option<String> {
    let prefix = format!("{dist}-");
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
    ctx: &mut JobCtx<'_>,
) -> error::Result<ProvisionedEngine> {
    let version = pinned_version
        .map(str::to_string)
        .unwrap_or_else(|| DBT_CORE_2X_VERSION.clone());
    // No Windmill image ships an engine, so this is the operator's own
    // pre-stage: `DBT_BUNDLED_DIR` populated in a derived image, for an
    // air-gapped instance or a fleet that should not fetch per worker. Checked
    // before the cache because it is read-only and shared, where the cache is a
    // per-worker volume.
    let bundled = PathBuf::from(&*DBT_BUNDLED_DIR)
        .join(format!("core2x-{version}"))
        .join("dbt-sa-cli");
    if bundled.exists() {
        return Ok(ProvisionedEngine {
            root: bundled.parent().map(Path::to_path_buf).unwrap_or_default(),
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
            root: dir.clone(),
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
    fetch_and_extract(&url, &dir, "dbt-sa-cli", job_id, w_id, conn, ctx).await?;
    Ok(ProvisionedEngine {
        root: dir,
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
    ctx: &mut JobCtx<'_>,
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
            root: dir.clone(),
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
    let script = fetch_under_job(
        "the Fusion installer",
        async {
            let net =
                |e: reqwest::Error| Error::internal_err(format!("fetching the installer: {e}"));
            Ok(windmill_common::utils::HTTP_CLIENT
                .get(&*DBT_FUSION_INSTALL_URL)
                .send()
                .await
                .and_then(|r| r.error_for_status())
                .map_err(net)?
                .text()
                .await
                .map_err(net)?)
        },
        job_id,
        w_id,
        conn,
        ctx,
    )
    .await?;
    // Install into a per-job sibling and rename, like the other two engines:
    // pointing the installer straight at the shared cache lets a second job
    // observe `bin/dbt` and execute it while the first is still writing.
    let staging_guard = Scratch::new(staging_path(&dir, job_id));
    let staging = staging_guard.path().to_path_buf();
    tokio::fs::remove_dir_all(&staging).await.ok();
    let script_guard =
        Scratch::new(std::env::temp_dir().join(format!("wm-fusion-install-{job_id}.sh")));
    let tmp = script_guard.path().to_path_buf();
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
    run_tool(&mut install, "fusion install", job_id, w_id, conn, ctx).await?;
    drop(script_guard);
    if !staging.join("dbt").exists() {
        return Err(Error::internal_err(
            "the Fusion installer did not produce a dbt binary".to_string(),
        ));
    }
    if tokio::fs::rename(&staging, &dir).await.is_ok() {
        staging_guard.keep();
    } else {
        if !bin.exists() {
            return Err(Error::internal_err(
                "could not install the Fusion engine".to_string(),
            ));
        }
    }
    Ok(ProvisionedEngine {
        root: dir.clone(),
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
/// A per-job path removed on drop unless the install claimed it.
///
/// Provisioning is a sequence of fallible awaits — a download, an installer, an
/// extraction — and each one can also be cancelled. Cleaning up after the `?`
/// only covers the paths someone remembered, so a run of failed or cancelled
/// first-use installs accumulates venvs and tarballs until the worker's disk is
/// gone. Dropping is the one exit every path takes.
struct Scratch(Option<PathBuf>);

impl Scratch {
    fn new(path: PathBuf) -> Self {
        Scratch(Some(path))
    }

    fn path(&self) -> &Path {
        self.0.as_deref().unwrap_or(Path::new(""))
    }

    /// The install succeeded and moved it: stop owning it.
    fn keep(mut self) {
        self.0 = None;
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let Some(p) = self.0.take() else {
            return;
        };
        let remove = move || {
            let _ = std::fs::remove_dir_all(&p);
            let _ = std::fs::remove_file(&p);
        };
        // Off the runtime thread: this removes thousands of files and `Drop` runs
        // inside the job future, so doing it synchronously stalls every other job
        // on that thread. Detached because a `Drop` cannot await.
        match tokio::runtime::Handle::try_current() {
            Ok(handle) => {
                handle.spawn_blocking(remove);
            }
            // No runtime to hand it to (a test, or shutdown): do it here rather
            // than leave the directory behind.
            Err(_) => remove(),
        }
    }
}

fn staging_path(dir: &Path, job_id: &Uuid) -> PathBuf {
    let name = dir.file_name().unwrap_or_default().to_string_lossy();
    dir.with_file_name(format!("{name}.staging-{job_id}"))
}

/// Fetch under the job's cancellation and timeout. The shared `HTTP_CLIENT`
/// sets no request timeout, so a stalled release download would otherwise hold
/// the worker for as long as the connection stays open.
async fn fetch_under_job<T, F>(
    what: &str,
    fetch: F,
    job_id: &Uuid,
    w_id: &str,
    conn: &Connection,
    ctx: &mut JobCtx<'_>,
) -> error::Result<T>
where
    F: std::future::Future<Output = error::Result<T>>,
{
    run_future_with_polling_update_job_poller(
        *job_id,
        ctx.timeout(),
        conn,
        ctx.mem_peak,
        ctx.canceled_by,
        async move {
            fetch
                .await
                .map_err(|e| Error::internal_err(format!("fetching {what}: {e}")))
        },
        ctx.worker_name,
        w_id,
        &mut Some(ctx.occupancy_metrics),
        Box::pin(futures::stream::once(async { 0 })),
    )
    .await
}

async fn fetch_and_extract(
    url: &str,
    dir: &Path,
    expected_bin: &str,
    job_id: &Uuid,
    w_id: &str,
    conn: &Connection,
    ctx: &mut JobCtx<'_>,
) -> error::Result<()> {
    let tarball_guard = Scratch::new(std::env::temp_dir().join(format!("wm-dbt-{job_id}.tar.gz")));
    let tarball = tarball_guard.path().to_path_buf();
    // STREAMED to disk, never held whole: an engine archive is ~290 MB, this
    // runs in the shared worker process rather than the job's subprocess, and
    // several cold jobs provision at once — buffering would multiply that until
    // the worker dies, taking every job on it along.
    fetch_under_job(
        url,
        async {
            use futures::StreamExt;
            use tokio::io::AsyncWriteExt;
            let net = |e: reqwest::Error| Error::internal_err(format!("fetching {url}: {e}"));
            let mut resp = windmill_common::utils::HTTP_CLIENT
                .get(url)
                .send()
                .await
                .and_then(|r| r.error_for_status())
                .map_err(net)?
                .bytes_stream();
            let mut file = tokio::fs::File::create(&tarball)
                .await
                .map_err(|e| Error::internal_err(format!("writing {url}: {e}")))?;
            while let Some(chunk) = resp.next().await {
                let chunk = chunk.map_err(net)?;
                file.write_all(&chunk)
                    .await
                    .map_err(|e| Error::internal_err(format!("writing {url}: {e}")))?;
            }
            file.flush()
                .await
                .map_err(|e| Error::internal_err(format!("writing {url}: {e}")))?;
            Ok(())
        },
        job_id,
        w_id,
        conn,
        ctx,
    )
    .await?;
    let staging_guard = Scratch::new(staging_path(dir, job_id));
    let staging = staging_guard.path().to_path_buf();
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
        job_id,
        w_id,
        conn,
        ctx,
    )
    .await?;
    drop(tarball_guard);
    if !staging.join(expected_bin).exists() {
        return Err(Error::internal_err(format!(
            "{url} did not contain the expected `{expected_bin}` binary"
        )));
    }
    if tokio::fs::rename(&staging, dir).await.is_ok() {
        staging_guard.keep();
    } else {
        if !dir.join(expected_bin).exists() {
            return Err(Error::internal_err(format!("could not install {url}")));
        }
    }
    Ok(())
}

/// Run a provisioning command to completion. Provisioning happens inside the
/// job that needs the engine, so it runs under that job's cancellation and
/// timeout: a cold `uv pip install` or Fusion download is the longest thing a
/// dbt job does, and a cancel that could not reach it would hold the worker
/// slot for the rest of the install. Its output is not streamed to the job log
/// — these are worker-level setup steps with no node to attribute progress to,
/// and failures surface with the tool's own stderr.
async fn run_tool(
    cmd: &mut Command,
    name: &str,
    job_id: &Uuid,
    w_id: &str,
    conn: &Connection,
    ctx: &mut JobCtx<'_>,
) -> error::Result<()> {
    let child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // The wait future owns the child, so cancellation dropping that future
        // is what terminates the install — dropping it alone would not.
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| Error::internal_err(format!("{name} could not be started: {e}")))?;
    let pid = child.id();
    let out = run_future_with_polling_update_job_poller(
        *job_id,
        ctx.timeout(),
        conn,
        ctx.mem_peak,
        ctx.canceled_by,
        async move {
            child
                .wait_with_output()
                .await
                .map_err(|e| Error::internal_err(format!("{name} failed: {e}")))
        },
        ctx.worker_name,
        w_id,
        &mut Some(ctx.occupancy_metrics),
        Box::pin(futures::stream::unfold((), move |_| async move {
            Some((get_mem_peak(pid, false).await, ()))
        })),
    )
    .await?;
    if !out.status.success() {
        return Err(Error::internal_err(format!(
            "{name} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        )));
    }
    Ok(())
}

#[cfg(test)]
mod core1x_tests {
    use super::*;
    use crate::dbt_profiles::KnownAdapter;

    // Several adapters cap dbt-core below what this runtime would ask for
    // (dbt-mysql ~=1.7, dbt-oracle and dbt-databricks below 1.12) and
    // dbt-salesforce has no 1.x package at all, so pinning core independently made
    // those projects fail at provisioning. The install names a ceiling instead.
    #[test]
    fn every_adapter_either_names_a_package_or_is_fusion_only() {
        for a in KnownAdapter::ALL {
            if matches!(a, KnownAdapter::Salesforce) {
                continue;
            }
            assert!(
                !a.pip_package().is_empty(),
                "{} must name a pip package for dbt-core 1.x",
                a.name()
            );
        }
        // Fusion has it built in, and there is no package to install.
        assert!(KnownAdapter::Salesforce.pip_package().is_empty());
        assert_eq!(KnownAdapter::Salesforce.name(), "salesforce");
    }

    // `dbt-` is not a reserved prefix on PyPI and this install is not sandboxed,
    // so the name a script author picks decides which package runs its build
    // backend as the worker.
    #[test]
    fn an_unvouched_adapter_is_not_installed() {
        let known = DbtAdapter::from_dbt_type("postgres").unwrap();
        assert!(ensure_adapter_installable(&known).is_ok());
        let published = DbtAdapter::from_dbt_type("trino").unwrap();
        assert!(ensure_adapter_installable(&published).is_ok());
        let squatted = DbtAdapter::from_dbt_type("totally-legit-adapter").unwrap();
        assert!(ensure_adapter_installable(&squatted).is_err());
    }

    /// A preview submits its own lockfile, so this string reaches a path join
    /// and a pip requirement straight from the caller, on the host and outside
    /// the jail.
    #[test]
    fn a_lockfile_version_cannot_leave_the_cache_directory() {
        for v in ["1.12.0", "2.0.0-alpha.5", "2.0.0-preview.202"] {
            assert_eq!(checked_version(Some(v), "engine_version").unwrap(), Some(v));
        }
        for v in [
            "../../../../etc",
            "1.0/../..",
            "..",
            ".ssh",
            "-rf",
            "1.0 --index-url=http://x",
            "1.0\n",
        ] {
            assert!(
                checked_version(Some(v), "engine_version").is_err(),
                "`{v}` must be refused before it reaches a path"
            );
        }
        assert!(checked_version(None, "engine_version").unwrap().is_none());
    }
}
