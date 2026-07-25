//! Running a dbt project as a Windmill job.
//!
//! One `dbt build` per job, not one job per model. That is the shape
//! astronomer-cosmos arrived at with `ExecutionMode.WATCHER` after per-model
//! Airflow tasks proved roughly 6x slower on a real project; dbt's own
//! threading provides the parallelism and Windmill provides the observability
//! (docs/dbt-runtime.md). Per-model status comes from dbt's JSON event stream
//! while the run is in flight, and the structured job result comes from
//! `run_results.json` at the end.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use sha2::{Digest, Sha256};
use tokio::process::Command;
use uuid::Uuid;
use windmill_common::client::AuthedClient;
use windmill_common::error::{self, Error};
use windmill_common::materialization::{
    record_materialization, MaterializationStatus, RecordMaterializationRequest,
};
use windmill_common::worker::{to_raw_value, write_file, Connection};
use windmill_parser_yaml::{parse_dbt_descriptor, DbtDescriptor, DbtTestBehavior, GitRepo};
use windmill_queue::{append_logs, CanceledBy, MiniPulledJob};

use crate::common::{start_child_process, OccupancyMetrics};
use crate::dbt_engine::{provision_engine, ProvisionedEngine, DBT_CACHE_DIR};
use crate::dbt_profiles::{ensure_adapter_licensed, render_profile, DbtAdapter};
use crate::git_clone::{
    clone_repo, clone_repo_without_history, get_git_repo_full_head_commit_hash,
    resolve_git_ref_to_commit,
};
use crate::handle_child::handle_child;
use crate::{GIT_PATH, PATH_ENV, PROXY_ENVS, TZ_ENV};

/// The profile name Windmill renders into `profiles.yml`. dbt takes the profile
/// to use from `dbt_project.yml`, so the rendered file must answer to whatever
/// name the project declares — resolved from the project file, with this as the
/// fallback for the (invalid) case where it declares none.
const FALLBACK_PROFILE_NAME: &str = "windmill";

/// Written to the script's lockfile at deploy. `commit` is empty under
/// `ref: latest`, which resolves HEAD per run by design (decision 5/12).
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct DbtDependencyLocks {
    pub repo_url: String,
    pub commit: String,
    pub manifest_digest: String,
    pub engine: String,
    pub engine_version: String,
}

/// Per-node outcome, from `run_results.json`.
#[derive(Serialize, Debug, Clone)]
pub struct DbtNodeResult {
    pub unique_id: String,
    pub status: String,
    pub execution_time: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rows_affected: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relation_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Test nodes: how many rows violated the assertion.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failures: Option<i64>,
}

/// The job result. Partial failure is dbt's normal case, so the result has to
/// be legible without reading the log: which models succeeded, which failed,
/// which tests failed and at what severity.
#[derive(Serialize, Debug)]
pub struct DbtRunResult {
    pub engine: String,
    pub engine_version: String,
    pub commit: String,
    pub command: String,
    pub totals: DbtTotals,
    pub nodes: Vec<DbtNodeResult>,
}

#[derive(Serialize, Debug, Default)]
pub struct DbtTotals {
    pub total: usize,
    pub success: usize,
    pub error: usize,
    pub warn: usize,
    pub skipped: usize,
}

#[derive(Deserialize, Debug)]
struct RunResults {
    #[serde(default)]
    results: Vec<RunResultNode>,
}

#[derive(Deserialize, Debug)]
struct RunResultNode {
    unique_id: String,
    status: String,
    #[serde(default)]
    execution_time: Option<f64>,
    #[serde(default)]
    adapter_response: serde_json::Value,
    #[serde(default)]
    relation_name: Option<String>,
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    failures: Option<i64>,
}

pub async fn handle_dbt_job(
    requirements_o: Option<&String>,
    job_dir: &str,
    worker_name: &str,
    job: &MiniPulledJob,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    conn: &Connection,
    client: &AuthedClient,
    inner_content: &str,
    base_internal_url: &str,
    envs: HashMap<String, String>,
    occupancy_metrics: &mut OccupancyMetrics,
) -> error::Result<Box<RawValue>> {
    let descriptor = parse_dbt_descriptor(inner_content)?;
    let locks: Option<DbtDependencyLocks> =
        requirements_o.and_then(|s| serde_json::from_str(s).ok());

    let args = job.args.as_ref().map(|a| a.0.clone()).unwrap_or_default();
    let prepared = prepare_project(
        &descriptor,
        locks.as_ref(),
        &args,
        job_dir,
        &job.id,
        &job.workspace_id,
        worker_name,
        conn,
        client,
        mem_peak,
        canceled_by,
        occupancy_metrics,
        base_internal_url,
    )
    .await?;

    let command = arg_str(&args, "dbt_command").unwrap_or_else(|| match descriptor.test_behavior {
        DbtTestBehavior::Build => "build".to_string(),
        DbtTestBehavior::AfterAll => "run".to_string(),
        DbtTestBehavior::None => "run".to_string(),
    });
    // `dbt retry` resumes from the previous run's `run_results.json`, which is
    // what makes one-job-per-invocation defensible: a partial failure does not
    // force a full rebuild. Windmill runs each attempt in a fresh job dir, so
    // the previous run's `target/` is restored from the worker-local state
    // cache before invoking it.
    if command == "retry" {
        restore_run_state(&prepared, &job.workspace_id).await?;
    }

    let mut run = run_dbt(
        &prepared,
        &command,
        &descriptor,
        &args,
        job,
        conn,
        mem_peak,
        canceled_by,
        occupancy_metrics,
        &envs,
        worker_name,
        true,
    )
    .await;

    // `after_all` is two invocations: models first, then tests, so a test
    // failure does not stop the models that were going to build anyway.
    if run.is_ok()
        && matches!(descriptor.test_behavior, DbtTestBehavior::AfterAll)
        && command == "run"
    {
        run = run_dbt(
            &prepared,
            "test",
            &descriptor,
            &args,
            job,
            conn,
            mem_peak,
            canceled_by,
            occupancy_metrics,
            &envs,
            worker_name,
            false,
        )
        .await;
    }

    let results = read_run_results(&prepared.project_dir).await;
    save_run_state(&prepared, &job.workspace_id).await.ok();
    reconcile_materializations(&prepared, &results, job, conn).await;

    // `ref: latest` executes whatever HEAD resolved to, so the graph must be
    // refreshed from this run's own manifest or it describes a different
    // commit than the one that ran (decision 12). The run already produced
    // `manifest.json`, so this costs no extra dbt invocation.
    if descriptor.is_latest_ref() {
        if let Err(e) = ingest_from_run(&prepared, &descriptor, job, conn).await {
            tracing::warn!("dbt graph refresh after a `latest` run failed: {e:#}");
        }
    }

    let result = build_result(&prepared, &command, results);
    match run {
        Ok(()) => Ok(to_raw_value(&result)),
        Err(e) => {
            // dbt's exit code already honors each test's own `severity`: a
            // failing `warn` test leaves the run successful. Overriding that
            // would make the same project behave differently on Windmill than
            // it does locally, which is the promise this feature is built on.
            append_logs(
                &job.id,
                &job.workspace_id,
                format!("\n{}\n", render_failures(&result)),
                conn,
            )
            .await;
            Err(Error::ExecutionErr(format!(
                "{e}\n\n{}",
                serde_json::to_string_pretty(&result).unwrap_or_default()
            )))
        }
    }
}

/// Deploy-time lock: resolve the ref to a commit, clone it, and parse the
/// project so its models land in the asset graph before it has ever run.
///
/// This is the one place where dbt does not fit the shape every other language
/// uses. `parse_assets_for_lang` is a pure function of the script content, and
/// dbt's assets are not derivable from the descriptor — they need a clone and a
/// dbt invocation. So the dependency job, which already runs on a worker with
/// git and the engine available, does the parse and writes the `asset` rows
/// itself; `parse_assets_for_lang` returns `None` for dbt and leaves them
/// alone. That also makes redeploy the graph-refresh mechanism for pinned refs,
/// with no separate concept (docs/dbt-runtime.md, decision 12).
#[allow(clippy::too_many_arguments)]
pub async fn dbt_dep(
    content: &str,
    job_id: &Uuid,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job_dir: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    worker_name: &str,
    w_id: &str,
    script_path: &str,
    occupancy_metrics: &mut OccupancyMetrics,
    token: &str,
    base_internal_url: &str,
) -> error::Result<String> {
    let descriptor = parse_dbt_descriptor(content)?;
    let conn = Connection::Sql(db.clone());
    let client = AuthedClient::new(
        base_internal_url.to_string(),
        w_id.to_string(),
        token.to_string(),
        None,
    );
    let mut prepared = prepare_project(
        &descriptor,
        None,
        &HashMap::new(),
        job_dir,
        job_id,
        w_id,
        worker_name,
        &conn,
        &client,
        mem_peak,
        canceled_by,
        occupancy_metrics,
        base_internal_url,
    )
    .await?;
    prepared.script_path = script_path.to_string();

    let out = dbt_command(&prepared, &["parse"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| Error::internal_err(format!("dbt parse could not be started: {e}")))?;
    append_logs(
        job_id,
        w_id,
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
        &conn,
    )
    .await;
    if !out.status.success() {
        return Err(Error::ExecutionErr("dbt parse failed".to_string()));
    }

    let selected = resolve_selection(&prepared, &descriptor).await?;
    let manifest = read_manifest(&prepared.project_dir).await?;
    let manifest_digest = digest(
        &tokio::fs::read_to_string(prepared.project_dir.join("target/manifest.json"))
            .await
            .unwrap_or_default(),
    );

    if let Some(resource_path) = prepared.resource_path.as_deref() {
        let ingested = windmill_common::dbt_manifest::ingest_manifest(
            &manifest,
            resource_path,
            selected.as_ref(),
        );
        let mut tx = db.begin().await?;
        windmill_common::dbt_manifest::replace_dbt_manifest(&mut tx, w_id, script_path, &ingested)
            .await?;
        windmill_common::assets::replace_static_asset_usage(
            &mut tx,
            w_id,
            script_path,
            &ingested.assets,
        )
        .await?;
        tx.commit().await?;
        append_logs(
            job_id,
            w_id,
            format!(
                "\nIngested {} dbt nodes and {} edges into the asset graph\n",
                ingested.nodes.len(),
                ingested.edges.len()
            ),
            &conn,
        )
        .await;
    } else {
        append_logs(
            job_id,
            w_id,
            "\nSkipping asset-graph ingest: the descriptor declares no `profile.resource`, \
             so there is no warehouse identity to key `table://` assets on\n"
                .to_string(),
            &conn,
        )
        .await;
    }

    serde_json::to_string_pretty(&DbtDependencyLocks {
        repo_url: prepared.repo_url.clone(),
        // Empty under `ref: latest` by design: the commit is resolved per run.
        commit: if descriptor.is_latest_ref() {
            String::new()
        } else {
            prepared.commit.clone()
        },
        manifest_digest,
        engine: prepared.engine.engine.as_str().to_string(),
        engine_version: prepared.engine.version.clone(),
    })
    .map_err(|e| Error::internal_err(format!("serializing the dbt lockfile: {e}")))
}

pub struct PreparedProject {
    pub project_dir: PathBuf,
    pub profiles_dir: PathBuf,
    pub engine: ProvisionedEngine,
    pub commit: String,
    pub repo_url: String,
    /// Windmill resource path of the warehouse, the `<resource_path>` component
    /// of every `table://` asset this project produces. `None` when the project
    /// brings its own `profiles.yml` and declares no resource, in which case
    /// there is no stable warehouse identity to key assets on.
    pub resource_path: Option<String>,
    pub script_path: String,
    pub env: Vec<(String, String)>,
}

#[allow(clippy::too_many_arguments)]
pub async fn prepare_project(
    descriptor: &DbtDescriptor,
    locks: Option<&DbtDependencyLocks>,
    args: &HashMap<String, Box<RawValue>>,
    job_dir: &str,
    job_id: &Uuid,
    w_id: &str,
    worker_name: &str,
    conn: &Connection,
    client: &AuthedClient,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    occupancy_metrics: &mut OccupancyMetrics,
    _base_internal_url: &str,
) -> error::Result<PreparedProject> {
    let repo_res = descriptor.repo.trim_start_matches("$res:").to_string();
    let repo_value: serde_json::Value = client
        .get_resource_value_interpolated(&repo_res, Some(job_id.to_string()))
        .await
        .map_err(|e| {
            Error::BadRequest(format!("could not read the git repository resource: {e}"))
        })?;
    let url = repo_value
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            Error::BadRequest(
                "the `repo` resource has no `url`; it must be of type git_repository".to_string(),
            )
        })?
        .to_string();
    let branch = repo_value
        .get("branch")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    // Under `latest` HEAD is resolved now; otherwise the lockfile's commit is
    // authoritative so a run reproduces its deploy exactly, and the descriptor's
    // ref is only the fallback for a script whose lock has not been generated.
    let interpolated_ref = descriptor
        .r#ref
        .as_deref()
        .map(|r| crate::common::interpolate_template(r, Some(args), "ref"))
        .transpose()?;
    let probe = GitRepo {
        url: url.clone(),
        commit: None,
        branch: branch.clone(),
        target_path: "dbt".to_string(),
    };
    let commit = if descriptor.is_latest_ref() {
        get_git_repo_full_head_commit_hash(&probe, "ssh").await?
    } else if let Some(locked) = locks.map(|l| l.commit.clone()).filter(|c| !c.is_empty()) {
        locked
    } else if let Some(r) = interpolated_ref.clone() {
        // The descriptor's ref before a lockfile exists (deploy). It has to be
        // resolved rather than used as-is: a branch name is not a pin, and the
        // clone cache below keys on the commit precisely because commits are
        // immutable and a branch name is not.
        resolve_git_ref_to_commit(&probe, "ssh", &r).await?
    } else {
        String::new()
    };

    let repo = GitRepo {
        url: url.clone(),
        commit: (!commit.is_empty()).then(|| commit.clone()),
        branch,
        target_path: "dbt".to_string(),
    };
    let checked_out = checkout(
        &repo,
        &commit,
        job_dir,
        job_id,
        w_id,
        worker_name,
        conn,
        mem_peak,
        canceled_by,
        occupancy_metrics,
    )
    .await?;

    let project_dir = match descriptor
        .project
        .as_deref()
        .map(str::trim)
        .filter(|p| !p.is_empty())
    {
        Some(sub) => {
            crate::common::validate_relative_path(sub, "project")?;
            PathBuf::from(job_dir).join("dbt").join(sub)
        }
        None => PathBuf::from(job_dir).join("dbt"),
    };
    if !project_dir.join("dbt_project.yml").exists() {
        return Err(Error::BadRequest(format!(
            "no dbt_project.yml at `{}` in the repo",
            descriptor.project.as_deref().unwrap_or(".")
        )));
    }

    let (profiles_dir, resource_path, adapter) =
        write_profiles(descriptor, &project_dir, job_dir, client, job_id).await?;
    let engine = provision_engine(descriptor.engine(), adapter, job_id, w_id, conn).await?;

    let mut env: Vec<(String, String)> = descriptor
        .env
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    // Both engines write their profile-independent state under the project;
    // pinning it inside the job dir keeps a job from touching a shared $HOME.
    env.push(("HOME".to_string(), job_dir.to_string()));

    let prepared = PreparedProject {
        project_dir,
        profiles_dir,
        engine,
        commit: checked_out,
        repo_url: url,
        resource_path,
        script_path: String::new(),
        env,
    };
    install_packages(&prepared, job_id, w_id, conn).await?;
    Ok(prepared)
}

/// Clone at `commit`, reusing the worker-local cache when the commit is pinned.
/// Commits are immutable, so a cached checkout of one is always current — which
/// is why `latest` never reads the cache.
#[allow(clippy::too_many_arguments)]
async fn checkout(
    repo: &GitRepo,
    commit: &str,
    job_dir: &str,
    job_id: &Uuid,
    w_id: &str,
    worker_name: &str,
    conn: &Connection,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    occupancy_metrics: &mut OccupancyMetrics,
) -> error::Result<String> {
    let dest = PathBuf::from(job_dir).join("dbt");
    if !commit.is_empty() {
        let cached = PathBuf::from(&*DBT_CACHE_DIR).join("repos").join(format!(
            "{}-{}",
            digest(&repo.url),
            commit
        ));
        if cached.join(".git").exists() {
            copy_dir(&cached, &dest).await?;
            append_logs(
                job_id,
                w_id,
                format!("\nReusing cached clone at {commit}\n"),
                conn,
            )
            .await;
            return Ok(commit.to_string());
        }
        clone_repo_without_history(
            repo,
            commit,
            job_dir,
            job_id,
            worker_name,
            conn,
            mem_peak,
            canceled_by,
            w_id,
            occupancy_metrics,
            "ssh",
        )
        .await?;
        tokio::fs::create_dir_all(cached.parent().unwrap())
            .await
            .ok();
        copy_dir(&dest, &cached).await.ok();
        return Ok(commit.to_string());
    }
    clone_repo(
        repo,
        job_dir,
        job_id,
        worker_name,
        conn,
        mem_peak,
        canceled_by,
        w_id,
        occupancy_metrics,
        "ssh",
    )
    .await
}

/// `dbt deps`, with `dbt_packages/` restored from a cache keyed by the digest
/// of `packages.yml` — the file that determines the whole tree.
async fn install_packages(
    p: &PreparedProject,
    job_id: &Uuid,
    w_id: &str,
    conn: &Connection,
) -> error::Result<()> {
    let manifest = ["packages.yml", "dependencies.yml"]
        .iter()
        .map(|f| p.project_dir.join(f))
        .find(|f| f.exists());
    let Some(manifest) = manifest else {
        return Ok(());
    };
    let content = tokio::fs::read_to_string(&manifest)
        .await
        .unwrap_or_default();
    let cached = PathBuf::from(&*DBT_CACHE_DIR)
        .join("packages")
        .join(digest(&content));
    let target = p.project_dir.join("dbt_packages");
    if cached.exists() {
        copy_dir(&cached, &target).await?;
        append_logs(
            job_id,
            w_id,
            "\nReusing cached dbt_packages\n".to_string(),
            conn,
        )
        .await;
        return Ok(());
    }
    let out = dbt_command(p, &["deps"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| Error::internal_err(format!("dbt deps could not be started: {e}")))?;
    append_logs(
        job_id,
        w_id,
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
        conn,
    )
    .await;
    if !out.status.success() {
        return Err(Error::ExecutionErr("dbt deps failed".to_string()));
    }
    if target.exists() {
        tokio::fs::create_dir_all(cached.parent().unwrap())
            .await
            .ok();
        copy_dir(&target, &cached).await.ok();
    }
    Ok(())
}

/// Write `profiles.yml`, either rendered from a Windmill resource or taken from
/// the project itself. Both paths are supported (decision 8): the resource path
/// is the ergonomic one, the project's own file is what makes an existing repo
/// run unchanged.
async fn write_profiles(
    descriptor: &DbtDescriptor,
    project_dir: &Path,
    job_dir: &str,
    client: &AuthedClient,
    job_id: &Uuid,
) -> error::Result<(PathBuf, Option<String>, DbtAdapter)> {
    let resource_path = descriptor
        .profile
        .resource
        .as_deref()
        .map(|r| r.trim_start_matches("$res:").to_string());

    let declared = descriptor
        .profile
        .adapter
        .as_deref()
        .map(|t| {
            DbtAdapter::from_resource_type(t).ok_or_else(|| {
                Error::BadRequest(format!(
                    "`profile.type: {t}` is not a supported dbt adapter \
                     (postgres, snowflake, bigquery, databricks)"
                ))
            })
        })
        .transpose()?;

    if let Some(own) = descriptor.profile.profiles_yml.as_deref() {
        crate::common::validate_relative_path(own, "profile.profiles_yml")?;
        let path = project_dir.join(own);
        let dir = path
            .parent()
            .ok_or_else(|| Error::BadRequest("profile.profiles_yml has no parent".to_string()))?
            .to_path_buf();
        // The adapter still has to be known, because the bundled dbt-core 1.x
        // engine needs the matching pip package installed. The project's own
        // file already spells it, so read it from there when not declared.
        let adapter = match declared {
            Some(a) => a,
            None => adapter_from_profiles_yml(&path).await?,
        };
        ensure_adapter_licensed(adapter)?;
        return Ok((dir, resource_path, adapter));
    }

    let resource_path = resource_path.ok_or_else(|| {
        Error::BadRequest(
            "the descriptor must set either `profile.resource` or `profile.profiles_yml`"
                .to_string(),
        )
    })?;
    let value: serde_json::Value = client
        .get_resource_value_interpolated(&resource_path, Some(job_id.to_string()))
        .await
        .map_err(|e| Error::BadRequest(format!("could not read the profile resource: {e}")))?;
    let adapter = declared
        .or_else(|| DbtAdapter::infer_from_resource(&value))
        .ok_or_else(|| {
            Error::BadRequest(format!(
                "could not tell which dbt adapter resource `{resource_path}` needs; \
                 set `profile.type` in the descriptor"
            ))
        })?;
    ensure_adapter_licensed(adapter)?;
    let profile_name = project_profile_name(project_dir).await;
    let target = descriptor.profile.target.as_deref().unwrap_or("default");
    let rendered = render_profile(
        adapter,
        &value,
        &profile_name,
        target,
        descriptor.threads,
        None,
    )?;
    let dir = PathBuf::from(job_dir).join("dbt_profiles");
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| Error::internal_err(format!("creating the profiles dir: {e}")))?;
    write_file(dir.to_str().unwrap(), "profiles.yml", &rendered.yaml)?;
    Ok((dir, Some(resource_path), adapter))
}

async fn adapter_from_profiles_yml(path: &Path) -> error::Result<DbtAdapter> {
    let content = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| Error::BadRequest(format!("could not read {}: {e}", path.display())))?;
    let v: serde_yml::Value = serde_yml::from_str(&content)
        .map_err(|e| Error::BadRequest(format!("could not parse {}: {e}", path.display())))?;
    // `type:` appears once per output; any of them identifies the adapter.
    fn find_type(v: &serde_yml::Value) -> Option<String> {
        match v {
            serde_yml::Value::Mapping(m) => {
                for (k, val) in m {
                    if k.as_str() == Some("type") {
                        if let Some(s) = val.as_str() {
                            return Some(s.to_string());
                        }
                    }
                    if let Some(found) = find_type(val) {
                        return Some(found);
                    }
                }
                None
            }
            _ => None,
        }
    }
    let t = find_type(&v)
        .ok_or_else(|| Error::BadRequest(format!("{} declares no `type`", path.display())))?;
    DbtAdapter::from_resource_type(&t)
        .ok_or_else(|| Error::BadRequest(format!("unsupported dbt adapter `{t}`")))
}

/// dbt takes the profile to use from `dbt_project.yml`, so a rendered
/// `profiles.yml` has to answer to that name rather than one of our choosing.
async fn project_profile_name(project_dir: &Path) -> String {
    let Ok(content) = tokio::fs::read_to_string(project_dir.join("dbt_project.yml")).await else {
        return FALLBACK_PROFILE_NAME.to_string();
    };
    serde_yml::from_str::<serde_yml::Value>(&content)
        .ok()
        .and_then(|v| {
            v.get("profile")
                .and_then(|p| p.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| FALLBACK_PROFILE_NAME.to_string())
}

pub fn dbt_command(p: &PreparedProject, args: &[&str]) -> Command {
    let mut cmd = Command::new(&p.engine.bin);
    cmd.current_dir(&p.project_dir)
        .env_clear()
        .envs(PROXY_ENVS.clone())
        .env("PATH", PATH_ENV.as_str())
        .env("TZ", TZ_ENV.as_str())
        .env("GIT_PATH", GIT_PATH.as_str())
        .envs(p.env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
        .args(args)
        .arg("--profiles-dir")
        .arg(&p.profiles_dir);
    cmd
}

#[allow(clippy::too_many_arguments)]
async fn run_dbt(
    p: &PreparedProject,
    command: &str,
    descriptor: &DbtDescriptor,
    args: &HashMap<String, Box<RawValue>>,
    job: &MiniPulledJob,
    conn: &Connection,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    occupancy_metrics: &mut OccupancyMetrics,
    envs: &HashMap<String, String>,
    worker_name: &str,
    with_selection: bool,
) -> error::Result<()> {
    let mut cmd = dbt_command(p, &[command]);
    cmd.envs(envs);
    // The console stays human-readable and goes straight to the job log; the
    // machine-readable copy goes to a file the progress reporter tails, so
    // neither purpose degrades the other.
    let log_dir = p.project_dir.join("logs");
    cmd.arg("--log-path")
        .arg(&log_dir)
        .args(["--log-format-file", "json"])
        .args(["--log-level-file", p.engine.engine.progress_log_level()]);

    if with_selection && command != "retry" {
        // Selectors are dbt's grammar and are passed verbatim — reimplementing
        // it is a standing source of divergence (docs/dbt-runtime.md).
        for s in arg_list(args, "select").unwrap_or_else(|| descriptor.select.clone()) {
            cmd.args(["--select", &s]);
        }
        for s in arg_list(args, "exclude").unwrap_or_else(|| descriptor.exclude.clone()) {
            cmd.args(["--exclude", &s]);
        }
        if let Some(sel) = descriptor.selector.as_deref() {
            cmd.args(["--selector", sel]);
        }
    }
    if command != "retry" {
        let vars = resolved_vars(descriptor, args)?;
        if !vars.is_empty() {
            cmd.args(["--vars", &serde_json::to_string(&vars).unwrap_or_default()]);
        }
        if let Some(t) = descriptor.threads {
            cmd.args(["--threads", &t.to_string()]);
        }
        let full_refresh = arg_bool(args, "full_refresh").unwrap_or(descriptor.full_refresh);
        if full_refresh && command != "test" {
            cmd.arg("--full-refresh");
        }
    }

    tokio::fs::remove_file(log_dir.join("dbt.log")).await.ok();
    // `handle_child` reads the job log off these pipes. Without them the child
    // inherits the worker's stdio and handle_child waits on streams that never
    // arrive, so the job hangs instead of running.
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let child = start_child_process(cmd, p.engine.bin.to_string_lossy().as_ref(), false).await?;
    let progress = spawn_progress_reporter(p, job, conn, log_dir.join("dbt.log"));
    let res = handle_child(
        &job.id,
        conn,
        mem_peak,
        canceled_by,
        child,
        false,
        worker_name,
        &job.workspace_id,
        &format!("dbt {command}"),
        job.timeout,
        false,
        &mut Some(occupancy_metrics),
        None,
        None,
    )
    .await;
    if let Some(h) = progress {
        h.abort();
    }
    res.map(|_| ())
}

/// Tail dbt's JSON event file and record each node's status as it finishes, so
/// the asset graph shows the run advancing rather than a single opaque job.
///
/// Only `dbt-core-1x` emits these events today: dbt-core 2.0.0-alpha.5 accepts
/// `--log-format-file json` but still writes a text log, so its runs surface
/// per-model state from `run_results.json` when the invocation ends. The
/// reporter is a no-op there rather than a second, format-sniffing code path.
fn spawn_progress_reporter(
    p: &PreparedProject,
    job: &MiniPulledJob,
    conn: &Connection,
    log_file: PathBuf,
) -> Option<tokio::task::JoinHandle<()>> {
    let Connection::Sql(db) = conn else {
        // Agent workers reach the DB only through the API; per-model progress
        // is reconciled from run_results.json at the end instead.
        return None;
    };
    let (db, w_id, job_id) = (db.clone(), job.workspace_id.clone(), job.id);
    let resource_path = p.resource_path.clone()?;
    Some(tokio::spawn(async move {
        let mut offset = 0u64;
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            let Ok(content) = tokio::fs::read_to_string(&log_file).await else {
                continue;
            };
            if (content.len() as u64) <= offset {
                continue;
            }
            let fresh = &content[offset as usize..];
            offset = content.len() as u64;
            for line in fresh.lines() {
                let Some(ev) = parse_node_event(line, &resource_path) else {
                    continue;
                };
                let _ = record_materialization(
                    &db,
                    &w_id,
                    ev.asset_kind,
                    &ev.asset_path,
                    &ev.partition,
                    ev.status,
                    None,
                    ev.row_count,
                    ev.job_id.or(Some(job_id)),
                    ev.error.as_deref(),
                )
                .await;
            }
        }
    }))
}

/// One `node_info`-carrying dbt log event turned into the materialization
/// record the asset graph reads. `None` for events that are not per-node, and
/// for nodes with no physical relation (tests, ephemeral models).
fn parse_node_event(line: &str, resource_path: &str) -> Option<RecordMaterializationRequest> {
    let line = line.trim();
    if !line.starts_with('{') {
        return None;
    }
    let v: serde_json::Value = serde_json::from_str(line).ok()?;
    let info = v.get("data")?.get("node_info")?;
    let rel = info.get("node_relation")?;
    let schema = rel.get("schema")?.as_str()?;
    let alias = rel.get("alias")?.as_str()?;
    if rel
        .get("relation_name")
        .and_then(|r| r.as_str())
        .unwrap_or("")
        .is_empty()
    {
        return None;
    }
    let status = match info.get("node_status")?.as_str()? {
        "started" => MaterializationStatus::Running,
        "success" | "pass" => MaterializationStatus::Materialized,
        "error" | "fail" | "runtime error" => MaterializationStatus::Failed,
        // `warn` is a passing test at reduced severity and `skipped` says
        // nothing about the relation's state; neither is a materialization.
        _ => return None,
    };
    let path = windmill_parser::asset_parser::canonicalize_table_asset_path(&format!(
        "{resource_path}/{schema}/{alias}"
    ));
    Some(RecordMaterializationRequest {
        asset_kind: windmill_common::assets::AssetKind::Table,
        asset_path: path,
        partition: windmill_common::materialization::UNPARTITIONED.to_string(),
        status,
        snapshot_id: None,
        row_count: None,
        job_id: None,
        error: v
            .get("info")
            .and_then(|i| i.get("msg"))
            .and_then(|m| m.as_str())
            .filter(|_| status == MaterializationStatus::Failed)
            .map(|s| s.to_string()),
        schema: None,
    })
}

/// Settle each model's materialization record from `run_results.json` once the
/// invocation ends.
///
/// This is what the live event tailer cannot do: the events carry no row count,
/// and the Rust engines do not emit them at all (dbt-core 2.0.0-alpha.5 accepts
/// `--log-format-file json` but still writes a text log). So the same records
/// the tailer has been updating are re-stated here from the authoritative
/// artifact, which both fills in `row_count` and gives those engines per-model
/// state — just at the end of the run rather than during it.
async fn reconcile_materializations(
    p: &PreparedProject,
    results: &[DbtNodeResult],
    job: &MiniPulledJob,
    conn: &Connection,
) {
    let (Connection::Sql(db), Some(resource_path)) = (conn, p.resource_path.as_deref()) else {
        return;
    };
    for r in results {
        let Some(path) = asset_path_of_relation(r.relation_name.as_deref(), resource_path) else {
            continue;
        };
        let status = match r.status.as_str() {
            "success" => MaterializationStatus::Materialized,
            "error" | "fail" | "runtime error" => MaterializationStatus::Failed,
            // Tests and skipped nodes say nothing about a relation's state.
            _ => continue,
        };
        if let Err(e) = record_materialization(
            db,
            &job.workspace_id,
            windmill_common::assets::AssetKind::Table,
            &path,
            windmill_common::materialization::UNPARTITIONED,
            status,
            None,
            r.rows_affected,
            Some(job.id),
            (status == MaterializationStatus::Failed)
                .then(|| r.message.as_deref())
                .flatten(),
        )
        .await
        {
            tracing::warn!("recording the materialization of {path} failed: {e:#}");
        }
    }
}

/// `"db"."schema"."name"` from dbt into the `table://` path of the relation.
/// The database segment is dropped: the resource identifies the warehouse.
fn asset_path_of_relation(relation_name: Option<&str>, resource_path: &str) -> Option<String> {
    let rel = relation_name?;
    let parts: Vec<&str> = rel.split('.').collect();
    let [.., schema, name] = parts.as_slice() else {
        return None;
    };
    Some(
        windmill_parser::asset_parser::canonicalize_table_asset_path(&format!(
            "{resource_path}/{schema}/{name}"
        )),
    )
}

async fn read_run_results(project_dir: &Path) -> Vec<DbtNodeResult> {
    let Ok(content) = tokio::fs::read_to_string(project_dir.join("target/run_results.json")).await
    else {
        return vec![];
    };
    let Ok(rr) = serde_json::from_str::<RunResults>(&content) else {
        return vec![];
    };
    rr.results
        .into_iter()
        .map(|r| DbtNodeResult {
            unique_id: r.unique_id,
            status: r.status,
            execution_time: r.execution_time,
            rows_affected: r
                .adapter_response
                .get("rows_affected")
                .and_then(|v| v.as_i64())
                // Adapters report -1 for statements with no row count.
                .filter(|v| *v >= 0),
            relation_name: r.relation_name,
            message: r.message,
            failures: r.failures,
        })
        .collect()
}

fn build_result(p: &PreparedProject, command: &str, nodes: Vec<DbtNodeResult>) -> DbtRunResult {
    let mut totals = DbtTotals { total: nodes.len(), ..Default::default() };
    for n in &nodes {
        match n.status.as_str() {
            "success" | "pass" => totals.success += 1,
            "warn" => totals.warn += 1,
            "skipped" => totals.skipped += 1,
            _ => totals.error += 1,
        }
    }
    DbtRunResult {
        engine: p.engine.engine.as_str().to_string(),
        engine_version: p.engine.version.clone(),
        commit: p.commit.clone(),
        command: command.to_string(),
        totals,
        nodes,
    }
}

fn render_failures(r: &DbtRunResult) -> String {
    let failed: Vec<&DbtNodeResult> = r
        .nodes
        .iter()
        .filter(|n| !matches!(n.status.as_str(), "success" | "pass" | "skipped"))
        .collect();
    if failed.is_empty() {
        return "dbt failed before any node ran".to_string();
    }
    let mut out = format!("{} dbt node(s) did not succeed:\n", failed.len());
    for n in failed {
        out.push_str(&format!(
            "  {} [{}]{}{}\n",
            n.unique_id,
            n.status,
            n.failures
                .map(|f| format!(" {f} failing row(s)"))
                .unwrap_or_default(),
            n.message
                .as_deref()
                .map(|m| format!(": {m}"))
                .unwrap_or_default(),
        ));
    }
    out
}

/// Refresh the stored graph from the manifest this run produced.
async fn ingest_from_run(
    p: &PreparedProject,
    descriptor: &DbtDescriptor,
    job: &MiniPulledJob,
    conn: &Connection,
) -> error::Result<()> {
    let (Connection::Sql(db), Some(resource_path)) = (conn, p.resource_path.as_deref()) else {
        return Ok(());
    };
    let Some(script_path) = job.runnable_path.as_deref() else {
        return Ok(());
    };
    let manifest = read_manifest(&p.project_dir).await?;
    let selected = resolve_selection(p, descriptor).await?;
    let ingested =
        windmill_common::dbt_manifest::ingest_manifest(&manifest, resource_path, selected.as_ref());
    let mut tx = db.begin().await?;
    windmill_common::dbt_manifest::replace_dbt_manifest(
        &mut tx,
        &job.workspace_id,
        script_path,
        &ingested,
    )
    .await?;
    windmill_common::assets::replace_static_asset_usage(
        &mut tx,
        &job.workspace_id,
        script_path,
        &ingested.assets,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

/// The node set the descriptor's selection resolves to, or `None` when it
/// selects everything.
///
/// Resolved by asking dbt (`dbt ls`) rather than by interpreting the selector
/// string: the grammar is dbt's, it is large, and reimplementing it is a
/// standing source of divergence — the mistake Cosmos's manifest path had to
/// make and keeps paying for.
async fn resolve_selection(
    p: &PreparedProject,
    descriptor: &DbtDescriptor,
) -> error::Result<Option<std::collections::HashSet<String>>> {
    if descriptor.select.is_empty()
        && descriptor.exclude.is_empty()
        && descriptor.selector.is_none()
    {
        return Ok(None);
    }
    let mut cmd = dbt_command(p, &["ls"]);
    // The types spelled out rather than `all`, which dbt-core 2.x rejects.
    for t in ["model", "source", "seed", "snapshot", "test"] {
        cmd.args(["--resource-type", t]);
    }
    cmd.args(["--output", "json", "--quiet"]);
    for x in &descriptor.select {
        cmd.args(["--select", x]);
    }
    for x in &descriptor.exclude {
        cmd.args(["--exclude", x]);
    }
    if let Some(sel) = descriptor.selector.as_deref() {
        cmd.args(["--selector", sel]);
    }
    let out = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| Error::internal_err(format!("dbt ls could not be started: {e}")))?;
    if !out.status.success() {
        return Err(Error::ExecutionErr(format!(
            "dbt ls failed to resolve the selection: {}",
            String::from_utf8_lossy(&out.stderr)
        )));
    }
    let mut set = std::collections::HashSet::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let line = line.trim();
        if !line.starts_with('{') {
            continue;
        }
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
            if let Some(id) = v.get("unique_id").and_then(|x| x.as_str()) {
                set.insert(id.to_string());
            }
        }
    }
    Ok(Some(set))
}

pub async fn read_manifest(
    project_dir: &Path,
) -> error::Result<windmill_common::dbt_manifest::Manifest> {
    let path = project_dir.join("target/manifest.json");
    let content = tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| Error::internal_err(format!("dbt produced no manifest.json: {e}")))?;
    serde_json::from_str(&content)
        .map_err(|e| Error::internal_err(format!("could not parse manifest.json: {e}")))
}

/// `dbt retry` reads `target/run_results.json` from the previous invocation.
/// Windmill gives each attempt a fresh job dir, so the state is kept in a
/// worker-local cache keyed by the script.
fn state_dir(w_id: &str, script_path: &str) -> PathBuf {
    PathBuf::from(&*DBT_CACHE_DIR)
        .join("state")
        .join(digest(&format!("{w_id}/{script_path}")))
}

async fn save_run_state(p: &PreparedProject, w_id: &str) -> error::Result<()> {
    let dir = state_dir(w_id, &p.script_path);
    tokio::fs::create_dir_all(&dir).await.ok();
    for f in ["run_results.json", "manifest.json"] {
        tokio::fs::copy(p.project_dir.join("target").join(f), dir.join(f))
            .await
            .ok();
    }
    Ok(())
}

async fn restore_run_state(p: &PreparedProject, w_id: &str) -> error::Result<()> {
    let dir = state_dir(w_id, &p.script_path);
    if !dir.join("run_results.json").exists() {
        return Err(Error::BadRequest(
            "no previous dbt run to retry from on this worker; run the script normally instead"
                .to_string(),
        ));
    }
    let target = p.project_dir.join("target");
    tokio::fs::create_dir_all(&target).await.ok();
    for f in ["run_results.json", "manifest.json"] {
        tokio::fs::copy(dir.join(f), target.join(f)).await.ok();
    }
    Ok(())
}

fn resolved_vars(
    descriptor: &DbtDescriptor,
    args: &HashMap<String, Box<RawValue>>,
) -> error::Result<serde_json::Map<String, serde_json::Value>> {
    let mut out = serde_json::Map::new();
    for (k, v) in &descriptor.vars {
        out.insert(
            k.clone(),
            serde_json::Value::String(crate::common::interpolate_template(
                v,
                Some(args),
                &format!("vars.{k}"),
            )?),
        );
    }
    if let Some(raw) = args.get("vars") {
        if let Ok(serde_json::Value::Object(m)) = serde_json::from_str(raw.get()) {
            out.extend(m);
        }
    }
    Ok(out)
}

fn arg_str(args: &HashMap<String, Box<RawValue>>, k: &str) -> Option<String> {
    serde_json::from_str::<String>(args.get(k)?.get())
        .ok()
        .filter(|s| !s.is_empty())
}

fn arg_bool(args: &HashMap<String, Box<RawValue>>, k: &str) -> Option<bool> {
    serde_json::from_str::<bool>(args.get(k)?.get()).ok()
}

fn arg_list(args: &HashMap<String, Box<RawValue>>, k: &str) -> Option<Vec<String>> {
    let v = serde_json::from_str::<Vec<String>>(args.get(k)?.get()).ok()?;
    (!v.is_empty()).then_some(v)
}

fn digest(s: &str) -> String {
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    format!("{:x}", h.finalize())[..32].to_string()
}

async fn copy_dir(from: &Path, to: &Path) -> error::Result<()> {
    tokio::fs::create_dir_all(to)
        .await
        .map_err(|e| Error::internal_err(format!("creating {to:?}: {e}")))?;
    let out = Command::new("cp")
        .arg("-a")
        .arg(format!("{}/.", from.display()))
        .arg(to)
        .output()
        .await
        .map_err(|e| Error::internal_err(format!("copying {from:?}: {e}")))?;
    if !out.status.success() {
        return Err(Error::internal_err(format!(
            "copying {from:?} to {to:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_events_become_materialization_records() {
        let started = r#"{"data":{"node_info":{"node_status":"started","materialized":"table",
            "node_relation":{"alias":"Customers","schema":"Analytics",
            "relation_name":"\"wh\".\"Analytics\".\"Customers\""}}},
            "info":{"name":"LogStartLine","msg":"start"}}"#;
        let ev = parse_node_event(started, "f/prod/wh").unwrap();
        assert_eq!(ev.status, MaterializationStatus::Running);
        // Same canonicalization as the manifest ingest, or a run would record
        // progress against a key no graph node has.
        assert_eq!(ev.asset_path, "f/prod/wh/analytics/customers");

        let failed = r#"{"data":{"node_info":{"node_status":"error",
            "node_relation":{"alias":"c","schema":"a","relation_name":"\"w\".\"a\".\"c\""}}},
            "info":{"name":"LogModelResult","msg":"boom"}}"#;
        let ev = parse_node_event(failed, "f/prod/wh").unwrap();
        assert_eq!(ev.status, MaterializationStatus::Failed);
        assert_eq!(ev.error.as_deref(), Some("boom"));
    }

    // The end-of-run reconciliation and the live tailer must derive the SAME
    // key, or a run would settle its progress against a path no graph node has.
    #[test]
    fn run_results_relations_canonicalize_like_the_live_events() {
        assert_eq!(
            asset_path_of_relation(Some("\"wh\".\"Analytics\".\"Customers\""), "f/prod/wh"),
            Some("f/prod/wh/analytics/customers".to_string())
        );
        let live = r#"{"data":{"node_info":{"node_status":"success",
            "node_relation":{"alias":"Customers","schema":"Analytics",
            "relation_name":"\"wh\".\"Analytics\".\"Customers\""}}},
            "info":{"name":"LogModelResult","msg":"ok"}}"#;
        assert_eq!(
            parse_node_event(live, "f/prod/wh").unwrap().asset_path,
            asset_path_of_relation(Some("\"wh\".\"Analytics\".\"Customers\""), "f/prod/wh")
                .unwrap()
        );
        // A test node has no relation of its own.
        assert_eq!(asset_path_of_relation(None, "f/prod/wh"), None);
    }

    #[test]
    fn events_without_a_relation_are_not_materializations() {
        // A test node has no relation of its own.
        let t = r#"{"data":{"node_info":{"node_status":"pass",
            "node_relation":{"alias":"unique_c","schema":"a_audit","relation_name":""}}},
            "info":{"name":"LogTestResult","msg":"ok"}}"#;
        assert!(parse_node_event(t, "f/prod/wh").is_none());
        assert!(parse_node_event("Running with dbt=1.12.0", "f/prod/wh").is_none());
        // `skipped` says nothing about the relation's state.
        let s = r#"{"data":{"node_info":{"node_status":"skipped",
            "node_relation":{"alias":"c","schema":"a","relation_name":"\"w\".\"a\".\"c\""}}},
            "info":{"name":"LogModelResult","msg":"skip"}}"#;
        assert!(parse_node_event(s, "f/prod/wh").is_none());
    }
}
