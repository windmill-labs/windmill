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
use crate::handle_child::{
    get_mem_peak, handle_child, run_future_with_polling_update_job_poller, JobCtx, JobDeadline,
};
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
    /// The `<schema>/<database>` the profile resolved to at deploy. The
    /// resource is re-read on every run, so a schema or catalog changed on it
    /// afterwards moves every relation the project builds — and the stored
    /// graph, which still names the old ones, has to be re-ingested.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_relation_root: Option<String>,
    /// The `git_repository` resource path, never the resolved URL. A token-auth
    /// URL carries the token, and the lockfile lands in script metadata and
    /// workspace exports.
    pub repo_resource: String,
    /// dbt-core 1.x only: its adapter is a separate package versioning
    /// independently of core, so pinning core alone still lets a rebuilt cache
    /// resolve different runtime behavior.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adapter_version: Option<String>,
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
    let inv = Invocation { args: args.clone(), envs: envs.clone(), strict: true };
    // One wall clock for the whole job. A dbt job is a sequence of
    // subprocesses — provision, clone, deps, parse, ls, build, then the
    // `after_all` tests — and each would otherwise resolve the job's full
    // timeout for itself.
    let deadline = JobDeadline::start(conn, &job.workspace_id, job.id, job.timeout).await;
    let prepared = prepare_project(
        &descriptor,
        inner_content,
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
        deadline,
    )
    .await?;

    // Before any invocation: a per-run graph has to be re-ingested for the run's
    // writes to dispatch correctly, and an agent worker reaches the DB only
    // through the API. Refusing after `dbt build` would leave the warehouse
    // written and the job failed, and a retry would repeat the write.
    if (descriptor.is_latest_ref() || prepared.graph_is_per_run)
        && prepared.resource_path.is_some()
        && !matches!(conn, Connection::Sql(_))
    {
        return Err(Error::BadRequest(
            "this dbt script resolves its commit or its models per run, so its asset graph must \
             be re-ingested after every run — which an agent worker cannot do. Pin `ref` to a \
             commit and use literal `vars`, or run it on a worker with a database connection."
                .to_string(),
        ));
    }

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
    // force a full rebuild. Each attempt gets a fresh job dir, so that state is
    // restored from the worker-local cache — along with the ARGUMENTS it ran
    // with, since dbt reuses that invocation's selection and vars and the graph
    // refresh, the build and the test phase must all agree with it.
    let inv = if command == "retry" {
        Invocation { args: restore_run_state(&prepared, &job.workspace_id, &inv).await?, ..inv }
    } else {
        inv
    };

    // A per-run graph is ingested BEFORE the build, from a `dbt parse` of the
    // commit this run resolved: asset dispatch fans out from the stored rows the
    // moment the job completes. Concurrent runs of one dynamic script still race
    // on the path-keyed rows; that needs a per-job dispatch snapshot
    // (docs/dbt-runtime.md).
    if descriptor.is_latest_ref() || prepared.graph_is_per_run {
        if command != "retry" {
            run_dbt_parse(
                &prepared,
                &descriptor,
                &inv,
                &mut JobCtx { mem_peak, canceled_by, occupancy_metrics, worker_name, deadline },
                &job.id,
                &job.workspace_id,
                conn,
            )
            .await?;
        }
        // For a retry the restored manifest already describes the invocation
        // being resumed, so only the ingest runs — with that invocation's
        // arguments, which the selection resolver needs to interpolate.
        ingest_from_run(
            &prepared,
            &descriptor,
            &inv,
            &mut JobCtx { mem_peak, canceled_by, occupancy_metrics, worker_name, deadline },
            job,
            conn,
        )
        .await?;
    }

    let mut run = run_dbt(
        &prepared,
        &command,
        &descriptor,
        &inv,
        job,
        conn,
        mem_peak,
        canceled_by,
        occupancy_metrics,
        worker_name,
        true,
        deadline,
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
            &inv,
            job,
            conn,
            mem_peak,
            canceled_by,
            occupancy_metrics,
            worker_name,
            // The tests must be scoped exactly like the models were: testing
            // the whole project would assert against models this script never
            // builds, the same failure the ingest-side scoping fixes.
            true,
            deadline,
        )
        .await;
        results.extend(read_run_results(&prepared.project_dir).await);
    }

    save_run_state(&prepared, &job.workspace_id, &job.id, &inv)
        .await
        .ok();
    reconcile_materializations(&prepared, &results, job, conn).await;

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
    // A dependency job carries no per-job timeout of its own, but its phases
    // still share one wall clock rather than each getting the instance-wide
    // one.
    let deadline = JobDeadline::start(&conn, w_id, *job_id, None).await;
    let prepared = prepare_project(
        &descriptor,
        content,
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
        deadline,
    )
    .await?;

    // A deploy has no job arguments and no job environment, so it tolerates the
    // `{{ }}` placeholders only a run can fill (see `Invocation::strict`).
    let inv = Invocation { strict: false, ..Default::default() };
    run_dbt_parse(
        &prepared,
        &descriptor,
        &inv,
        &mut JobCtx { mem_peak, canceled_by, occupancy_metrics, worker_name, deadline },
        job_id,
        w_id,
        &conn,
    )
    .await?;

    let selected = resolve_selection(
        &prepared,
        &descriptor,
        &inv,
        &mut JobCtx { mem_peak, canceled_by, occupancy_metrics, worker_name, deadline },
        job_id,
        w_id,
        &conn,
    )
    .await?;
    let manifest = read_manifest(&prepared.project_dir).await?;
    let manifest_digest = digest(
        &tokio::fs::read_to_string(prepared.project_dir.join("target/manifest.json"))
            .await
            .unwrap_or_default(),
    );

    // Two deploys of one path can run concurrently — nothing serializes
    // dependency jobs — and the graph is keyed by path, not by version, so the
    // publication is claimed against this job's own version. It still returns
    // its lock, which belongs to that version.
    let deploying_hash = deploying_script_hash(db, job_id).await;
    let superseded = if let Some(resource_path) = prepared.resource_path.as_deref() {
        let ingested = windmill_common::dbt_manifest::ingest_manifest(
            &manifest,
            resource_path,
            prepared.default_database.as_deref(),
            selected.as_ref(),
        );
        let published = persist_ingest(
            db,
            w_id,
            script_path,
            &ingested,
            &DescriptorTriggers::parse(content),
            &prepared.relation_root(),
            deploying_hash,
        )
        .await?;
        if published {
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
        }
        !published
    } else {
        // No warehouse identity, so nothing can be ingested — but the previous
        // deploy's rows must still go. Leaving them means a descriptor edited
        // to use its own profiles.yml keeps claiming ownership of relations it
        // no longer describes, and keeps cascading from them.
        // The clear is a publication too — a newer deploy's graph must not be
        // wiped by an older job that no longer describes the script.
        let mut tx = db.begin().await?;
        let published =
            claim_graph_publication(&mut tx, w_id, script_path, deploying_hash).await?;
        if published {
            windmill_common::dbt_manifest::clear_dbt_manifest(&mut tx, w_id, script_path).await?;
            windmill_common::assets::replace_static_asset_usage(&mut tx, w_id, script_path, &[])
                .await?;
            tx.commit().await?;
            append_logs(
                job_id,
                w_id,
                "\nNo asset-graph ingest: the descriptor declares no `profile.resource`, so \
                 there is no warehouse identity to key `table://` assets on. Any previously \
                 ingested nodes for this script have been cleared.\n"
                    .to_string(),
                &conn,
            )
            .await;
        }
        !published
    };
    if superseded {
        append_logs(
            job_id,
            w_id,
            "\nA newer version of this script was deployed while this job ran, so the asset \
             graph was left describing that one.\n"
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
        profile_relation_root: Some(prepared.relation_root()),
        engine: prepared.engine.engine.as_str().to_string(),
        engine_version: prepared.engine.version.clone(),
        adapter_version: prepared.engine.adapter_version.clone(),
    })
    .map_err(|e| Error::internal_err(format!("serializing the dbt lockfile: {e}")))
}

/// The script version this dependency job is deploying. `None` for a raw
/// dependency job (the CLI's lock generation), which has no script row.
async fn deploying_script_hash(db: &sqlx::Pool<sqlx::Postgres>, job_id: &Uuid) -> Option<i64> {
    sqlx::query_scalar!("SELECT runnable_id FROM v2_job WHERE id = $1", job_id)
        .fetch_optional(db)
        .await
        .ok()
        .flatten()
        .flatten()
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
    /// The descriptor's `profile.target`, passed as `--target` so it applies to
    /// a project-owned `profiles.yml` as well as a rendered one.
    pub target: Option<String>,
    /// The profile target's database. Nodes that override it qualify their
    /// `table://` schema segment so two databases cannot collapse onto one node.
    pub default_database: Option<String>,
    /// The profile target's schema, for the drift check against the lockfile.
    pub default_schema: Option<String>,
    pub script_path: String,
    pub env: Vec<(String, String)>,
    /// The descriptor body, kept so an ingest can re-read its `# on` / `# mute`
    /// annotations without threading the content through every caller.
    pub descriptor_content: String,
    /// Digest of the SSH key material this run authenticates with. Scopes the
    /// worker-global caches; stable across jobs, unlike the key files' paths.
    pub credential_identity: String,
    /// The descriptor's `env`, resolved, in a stable order. Feeds run identity;
    /// `env` itself is not usable there because it carries per-job values.
    pub descriptor_env: std::collections::BTreeMap<String, String>,
    /// One-way digest of the rendered profile — the resolved connection, not
    /// just the names it exposes. A resource repointed from one warehouse to
    /// another that happens to use the same database and schema names is
    /// invisible to `relation_root`, and a retry would then execute the saved
    /// failures against a warehouse where the successful nodes do not exist.
    pub profile_digest: String,
}

impl PreparedProject {
    /// Where this run's relations live: the resolved schema and database. Drift
    /// here since the deploy means the stored graph names relations that no
    /// longer exist.
    fn relation_root(&self) -> String {
        format!(
            "{}|{}",
            self.default_schema.as_deref().unwrap_or(""),
            self.default_database.as_deref().unwrap_or(""),
        )
    }

    /// Everything that decides which relations a run produces, for the
    /// retry-state check: same repo, same project within it, same commit, same
    /// warehouse and target, same engine. Identity only, never credentials —
    /// `repo_resource` is a path and the profile is named, not rendered.
    ///
    /// Anything omitted here is something a redeploy could change while a
    /// stale `run_results.json` stays eligible, so `dbt retry` would resume one
    /// project's failures inside another.
    /// A digest of the DESCRIPTOR's resolved environment. It belongs in run
    /// identity because `env_var()` can drive a model's schema, database, alias
    /// or `enabled`, so changing a `$var:` value after a failed run makes the
    /// saved `run_results.json` describe relations this run would not produce.
    /// Digested rather than listed: the values are resolved secrets.
    ///
    /// Only the descriptor's own entries. `env` additionally carries `HOME`,
    /// set to this job's directory, which differs on every attempt — hashing it
    /// would make a retry reject its own predecessor every time.
    fn env_digest(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        for (k, v) in &self.descriptor_env {
            k.hash(&mut h);
            v.hash(&mut h);
        }
        h.finish()
    }

    fn run_identity(&self) -> String {
        // The descriptor is digested whole rather than field by field:
        // `select`, `exclude`, `selector`, `vars`, `full_refresh` and
        // `test_behavior` all change which nodes a run touches, and enumerating
        // them means the next field added to the descriptor is silently left
        // out of the check.
        format!(
            "{}|{}|{}|{}|{}|{:x}|{}|{}",
            self.repo_resource,
            self.project_subdir,
            self.commit,
            self.engine.engine.as_str(),
            digest(&self.descriptor_content),
            self.env_digest(),
            self.relation_root(),
            self.profile_digest,
        )
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn prepare_project(
    descriptor: &DbtDescriptor,
    descriptor_content: &str,
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
    deadline: JobDeadline,
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
    // outright; minting for an arbitrary runnable needs its own authorization
    // path. Refuse with the reason rather than letting
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
    let (git_ssh_cmd, credential_identity) = git_ssh_cmd(descriptor, job_dir, client).await?;
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
    // Properties of the DESCRIPTOR only, never of one run's arguments. Vars can
    // drive `enabled`, alias, schema, database and materialization, so a
    // placeholder var, a dynamic ref or a `$var:` env value (re-resolved every
    // run) each make the deploy-time graph a guess and must be re-ingested.
    //
    // A per-run `vars` override is deliberately NOT here: gating on it would
    // leave the override's graph in place for the next default run, which then
    // builds the descriptor's relations and dispatches from the override's.
    // Overrides are ad-hoc builds and follow the deployed descriptor for
    // cascade purposes — the same boundary a run-arg `select` override has
    // (docs/dbt-runtime.md).
    let graph_is_per_run = ref_is_per_run
        || descriptor
            .vars
            .values()
            .flat_map(windmill_parser_yaml::dbt::string_leaves)
            .any(has_placeholder)
        || descriptor.env.values().any(|v| v.starts_with("$var:"));
    let probe = GitRepo {
        url: url.clone(),
        commit: None,
        branch: branch.clone(),
        target_path: "dbt".to_string(),
    };
    // Borrowed only for the ref probes: `checkout` below takes the same state.
    let mut probe_ctx = JobCtx { mem_peak, canceled_by, occupancy_metrics, worker_name, deadline };
    let probe_job = (job_id, w_id, conn, &mut probe_ctx);
    let commit = if descriptor.is_latest_ref() {
        get_git_repo_full_head_commit_hash(&probe, &git_ssh_cmd, Some(probe_job)).await?
    } else if let Some(r) = interpolated_ref
        .clone()
        // A ref the descriptor spells with a placeholder is chosen by the run,
        // so the run's value wins over whatever the deploy happened to lock.
        .filter(|_| {
            descriptor
                .r#ref
                .as_deref()
                .is_some_and(|r| r.contains("{{"))
        })
    {
        resolve_git_ref_to_commit(&probe, &git_ssh_cmd, &r, Some(probe_job)).await?
    } else if let Some(locked) = locks.map(|l| l.commit.clone()).filter(|c| !c.is_empty()) {
        locked
    } else if let Some(r) = interpolated_ref.clone() {
        // The descriptor's ref before a lockfile exists (deploy). It has to be
        // resolved rather than used as-is: a branch name is not a pin, and the
        // clone cache below keys on the commit precisely because commits are
        // immutable and a branch name is not.
        resolve_git_ref_to_commit(&probe, &git_ssh_cmd, &r, Some(probe_job)).await?
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
        &credential_identity,
        job_dir,
        job_id,
        w_id,
        worker_name,
        conn,
        mem_peak,
        canceled_by,
        occupancy_metrics,
        &git_ssh_cmd,
        deadline,
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

    let (profiles_dir, resource_path, adapter, default_database, default_schema, profile_digest) =
        write_profiles(descriptor, &project_dir, job_dir, client, job_id).await?;
    // The lockfile's version, when it pinned one for this same engine — a
    // descriptor edited to another engine invalidates the pin.
    let pinned_version = locks
        .filter(|l| l.engine == descriptor.engine().as_str())
        .map(|l| l.engine_version.as_str())
        .filter(|v| !v.is_empty());
    let engine = provision_engine(
        descriptor.engine(),
        adapter,
        pinned_version,
        locks
            .filter(|l| l.engine == descriptor.engine().as_str())
            .and_then(|l| l.adapter_version.as_deref())
            .filter(|v| !v.is_empty()),
        job_id,
        w_id,
        conn,
        &mut JobCtx { mem_peak, canceled_by, occupancy_metrics, worker_name, deadline },
    )
    .await?;

    let resolved_env = resolve_env(descriptor, client).await?;
    let descriptor_env: std::collections::BTreeMap<String, String> =
        resolved_env.iter().cloned().collect();
    let mut env = resolved_env;
    // Both engines write their profile-independent state under the project;
    // pinning it inside the job dir keeps a job from touching a shared $HOME.
    env.push(("HOME".to_string(), job_dir.to_string()));

    let mut prepared = PreparedProject {
        profile_digest,
        project_dir,
        profiles_dir,
        engine,
        commit: checked_out,
        ref_is_per_run,
        graph_is_per_run,
        project_subdir,
        repo_resource: repo_res,
        resource_path,
        target: descriptor.profile.target.clone(),
        descriptor_content: descriptor_content.to_string(),
        descriptor_env,
        credential_identity,

        default_database,
        default_schema,
        script_path: script_path.to_string(),
        env,
    };
    let mut ctx = JobCtx { mem_peak, canceled_by, occupancy_metrics, worker_name, deadline };
    // A profile resource moved — a changed schema, dataset or catalog — relocates
    // every relation the project builds, so the stored graph names ones that no
    // longer exist. Compared against the graph AS STORED, not against the deploy
    // lock: moving A→B then back to A matches the lock again while the stored
    // graph is still at B.
    match conn {
        Connection::Sql(db) => {
            if let Some(stored) = sqlx::query_scalar!(
                "SELECT relation_root FROM dbt_node
                  WHERE workspace_id = $1 AND script_path = $2 AND relation_root IS NOT NULL
                  LIMIT 1",
                w_id,
                script_path,
            )
            .fetch_optional(db)
            .await?
            .flatten()
            {
                if stored != prepared.relation_root() {
                    prepared.graph_is_per_run = true;
                }
            }
        }
        // An agent worker can neither read the stored root nor re-ingest, so it
        // cannot establish that the graph still describes what this run will
        // build. Matching the deploy lock is not proof: another worker may have
        // re-ingested a different root after a drift that has since been undone.
        // Refuse only when the profile is one Windmill resolves — a project
        // bringing its own `profiles.yml` has no root for Windmill to track, and
        // its graph was never keyed on one.
        Connection::Http(_) => {
            if prepared.resource_path.is_some() {
                return Err(Error::BadRequest(format!(
                    "this dbt script's asset graph is keyed on the relations its profile \
                     resolves to (`{}`), and an agent worker can neither verify nor re-ingest \
                     that — so a profile changed since the last ingest would silently cascade \
                     from the wrong relations. Run it on a worker with a database connection",
                    prepared.relation_root()
                )));
            }
        }
    }
    install_packages(&prepared, &mut ctx, job_id, w_id, conn).await?;
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
) -> error::Result<(String, String)> {
    let mut identities = String::new();
    // Digested from the key material, not from the `-i <path>` command: those
    // paths live under the per-job dir, so hashing the command would make every
    // run a cache miss and re-clone a repo the cache was supposed to hold.
    let mut credential = std::collections::hash_map::DefaultHasher::new();
    for (i, var_path) in descriptor.git_ssh_identity.iter().enumerate() {
        let name = format!(".ssh_id_priv_dbt_{i}");
        let loc = windmill_common::worker::is_allowed_file_location(job_dir, &name)?;
        let mut content = client.get_variable_value(var_path).await.map_err(|e| {
            Error::NotFound(format!(
                "variable {var_path} not found for `git_ssh_identity`: {e:#}"
            ))
        })?;
        content.push('\n');
        {
            use std::hash::Hash;
            var_path.hash(&mut credential);
            content.hash(&mut credential);
        }
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
    use std::hash::Hasher;
    Ok((
        format!("ssh -o StrictHostKeyChecking=no{identities}"),
        format!("{:x}", credential.finish()),
    ))
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
    // Identifies WHO may reuse a cached private checkout, from the key material
    // rather than the per-job paths it was written to.
    credential_identity: &str,
    job_dir: &str,
    job_id: &Uuid,
    w_id: &str,
    worker_name: &str,
    conn: &Connection,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    occupancy_metrics: &mut OccupancyMetrics,
    git_ssh_cmd: &str,
    deadline: JobDeadline,
) -> error::Result<String> {
    let dest = PathBuf::from(job_dir).join("dbt");
    if !commit.is_empty() {
        // Keyed by workspace and credential as well as the commit. The cache
        // holds a checkout of a private repository, and a literal 40-char ref
        // needs no `ls-remote` to resolve — so a key of url+commit alone would
        // let any workspace that knows both read a repo it cannot authenticate
        // to.
        let cached = PathBuf::from(&*DBT_CACHE_DIR).join("repos").join(format!(
            "{}-{}",
            digest(&format!("{w_id}\n{}\n{credential_identity}", repo.url)),
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
            deadline,
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
        deadline,
    )
    .await
}

/// `dbt deps`, with `dbt_packages/` restored from a cache keyed by the digest
/// of `packages.yml` — the file that determines the whole tree.
async fn install_packages(
    p: &PreparedProject,
    ctx: &mut JobCtx<'_>,
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
    // Workspace and credential identity are in the key: `dbt deps` runs with the
    // descriptor's environment and can fetch private git packages, so a shared
    // tree would let one workspace execute another's private package code
    // without ever authenticating.
    let mut key = format!(
        "{w_id}\n{}\n{}\n{}\n{:x}\n",
        p.commit,
        p.project_subdir,
        p.credential_identity,
        p.env_digest(),
    );
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
    run_prep_command(
        p,
        dbt_command(p, &["deps"]),
        "dbt deps",
        ctx,
        job_id,
        w_id,
        conn,
    )
    .await?;
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
    let Some(parent) = cached.parent() else {
        return;
    };
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
) -> error::Result<(
    PathBuf,
    Option<String>,
    DbtAdapter,
    Option<String>,
    Option<String>,
    String,
)> {
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
            None => {
                adapter_from_profiles_yml(
                    &path,
                    &project_profile_name(project_dir).await,
                    descriptor.profile.target.as_deref(),
                )
                .await?
            }
        };
        ensure_adapter_licensed(adapter)?;
        // The project owns its profile, so Windmill knows neither its database
        // nor its schema. `table_asset_path` then qualifies every relation that
        // names a database, because assuming two share one is what would
        // collapse distinct relations onto a single node.
        let profile_digest = digest(&tokio::fs::read_to_string(&path).await.unwrap_or_default());
        return Ok((dir, resource_path, adapter, None, None, profile_digest));
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
    let dir = PathBuf::from(job_dir).join("dbt_profiles");
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| Error::internal_err(format!("creating the profiles dir: {e}")))?;
    let rendered = render_profile(
        adapter,
        &value,
        &profile_name,
        target,
        descriptor.threads,
        descriptor.profile.schema.as_deref(),
        &dir,
    )?;
    write_file(dir.to_str().unwrap(), "profiles.yml", &rendered.yaml)?;
    if let Some(pem) = rendered.root_certificate_pem.as_deref() {
        write_file(
            dir.to_str().unwrap(),
            crate::dbt_profiles::ROOT_CERT_FILENAME,
            pem,
        )?;
    }
    let profile_digest = profile_identity_digest(
        &rendered.yaml,
        &dir,
        rendered.root_certificate_pem.as_deref(),
    );
    Ok((
        dir,
        Some(resource_path),
        adapter,
        rendered.database,
        rendered.schema,
        profile_digest,
    ))
}

/// Identifies the connection a rendered profile describes, for run identity.
///
/// The per-job profiles dir is spelled out in the YAML when a private CA is
/// configured (`sslrootcert`) and differs on every attempt, so hashing the
/// rendered text as-is would make a retry reject its own predecessor. The
/// certificate is part of the connection, so it is hashed in place of its path.
fn profile_identity_digest(yaml: &str, profiles_dir: &Path, root_cert_pem: Option<&str>) -> String {
    digest(&format!(
        "{}\n{}",
        yaml.replace(profiles_dir.to_str().unwrap_or_default(), "$PROFILES_DIR"),
        root_cert_pem.unwrap_or_default(),
    ))
}

async fn adapter_from_profiles_yml(
    path: &Path,
    profile_name: &str,
    target: Option<&str>,
) -> error::Result<DbtAdapter> {
    let content = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| Error::BadRequest(format!("could not read {}: {e}", path.display())))?;
    let v: serde_yml::Value = serde_yml::from_str(&content)
        .map_err(|e| Error::BadRequest(format!("could not parse {}: {e}", path.display())))?;
    // The profile the project names and the target actually in use, not the
    // first `type:` in the file: a `profiles.yml` may define several profiles
    // and several targets, and provisioning the wrong adapter installs the
    // wrong package and license-checks the wrong warehouse.
    let outputs = v
        .get(profile_name)
        .and_then(|p| p.get("outputs"))
        .ok_or_else(|| {
            Error::BadRequest(format!(
                "{} declares no profile named `{profile_name}` (the name in dbt_project.yml)",
                path.display()
            ))
        })?;
    let target = target
        .or_else(|| {
            v.get(profile_name)
                .and_then(|p| p.get("target"))
                .and_then(|t| t.as_str())
        })
        .ok_or_else(|| {
            Error::BadRequest(format!(
                "{} names no default target for `{profile_name}`; set `profile.target`",
                path.display()
            ))
        })?;
    let t = outputs
        .get(target)
        .and_then(|o| o.get("type"))
        .and_then(|t| t.as_str())
        .ok_or_else(|| {
            Error::BadRequest(format!(
                "{} has no `{target}` target with a `type` under `{profile_name}`",
                path.display()
            ))
        })?;
    DbtAdapter::from_resource_type(t)
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
    // The rendered profile is written with this target as its only output, but
    // a project-owned `profiles.yml` has its own default — silently building
    // `dev` when the descriptor asked for `prod` writes to the wrong warehouse.
    if let Some(target) = p.target.as_deref() {
        cmd.args(["--target", target]);
    }
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
    inv: &Invocation,
    job: &MiniPulledJob,
    conn: &Connection,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    occupancy_metrics: &mut OccupancyMetrics,
    worker_name: &str,
    with_selection: bool,
    deadline: JobDeadline,
) -> error::Result<()> {
    let mut cmd = dbt_command(p, &[command]);
    cmd.envs(&inv.envs);
    // The console stays human-readable and goes straight to the job log; the
    // machine-readable copy goes to a file the progress reporter tails, so
    // neither purpose degrades the other.
    let log_dir = p.project_dir.join("logs");
    cmd.arg("--log-path")
        .arg(&log_dir)
        .args(["--log-format-file", "json"])
        .args(["--log-level-file", p.engine.engine.progress_log_level()]);

    if with_selection && command != "retry" {
        add_selection(&mut cmd, descriptor, inv);
    }
    if command != "retry" {
        add_vars(&mut cmd, descriptor, inv)?;
        if let Some(t) = descriptor.threads {
            cmd.args(["--threads", &t.to_string()]);
        }
        let full_refresh = arg_bool(&inv.args, "full_refresh").unwrap_or(descriptor.full_refresh);
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
        // What is left of the job's wall clock: `dbt build` follows the whole
        // preparation sequence, and the `after_all` tests follow it.
        deadline.remaining_secs(),
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
        // Agent workers reach the DB only through the API. Their per-model state
        // is settled from run_results.json at the end of the run instead — at
        // the end rather than live, but recorded.
        return None;
    };
    if !p.engine.engine.emits_node_events() {
        // Nothing to read: those engines write a text file log, so tailing it
        // would burn a task per run for no events.
        return None;
    }
    let (db, w_id, job_id) = (db.clone(), job.workspace_id.clone(), job.id);
    let resource_path = p.resource_path.clone()?;
    let default_database = p.default_database.clone();
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
                let Some(ev) = parse_node_event(line, &resource_path, default_database.as_deref())
                else {
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
fn parse_node_event(
    line: &str,
    resource_path: &str,
    default_database: Option<&str>,
) -> Option<RecordMaterializationRequest> {
    let line = line.trim();
    if !line.starts_with('{') {
        return None;
    }
    let v: serde_json::Value = serde_json::from_str(line).ok()?;
    let info = v.get("data")?.get("node_info")?;
    let rel = info.get("node_relation")?;
    let schema = rel.get("schema")?.as_str()?;
    let alias = rel.get("alias")?.as_str()?;
    let database = rel.get("database").and_then(|d| d.as_str());
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
    let path = windmill_common::dbt_manifest::table_asset_path(
        resource_path,
        database,
        schema,
        alias,
        default_database,
    );
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
    let Some(resource_path) = p.resource_path.as_deref() else {
        return;
    };
    for r in results {
        let Some(path) = asset_path_of_relation(
            r.relation_name.as_deref(),
            resource_path,
            p.default_database.as_deref(),
        ) else {
            continue;
        };
        let status = match r.status.as_str() {
            "success" => MaterializationStatus::Materialized,
            "error" | "fail" | "runtime error" => MaterializationStatus::Failed,
            // Tests and skipped nodes say nothing about a relation's state.
            _ => continue,
        };
        let error = (status == MaterializationStatus::Failed)
            .then(|| r.message.as_deref())
            .flatten();
        // An agent worker has no direct DB, so its outcomes go through the API —
        // otherwise a successful agent run leaves every model with no recorded
        // status or row count.
        let recorded = match conn {
            Connection::Sql(db) => record_materialization(
                db,
                &job.workspace_id,
                windmill_common::assets::AssetKind::Table,
                &path,
                windmill_common::materialization::UNPARTITIONED,
                status,
                None,
                r.rows_affected,
                Some(job.id),
                error,
            )
            .await
            .map_err(|e| e.to_string()),
            Connection::Http(http) => crate::agent_workers::record_materialization_from_agent_http(
                http,
                &job.workspace_id,
                &RecordMaterializationRequest {
                    asset_kind: windmill_common::assets::AssetKind::Table,
                    asset_path: path.clone(),
                    partition: windmill_common::materialization::UNPARTITIONED.to_string(),
                    status,
                    snapshot_id: None,
                    row_count: r.rows_affected,
                    job_id: Some(job.id),
                    error: error.map(|e| e.to_string()),
                    schema: None,
                },
            )
            .await
            .map_err(|e| e.to_string()),
        };
        if let Err(e) = recorded {
            tracing::warn!("recording the materialization of {path} failed: {e}");
        }
    }
}

/// `"db"."schema"."name"` from dbt into the `table://` path of the relation,
/// through the same derivation the manifest ingest and the live events use.
fn asset_path_of_relation(
    relation_name: Option<&str>,
    resource_path: &str,
    default_database: Option<&str>,
) -> Option<String> {
    let parts = split_relation(relation_name?);
    let (database, schema, name) = match parts.as_slice() {
        [db, schema, name] => (Some(db.as_str()), schema.as_str(), name.as_str()),
        [schema, name] => (None, schema.as_str(), name.as_str()),
        _ => return None,
    };
    Some(windmill_common::dbt_manifest::table_asset_path(
        resource_path,
        database,
        schema,
        name,
        default_database,
    ))
}

/// Split `"db"."schema"."name"` on the separators BETWEEN identifiers only.
///
/// A period inside a quoted identifier is part of the name — `"analytics.v2"`
/// is one schema, not two — and splitting on every period discards the relation
/// entirely, so the model silently records no status at all.
fn split_relation(rel: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    for c in rel.chars() {
        match quote {
            Some(q) => {
                if c == q || (q == '[' && c == ']') {
                    quote = None;
                } else {
                    current.push(c);
                }
            }
            None if c == '"' || c == '`' || c == '[' => quote = Some(c),
            None if c == '.' => parts.push(std::mem::take(&mut current)),
            None => current.push(c),
        }
    }
    parts.push(current);
    parts.into_iter().map(|p| p.trim().to_string()).collect()
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
    inv: &Invocation,
    ctx: &mut JobCtx<'_>,
    job: &MiniPulledJob,
    conn: &Connection,
) -> error::Result<()> {
    // No warehouse identity means there is no graph that could go stale.
    let Some(resource_path) = p.resource_path.as_deref() else {
        return Ok(());
    };
    // The caller rejects this configuration before invoking dbt; this is the
    // backstop for any other path into the ingest.
    let Connection::Sql(db) = conn else {
        return Err(Error::internal_err(
            "cannot re-ingest a dbt graph without a database connection".to_string(),
        ));
    };
    let Some(script_path) = job.runnable_path.as_deref() else {
        return Ok(());
    };
    let manifest = read_manifest(&p.project_dir).await?;
    // The run's own arguments: resolving the selection with empty vars could
    // filter this run's manifest by a different node set than it built.
    let selected =
        resolve_selection(p, descriptor, inv, ctx, &job.id, &job.workspace_id, conn).await?;
    let ingested = windmill_common::dbt_manifest::ingest_manifest(
        &manifest,
        resource_path,
        p.default_database.as_deref(),
        selected.as_ref(),
    );
    // A run refreshes the graph of the version it IS, so it always publishes:
    // it was pulled for the hash it runs, and a newer deploy's own dependency
    // job publishes after it.
    persist_ingest(
        db,
        &job.workspace_id,
        script_path,
        &ingested,
        &DescriptorTriggers::parse(&p.descriptor_content),
        &p.relation_root(),
        None,
    )
    .await?;
    // Synchronously, not through the notify poller: dispatch for THIS job runs
    // in this process once the job completes, and the poll is seconds away, so
    // a fast build would otherwise fan out from the pre-refresh cache. The
    // `notify_event` the transaction emitted still reaches every other process.
    windmill_queue::asset_dispatch::ASSET_PRODUCER_WRITES_CACHE.remove(&job.workspace_id);
    Ok(())
}

/// The descriptor's own `# on` / `# mute` / `# debounce` / `# retry`, which the
/// derived subscriptions must respect rather than overwrite.
pub struct DescriptorTriggers {
    explicit_refs: std::collections::HashSet<String>,
    muted_refs: std::collections::HashSet<String>,
    mute_all: bool,
    join_all: bool,
    debounce_s: Option<i32>,
    retry_count: Option<i16>,
    retry_delay_s: Option<i32>,
}

impl DescriptorTriggers {
    /// Parsed from the descriptor body, which is YAML with `#` comments — the
    /// same annotation grammar every other language uses.
    fn parse(content: &str) -> Self {
        use windmill_common::assets::{trigger_spec_to_row, ScriptTriggerKind};
        let a = windmill_common::assets::parse_pipeline_annotations(content);
        let refs = |specs: &[windmill_parser::asset_parser::TriggerSpec]| {
            specs
                .iter()
                .filter_map(|s| {
                    trigger_spec_to_row(s)
                        .filter(|(k, _)| *k == ScriptTriggerKind::Asset)
                        .map(|(_, r)| r)
                })
                .collect()
        };
        Self {
            explicit_refs: refs(&a.triggers),
            muted_refs: refs(&a.mute),
            mute_all: a.mute_all,
            join_all: !a.join_mode.is_any(),
            debounce_s: a
                .debounce_default
                .as_deref()
                .and_then(windmill_common::assets::parse_duration_secs),
            retry_count: a
                .retry
                .as_ref()
                .map(|r| r.count.min(i16::MAX as u32) as i16),
            retry_delay_s: a
                .retry
                .as_ref()
                .and_then(|r| r.delay.as_deref())
                .and_then(windmill_common::assets::parse_duration_secs),
        }
    }
}

/// Write one ingest: the sidecar rows, the `asset` usages, and the cascade
/// subscriptions its reads imply.
///
/// The subscriptions have to happen here rather than in the deploy's generic
/// derivation, which runs before these assets exist — it reads the script's
/// parsed content, and a dbt script's assets come from a manifest the
/// dependency job produces afterwards. Without them a project split across
/// scripts renders its upstream read edges and never actually cascades along
/// them, which is decision 6's whole point.
/// Replace this script's graph, unless a newer version of it has been deployed.
///
/// Returns whether it published. The check runs inside the same transaction as
/// the writes and behind a lock every publisher for this path takes: checking
/// first and writing after leaves a window where a newer job publishes in
/// between and the older one overwrites it, which no later job repairs.
async fn persist_ingest(
    db: &sqlx::Pool<sqlx::Postgres>,
    w_id: &str,
    script_path: &str,
    ingested: &windmill_common::dbt_manifest::IngestedManifest,
    annotations: &DescriptorTriggers,
    relation_root: &str,
    // The version being deployed. `None` for a raw dependency job (the CLI's
    // lock generation), which has no script row to compare against.
    deploying_hash: Option<i64>,
) -> error::Result<bool> {
    use windmill_common::assets::{AssetUsageKind, ScriptTriggerKind};
    let mut tx = db.begin().await?;
    if !claim_graph_publication(&mut tx, w_id, script_path, deploying_hash).await? {
        return Ok(false);
    }
    windmill_common::dbt_manifest::replace_dbt_manifest(
        &mut tx,
        w_id,
        script_path,
        ingested,
        relation_root,
    )
    .await?;
    windmill_common::assets::replace_static_asset_usage(
        &mut tx,
        w_id,
        script_path,
        &ingested.assets,
    )
    .await?;
    // Only the manifest-DERIVED subscriptions are replaced. The deploy already
    // inserted whatever the descriptor authored with `# on`, carrying its
    // debounce/retry/join opts, and `# mute` opted refs out; wiping every
    // trigger here would silently discard all of that.
    let derived = windmill_common::assets::derive_pipeline_asset_trigger_refs(
        &ingested.assets,
        &annotations.explicit_refs,
        &annotations.muted_refs,
        annotations.mute_all,
    );
    // Delete every derived row, including the ones about to be reinserted:
    // `script_trigger` has no uniqueness constraint and the subscriber lookup
    // does not dedupe, so keeping them would double the downstream jobs on the
    // first refresh and add another copy on every one after. Authored refs are
    // excluded from the delete — they carry opts this ingest cannot rebuild.
    let authored: Vec<String> = annotations.explicit_refs.iter().cloned().collect();
    sqlx::query!(
        "DELETE FROM script_trigger
          WHERE workspace_id = $1 AND runnable_kind = 'script' AND runnable_path = $2
            AND trigger_kind = 'asset' AND trigger_ref LIKE 'table://%'
            AND NOT (trigger_ref = ANY($3))",
        w_id,
        script_path,
        &authored[..],
    )
    .execute(&mut *tx)
    .await?;
    for trigger_ref in derived {
        windmill_common::assets::insert_script_trigger(
            &mut *tx,
            w_id,
            AssetUsageKind::Script,
            script_path,
            ScriptTriggerKind::Asset,
            &trigger_ref,
            annotations.join_all,
            annotations.debounce_s,
            annotations.retry_count,
            annotations.retry_delay_s,
        )
        .await?;
    }
    tx.commit().await?;
    Ok(true)
}

/// Serialize publishers for one script path and confirm this job's version is
/// still the newest. Both happen inside the caller's transaction, so a newer
/// publisher either commits before this check sees it, or waits behind it and
/// overwrites afterwards — which is the correct order either way.
async fn claim_graph_publication(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    w_id: &str,
    script_path: &str,
    deploying_hash: Option<i64>,
) -> error::Result<bool> {
    sqlx::query!(
        "SELECT pg_advisory_xact_lock(hashtext($1))",
        format!("dbt_graph:{w_id}:{script_path}")
    )
    .execute(&mut **tx)
    .await?;
    let Some(mine) = deploying_hash else {
        return Ok(true);
    };
    // Not `get_latest_script_hash`: its `lock IS NOT NULL` predicate names the
    // PREVIOUS version while this one is being deployed, so every deploy would
    // look superseded.
    let latest = sqlx::query_scalar!(
        "SELECT hash FROM script WHERE workspace_id = $1 AND path = $2 AND deleted = false \
         ORDER BY created_at DESC LIMIT 1",
        w_id,
        script_path
    )
    .fetch_optional(&mut **tx)
    .await?;
    Ok(latest.is_none_or(|latest| latest == mine))
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
    inv: &Invocation,
    ctx: &mut JobCtx<'_>,
    job_id: &Uuid,
    w_id: &str,
    conn: &Connection,
) -> error::Result<Option<std::collections::HashSet<String>>> {
    if !has_selection(descriptor, inv) {
        return Ok(None);
    }
    let mut cmd = dbt_command(p, &["ls"]);
    cmd.envs(&inv.envs);
    // A project whose models call `var()` without a default fails to parse
    // without these, so the selection resolver needs them exactly as the run
    // does. Placeholders that only a run can fill are dropped rather than
    // failing the deploy.
    add_vars(&mut cmd, descriptor, inv)?;
    // The types spelled out rather than `all`, which dbt-core 2.x rejects.
    for t in ["model", "source", "seed", "snapshot", "test"] {
        cmd.args(["--resource-type", t]);
    }
    cmd.args(["--output", "json", "--quiet"]);
    add_selection(&mut cmd, descriptor, inv);
    // Captured directly rather than through `handle_child`: its `pipe_stdout`
    // path runs the output through the job-log writer, which `NO_LOGS_AT_ALL`
    // discards — the selection would then resolve to the empty set and the
    // ingest would wipe the script's assets and subscriptions while dbt went on
    // building the descriptor's models.
    let stdout = run_capturing(cmd, "dbt ls", ctx, job_id, w_id, conn).await?;
    let mut set = std::collections::HashSet::new();
    for line in stdout.lines() {
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
    if set.is_empty() {
        // A selection that matches nothing would be ingested as "this script
        // owns no relations", wiping its graph and cascade edges — the same
        // outcome a failed capture produces, and indistinguishable from it.
        // Refuse rather than silently un-wire the script.
        return Err(Error::ExecutionErr(
            "the descriptor's `select`/`exclude` matched no dbt nodes; fix the selection rather \
             than deploying a script that owns nothing"
                .to_string(),
        ));
    }
    Ok(Some(set))
}

/// Run a command for its stdout under the job's cancellation and timeout.
/// The same poller `handle_child` uses drives them, so a cancel or a deadline
/// drops the wait future — which owns the child, and `kill_on_drop` then
/// terminates it. Dropping a wait future does NOT by itself kill a process, so
/// without that flag the child would outlive the job.
async fn run_capturing(
    mut cmd: Command,
    name: &str,
    ctx: &mut JobCtx<'_>,
    job_id: &Uuid,
    w_id: &str,
    conn: &Connection,
) -> error::Result<String> {
    let child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
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
        return Err(Error::ExecutionErr(format!(
            "{name} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        )));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

/// Run a preparation command through the same child handler the build uses, so
/// cancellation and the job timeout apply to it too.
async fn run_prep_command(
    p: &PreparedProject,
    mut cmd: Command,
    name: &str,
    ctx: &mut JobCtx<'_>,
    job_id: &Uuid,
    w_id: &str,
    conn: &Connection,
) -> error::Result<()> {
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let child = start_child_process(cmd, p.engine.bin.to_string_lossy().as_ref(), false).await?;
    handle_child(
        job_id,
        conn,
        ctx.mem_peak,
        ctx.canceled_by,
        child,
        false,
        ctx.worker_name,
        w_id,
        name,
        ctx.timeout(),
        false,
        &mut Some(ctx.occupancy_metrics),
        None,
        None,
    )
    .await
    .map(|_| ())
}

/// `dbt parse`, which writes `target/manifest.json` without touching the
/// warehouse. Both the deploy and a per-run graph refresh need the manifest
/// before anything else happens.
async fn run_dbt_parse(
    p: &PreparedProject,
    descriptor: &DbtDescriptor,
    inv: &Invocation,
    ctx: &mut JobCtx<'_>,
    job_id: &Uuid,
    w_id: &str,
    conn: &Connection,
) -> error::Result<()> {
    let mut cmd = dbt_command(p, &["parse"]);
    cmd.envs(&inv.envs);
    add_vars(&mut cmd, descriptor, inv)?;
    run_prep_command(p, cmd, "dbt parse", ctx, job_id, w_id, conn).await
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

async fn save_run_state(
    p: &PreparedProject,
    w_id: &str,
    // Scopes the staging directory. Keyed by commit, two concurrent runs of one
    // script would stage into the same place and publish a mixture.
    job_id: &Uuid,
    inv: &Invocation,
) -> error::Result<()> {
    if p.script_path.is_empty() {
        return Ok(());
    }
    // Each run writes its OWN generation directory, which is never modified
    // afterwards, and publication is a rename of the pointer naming it. Runs
    // that replaced one live directory could interleave — B's results beside
    // A's manifest and arguments — and a reader copying that directory could
    // straddle a replacement and take half of each. A retry resuming a mixture
    // is worse than one that finds nothing.
    let dir = state_dir(w_id, &p.script_path);
    let generation = format!("gen-{job_id}");
    let staging = dir.join(&generation);
    tokio::fs::remove_dir_all(&staging).await.ok();
    if tokio::fs::create_dir_all(&staging).await.is_err() {
        return Ok(());
    }
    for f in ["run_results.json", "manifest.json"] {
        if tokio::fs::copy(p.project_dir.join("target").join(f), staging.join(f))
            .await
            .is_err()
        {
            tokio::fs::remove_dir_all(&staging).await.ok();
            return Ok(());
        }
    }
    // What produced it. `latest` and placeholder refs move, and a redeploy can
    // repoint the profile, so resuming one invocation's failed nodes against a
    // different checkout — or a different warehouse — is worse than not
    // resuming at all. The arguments come back too: `dbt retry` reuses the
    // original invocation's selection and vars, so refreshing the graph for it
    // needs those, not this job's.
    let state = SavedRunState {
        // Includes the invocation's own environment: script-level variables are
        // applied to parse, ls and the build just as the descriptor's are, so a
        // change to one after a failure makes the saved results describe
        // relations a retry would not produce.
        identity: format!("{}|{:x}", p.run_identity(), inv.env_digest()),
        args: inv
            .args
            .iter()
            .map(|(k, v)| (k.clone(), v.get().to_string()))
            .collect(),
    };
    if tokio::fs::write(
        staging.join("state.json"),
        serde_json::to_vec(&state).unwrap_or_default(),
    )
    .await
    .is_err()
    {
        tokio::fs::remove_dir_all(&staging).await.ok();
        return Ok(());
    }
    // Publishing is one rename over the pointer file. A reader either sees the
    // previous generation's name or this one's, never a directory being
    // rebuilt under it.
    let pointer_staging = dir.join(format!(".{generation}.pointer"));
    if tokio::fs::write(&pointer_staging, generation.as_bytes())
        .await
        .is_err()
        || tokio::fs::rename(&pointer_staging, dir.join(CURRENT_GENERATION))
            .await
            .is_err()
    {
        tokio::fs::remove_file(&pointer_staging).await.ok();
        tokio::fs::remove_dir_all(&staging).await.ok();
        return Ok(());
    }
    prune_old_generations(&dir, &generation).await;
    Ok(())
}

/// Names the generation directory a retry reads. Replaced by rename, so it is
/// always one name or the other.
const CURRENT_GENERATION: &str = "current";

/// How long a superseded generation stays readable. A retry that has already
/// read the pointer is still copying out of that directory, so it cannot be
/// removed the moment the next run publishes.
const GENERATION_GRACE_SECS: u64 = 3600;

async fn prune_old_generations(dir: &Path, keep: &str) {
    let Ok(mut entries) = tokio::fs::read_dir(dir).await else {
        return;
    };
    let now = std::time::SystemTime::now();
    while let Ok(Some(e)) = entries.next_entry().await {
        let name = e.file_name().to_string_lossy().to_string();
        if !name.starts_with("gen-") || name == keep {
            continue;
        }
        let stale = e
            .metadata()
            .await
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| now.duration_since(t).ok())
            .is_some_and(|age| age.as_secs() > GENERATION_GRACE_SECS);
        if stale {
            tokio::fs::remove_dir_all(e.path()).await.ok();
        }
    }
}

/// Everything a dbt invocation is parameterized by. One struct because every
/// command in a run — `parse`, `ls`, `build` — must see the SAME arguments and
/// environment: a difference between any two of them means the graph describes
/// something other than what was built, silently.
#[derive(Clone, Default)]
pub struct Invocation {
    pub args: HashMap<String, Box<RawValue>>,
    pub envs: HashMap<String, String>,
    /// A run must fail on a `{{ }}` placeholder it cannot fill; a deploy, which
    /// has no arguments at all, tolerates them. Declared rather than inferred
    /// from the argument count: a run submitted with `{}` is still a run, and
    /// treating it as a deploy would blank its placeholders and build against
    /// an unintended schema or alias.
    pub strict: bool,
}

impl Invocation {
    /// Digest of the script-level environment, for retry identity. Values are
    /// secrets, so they are hashed rather than stored.
    fn env_digest(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut sorted: Vec<(&String, &String)> = self.envs.iter().collect();
        sorted.sort();
        let mut h = std::collections::hash_map::DefaultHasher::new();
        for (k, v) in sorted {
            k.hash(&mut h);
            v.hash(&mut h);
        }
        h.finish()
    }
}

/// What an invocation was, so a later `dbt retry` can prove it is resuming the
/// same thing rather than replaying failures somewhere else.
#[derive(Serialize, Deserialize, Debug, Default)]
struct SavedRunState {
    /// Repo, project, commit, warehouse and engine — everything that decides
    /// which relations the restored `run_results.json` describes.
    identity: String,
    /// The invocation's job arguments, as raw JSON per key. `dbt retry` reuses
    /// the original selection and vars, so refreshing the graph for it needs
    /// these rather than the retry request's.
    args: HashMap<String, String>,
}

/// Restore the previous invocation and return ITS arguments, which is what the
/// graph refresh for a retry must use.
async fn restore_run_state(
    p: &PreparedProject,
    w_id: &str,
    inv: &Invocation,
) -> error::Result<HashMap<String, Box<RawValue>>> {
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
    // The pointer is resolved ONCE and everything is read out of the generation
    // it names. Generations are immutable, so the arguments, the manifest and
    // the results necessarily describe the same invocation; resolving the
    // directory again per file could pair one run's arguments with another's
    // results.
    let no_state = || {
        Error::BadRequest(
            "no previous dbt run to retry from on this worker. `dbt retry` resumes from the \
             `run_results.json` the failed run left behind, which lives in that worker's local \
             cache — so on a multi-worker group a retry only finds it if it lands on the same \
             worker. Give the script a dedicated `# tag` to pin it, or run it normally to \
             rebuild"
                .to_string(),
        )
    };
    let generation = tokio::fs::read_to_string(dir.join(CURRENT_GENERATION))
        .await
        .map_err(|_| no_state())?;
    let snapshot = dir.join(generation.trim());
    if !snapshot.join("run_results.json").exists() {
        return Err(no_state());
    }
    let saved: SavedRunState = tokio::fs::read_to_string(snapshot.join("state.json"))
        .await
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    if saved.identity != format!("{}|{:x}", p.run_identity(), inv.env_digest()) {
        return Err(Error::BadRequest(
            "the last dbt run on this worker was a different project, commit, warehouse or \
             engine, so its failures do not describe this one; run the script normally instead"
                .to_string(),
        ));
    }
    let target = p.project_dir.join("target");
    tokio::fs::create_dir_all(&target).await.ok();
    for f in ["run_results.json", "manifest.json"] {
        tokio::fs::copy(snapshot.join(f), target.join(f)).await.ok();
    }
    Ok(saved
        .args
        .into_iter()
        .filter_map(|(k, v)| Some((k, RawValue::from_string(v).ok()?)))
        .collect())
}

/// Append `--vars` if the descriptor (or the run) declares any.
fn add_vars(cmd: &mut Command, descriptor: &DbtDescriptor, inv: &Invocation) -> error::Result<()> {
    let vars = resolved_vars(descriptor, &inv.args, inv.strict)?;
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
        out.insert(
            k.clone(),
            interpolate_value(v, args, &format!("vars.{k}"), strict)?,
        );
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

/// Substitute `{{ arg }}` in every string leaf, leaving numbers, booleans and
/// structure exactly as the descriptor spelled them.
fn interpolate_value(
    v: &serde_json::Value,
    args: &HashMap<String, Box<RawValue>>,
    field: &str,
    strict: bool,
) -> error::Result<serde_json::Value> {
    Ok(match v {
        serde_json::Value::String(s) => {
            match crate::common::interpolate_template(s, Some(args), field) {
                // A var whose ENTIRE value is one placeholder takes the
                // argument's own type: `strict: "{{ strict }}"` given `false`
                // must reach dbt as a boolean, since the string "false" is
                // truthy in Jinja — the same trap literal vars were fixed for.
                // A placeholder embedded in surrounding text stays a string,
                // which is what interpolation into text means.
                Ok(v) => match sole_placeholder(s).and_then(|name| args.get(name)) {
                    Some(raw) => {
                        serde_json::from_str(raw.get()).unwrap_or(serde_json::Value::String(v))
                    }
                    None => serde_json::Value::String(v),
                },
                Err(e) if strict => return Err(e),
                Err(_) => serde_json::Value::String(String::new()),
            }
        }
        serde_json::Value::Array(a) => serde_json::Value::Array(
            a.iter()
                .map(|x| interpolate_value(x, args, field, strict))
                .collect::<error::Result<_>>()?,
        ),
        serde_json::Value::Object(o) => serde_json::Value::Object(
            o.iter()
                .map(|(k, x)| Ok((k.clone(), interpolate_value(x, args, field, strict)?)))
                .collect::<error::Result<_>>()?,
        ),
        other => other.clone(),
    })
}

/// The argument name when a value is exactly one `{{ placeholder }}` and
/// nothing else.
fn sole_placeholder(s: &str) -> Option<&str> {
    let inner = s.trim().strip_prefix("{{")?.strip_suffix("}}")?.trim();
    (!inner.is_empty()
        && !inner.contains("{{")
        && inner.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
    .then_some(inner)
}

fn arg_str(args: &HashMap<String, Box<RawValue>>, k: &str) -> Option<String> {
    serde_json::from_str::<String>(args.get(k)?.get())
        .ok()
        .filter(|s| !s.is_empty())
}

fn arg_bool(args: &HashMap<String, Box<RawValue>>, k: &str) -> Option<bool> {
    serde_json::from_str::<bool>(args.get(k)?.get()).ok()
}

/// The selectors a given invocation runs with: the descriptor's, unless the
/// run overrode them. Shared by the build and by the resolver that decides
/// which nodes the run claims, which must agree — a resolver reading the
/// descriptor while dbt builds an override filters the graph by a set the run
/// never built.
///
/// Selectors are dbt's grammar and are passed verbatim — reimplementing it is a
/// standing source of divergence (docs/dbt-runtime.md).
fn add_selection(cmd: &mut Command, descriptor: &DbtDescriptor, inv: &Invocation) {
    for s in arg_list(&inv.args, "select").unwrap_or_else(|| descriptor.select.clone()) {
        cmd.args(["--select", &s]);
    }
    for s in arg_list(&inv.args, "exclude").unwrap_or_else(|| descriptor.exclude.clone()) {
        cmd.args(["--exclude", &s]);
    }
    if let Some(sel) = descriptor.selector.as_deref() {
        cmd.args(["--selector", sel]);
    }
}

/// Whether an invocation selects a subset at all. `[]` from a run clears the
/// descriptor's selector, which puts the run back to the whole project.
fn has_selection(descriptor: &DbtDescriptor, inv: &Invocation) -> bool {
    !arg_list(&inv.args, "select")
        .unwrap_or_else(|| descriptor.select.clone())
        .is_empty()
        || !arg_list(&inv.args, "exclude")
            .unwrap_or_else(|| descriptor.exclude.clone())
            .is_empty()
        || descriptor.selector.is_some()
}

/// An explicitly supplied list, including an empty one — passing `[]` is how a
/// run clears a selector the descriptor sets, so it must not read as "absent"
/// and fall back to the descriptor.
fn arg_list(args: &HashMap<String, Box<RawValue>>, k: &str) -> Option<Vec<String>> {
    serde_json::from_str::<Vec<String>>(args.get(k)?.get()).ok()
}

pub(crate) fn digest(s: &str) -> String {
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
        let ev = parse_node_event(started, "f/prod/wh", Some("wh")).unwrap();
        assert_eq!(ev.status, MaterializationStatus::Running);
        // Same canonicalization as the manifest ingest, or a run would record
        // progress against a key no graph node has.
        assert_eq!(ev.asset_path, "f/prod/wh/analytics/customers");

        let failed = r#"{"data":{"node_info":{"node_status":"error",
            "node_relation":{"alias":"c","schema":"a","relation_name":"\"w\".\"a\".\"c\""}}},
            "info":{"name":"LogModelResult","msg":"boom"}}"#;
        let ev = parse_node_event(failed, "f/prod/wh", Some("wh")).unwrap();
        assert_eq!(ev.status, MaterializationStatus::Failed);
        assert_eq!(ev.error.as_deref(), Some("boom"));
    }

    // A run clears a descriptor selector by passing `[]`. Reading that as
    // "absent" would fall back to the descriptor and build a different model
    // set than the run asked for.
    #[test]
    fn an_empty_override_clears_the_descriptor_selection() {
        let descriptor =
            DbtDescriptor { select: vec!["tag:nightly".to_string()], ..Default::default() };
        let cleared = Invocation {
            args: [(
                "select".to_string(),
                serde_json::value::RawValue::from_string("[]".to_string()).unwrap(),
            )]
            .into_iter()
            .collect(),
            ..Default::default()
        };
        assert!(has_selection(&descriptor, &Invocation::default()));
        assert!(!has_selection(&descriptor, &cleared));
    }

    // A retry runs in a new job directory, and a profile with a private CA
    // names that directory in `sslrootcert`. Hashing the rendered text as-is
    // would make every such retry look like a different warehouse and reject
    // its own predecessor's state.
    #[test]
    fn profile_identity_ignores_the_job_dir_but_not_the_connection() {
        let yaml = |dir: &str, host: &str| {
            format!("host: \"{host}\"\nsslrootcert: \"{dir}/server-ca.pem\"\n")
        };
        let first = profile_identity_digest(
            &yaml("/tmp/windmill/w/job-1/profiles", "wh.internal"),
            Path::new("/tmp/windmill/w/job-1/profiles"),
            Some("PEM"),
        );
        let retry = profile_identity_digest(
            &yaml("/tmp/windmill/w/job-2/profiles", "wh.internal"),
            Path::new("/tmp/windmill/w/job-2/profiles"),
            Some("PEM"),
        );
        assert_eq!(first, retry);

        let repointed = profile_identity_digest(
            &yaml("/tmp/windmill/w/job-2/profiles", "other.internal"),
            Path::new("/tmp/windmill/w/job-2/profiles"),
            Some("PEM"),
        );
        let recerted = profile_identity_digest(
            &yaml("/tmp/windmill/w/job-2/profiles", "wh.internal"),
            Path::new("/tmp/windmill/w/job-2/profiles"),
            Some("OTHER PEM"),
        );
        assert_ne!(first, repointed);
        assert_ne!(first, recerted);
    }

    // THREE sites derive a `table://` key: the manifest ingest (which creates
    // the graph node), the live events, and the end-of-run settlement. They must
    // agree exactly — a site that derives it differently records progress
    // against a path no node has, the run still succeeds, and the graph simply
    // never moves. Nothing else catches that.
    #[test]
    fn all_three_key_derivations_agree() {
        use windmill_common::dbt_manifest::{ingest_manifest, Manifest};
        let manifest: Manifest = serde_json::from_str(
            r#"{"nodes":{"model.p.customers":{
                 "resource_type":"model","name":"customers","alias":"Customers",
                 "schema":"Analytics","database":"Archive",
                 "relation_name":"\"Archive\".\"Analytics\".\"Customers\""}}}"#,
        )
        .unwrap();
        let ingested = ingest_manifest(&manifest, "f/prod/wh", Some("wh"), None);
        let from_manifest = ingested.nodes[0].asset_path.clone().unwrap();

        let relation = "\"Archive\".\"Analytics\".\"Customers\"";
        let from_results = asset_path_of_relation(Some(relation), "f/prod/wh", Some("wh")).unwrap();
        let live = r#"{"data":{"node_info":{"node_status":"success",
            "node_relation":{"alias":"Customers","schema":"Analytics","database":"Archive",
            "relation_name":"\"Archive\".\"Analytics\".\"Customers\""}}},
            "info":{"name":"LogModelResult","msg":"ok"}}"#;
        let from_events = parse_node_event(live, "f/prod/wh", Some("wh"))
            .unwrap()
            .asset_path;

        // The model overrode its database, so all three must qualify.
        assert_eq!(from_manifest, "f/prod/wh/archive.analytics/customers");
        assert_eq!(from_results, from_manifest);
        assert_eq!(from_events, from_manifest);

        // And in the target's own database, all three drop it.
        let plain = "\"wh\".\"Analytics\".\"Customers\"";
        assert_eq!(
            asset_path_of_relation(Some(plain), "f/prod/wh", Some("wh")).unwrap(),
            "f/prod/wh/analytics/customers"
        );
        // A test node has no relation of its own.
        assert_eq!(asset_path_of_relation(None, "f/prod/wh", Some("wh")), None);

        // A period INSIDE a quoted identifier is part of the name. Splitting on
        // every period yields four parts and discards the relation, so the model
        // records no status at all — invisible except as a graph that never
        // moves.
        assert_eq!(
            asset_path_of_relation(
                Some("\"wh\".\"analytics.v2\".\"orders\""),
                "f/prod/wh",
                Some("wh")
            ),
            Some("f/prod/wh/analytics.v2/orders".to_string())
        );
    }

    // `dbt retry` restores the previous run's target/ from this directory. Two
    // dbt scripts in one workspace must not share it, or a retry resumes
    // another project's run_results.json against this project's checkout — and
    // an empty script_path is exactly how that happened.
    // dbt vars are typed, and Jinja treats the string "false" as truthy. A var
    // that IS a placeholder must therefore carry the argument's own type
    // through, while one embedded in text stays the string it interpolates to.
    #[test]
    fn placeholder_vars_keep_the_arguments_type() {
        use windmill_parser_yaml::parse_dbt_descriptor;
        let d = parse_dbt_descriptor(
            "repo: r\nvars:\n  strict: \"{{ strict }}\"\n  n: \"{{ n }}\"\n  label: \"run-{{ name }}\"\n",
        )
        .unwrap();
        let args: HashMap<String, Box<RawValue>> =
            [("strict", "false"), ("n", "7"), ("name", "\"nightly\"")]
                .into_iter()
                .map(|(k, v)| (k.to_string(), RawValue::from_string(v.to_string()).unwrap()))
                .collect();
        let vars = resolved_vars(&d, &args, true).unwrap();
        assert_eq!(vars["strict"], serde_json::json!(false));
        assert_eq!(vars["n"], serde_json::json!(7));
        assert_eq!(vars["label"], serde_json::json!("run-nightly"));
    }

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
        assert!(parse_node_event(t, "f/prod/wh", Some("wh")).is_none());
        assert!(parse_node_event("Running with dbt=1.12.0", "f/prod/wh", Some("wh")).is_none());
        // `skipped` says nothing about the relation's state.
        let s = r#"{"data":{"node_info":{"node_status":"skipped",
            "node_relation":{"alias":"c","schema":"a","relation_name":"\"w\".\"a\".\"c\""}}},
            "info":{"name":"LogModelResult","msg":"skip"}}"#;
        assert!(parse_node_event(s, "f/prod/wh", Some("wh")).is_none());
    }
}
