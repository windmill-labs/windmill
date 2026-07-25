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
use windmill_parser_yaml::{
    parse_dbt_descriptor, DbtDescriptor, DbtTestBehavior, GitRepo, DBT_COMMANDS,
};
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
    /// The `git_repository` resource path, never the resolved URL. A token-auth
    /// URL carries the token, and the lockfile lands in script metadata and
    /// workspace exports.
    pub repo_resource: String,
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
        job.runnable_path.as_deref().unwrap_or_default(),
        true,
        worker_name,
        conn,
        client,
        mem_peak,
        canceled_by,
        occupancy_metrics,
    )
    .await?;

    let command = match arg_str(&args, "dbt_command") {
        // Validated against an allowlist rather than passed through: the value
        // becomes the dbt subcommand, and running a script needs weaker
        // permission than editing it — an unchecked arg would let a runner
        // invoke `clean`, `seed` or `source freshness` on the descriptor's
        // warehouse.
        Some(c) if DBT_COMMANDS.contains(&c.as_str()) => c,
        Some(c) => {
            return Err(Error::BadRequest(format!(
                "`dbt_command` must be one of {}, got `{c}`",
                DBT_COMMANDS.join(", ")
            )))
        }
        None => match descriptor.test_behavior {
            DbtTestBehavior::Build => "build".to_string(),
            DbtTestBehavior::AfterAll | DbtTestBehavior::None => "run".to_string(),
        },
    };
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
    // failure does not stop the models that were going to build anyway. Each
    // invocation REWRITES `run_results.json`, so the model results have to be
    // read before the test phase overwrites them — otherwise the job reports
    // tests only, and nothing settles the models' materializations.
    let mut results = read_run_results(&prepared.project_dir).await;
    // `retry` counts as the model phase too: a run that failed midway and was
    // retried to success would otherwise return green having never tested.
    if run.is_ok()
        && matches!(descriptor.test_behavior, DbtTestBehavior::AfterAll)
        && matches!(command.as_str(), "run" | "retry")
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
            // The tests must be scoped exactly like the models were: testing
            // the whole project would assert against models this script never
            // builds, the same failure the ingest-side scoping fixes.
            true,
        )
        .await;
        results.extend(read_run_results(&prepared.project_dir).await);
    }

    save_run_state(&prepared, &job.workspace_id).await.ok();
    reconcile_materializations(&prepared, &results, job, conn).await;

    // `ref: latest` executes whatever HEAD resolved to, so the graph must be
    // refreshed from this run's own manifest or it describes a different
    // commit than the one that ran (decision 12). The run already produced
    // `manifest.json`, so this costs no extra dbt invocation.
    // Whenever the commit is chosen per run rather than pinned at deploy — both
    // `latest` and a placeholder ref — the deployed graph describes a different
    // commit than the one that just executed, so it has to be refreshed from
    // this run's manifest (decision 12). Free: the run already wrote it.
    if descriptor.is_latest_ref() || prepared.graph_is_per_run {
        if let Err(e) = ingest_from_run(&prepared, &descriptor, &args, job, conn).await {
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
    let prepared = prepare_project(
        &descriptor,
        None,
        &HashMap::new(),
        job_dir,
        job_id,
        w_id,
        script_path,
        false,
        worker_name,
        &conn,
        &client,
        mem_peak,
        canceled_by,
        occupancy_metrics,
    )
    .await?;

    let mut parse_cmd = dbt_command(&prepared, &["parse"]);
    add_vars(&mut parse_cmd, &descriptor, &HashMap::new(), false)?;
    let out = parse_cmd
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

    let selected = resolve_selection(&prepared, &descriptor, &HashMap::new(), false).await?;
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
            prepared.default_database.as_deref(),
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
        // No warehouse identity, so nothing can be ingested — but the previous
        // deploy's rows must still go. Leaving them means a descriptor edited
        // to use its own profiles.yml keeps claiming ownership of relations it
        // no longer describes, and keeps cascading from them.
        let mut tx = db.begin().await?;
        windmill_common::dbt_manifest::clear_dbt_manifest(&mut tx, w_id, script_path).await?;
        windmill_common::assets::replace_static_asset_usage(&mut tx, w_id, script_path, &[])
            .await?;
        tx.commit().await?;
        append_logs(
            job_id,
            w_id,
            "\nNo asset-graph ingest: the descriptor declares no `profile.resource`, so there \
             is no warehouse identity to key `table://` assets on. Any previously ingested \
             nodes for this script have been cleared.\n"
                .to_string(),
            &conn,
        )
        .await;
    }

    serde_json::to_string_pretty(&DbtDependencyLocks {
        repo_resource: prepared.repo_resource.clone(),
        // Empty when the commit is chosen per run rather than at deploy: under
        // `ref: latest`, and when the ref is a placeholder only a run can fill.
        commit: if descriptor.is_latest_ref() || prepared.ref_is_per_run {
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
    /// The descriptor's `ref` holds a placeholder no deploy can resolve, so the
    /// lockfile must not pin a commit.
    pub ref_is_per_run: bool,
    /// The deploy-time graph is a guess — a per-run ref or a var that can steer
    /// what the project even produces — so each run re-ingests its own manifest.
    pub graph_is_per_run: bool,
    /// The project's path relative to the checkout root, which distinguishes
    /// two same-named project dirs in one repo.
    pub project_subdir: String,
    /// The `git_repository` resource path. The resolved URL is deliberately not
    /// kept: it can carry a token, and this ends up in the lockfile.
    pub repo_resource: String,
    /// Windmill resource path of the warehouse, the `<resource_path>` component
    /// of every `table://` asset this project produces. `None` when the project
    /// brings its own `profiles.yml` and declares no resource, in which case
    /// there is no stable warehouse identity to key assets on.
    pub resource_path: Option<String>,
    /// The profile target's database. Nodes that override it qualify their
    /// `table://` schema segment so two databases cannot collapse onto one node.
    pub default_database: Option<String>,
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
    // Keys the per-script retry-state cache. Passed in rather than patched onto
    // the result afterwards: an empty value silently shares one state directory
    // across every dbt script in the workspace, so a retry resumes another
    // project's run_results.json.
    script_path: &str,
    // False only for the deploy, which has no job arguments and so must tolerate
    // `{{ }}` placeholders it cannot fill. A run must not.
    strict_args: bool,
    worker_name: &str,
    conn: &Connection,
    client: &AuthedClient,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    occupancy_metrics: &mut OccupancyMetrics,
) -> error::Result<PreparedProject> {
    let repo_res = descriptor.repo.trim_start_matches("$res:").to_string();
    let repo_value: serde_json::Value = client
        .get_resource_value_interpolated(&repo_res, Some(job_id.to_string()))
        .await
        .map_err(|e| {
            Error::BadRequest(format!("could not read the git repository resource: {e}"))
        })?;
    let mut url = repo_value
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            Error::BadRequest(
                "the `repo` resource has no `url`; it must be of type git_repository".to_string(),
            )
        })?
        .to_string();
    // A GitHub App resource carries no credential of its own: an installation
    // token has to be minted per use. The only helper that does that
    // (`get_github_app_token_internal`) authorizes by checking the job against
    // the workspace's configured git-sync scripts, so it rejects a dbt job
    // outright — minting for an arbitrary runnable needs its own authorization
    // path, which is not this PR. Refuse with the reason rather than letting
    // the clone fail on a bare auth error: a wrong-looking credential is far
    // harder to diagnose than a stated limitation.
    if repo_value
        .get("is_github_app")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        return Err(Error::BadRequest(
            "dbt cannot use a GitHub App git_repository resource yet: installation tokens are \
             minted only for git-sync jobs. Use a token in the resource URL, or \
             `git_ssh_identity` for an SSH remote."
                .to_string(),
        ));
    }
    let _ = &mut url;
    let git_ssh_cmd = git_ssh_cmd(descriptor, job_dir, client).await?;
    let branch = repo_value
        .get("branch")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    // Under `latest` HEAD is resolved now; otherwise the lockfile's commit is
    // authoritative so a run reproduces its deploy exactly, and the descriptor's
    // ref is only the fallback for a script whose lock has not been generated.
    // At deploy there are no job args, so a `ref: "{{ commit }}"` cannot be
    // resolved. That is not a deploy failure — the ref is simply only knowable
    // per run. `ref_is_per_run` carries that fact to the lockfile, which must
    // then pin nothing: locking the default branch's hash would make every run
    // ignore its own `commit` argument and replay the deploy's checkout.
    let interpolated_ref = match descriptor.r#ref.as_deref() {
        Some(r) => match crate::common::interpolate_template(r, Some(args), "ref") {
            Ok(v) => Some(v),
            // A run that cannot fill its own ref must fail: falling back to the
            // default branch would silently execute a commit nobody asked for
            // and report success.
            Err(e) if strict_args => return Err(e),
            Err(_) => None,
        },
        None => None,
    };
    // A property of the descriptor, not of whether this particular caller could
    // interpolate: runs are strict, so deciding it from a failed interpolation
    // would make it permanently false on the run path and the per-run graph
    // refresh below would never fire.
    let has_placeholder = |v: &str| v.contains("{{");
    let ref_is_per_run = descriptor.r#ref.as_deref().is_some_and(has_placeholder);
    // Vars can drive `enabled`, alias, schema, database and materialization, so
    // a var only a run can fill means the deploy-time graph is a guess. Treat it
    // like a per-run ref: refresh from each run's own manifest.
    let graph_is_per_run =
        ref_is_per_run || descriptor.vars.values().any(|v| has_placeholder(v));
    let probe = GitRepo {
        url: url.clone(),
        commit: None,
        branch: branch.clone(),
        target_path: "dbt".to_string(),
    };
    let commit = if descriptor.is_latest_ref() {
        get_git_repo_full_head_commit_hash(&probe, &git_ssh_cmd).await?
    } else if let Some(r) = interpolated_ref
        .clone()
        // A ref the descriptor spells with a placeholder is chosen by the run,
        // so the run's value wins over whatever the deploy happened to lock.
        .filter(|_| descriptor.r#ref.as_deref().is_some_and(|r| r.contains("{{")))
    {
        resolve_git_ref_to_commit(&probe, &git_ssh_cmd, &r).await?
    } else if let Some(locked) = locks.map(|l| l.commit.clone()).filter(|c| !c.is_empty()) {
        locked
    } else if let Some(r) = interpolated_ref.clone() {
        // The descriptor's ref before a lockfile exists (deploy). It has to be
        // resolved rather than used as-is: a branch name is not a pin, and the
        // clone cache below keys on the commit precisely because commits are
        // immutable and a branch name is not.
        resolve_git_ref_to_commit(&probe, &git_ssh_cmd, &r).await?
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
        &git_ssh_cmd,
    )
    .await?;

    let project_subdir = descriptor
        .project
        .as_deref()
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .unwrap_or(".")
        .to_string();
    if project_subdir != "." {
        crate::common::validate_relative_path(&project_subdir, "project")?;
    }
    let project_dir = PathBuf::from(job_dir).join("dbt").join(&project_subdir);
    if !project_dir.join("dbt_project.yml").exists() {
        return Err(Error::BadRequest(format!(
            "no dbt_project.yml at `{}` in the repo",
            descriptor.project.as_deref().unwrap_or(".")
        )));
    }

    let (profiles_dir, resource_path, adapter, default_database) =
        write_profiles(descriptor, &project_dir, job_dir, client, job_id).await?;
    let engine = provision_engine(descriptor.engine(), adapter, job_id, w_id, conn).await?;

    let mut env = resolve_env(descriptor, client).await?;
    // Both engines write their profile-independent state under the project;
    // pinning it inside the job dir keeps a job from touching a shared $HOME.
    env.push(("HOME".to_string(), job_dir.to_string()));

    let prepared = PreparedProject {
        project_dir,
        profiles_dir,
        engine,
        commit: checked_out,
        ref_is_per_run,
        graph_is_per_run,
        project_subdir,
        repo_resource: repo_res,
        resource_path,
        default_database,
        script_path: script_path.to_string(),
        env,
    };
    install_packages(&prepared, job_id, w_id, conn).await?;
    Ok(prepared)
}

/// `GIT_SSH_COMMAND` for the clone, with any descriptor-declared private keys
/// written into the job dir (torn down with the job). Mirrors the Ansible
/// executor's identity handling — the clone helpers are shared, so the auth
/// story should be too.
async fn git_ssh_cmd(
    descriptor: &DbtDescriptor,
    job_dir: &str,
    client: &AuthedClient,
) -> error::Result<String> {
    let mut identities = String::new();
    for (i, var_path) in descriptor.git_ssh_identity.iter().enumerate() {
        let name = format!(".ssh_id_priv_dbt_{i}");
        let loc = windmill_common::worker::is_allowed_file_location(job_dir, &name)?;
        let mut content = client.get_variable_value(var_path).await.map_err(|e| {
            Error::NotFound(format!(
                "variable {var_path} not found for `git_ssh_identity`: {e:#}"
            ))
        })?;
        content.push('\n');
        let file = write_file(job_dir, &name, &content)?;
        #[cfg(unix)]
        file.set_permissions(std::os::unix::fs::PermissionsExt::from_mode(0o600))?;
        #[cfg(not(unix))]
        let _ = file;
        identities.push_str(&format!(
            " -i '{}'",
            loc.to_string_lossy().replace('\'', r"'\''")
        ));
    }
    Ok(format!("ssh -o StrictHostKeyChecking=no{identities}"))
}

/// Resolve `$var:<path>` values in the descriptor's `env`. This is the only way
/// a project using its own `profiles.yml` can get a secret into
/// `{{ env_var() }}` without writing it into versioned script content.
async fn resolve_env(
    descriptor: &DbtDescriptor,
    client: &AuthedClient,
) -> error::Result<Vec<(String, String)>> {
    let mut out = Vec::with_capacity(descriptor.env.len());
    for (k, v) in &descriptor.env {
        let value = match v.strip_prefix("$var:") {
            Some(path) => client.get_variable_value(path.trim()).await.map_err(|e| {
                Error::NotFound(format!("variable {path} not found for `env.{k}`: {e:#}"))
            })?,
            None => v.clone(),
        };
        out.push((k.clone(), value));
    }
    Ok(out)
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
    git_ssh_cmd: &str,
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
            git_ssh_cmd,
        )
        .await?;
        publish_to_cache(&dest, &cached, job_id).await;
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
        git_ssh_cmd,
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
    // A cache hit skips `dbt deps` entirely, so the key has to cover everything
    // that determines the resolved tree: `package-lock.yml` pins versions two
    // projects with identical ranges would otherwise resolve differently, and a
    // `local:` dependency's content is not in any of these files at all — the
    // commit and project subdir stand in for it. The cache is worker-global, so
    // without those two, byte-identical `packages.yml` files in unrelated repos
    // (and workspaces) would share one tree.
    // The project's path RELATIVE to the checkout, not its basename:
    // `team_a/analytics` and `team_b/analytics` in one repo resolve different
    // `local:` dependencies from identical manifests.
    let mut key = format!("{}\n{}\n", p.commit, p.project_subdir);
    let mut declares_packages = false;
    for f in ["packages.yml", "dependencies.yml", "package-lock.yml"] {
        let path = p.project_dir.join(f);
        if !path.exists() {
            continue;
        }
        declares_packages |= f != "package-lock.yml";
        key.push_str(f);
        key.push('\n');
        key.push_str(&tokio::fs::read_to_string(&path).await.unwrap_or_default());
    }
    if !declares_packages {
        return Ok(());
    }
    let cached = PathBuf::from(&*DBT_CACHE_DIR)
        .join("packages")
        .join(digest(&key));
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
        publish_to_cache(&target, &cached, job_id).await;
    }
    Ok(())
}

/// Copy `from` into a sibling of `cached`, then move it into place.
///
/// The rename is the point. `copy_dir` creates its destination and then fills
/// it, so a concurrent job on the same host — worker processes share
/// `DBT_CACHE_DIR` — would see `cached` exist and copy a checkout with no
/// `dbt_project.yml`, failing with an error that blames the user's descriptor.
/// Worse, a copy interrupted by cancellation or disk pressure would leave that
/// partial tree in place for every later job, so a transient failure becomes
/// permanent. Staging keeps a half-written tree under a name nothing looks up.
/// Same pattern as the engine provisioning; best-effort, since losing the race
/// only means the next job repopulates.
async fn publish_to_cache(from: &Path, cached: &Path, job_id: &Uuid) {
    let Some(parent) = cached.parent() else { return };
    if tokio::fs::create_dir_all(parent).await.is_err() {
        return;
    }
    let name = cached.file_name().unwrap_or_default().to_string_lossy();
    let staging = cached.with_file_name(format!("{name}.staging-{job_id}"));
    tokio::fs::remove_dir_all(&staging).await.ok();
    if copy_dir(from, &staging).await.is_err()
        || strip_git_remote(&staging).await.is_err()
        || tokio::fs::rename(&staging, cached).await.is_err()
    {
        tokio::fs::remove_dir_all(&staging).await.ok();
    }
}

/// Drop the origin remote from a checkout on its way into the cache.
///
/// `git clone` writes the URL it was given into `.git/config`, and for token
/// auth or a GitHub App that URL *is* the credential. The cache is
/// worker-global and outlives the job, so copying the checkout verbatim would
/// leave a live token readable by every later job on the host. The cache is
/// only ever restored at an already-known commit, so it needs no remote.
async fn strip_git_remote(dir: &Path) -> std::io::Result<()> {
    let config = dir.join(".git").join("config");
    if !config.exists() {
        return Ok(());
    }
    let content = tokio::fs::read_to_string(&config).await?;
    let mut out = String::with_capacity(content.len());
    let mut in_remote = false;
    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('[') {
            in_remote = trimmed.starts_with("[remote ");
        }
        if !in_remote {
            out.push_str(line);
            out.push('\n');
        }
    }
    tokio::fs::write(&config, out).await
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
) -> error::Result<(PathBuf, Option<String>, DbtAdapter, Option<String>)> {
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
        // The project owns its profile, so Windmill does not know its database;
        // nodes then qualify only against `None`, i.e. never.
        return Ok((dir, resource_path, adapter, None));
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
        descriptor.profile.schema.as_deref(),
    )?;
    let dir = PathBuf::from(job_dir).join("dbt_profiles");
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| Error::internal_err(format!("creating the profiles dir: {e}")))?;
    write_file(dir.to_str().unwrap(), "profiles.yml", &rendered.yaml)?;
    Ok((dir, Some(resource_path), adapter, rendered.database))
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
        add_vars(&mut cmd, descriptor, args, true)?;
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
    if !p.engine.engine.emits_node_events() {
        // Nothing to read: those engines write a text file log, so tailing it
        // would burn a task per run for no events.
        return None;
    }
    let (db, w_id, job_id) = (db.clone(), job.workspace_id.clone(), job.id);
    let resource_path = p.resource_path.clone()?;
    Some(tokio::spawn(async move {
        use tokio::io::{AsyncReadExt, AsyncSeekExt};
        let mut offset = 0u64;
        // A tick can land mid-write, leaving a trailing partial line; hold it
        // over rather than dropping the event it belongs to.
        let mut carry = String::new();
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            let Ok(mut f) = tokio::fs::File::open(&log_file).await else {
                continue;
            };
            // dbt truncates/rotates its log; a shrunk file means the offsets
            // from the previous incarnation are meaningless, and keeping them
            // would silence the tailer for the rest of the run.
            let len = f.metadata().await.map(|m| m.len()).unwrap_or(0);
            if len < offset {
                offset = 0;
                carry.clear();
            }
            if len == offset {
                continue;
            }
            // Seek rather than re-read: a long run's log grows without bound
            // and reading it whole every tick is quadratic in its size.
            if f.seek(std::io::SeekFrom::Start(offset)).await.is_err() {
                continue;
            }
            let mut buf = Vec::new();
            if f.read_to_end(&mut buf).await.is_err() {
                continue;
            }
            offset += buf.len() as u64;
            // Append BEFORE looking for the last newline: a line spanning three
            // reads would otherwise have its middle fragment replace the first,
            // and the reassembled line would be invalid JSON.
            carry.push_str(&String::from_utf8_lossy(&buf));
            let complete_upto = carry.rfind('\n').map(|i| i + 1).unwrap_or(0);
            let chunk = carry[..complete_upto].to_string();
            carry = carry[complete_upto..].to_string();
            for line in chunk.lines() {
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
    args: &HashMap<String, Box<RawValue>>,
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
    // The run's own arguments: resolving the selection with empty vars could
    // filter this run's manifest by a different node set than it built.
    let selected = resolve_selection(p, descriptor, args, true).await?;
    let ingested =
        windmill_common::dbt_manifest::ingest_manifest(
        &manifest,
        resource_path,
        p.default_database.as_deref(),
        selected.as_ref(),
    );
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
    args: &HashMap<String, Box<RawValue>>,
    strict: bool,
) -> error::Result<Option<std::collections::HashSet<String>>> {
    if descriptor.select.is_empty()
        && descriptor.exclude.is_empty()
        && descriptor.selector.is_none()
    {
        return Ok(None);
    }
    let mut cmd = dbt_command(p, &["ls"]);
    // A project whose models call `var()` without a default fails to parse
    // without these, so the selection resolver needs them exactly as the run
    // does. Placeholders that only a run can fill are dropped rather than
    // failing the deploy.
    add_vars(&mut cmd, descriptor, args, strict)?;
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
/// worker-local cache keyed by the script — which is also its limitation: a
/// retry that lands on a different worker finds nothing and says so, rather
/// than silently rebuilding everything.
fn state_dir(w_id: &str, script_path: &str) -> PathBuf {
    PathBuf::from(&*DBT_CACHE_DIR)
        .join("state")
        .join(digest(&format!("{w_id}/{script_path}")))
}

async fn save_run_state(p: &PreparedProject, w_id: &str) -> error::Result<()> {
    if p.script_path.is_empty() {
        return Ok(());
    }
    let dir = state_dir(w_id, &p.script_path);
    tokio::fs::create_dir_all(&dir).await.ok();
    for f in ["run_results.json", "manifest.json"] {
        tokio::fs::copy(p.project_dir.join("target").join(f), dir.join(f))
            .await
            .ok();
    }
    // Which checkout produced it. `latest` and placeholder refs move, and
    // resuming commit A's failed nodes against commit B's project is worse than
    // not resuming at all.
    tokio::fs::write(dir.join("commit"), &p.commit).await.ok();
    Ok(())
}

async fn restore_run_state(p: &PreparedProject, w_id: &str) -> error::Result<()> {
    if p.script_path.is_empty() {
        // A preview has no path to key state on, and an empty key is the one
        // that used to be shared by every dbt script in the workspace.
        return Err(Error::BadRequest(
            "`dbt_command: retry` needs a deployed script; a preview run has no state to \
             resume from"
                .to_string(),
        ));
    }
    let dir = state_dir(w_id, &p.script_path);
    if !dir.join("run_results.json").exists() {
        return Err(Error::BadRequest(
            "no previous dbt run to retry from on this worker; run the script normally instead"
                .to_string(),
        ));
    }
    let saved_commit = tokio::fs::read_to_string(dir.join("commit"))
        .await
        .unwrap_or_default();
    if saved_commit != p.commit {
        return Err(Error::BadRequest(format!(
            "the last dbt run on this worker was at commit {}, but this run resolved {}. \
             `dbt retry` resumes a specific checkout's failures; run the script normally instead",
            if saved_commit.is_empty() { "an unknown revision" } else { &saved_commit },
            p.commit
        )));
    }
    let target = p.project_dir.join("target");
    tokio::fs::create_dir_all(&target).await.ok();
    for f in ["run_results.json", "manifest.json"] {
        tokio::fs::copy(dir.join(f), target.join(f)).await.ok();
    }
    Ok(())
}

/// Append `--vars` if the descriptor (or the run) declares any.
fn add_vars(
    cmd: &mut Command,
    descriptor: &DbtDescriptor,
    args: &HashMap<String, Box<RawValue>>,
    strict: bool,
) -> error::Result<()> {
    let vars = resolved_vars(descriptor, args, strict)?;
    if !vars.is_empty() {
        cmd.args(["--vars", &serde_json::to_string(&vars).unwrap_or_default()]);
    }
    Ok(())
}

/// `strict` is the difference between the two callers. A run MUST fail on a
/// placeholder it cannot fill — silently substituting an empty string would let
/// the job build the wrong slice and report success. A deploy has no arguments
/// at all, so the same placeholder is expected there; the var still has to be
/// *defined* or a project calling `var("run_date")` without its own default
/// cannot be parsed, but its value is irrelevant to the graph.
fn resolved_vars(
    descriptor: &DbtDescriptor,
    args: &HashMap<String, Box<RawValue>>,
    strict: bool,
) -> error::Result<serde_json::Map<String, serde_json::Value>> {
    let mut out = serde_json::Map::new();
    for (k, v) in &descriptor.vars {
        let field = format!("vars.{k}");
        let value = match crate::common::interpolate_template(v, Some(args), &field) {
            Ok(value) => value,
            Err(e) if strict => return Err(e),
            Err(_) => String::new(),
        };
        out.insert(k.clone(), serde_json::Value::String(value));
    }
    // The run argument overrides; it never carries the descriptor's own values
    // back (its signature default is empty), so this cannot clobber what was
    // just interpolated above.
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

    // `dbt retry` restores the previous run's target/ from this directory. Two
    // dbt scripts in one workspace must not share it, or a retry resumes
    // another project's run_results.json against this project's checkout — and
    // an empty script_path is exactly how that happened.
    #[test]
    fn retry_state_is_per_script() {
        assert_ne!(state_dir("ws", "f/a/one"), state_dir("ws", "f/a/two"));
        assert_ne!(state_dir("ws1", "f/a/one"), state_dir("ws2", "f/a/one"));
        assert_eq!(state_dir("ws", "f/a/one"), state_dir("ws", "f/a/one"));
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
