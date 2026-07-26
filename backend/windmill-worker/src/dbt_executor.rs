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
use windmill_parser_yaml::{parse_dbt_descriptor, DbtDescriptor, DbtTestBehavior, DBT_COMMANDS};
use windmill_queue::{append_logs, CanceledBy, MiniPulledJob};

use crate::common::{
    render_nsjail_rlimit_as, resolve_nsjail_timeout, resolve_nsjail_tmp_mount_block,
};
use crate::common::{start_child_process, OccupancyMetrics};
use crate::dbt_engine::{provision_engine, ProvisionedEngine, DBT_CACHE_DIR};
use crate::dbt_profiles::{ensure_adapter_licensed, render_profile, DbtAdapter};
use crate::handle_child::{
    get_mem_peak, handle_child, run_future_with_polling_update_job_poller, JobCtx, JobDeadline,
};
use crate::worker::write_module_files;
use crate::{
    is_sandboxing_enabled, GIT_PATH, NSJAIL_DBT_RLIMIT_AS_MB, NSJAIL_PATH, PATH_ENV, PROXY_ENVS,
    TZ_ENV,
};

/// The profile name Windmill renders into `profiles.yml`. dbt takes the profile
/// to use from `dbt_project.yml`, so the rendered file must answer to whatever
/// name the project declares — resolved from the project file, with this as the
/// fallback for the (invalid) case where it declares none.
const FALLBACK_PROFILE_NAME: &str = "windmill";

/// Written to the script's lockfile at deploy.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct DbtDependencyLocks {
    /// The `<schema>/<database>` the profile resolved to at deploy. The
    /// resource is re-read on every run, so a schema or catalog changed on it
    /// afterwards moves every relation the project builds — and the stored
    /// graph, which still names the old ones, has to be re-ingested.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_relation_root: Option<String>,
    /// dbt-core 1.x only: its adapter is a separate package versioning
    /// independently of core, so pinning core alone still lets a rebuilt cache
    /// resolve different runtime behavior.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adapter_version: Option<String>,
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
    modules: Option<&HashMap<String, windmill_common::scripts::ScriptModule>>,
) -> error::Result<Box<RawValue>> {
    let descriptor = parse_dbt_descriptor(inner_content)?;
    let locks: Option<DbtDependencyLocks> =
        requirements_o.and_then(|s| serde_json::from_str(s).ok());

    let args = job.args.as_ref().map(|a| a.0.clone()).unwrap_or_default();
    let inv = Invocation { args: args.clone(), envs: envs.clone(), strict: true };
    // One wall clock for the whole job. A dbt job is a sequence of
    // subprocesses — provision, deps, parse, ls, build, then the
    // `after_all` tests — and each would otherwise resolve the job's full
    // timeout for itself.
    let deadline = JobDeadline::start(conn, &job.workspace_id, job.id, job.timeout).await;
    let prepared = prepare_project(
        &descriptor,
        inner_content,
        locks.as_ref(),
        job_dir,
        &job.id,
        &job.workspace_id,
        job.runnable_path.as_deref().unwrap_or_default(),
        worker_name,
        conn,
        client,
        mem_peak,
        canceled_by,
        occupancy_metrics,
        deadline,
        &envs,
        modules,
    )
    .await?;

    // Before any invocation: a per-run graph has to be re-ingested for the run's
    // writes to dispatch correctly, and an agent worker reaches the DB only
    // through the API. Refusing after `dbt build` would leave the warehouse
    // written and the job failed, and a retry would repeat the write.
    if prepared.graph_is_per_run
        && prepared.resource_path.is_some()
        && !matches!(conn, Connection::Sql(_))
    {
        return Err(Error::BadRequest(
            "this dbt script resolves its models per run, so its asset graph must be re-ingested \
             after every run — which an agent worker cannot do. Use literal `vars` and no \
             `$var:` env, or run it on a worker with a database connection."
                .to_string(),
        ));
    }

    let command = match arg_str(&args, "dbt_command")? {
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
        None => windmill_parser_yaml::default_dbt_command(&descriptor).to_string(),
    };
    // `dbt retry` resumes from the previous run's `run_results.json`, which is
    // what makes one-job-per-invocation defensible: a partial failure does not
    // force a full rebuild. Each attempt gets a fresh job dir, so that state is
    // restored — along with the ARGUMENTS it ran with, since dbt reuses that
    // invocation's selection and vars and the graph refresh, the build and the
    // test phase must all agree with it.
    let mut ctx_for_restore =
        JobCtx { mem_peak, canceled_by, occupancy_metrics, worker_name, deadline };
    let inv = if command == "retry" {
        let restored = restore_run_state(
            &prepared,
            &descriptor,
            &job.workspace_id,
            &inv,
            &mut ctx_for_restore,
            &job.id,
            conn,
        )
        .await?;
        Invocation { args: restored, ..inv }
    } else {
        inv
    };

    // A per-run graph is ingested BEFORE the build, from a `dbt parse` with this
    // run's vars: asset dispatch fans out from the stored rows the moment the
    // job completes. Concurrent runs of one dynamic script still race
    // on the path-keyed rows; that needs a per-job dispatch snapshot
    // (docs/dbt-runtime.md).
    if prepared.graph_is_per_run {
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

    // Automatic node-level retry, inside this job. A `dbt retry` rebuilds only
    // the failed and skipped nodes, so a transient warehouse error costs those
    // rather than the project — and doing it here means the previous attempt's
    // `run_results.json` is still in the job directory, with no state to
    // persist and no worker to land back on.
    if let Some(policy) = descriptor.retry_failed_nodes.filter(|_| run.is_err()) {
        for attempt in 1..=policy.attempts() {
            if !current_results_are_retryable(&prepared).await {
                break;
            }
            if policy.delay_seconds > 0 {
                tokio::time::sleep(std::time::Duration::from_secs(policy.delay_seconds)).await;
            }
            append_logs(
                &job.id,
                &job.workspace_id,
                format!(
                    "\nRetrying the nodes that failed (attempt {attempt} of {})\n",
                    policy.attempts()
                ),
                conn,
            )
            .await;
            run = run_dbt(
                &prepared,
                "retry",
                &descriptor,
                &inv,
                job,
                conn,
                mem_peak,
                canceled_by,
                occupancy_metrics,
                worker_name,
                // `dbt retry` reuses the previous invocation's selection; adding
                // one would narrow what it resumes.
                false,
                deadline,
            )
            .await;
            // A retry's `run_results.json` describes only the nodes it redid, so
            // it OVERLAYS the previous attempt's rather than replacing it. The
            // job's result has to be every node this job touched.
            merge_results(&mut results, read_run_results(&prepared.project_dir).await);
            if run.is_ok() {
                break;
            }
        }
    }
    // `retry` counts as the model phase too: a run that failed midway and was
    // retried to success would otherwise return green having never tested.
    if run.is_ok()
        && matches!(descriptor.test_behavior, DbtTestBehavior::AfterAll)
        && matches!(command.as_str(), "build" | "retry")
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

    save_run_state(&prepared, &job.workspace_id, &job.id, &inv, conn)
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

/// Deploy-time lock: materialise the script's project and parse it, so its
/// models land in the asset graph before it has ever run.
///
/// This is the one place where dbt does not fit the shape every other language
/// uses. `parse_assets_for_lang` is a pure function of the script content, and
/// dbt's assets are not derivable from the descriptor — they need the project
/// on disk and a dbt invocation. So the dependency job, which already runs on a
/// worker with the engine available, does the parse and writes the `asset` rows
/// itself; `parse_assets_for_lang` returns `None` for dbt and leaves them
/// alone. That also makes redeploy the graph-refresh mechanism, with no
/// separate concept (docs/dbt-runtime.md, decision 12).
#[allow(clippy::too_many_arguments)]
pub async fn dbt_dep(
    content: &str,
    // The project this dependency job is deploying: a dependency job has no
    // generic module-writing step, so the executor materialises them.
    modules: Option<&HashMap<String, windmill_common::scripts::ScriptModule>>,
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
    // The script's own `envs`, exactly as a run gets them. A project can drive a
    // model's schema, alias or `enabled` from `env_var()`, so parsing with an
    // empty environment would record one relation at deploy and build another
    // at run time — with no per-run refresh to correct it.
    let envs = script_envs(db, job_id, w_id).await;
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
        job_dir,
        job_id,
        w_id,
        script_path,
        worker_name,
        &conn,
        &client,
        mem_peak,
        canceled_by,
        occupancy_metrics,
        deadline,
        &envs,
        modules,
    )
    .await?;

    // A deploy has no job arguments, so it tolerates the `{{ }}` placeholders
    // only a run can fill (see `Invocation::strict`). Its environment is the
    // script's, matching what the run will parse with.
    let inv = Invocation { envs: envs.clone(), strict: false, ..Default::default() };
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
        &tokio::fs::read_to_string(
            prepared
                .project_dir
                .join(ARTIFACTS_DIR)
                .join("manifest.json"),
        )
        .await
        .unwrap_or_default(),
    );

    // Two deploys of one path can run concurrently — nothing serializes
    // dependency jobs — and the graph is keyed by path, not by version, so the
    // publication is claimed against this job's own version. It still returns
    // its lock, which belongs to that version.
    let publisher = match deploying_script_hash(db, job_id).await {
        Some(hash) => GraphPublisher::Version(hash),
        None => GraphPublisher::Unversioned,
    };
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
            publisher,
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
        let published = claim_graph_publication(&mut tx, w_id, script_path, publisher).await?;
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
        manifest_digest,
        profile_relation_root: Some(prepared.relation_root()),
        engine: prepared.engine.engine.as_str().to_string(),
        engine_version: prepared.engine.version.clone(),
        adapter_version: prepared.engine.adapter_version.clone(),
    })
    .map_err(|e| Error::internal_err(format!("serializing the dbt lockfile: {e}")))
}

/// The `envs` of the script this dependency job is deploying, in the same shape
/// a run receives them. Empty when the version cannot be resolved — a raw
/// dependency job has no script row.
async fn script_envs(
    db: &sqlx::Pool<sqlx::Postgres>,
    job_id: &Uuid,
    w_id: &str,
) -> HashMap<String, String> {
    let Some(hash) = deploying_script_hash(db, job_id).await else {
        return HashMap::new();
    };
    let envs = sqlx::query_scalar!(
        "SELECT envs FROM script WHERE workspace_id = $1 AND hash = $2",
        w_id,
        hash
    )
    .fetch_optional(db)
    .await
    .ok()
    .flatten()
    .flatten();
    crate::worker::build_envs(envs.as_ref()).unwrap_or_default()
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
    /// A var that can steer what the project produces makes the deploy-time
    /// graph a guess, so each run re-ingests its own manifest.
    pub graph_is_per_run: bool,
    /// Digest of the project's own files: the identity of the code that runs.
    /// It keys the package cache (a `local:` dependency's content appears in no
    /// manifest) and gates retry state, so a project edited between attempts
    /// cannot resume the old one.
    pub project_digest: String,
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
    /// The descriptor's `env`, resolved, in a stable order. Feeds run identity;
    /// `env` itself is not usable there because it carries per-job values.
    pub descriptor_env: std::collections::BTreeMap<String, String>,
    /// The invocation's own environment (the script's `envs`), in a stable
    /// order. Every phase gets it, `dbt deps` included: `packages.yml` can
    /// resolve a private package URL through `env_var()`, and a phase that saw
    /// a different environment from the one the cache key was built on would
    /// populate that key with the wrong tree.
    pub invocation_env: Vec<(String, String)>,
    /// Digest of the above. Keys the package cache alongside the descriptor's
    /// environment; digested because the values are resolved secrets.
    pub invocation_env_digest: u64,
    /// Written nsjail profile for this job, when the worker sandboxes jobs.
    /// `None` means the phases run unsandboxed, exactly as before.
    pub sandbox_config: Option<SandboxProfile>,
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
    /// retry-state check: same project files, same warehouse and target, same
    /// engine. Identity only, never credentials — the profile is digested, and
    /// the digest is one-way.
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
            "{}|{}|{}|{:x}|{}|{}",
            self.project_digest,
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
    job_dir: &str,
    job_id: &Uuid,
    w_id: &str,
    // Keys the per-script retry-state cache. Passed in rather than patched onto
    // the result afterwards: an empty value silently shares one state directory
    // across every dbt script in the workspace, so a retry resumes another
    // project's run_results.json.
    script_path: &str,
    worker_name: &str,
    conn: &Connection,
    client: &AuthedClient,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    occupancy_metrics: &mut OccupancyMetrics,
    deadline: JobDeadline,
    // The invocation's own environment (script-level `envs`). Needed here
    // because under a sandbox it must travel in the jail profile rather than
    // on the process that execs nsjail.
    invocation_env: &HashMap<String, String>,
    // The script's files: the dbt project itself.
    modules: Option<&HashMap<String, windmill_common::scripts::ScriptModule>>,
) -> error::Result<PreparedProject> {
    // The project IS this script's files. Nothing is fetched: the bundle is the
    // project. A dependency job has no generic module-writing step, so it does
    // the writing here; a run rewrites the same bytes, which costs nothing next
    // to the dbt invocations that follow.
    if let Some(modules) = modules {
        write_module_files(job_dir, modules, None).await?;
    }
    let project_dir = PathBuf::from(job_dir);
    if !project_dir.join("dbt_project.yml").exists() {
        return Err(Error::BadRequest(
            "this dbt script carries no project: `dbt_project.yml` was not found. Copy a dbt \
             project into its `<script>__dbt/` folder and push it (`wmill sync push`)"
                .to_string(),
        ));
    }
    // Vars can drive `enabled`, alias, schema, database and materialization, so
    // a placeholder var or a `$var:` env value (re-resolved every run) makes the
    // deploy-time graph a guess, and each run re-ingests its own manifest.
    //
    // A per-run `vars` override is deliberately NOT here: gating on it would
    // leave the override's graph in place for the next default run, which then
    // builds the descriptor's relations and dispatches from the override's.
    let has_placeholder = |v: &str| v.contains("{{");
    let graph_is_per_run = descriptor
        .vars
        .values()
        .flat_map(windmill_parser_yaml::dbt::string_leaves)
        .any(has_placeholder)
        || descriptor.env.values().any(|v| v.starts_with("$var:"));

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
    reject_reserved_env(
        resolved_env.iter().map(|(k, _)| k),
        "the descriptor's `env`",
    )?;
    reject_reserved_env(invocation_env.keys(), "the script's environment variables")?;
    let descriptor_env: std::collections::BTreeMap<String, String> =
        resolved_env.iter().cloned().collect();
    let mut env = resolved_env;
    // Both engines write their profile-independent state under the project;
    // pinning it inside the job dir keeps a job from touching a shared $HOME.
    env.push(("HOME".to_string(), job_dir.to_string()));

    // The engines are provisioned per (version, adapter) under one cache root,
    // and mounting that root rather than the resolved engine directory keeps
    // the profile identical for every job on this worker.
    let sandbox_config: Option<SandboxProfile> = if is_sandboxing_enabled() {
        let nsjail_timeout =
            resolve_nsjail_timeout(conn, w_id, *job_id, deadline.remaining_secs()).await;
        // A SIBLING of the job directory: that directory is mounted read-write
        // into the jail, and every phase re-reads this file.
        let sandbox_dir = PathBuf::from(format!("{job_dir}.dbt-sandbox"));
        tokio::fs::create_dir_all(&sandbox_dir).await.map_err(|e| {
            Error::internal_err(format!("could not create the dbt sandbox dir: {e}"))
        })?;
        write_file(
            &sandbox_dir.to_string_lossy(),
            SANDBOX_PROFILE_NAME,
            &NSJAIL_CONFIG_RUN_DBT_CONTENT
                .replace(
                    "{RLIMIT_AS}",
                    &render_nsjail_rlimit_as(NSJAIL_DBT_RLIMIT_AS_MB.as_deref(), 4096),
                )
                .replace("{JOB_DIR}", &escape_textproto(job_dir))
                .replace(
                    "{PROJECT_DIR}",
                    &escape_textproto(&project_dir.to_string_lossy()),
                )
                // The engine's own directory, NOT the cache root: its siblings
                // are other workspaces' package trees, kept apart by cache key
                // rather than by permissions.
                .replace(
                    "{ENGINE_DIR}",
                    &escape_textproto(&engine.root.to_string_lossy()),
                )
                .replace(
                    "{PY_INSTALL_DIR}",
                    &escape_textproto(&crate::PY_INSTALL_DIR),
                )
                .replace("{CLONE_NEWUSER}", &(!*crate::DISABLE_NUSER).to_string())
                // Both environments the child needs: the descriptor's and the
                // invocation's. Neither may sit on the launcher.
                .replace(
                    "{ENVARS}",
                    &jail_envars(
                        env.iter()
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .chain(invocation_env.iter().map(|(k, v)| (k.clone(), v.clone())))
                            .collect::<Vec<_>>()
                            .as_slice(),
                    ),
                )
                .replace("{SHARED_MOUNT}", "")
                .replace(
                    "{TMP_MOUNT_BLOCK}",
                    &resolve_nsjail_tmp_mount_block(job_dir).await,
                )
                .replace("{TIMEOUT}", &nsjail_timeout),
        )
        // Fail the job rather than fall back: a `None` here means every
        // project-controlled phase would run unsandboxed on a worker configured
        // to isolate them, and a project can make this write fail on purpose by
        // filling the job filesystem first.
        .map_err(|e| {
            Error::internal_err(format!("could not write the dbt sandbox profile: {e}"))
        })?;
        Some(SandboxProfile(sandbox_dir.join(SANDBOX_PROFILE_NAME)))
    } else {
        None
    };

    let project_digest = project_digest(modules);

    // Sorted so the digest depends on the values rather than on map ordering.
    let sorted_invocation_env = {
        let mut v: Vec<(String, String)> = invocation_env
            .iter()
            .map(|(k, val)| (k.clone(), val.clone()))
            .collect();
        v.sort();
        v
    };
    let mut prepared = PreparedProject {
        project_digest,
        invocation_env: sorted_invocation_env.clone(),
        invocation_env_digest: {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            for (k, v) in &sorted_invocation_env {
                k.hash(&mut h);
                v.hash(&mut h);
            }
            h.finish()
        },
        sandbox_config,
        profile_digest,
        project_dir,
        profiles_dir,
        engine,
        graph_is_per_run,
        resource_path,
        target: descriptor.profile.target.clone(),
        descriptor_content: descriptor_content.to_string(),
        descriptor_env,

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
/// Identity of the project's own files: what the run reproduces, and what a
/// retry must match to be allowed to resume.
///
/// Sorted, so it depends on the files rather than on map ordering: a digest
/// that moved between two runs of one project would evict the package cache
/// every time and reject every retry. Every caller must pass the bundle; an
/// empty one collapses every project in the workspace onto one digest, which
/// silently lets a retry resume a DIFFERENT project's `run_results.json`.
fn project_digest(
    modules: Option<&HashMap<String, windmill_common::scripts::ScriptModule>>,
) -> String {
    let mut names: Vec<&String> = modules.map(|m| m.keys().collect()).unwrap_or_default();
    names.sort();
    let mut h = Sha256::new();
    for name in names {
        h.update(name.as_bytes());
        h.update([0u8]);
        if let Some(m) = modules.and_then(|m| m.get(name)) {
            h.update(m.content.as_bytes());
        }
        h.update([0u8]);
    }
    format!("{:x}", h.finalize())[..32].to_string()
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
    // that determines the resolved tree: the declared packages, a
    // `package-lock.yml` pinning versions two projects with identical ranges
    // would resolve differently, and any `local:` dependency's content, which is
    // in no manifest at all — the project digest stands in for it.
    //
    // Workspace identity is in the key: `dbt deps` can fetch private git
    // packages, so a shared tree would let one workspace execute another's
    // private package code without ever authenticating. Both environments are
    // keyed too, since `packages.yml` can read `env_var()`.
    let mut key = format!(
        "{w_id}\n{}\n{:x}\n{:x}\n",
        p.project_digest,
        p.env_digest(),
        p.invocation_env_digest,
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
    // Where `dbt deps` actually writes. `packages-install-path` is a project
    // setting, and assuming the default means a project that moved it gets no
    // cache at all: the publish finds nothing to copy and every job resolves
    // its dependencies over the network again.
    let target = p
        .project_dir
        .join(packages_install_path(&p.project_dir).await);
    if cached.exists() {
        copy_dir_watched(&cached, &target, "restoring cached dbt_packages", ctx, job_id, w_id, conn)
            .await?;
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
        publish_to_cache(&target, &cached, ctx, job_id, w_id, conn).await;
    }
    Ok(())
}

/// Copy `from` into a sibling of `cached`, then move it into place.
///
/// The rename is the point. The copy creates its destination and then fills it,
/// so a concurrent job on the same host — worker processes share
/// `DBT_CACHE_DIR` — would see `cached` exist and restore a half-written
/// package tree. Worse, a copy interrupted by cancellation or disk pressure
/// would leave that tree in place for every later job, so a transient failure
/// becomes permanent. Staging keeps it under a name nothing looks up.
/// Same pattern as the engine provisioning; best-effort, since losing the race
/// only means the next job repopulates.
async fn publish_to_cache(
    from: &Path,
    cached: &Path,
    ctx: &mut JobCtx<'_>,
    job_id: &Uuid,
    w_id: &str,
    conn: &Connection,
) {
    let Some(parent) = cached.parent() else {
        return;
    };
    if tokio::fs::create_dir_all(parent).await.is_err() {
        return;
    }
    let name = cached.file_name().unwrap_or_default().to_string_lossy();
    let staging = cached.with_file_name(format!("{name}.staging-{job_id}"));
    tokio::fs::remove_dir_all(&staging).await.ok();
    if copy_dir_watched(from, &staging, "caching dbt_packages", ctx, job_id, w_id, conn)
        .await
        .is_err()
        || strip_git_remotes(&staging).await.is_err()
        || tokio::fs::rename(&staging, cached).await.is_err()
    {
        tokio::fs::remove_dir_all(&staging).await.ok();
    }
}

/// Drop the origin remotes from a `dbt_packages` tree on its way into the cache.
///
/// `dbt deps` clones `git:` packages and leaves each one's `.git` behind, with
/// the URL it was given in `.git/config` — and for token auth that URL *is* the
/// credential, since `packages.yml` renders it from `env_var()`. The cache is
/// worker-global and outlives the job, so copying the tree verbatim would leave
/// a live token readable by every later job on the host. Restores never fetch,
/// so no remote is needed.
async fn strip_git_remotes(dir: &Path) -> std::io::Result<()> {
    let mut entries = tokio::fs::read_dir(dir).await?;
    while let Some(e) = entries.next_entry().await? {
        strip_git_remote(&e.path()).await?;
    }
    Ok(())
}

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
        // A resource alongside the project's own file names the warehouse for
        // asset identity only: the connection comes from the file. It is still
        // READ here, and only here, because reading is what authorizes it —
        // otherwise a script editor could publish `table://<any resource>/...`
        // writes, and wake that warehouse's subscribers, while connecting
        // somewhere else entirely.
        if let Some(rp) = resource_path.as_deref() {
            client
                .get_resource_value_interpolated::<serde_json::Value>(
                    rp,
                    Some(job_id.to_string()),
                )
                .await
                .map_err(|e| {
                    Error::BadRequest(format!(
                        "`profile.resource` names the warehouse this project's assets are keyed \
                         on, so it must be readable even when `profile.profiles_yml` provides \
                         the connection: {e}"
                    ))
                })?;
        }
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

/// Where the project has `dbt deps` install its packages, defaulting to dbt's
/// own `dbt_packages`. Relative to the project root, and validated as such: it
/// is project-controlled, so an absolute or `..`-bearing value would make the
/// cache copy read and write outside the job directory.
async fn packages_install_path(project_dir: &Path) -> String {
    const DEFAULT: &str = "dbt_packages";
    let Ok(content) = tokio::fs::read_to_string(project_dir.join("dbt_project.yml")).await else {
        return DEFAULT.to_string();
    };
    serde_yml::from_str::<serde_yml::Value>(&content)
        .ok()
        .and_then(|v| {
            v.get("packages-install-path")
                .and_then(|p| p.as_str())
                .map(|s| s.trim().trim_start_matches("./").to_string())
        })
        .filter(|s| {
            !s.is_empty()
                && Path::new(s)
                    .components()
                    .all(|c| matches!(c, std::path::Component::Normal(_)))
        })
        .unwrap_or_else(|| DEFAULT.to_string())
}

/// The nsjail profile a sandboxed dbt phase runs under.
const NSJAIL_CONFIG_RUN_DBT_CONTENT: &str = include_str!("../nsjail/run.dbt.config.proto");

/// Build the command for a dbt phase, inside the job's sandbox when the worker
/// has one configured.
///
/// Every phase is project-controlled — `dbt deps` fetches packages the project
/// names, `parse` and `build` render project macros, and a DuckDB profile
/// reads and writes local files — so they are the project's code, not
/// Windmill's, and the same isolation every other executor applies has to
/// apply here.
/// Apply the invocation's environment to a dbt command.
///
/// Under a sandbox this must NOT touch the launcher: `dbt_command` returns the
/// process that execs nsjail, and these values come from caller-controlled
/// script metadata — `LD_PRELOAD` naming a library from the checkout would be
/// loaded by the dynamic linker as the worker, before isolation exists. The
/// jail profile carries them to the child instead (see `sandbox_config`).
pub fn dbt_command(p: &PreparedProject, args: &[&str]) -> Command {
    let mut cmd = match p.sandbox_config.as_ref().map(|c| c.path()) {
        Some(config) => {
            let mut nsjail = Command::new(NSJAIL_PATH.as_str());
            nsjail
                .arg("--config")
                .arg(config)
                .arg("--")
                .arg(&p.engine.bin);
            nsjail
        }
        None => Command::new(&p.engine.bin),
    };
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
        .env("GIT_PATH", GIT_PATH.as_str());
    // The descriptor's environment is the PROJECT's, and the invocation's is
    // the script's; both belong to the child. Under a sandbox they reach it
    // through the jail profile instead of this process, because placing them
    // here would hand them to the dynamic loader that execs nsjail itself —
    // `LD_PRELOAD` naming a library from the project would then run as the
    // worker, before any isolation exists.
    if p.sandbox_config.is_none() {
        cmd.envs(p.env.iter().map(|(k, v)| (k.as_str(), v.as_str())));
        cmd.envs(p.invocation_env.iter().map(|(k, v)| (k.as_str(), v.as_str())));
    }
    cmd.args(args)
        .arg("--profiles-dir")
        .arg(&p.profiles_dir)
        // Where `manifest.json` and `run_results.json` land. A project may set
        // `target-path` in `dbt_project.yml`, and every artifact this runtime
        // reads — the graph, the per-node results, the retry state — is found
        // by path, so the location is Windmill's to decide. As an env var
        // rather than `--target-path`, which `dbt deps` rejects outright. Set
        // last so neither environment above can displace it, belt to the
        // braces of `reject_reserved_env`.
        .env("DBT_TARGET_PATH", ARTIFACTS_DIR);
    cmd
}

/// The project's environment, as jail directives rather than launcher
/// environment. `keep_env` passes nsjail's own environment through, so these
/// are added on top of the Windmill-controlled ones the launcher carries.
fn jail_envars(env: &[(String, String)]) -> String {
    env.iter()
        .map(|(k, v)| format!("envar: \"{}={}\"", escape_textproto(k), escape_textproto(v)))
        .collect::<Vec<_>>()
        .join("\n")
}

const SANDBOX_PROFILE_NAME: &str = "dbt.nsjail.config.proto";

/// The written nsjail profile, kept OUTSIDE every path the jailed child can
/// reach and removed when the job ends.
///
/// It cannot live in the job directory: the jail mounts that read-write, and
/// each phase launches a fresh `nsjail --config` against this file — so a
/// project able to write files during `build` (a DuckDB one can, through
/// `shellfs`) could rewrite the profile and have the `after_all` test phase
/// start with mounts of its choosing. A sibling of the job directory is not
/// mounted at all, so the child never sees it.
pub struct SandboxProfile(PathBuf);

impl SandboxProfile {
    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for SandboxProfile {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(self.0.parent().unwrap_or(&self.0));
    }
}

/// Escape a value for a protobuf text-format string literal.
///
/// Every path and environment value interpolated into the jail profile is
/// caller-influenced — a repository directory is named by whoever wrote the
/// repo — and a bare `"` or newline would close the string and let the rest be
/// read as further directives, including host bind mounts.
fn escape_textproto(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                out.push_str(&format!("\\x{:02x}", c as u32))
            }
            c => out.push(c),
        }
    }
    out
}

/// Environment this runtime owns. A project setting one of these would redirect
/// where dbt writes the artifacts Windmill reads by path — the graph, the
/// per-node results, the retry state — so a run could succeed while Windmill
/// records nothing. Refused rather than silently stripped, so the descriptor's
/// author is told.
const RESERVED_ENV_KEYS: &[&str] = &["DBT_TARGET_PATH", "DBT_PROFILES_DIR", "DBT_PROJECT_DIR"];

fn reject_reserved_env<'a>(
    env: impl IntoIterator<Item = &'a String>,
    source: &str,
) -> error::Result<()> {
    for k in env {
        if RESERVED_ENV_KEYS.iter().any(|r| r.eq_ignore_ascii_case(k)) {
            return Err(Error::BadRequest(format!(
                "`{k}` is set by Windmill and cannot be overridden from {source}: it decides \
                 where dbt writes the artifacts this runtime reads"
            )));
        }
    }
    Ok(())
}

/// Fixed artifact directory, relative to the project root: `dbt_project.yml`
/// may point `target-path` anywhere, and this runtime reads every artifact by
/// path. Relative rather than absolute so it stays inside whatever sandbox the
/// project runs in.
pub const ARTIFACTS_DIR: &str = "wm_target";

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
    // The console stays human-readable and goes straight to the job log; the
    // machine-readable copy goes to a file the progress reporter tails, so
    // neither purpose degrades the other.
    let log_dir = p.project_dir.join("logs");
    cmd.arg("--log-path")
        .arg(&log_dir)
        .args(["--log-format-file", "json"])
        .args(["--log-level-file", p.engine.engine.progress_log_level()]);

    if with_selection && command != "retry" {
        add_selection(&mut cmd, descriptor, inv)?;
    }
    // The model phase of `after_all` — and every phase of `none` — builds
    // everything the selection names EXCEPT tests. `dbt run` would be the
    // obvious command but covers models only, silently skipping seeds and
    // snapshots the descriptor selected.
    if command == "build" && !matches!(descriptor.test_behavior, DbtTestBehavior::Build) {
        cmd.args([
            "--exclude-resource-type",
            "test",
            "--exclude-resource-type",
            "unit_test",
        ]);
    }
    if command != "retry" {
        add_vars(&mut cmd, descriptor, inv)?;
        if let Some(t) = descriptor.threads {
            cmd.args(["--threads", &t.to_string()]);
        }
        let full_refresh = arg_bool(&inv.args, "full_refresh")?.unwrap_or(descriptor.full_refresh);
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
    let Ok(content) =
        tokio::fs::read_to_string(project_dir.join(ARTIFACTS_DIR).join("run_results.json")).await
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
    persist_ingest(
        db,
        &job.workspace_id,
        script_path,
        &ingested,
        &DescriptorTriggers::parse(&p.descriptor_content),
        &p.relation_root(),
        job.runnable_id
            .map(|h| GraphPublisher::Version(h.0))
            .unwrap_or(GraphPublisher::Unversioned),
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
/// Who is publishing a graph, which decides whether it may.
#[derive(Clone, Copy)]
enum GraphPublisher {
    /// The script version this job belongs to, whether it is deploying that
    /// version or running it. Publishes while the version is still the newest
    /// for the path: a slow deploy — or a long run — of an older version
    /// finishing later would otherwise describe code no longer deployed.
    Version(i64),
    /// No version behind the job: a preview, or a raw dependency job whose
    /// `script_path` is chosen by a caller who needs only `jobs:run`. Never
    /// publishes, so a run-only principal cannot rewrite another script's
    /// graph.
    Unversioned,
}

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
    publisher: GraphPublisher,
) -> error::Result<bool> {
    use windmill_common::assets::{AssetUsageKind, ScriptTriggerKind};
    let mut tx = db.begin().await?;
    if !claim_graph_publication(&mut tx, w_id, script_path, publisher).await? {
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
    publisher: GraphPublisher,
) -> error::Result<bool> {
    let mine = match publisher {
        GraphPublisher::Unversioned => return Ok(false),
        GraphPublisher::Version(hash) => hash,
    };
    sqlx::query!(
        "SELECT pg_advisory_xact_lock(hashtext($1))",
        format!("dbt_graph:{w_id}:{script_path}")
    )
    .execute(&mut **tx)
    .await?;
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
    if !has_selection(descriptor, inv)? {
        return Ok(None);
    }
    let mut cmd = dbt_command(p, &["ls"]);
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
    add_selection(&mut cmd, descriptor, inv)?;
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

/// `dbt parse`, which writes the manifest without touching the
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
    add_vars(&mut cmd, descriptor, inv)?;
    run_prep_command(p, cmd, "dbt parse", ctx, job_id, w_id, conn).await
}

pub async fn read_manifest(
    project_dir: &Path,
) -> error::Result<windmill_common::dbt_manifest::Manifest> {
    let path = project_dir.join(ARTIFACTS_DIR).join("manifest.json");
    let content = tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| Error::internal_err(format!("dbt produced no manifest.json: {e}")))?;
    serde_json::from_str(&content)
        .map_err(|e| Error::internal_err(format!("could not parse manifest.json: {e}")))
}

/// `dbt retry` reads `run_results.json` from the previous invocation.
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
    // Scopes the staging directory. Keyed by project digest, two concurrent
    // runs of one script would stage into the same place and publish a mixture.
    job_id: &Uuid,
    inv: &Invocation,
    conn: &Connection,
) -> error::Result<()> {
    if p.script_path.is_empty() {
        return Ok(());
    }
    let identity = format!("{}|{:x}", p.run_identity(), inv.env_digest());
    let args: HashMap<String, String> = inv
        .args
        .iter()
        .map(|(k, v)| (k.clone(), v.get().to_string()))
        .collect();
    // The durable copy, so a retry works from any worker of the group. Only
    // `run_results.json`: the manifest is a pure function of what `identity`
    // already pins, so the resuming worker re-derives it with a `dbt parse`.
    if let Connection::Sql(db) = conn {
        if let Ok(results) =
            tokio::fs::read_to_string(p.project_dir.join(ARTIFACTS_DIR).join("run_results.json"))
                .await
        {
            let _ = sqlx::query!(
                "INSERT INTO dbt_run_state (workspace_id, script_path, identity, args, run_results, job_id, updated_at)
                 VALUES ($1, $2, $3, $4, $5, $6, now())
                 ON CONFLICT (workspace_id, script_path) DO UPDATE SET
                   identity = EXCLUDED.identity, args = EXCLUDED.args,
                   run_results = EXCLUDED.run_results, job_id = EXCLUDED.job_id,
                   updated_at = now()",
                w_id,
                &p.script_path,
                identity,
                serde_json::to_value(&args).unwrap_or_default(),
                results,
                job_id,
            )
            .execute(db)
            .await;
        }
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
        if tokio::fs::copy(p.project_dir.join(ARTIFACTS_DIR).join(f), staging.join(f))
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
    // Includes the invocation's own environment: script-level variables are
    // applied to parse, ls and the build just as the descriptor's are, so a
    // change to one after a failure makes the saved results describe relations
    // a retry would not produce.
    let state = SavedRunState { identity, args };
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
    /// Project digest, warehouse and engine — everything that decides which
    /// relations the restored `run_results.json` describes.
    identity: String,
    /// The invocation's job arguments, as raw JSON per key. `dbt retry` reuses
    /// the original selection and vars, so refreshing the graph for it needs
    /// these rather than the retry request's.
    args: HashMap<String, String>,
}

/// The durable half of the restore: the worker-local generation is gone (or this
/// is another worker of the group), so the state comes from the database.
///
/// `run_results.json` is all that is stored. `dbt retry` also reads
/// `manifest.json`, which is far larger and grows with the project, so it is
/// re-derived here with a `dbt parse` — sound because `identity` pins the
/// project digest, the warehouse and the engine, which is everything the
/// manifest is a function of.
#[allow(clippy::too_many_arguments)]
async fn restore_from_db(
    p: &PreparedProject,
    descriptor: &DbtDescriptor,
    w_id: &str,
    inv: &Invocation,
    ctx: &mut JobCtx<'_>,
    job_id: &Uuid,
    conn: &Connection,
    no_state: Error,
) -> error::Result<HashMap<String, Box<RawValue>>> {
    let Connection::Sql(db) = conn else {
        // An agent worker reaches the database only through the API, and this
        // state is not exposed there.
        return Err(no_state);
    };
    let Some(row) = sqlx::query!(
        "SELECT identity, args, run_results FROM dbt_run_state
         WHERE workspace_id = $1 AND script_path = $2",
        w_id,
        &p.script_path
    )
    .fetch_optional(db)
    .await?
    else {
        return Err(no_state);
    };
    if row.identity != format!("{}|{:x}", p.run_identity(), inv.env_digest()) {
        return Err(different_project());
    }
    if !has_retryable_node(&row.run_results) {
        return Err(nothing_to_retry());
    }
    let target = p.project_dir.join(ARTIFACTS_DIR);
    tokio::fs::create_dir_all(&target).await.ok();
    tokio::fs::write(target.join("run_results.json"), &row.run_results)
        .await
        .map_err(|e| Error::internal_err(format!("restoring run_results.json: {e}")))?;
    // The arguments first: the parse below has to see the invocation dbt will
    // retry, not this request's.
    let args = restored_args(row.args);
    run_dbt_parse(
        p,
        descriptor,
        &Invocation { args: args.clone(), ..inv.clone() },
        ctx,
        job_id,
        w_id,
        conn,
    )
    .await?;
    Ok(args)
}

/// Job arguments as stored, each value a raw JSON string.
fn restored_args(args: serde_json::Value) -> HashMap<String, Box<RawValue>> {
    serde_json::from_value::<HashMap<String, String>>(args)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(k, v)| Some((k, RawValue::from_string(v).ok()?)))
        .collect()
}

fn different_project() -> Error {
    Error::BadRequest(
        "the last dbt run was of a different project, warehouse or engine, so its failures do \
         not describe this one; run the script normally instead"
            .to_string(),
    )
}

fn nothing_to_retry() -> Error {
    Error::BadRequest(
        "the last dbt run succeeded, so there is nothing to retry: `dbt retry` resumes the \
         previous run's failed and skipped nodes. Run the script normally to rebuild"
            .to_string(),
    )
}

/// Restore the previous invocation and return ITS arguments, which is what the
/// graph refresh for a retry must use.
async fn restore_run_state(
    p: &PreparedProject,
    descriptor: &DbtDescriptor,
    w_id: &str,
    inv: &Invocation,
    ctx: &mut JobCtx<'_>,
    job_id: &Uuid,
    conn: &Connection,
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
            "no previous dbt run to retry from. `dbt retry` resumes from the \
             `run_results.json` the failed run left behind; run the script normally to rebuild"
                .to_string(),
        )
    };
    let Ok(generation) = tokio::fs::read_to_string(dir.join(CURRENT_GENERATION)).await else {
        // Nothing on this worker: fall back to the durable copy, which is what
        // lets a retry land anywhere in the group.
        return restore_from_db(p, descriptor, w_id, inv, ctx, job_id, conn, no_state()).await;
    };
    let snapshot = dir.join(generation.trim());
    let saved_results = tokio::fs::read_to_string(snapshot.join("run_results.json"))
        .await
        .map_err(|_| no_state())?;
    // dbt builds a retry's graph from the error, fail and skipped nodes alone,
    // so retrying an all-green run selects nothing and writes nothing — and a
    // job that succeeds having written nothing still dispatches every
    // deploy-time write, waking every downstream consumer for relations no one
    // touched. Refuse instead.
    if !has_retryable_node(&saved_results) {
        return Err(nothing_to_retry());
    }
    let saved: SavedRunState = tokio::fs::read_to_string(snapshot.join("state.json"))
        .await
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    if saved.identity != format!("{}|{:x}", p.run_identity(), inv.env_digest()) {
        return Err(different_project());
    }
    let target = p.project_dir.join(ARTIFACTS_DIR);
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

/// Whether the artifacts in the job directory still name something to retry.
async fn current_results_are_retryable(p: &PreparedProject) -> bool {
    match tokio::fs::read_to_string(p.project_dir.join(ARTIFACTS_DIR).join("run_results.json"))
        .await
    {
        Ok(s) => has_retryable_node(&s),
        Err(_) => false,
    }
}

/// Overlay a retry's results onto the attempt they resumed.
///
/// dbt writes only the nodes it redid, so replacing the accumulated results
/// would drop every node that succeeded before the retry — the job would then
/// report a handful of nodes and settle materializations for no others.
fn merge_results(into: &mut Vec<DbtNodeResult>, from: Vec<DbtNodeResult>) {
    for node in from {
        match into.iter_mut().find(|n| n.unique_id == node.unique_id) {
            Some(existing) => *existing = node,
            None => into.push(node),
        }
    }
}

/// Whether a saved `run_results.json` has a WRITE that `dbt retry` would redo.
///
/// Two conditions, and both matter.
///
/// Retryable at all is dbt's own rule: `error`, `fail` and `skipped`. A
/// `partial success` counts too — dbt spells that for a node that built but
/// whose tests failed, and its retry redoes the node.
///
/// And the node has to produce a relation. A retry selecting only `test.*`
/// nodes reruns tests, materialises nothing and succeeds, but a successful dbt
/// job dispatches every deploy-time write, so it would wake every downstream
/// consumer for relations no one touched. That is the same false cascade that
/// keeps `test` out of `DBT_COMMANDS`, reached the other way: with
/// `test_behavior: after_all`, a failing test is what `run_results.json` ends
/// up describing.
fn has_retryable_node(run_results: &str) -> bool {
    serde_json::from_str::<RunResults>(run_results)
        .map(|r| {
            r.results.iter().any(|n| {
                matches!(
                    n.status.to_ascii_lowercase().as_str(),
                    "error" | "fail" | "skipped" | "partial success"
                ) && writes_a_relation(&n.unique_id)
            })
        })
        // Unreadable results are not "nothing to retry": let dbt decide rather
        // than refusing a retry the user may well need.
        .unwrap_or(true)
}

/// Whether a dbt `unique_id` names a node that materialises something.
///
/// The prefix IS the resource type — dbt builds these ids as
/// `<resource_type>.<package>.<name>`. `test`, `analysis`, `unit_test`,
/// `source` and `exposure` write nothing.
fn writes_a_relation(unique_id: &str) -> bool {
    matches!(
        unique_id.split('.').next().unwrap_or_default(),
        "model" | "seed" | "snapshot"
    )
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
    if let Some(raw) = args.get("vars").filter(|r| r.get().trim() != "null") {
        // A wrong-typed override is refused rather than ignored: argument-schema
        // validation is opt-in, so silently running the descriptor's own vars
        // could build against a different schema or alias than the caller asked
        // for — the same reason `select`/`exclude` reject theirs.
        match serde_json::from_str(raw.get()) {
            Ok(serde_json::Value::Object(m)) => out.extend(m),
            _ => {
                return Err(Error::BadRequest(
                    "`vars` must be an object mapping dbt var names to values".to_string(),
                ))
            }
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

/// A supplied scalar, refusing a wrong-typed one rather than reading it as
/// absent: argument-schema validation is opt-in, so `dbt_command: 1` would
/// otherwise run the default command and `full_refresh: "false"` would still
/// full-refresh — the caller silently getting something else than asked for.
fn arg_str(args: &HashMap<String, Box<RawValue>>, k: &str) -> error::Result<Option<String>> {
    let Some(raw) = args.get(k).filter(|r| r.get().trim() != "null") else {
        return Ok(None);
    };
    serde_json::from_str::<String>(raw.get())
        .map(|s| Some(s).filter(|s| !s.is_empty()))
        .map_err(|e| Error::BadRequest(format!("`{k}` must be a string: {e}")))
}

fn arg_bool(args: &HashMap<String, Box<RawValue>>, k: &str) -> error::Result<Option<bool>> {
    let Some(raw) = args.get(k).filter(|r| r.get().trim() != "null") else {
        return Ok(None);
    };
    serde_json::from_str::<bool>(raw.get())
        .map(Some)
        .map_err(|e| Error::BadRequest(format!("`{k}` must be a boolean: {e}")))
}

/// The selectors a given invocation runs with: the descriptor's, unless the
/// run overrode them. Shared by the build and by the resolver that decides
/// which nodes the run claims, which must agree — a resolver reading the
/// descriptor while dbt builds an override filters the graph by a set the run
/// never built.
///
/// Selectors are dbt's grammar and are passed verbatim — reimplementing it is a
/// standing source of divergence (docs/dbt-runtime.md).
fn add_selection(
    cmd: &mut Command,
    descriptor: &DbtDescriptor,
    inv: &Invocation,
) -> error::Result<()> {
    for s in effective_select(descriptor, inv)? {
        cmd.args(["--select", &s]);
    }
    for s in effective_exclude(descriptor, inv)? {
        cmd.args(["--exclude", &s]);
    }
    if let Some(sel) = descriptor.selector.as_deref() {
        cmd.args(["--selector", sel]);
    }
    Ok(())
}

fn effective_select(descriptor: &DbtDescriptor, inv: &Invocation) -> error::Result<Vec<String>> {
    Ok(arg_list(&inv.args, "select")?.unwrap_or_else(|| descriptor.select.clone()))
}

fn effective_exclude(descriptor: &DbtDescriptor, inv: &Invocation) -> error::Result<Vec<String>> {
    Ok(arg_list(&inv.args, "exclude")?.unwrap_or_else(|| descriptor.exclude.clone()))
}

/// Whether an invocation selects a subset at all. `[]` from a run clears the
/// descriptor's selector, which puts the run back to the whole project.
fn has_selection(descriptor: &DbtDescriptor, inv: &Invocation) -> error::Result<bool> {
    Ok(!effective_select(descriptor, inv)?.is_empty()
        || !effective_exclude(descriptor, inv)?.is_empty()
        || descriptor.selector.is_some())
}

/// An explicitly supplied list, including an empty one — passing `[]` is how a
/// run clears a selector the descriptor sets, so it must not read as "absent"
/// and fall back to the descriptor.
///
/// A malformed one is an error rather than an absence: argument-schema
/// validation is opt-in, so treating `"stg_orders"` as unset would silently run
/// the descriptor's broader selection instead of the one the caller asked for.
fn arg_list(args: &HashMap<String, Box<RawValue>>, k: &str) -> error::Result<Option<Vec<String>>> {
    let Some(raw) = args.get(k) else {
        return Ok(None);
    };
    if raw.get().trim() == "null" {
        return Ok(None);
    }
    serde_json::from_str::<Vec<String>>(raw.get())
        .map(Some)
        .map_err(|e| Error::BadRequest(format!("`{k}` must be a list of strings: {e}")))
}

pub(crate) fn digest(s: &str) -> String {
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    format!("{:x}", h.finalize())[..32].to_string()
}

fn copy_dir_command(from: &Path, to: &Path) -> Command {
    let mut cmd = Command::new("cp");
    cmd.arg("-a").arg(format!("{}/.", from.display())).arg(to);
    cmd
}

/// Copy a package tree, bounded by the job.
///
/// The tree is the project's, so its size is not ours to assume: run it under
/// the poller like every other phase, or a cancelled or timed-out job keeps its
/// worker slot until `cp` finishes on its own.
async fn copy_dir_watched(
    from: &Path,
    to: &Path,
    label: &str,
    ctx: &mut JobCtx<'_>,
    job_id: &Uuid,
    w_id: &str,
    conn: &Connection,
) -> error::Result<()> {
    tokio::fs::create_dir_all(to)
        .await
        .map_err(|e| Error::internal_err(format!("creating {to:?}: {e}")))?;
    let mut cmd = copy_dir_command(from, to);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let child = start_child_process(cmd, "cp", false).await?;
    handle_child(
        job_id,
        conn,
        ctx.mem_peak,
        ctx.canceled_by,
        child,
        false,
        ctx.worker_name,
        w_id,
        label,
        ctx.timeout(),
        false,
        &mut Some(ctx.occupancy_metrics),
        None,
        None,
    )
    .await
    .map(|_| ())
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
        assert!(has_selection(&descriptor, &Invocation::default()).unwrap());
        assert!(!has_selection(&descriptor, &cleared).unwrap());
        // A wrong-typed override is refused rather than read as absent, which
        // would silently run the descriptor's broader selection.
        let malformed = Invocation {
            args: [(
                "select".to_string(),
                serde_json::value::RawValue::from_string("\"stg_orders\"".to_string()).unwrap(),
            )]
            .into_iter()
            .collect(),
            ..Default::default()
        };
        assert!(has_selection(&descriptor, &malformed).is_err());
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

    // The jail profile is protobuf text format, and both the project path (a
    // directory named by whoever wrote the repo) and the descriptor's
    // environment land inside string literals. An unescaped quote or newline
    // would close the literal and let the rest be read as further directives —
    // extra host bind mounts, for one.
    #[test]
    fn jail_values_cannot_close_their_string_and_add_directives() {
        let hostile = "proj\"\nmount {\n src: \"/\"\n dst: \"/host\"\n}\n#";
        let escaped = escape_textproto(hostile);
        assert!(!escaped.contains('\n'), "{escaped}");
        // Every quote that survives is escaped, so none of them terminates the
        // literal.
        let mut chars = escaped.chars().peekable();
        let mut prev = None;
        while let Some(c) = chars.next() {
            if c == '"' {
                assert_eq!(prev, Some('\\'), "unescaped quote in {escaped}");
            }
            // A doubled backslash is a literal one, so it does not escape what
            // follows it.
            prev = if c == '\\' && prev == Some('\\') {
                None
            } else {
                Some(c)
            };
        }

        let envars = jail_envars(&[("LD_PRELOAD".to_string(), hostile.to_string())]);
        assert_eq!(envars.lines().count(), 1, "{envars}");
        assert!(envars.starts_with("envar: \"LD_PRELOAD="), "{envars}");
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
    // another project's run_results.json against this project — and an empty
    // script_path is exactly how that happened.
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

    // The digest gates the retry state and keys the package cache, so a value
    // that depends on map ordering would evict and reject on every run, and one
    // that ignores content would let an edited project resume the previous
    // attempt's failures against models it no longer builds.
    #[test]
    fn the_project_digest_is_content_addressed_and_order_free() {
        use windmill_common::scripts::{ScriptLang, ScriptModule};
        let m = |pairs: &[(&str, &str)]| {
            pairs
                .iter()
                .map(|(k, v)| {
                    (
                        k.to_string(),
                        ScriptModule {
                            content: v.to_string(),
                            language: ScriptLang::Dbt,
                            lock: None,
                        },
                    )
                })
                .collect::<HashMap<_, _>>()
        };
        let a = m(&[("models/a.sql", "select 1"), ("dbt_project.yml", "name: p")]);
        let b = m(&[("dbt_project.yml", "name: p"), ("models/a.sql", "select 1")]);
        assert_eq!(project_digest(Some(&a)), project_digest(Some(&b)));
        let edited = m(&[("models/a.sql", "select 2"), ("dbt_project.yml", "name: p")]);
        assert_ne!(project_digest(Some(&a)), project_digest(Some(&edited)));
        // A file renamed with the same body is a different project too: the
        // separators keep `ab|c` from digesting the same as `a|bc`.
        let renamed = m(&[("models/b.sql", "select 1"), ("dbt_project.yml", "name: p")]);
        assert_ne!(project_digest(Some(&a)), project_digest(Some(&renamed)));
    }

    // The path is project-controlled and both cache copies are rooted at it, so
    // an absolute or `..`-bearing value would read and write outside the job.
    #[tokio::test]
    async fn a_projects_packages_path_cannot_escape_the_project() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let write = |yml: &str| std::fs::write(root.join("dbt_project.yml"), yml).unwrap();

        write("name: p\n");
        assert_eq!(packages_install_path(root).await, "dbt_packages");
        write("name: p\npackages-install-path: ./vendor\n");
        assert_eq!(packages_install_path(root).await, "vendor");
        for escape in ["/etc", "../../etc", "a/../../b", ""] {
            write(&format!("name: p\npackages-install-path: \"{escape}\"\n"));
            assert_eq!(
                packages_install_path(root).await,
                "dbt_packages",
                "{escape} must not be honoured"
            );
        }
    }

    // A retry of an all-green run selects nothing, writes nothing, and still
    // succeeds — and a successful dbt job dispatches every deploy-time write,
    // so it would wake every downstream consumer for relations no one touched.
    // A retry rewrites `run_results.json` with only the nodes it redid, so the
    // job's own result has to be the union: replacing would drop every node that
    // succeeded before it, and nothing would settle their materializations.
    #[test]
    fn a_retrys_results_overlay_the_attempt_they_resumed() {
        let node = |id: &str, status: &str| DbtNodeResult {
            unique_id: id.to_string(),
            status: status.to_string(),
            execution_time: None,
            rows_affected: None,
            relation_name: None,
            message: None,
            failures: None,
        };
        let mut acc = vec![
            node("model.p.a", "success"),
            node("model.p.b", "error"),
            node("model.p.c", "skipped"),
        ];
        merge_results(
            &mut acc,
            vec![node("model.p.b", "success"), node("model.p.c", "success")],
        );
        assert_eq!(acc.len(), 3, "the untouched node must survive the retry");
        let by = |id: &str| acc.iter().find(|n| n.unique_id == id).unwrap();
        assert_eq!(by("model.p.a").status, "success");
        assert_eq!(by("model.p.b").status, "success");
        assert_eq!(by("model.p.c").status, "success");
        // A node the retry introduces is kept rather than dropped.
        merge_results(&mut acc, vec![node("test.p.d", "fail")]);
        assert_eq!(acc.len(), 4);
    }

    #[test]
    fn a_retry_needs_something_to_retry() {
        let results = |statuses: &[&str]| {
            format!(
                r#"{{"results":[{}]}}"#,
                statuses
                    .iter()
                    .map(|s| format!(r#"{{"unique_id":"model.p.m","status":"{s}"}}"#))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        };
        assert!(!has_retryable_node(&results(&["success"])));
        assert!(!has_retryable_node(&results(&["success", "pass"])));
        assert!(!has_retryable_node(r#"{"results":[]}"#));
        for retryable in ["error", "fail", "skipped", "partial success"] {
            assert!(
                has_retryable_node(&results(&["success", retryable])),
                "{retryable} must be retryable"
            );
        }
        // A failed TEST is retryable to dbt but writes nothing, so a retry of
        // it would succeed having materialised nothing and still dispatch every
        // deploy-time write. `test_behavior: after_all` is exactly how
        // `run_results.json` comes to describe tests alone.
        let tests_only = r#"{"results":[
            {"unique_id":"test.p.not_null_orders_id.ab","status":"fail"},
            {"unique_id":"test.p.unique_orders_id.cd","status":"error"}]}"#;
        assert!(!has_retryable_node(tests_only));
        let with_a_model = r#"{"results":[
            {"unique_id":"test.p.not_null_orders_id.ab","status":"fail"},
            {"unique_id":"snapshot.p.customers","status":"skipped"}]}"#;
        assert!(has_retryable_node(with_a_model));
        // Unreadable results let dbt decide rather than refusing a retry the
        // user may well need.
        assert!(has_retryable_node("not json"));
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
