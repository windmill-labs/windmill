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
    parse_dbt_descriptor, DbtDescriptor, DbtTestBehavior, DBT_COMMANDS, DBT_COMMAND_ARG,
    DBT_COMMAND_LABEL, DBT_DEFAULT_WAREHOUSE,
};
use windmill_queue::{append_logs, CanceledBy, MiniPulledJob};

use crate::common::{
    render_nsjail_rlimit_as, resolve_nsjail_timeout, resolve_nsjail_tmp_mount_block,
};
use crate::common::{start_child_process, OccupancyMetrics};
use crate::dbt_engine::{provision_engine, ProvisionedEngine, DBT_CACHE_DIR};
use crate::dbt_profiles::{
    ensure_adapter_licensed, render_dbt_profile, render_profile, DbtAdapter, KnownAdapter,
};
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
    /// Digest of the `package-lock.yml` produced at deploy. Package trees are
    /// worker-local, so a cache miss on another worker must prove it resolved
    /// the same dependencies before it may run this script version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package_lock_digest: Option<String>,
    pub manifest_digest: String,
    pub engine: String,
    pub engine_version: String,
}

/// Per-node outcome, from `run_results.json`.
#[derive(Serialize, Debug, Clone)]
pub struct DbtNodeResult {
    pub unique_id: String,
    /// dbt's own status word, verbatim (`success`, `error`, `partial success`,
    /// `no-op`, …). Kept because it is what the log and dbt's docs say, but it
    /// is dbt's vocabulary to change — read `outcome` to make a decision.
    pub status: String,
    /// The same result in Windmill's terms, which is the stable half of this
    /// contract: `passed` | `failed` | `warned` | `skipped` | `no_op` |
    /// `unknown`. A dbt release that renames a status, or adds one, moves
    /// `status` and leaves this alone.
    pub outcome: &'static str,
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
    /// The arguments this invocation ran with, as SUBMITTED — a `$var:` stays a
    /// reference, so no resolved value (and no secret) is published.
    ///
    /// Present because a `dbt retry` restores the failed run's arguments inside
    /// the worker and they are never written back to the retry job: its own
    /// args are just `{"dbt_command": "retry"}`. Anything that needs to act on
    /// what the run actually used — the row preview, which is a `dbt show` of
    /// the same project — cannot get them from the job.
    #[serde(skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub invocation_args: std::collections::HashMap<String, Box<RawValue>>,
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

pub(crate) async fn handle_dbt_job(
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
    let locks: Option<DbtDependencyLocks> = requirements_o
        .map(|s| {
            serde_json::from_str(s)
                .map_err(|e| Error::internal_err(format!("reading the dbt lockfile: {e}")))
        })
        .transpose()?;

    // Through `build_args_map`, like every other executor: dbt cannot resolve a
    // `$var:` / `$res:` / `$encrypted:` reference, so passing one raw sends the
    // literal string to `--vars` — a placeholder holding a schema or an `enabled`
    // flag would then build a different slice of the project than was asked for.
    let args = flatten_command(
        crate::common::build_args_map(job, client, conn)
            .await?
            .unwrap_or_else(|| job.args.as_ref().map(|a| a.0.clone()).unwrap_or_default()),
    )?;
    // As submitted, command block and all: this is what the state saves and the
    // result publishes, and both describe an invocation of this script, not one
    // executor's view of it.
    let raw_args = job.args.as_ref().map(|a| a.0.clone()).unwrap_or_default();
    let inv = Invocation { args: args.clone(), raw_args, envs: envs.clone(), strict: true };
    // One wall clock for the whole job. A dbt job is a sequence of
    // subprocesses — provision, deps, parse, ls, build, then the
    // `after_all` tests — and each would otherwise resolve the job's full
    // timeout for itself.
    let deadline = JobDeadline::start(conn, &job.workspace_id, job.id, job.timeout).await;
    // Built once and reborrowed into every phase. The five fields travel
    // together through the whole executor, so passing them apart means each new
    // phase grows another five parameters and another copy of this literal.
    let mut ctx = JobCtx { mem_peak, canceled_by, occupancy_metrics, worker_name, deadline };
    // From EVERY dbt run, not from the progress reporter: that reporter runs only
    // for engines emitting node events, so hanging the prune off it leaves a
    // Fusion-only or dbt-core-2x instance accumulating rows nothing deletes.
    if let Connection::Sql(pool) = conn {
        let (pool, prune_w_id) = (pool.clone(), job.workspace_id.clone());
        let prune_path = job.runnable_path.clone().unwrap_or_default();
        tokio::spawn(async move {
            windmill_common::dbt_manifest::prune_run_progress(&pool, &prune_w_id).await;
            if let Err(e) =
                windmill_common::dbt_manifest::prune_dbt_run_graphs(&pool, &prune_path, &prune_w_id)
                    .await
            {
                tracing::warn!("pruning dbt run graph snapshots: {e:#}");
            }
        });
    }
    let prepared = prepare_project(
        &descriptor,
        inner_content,
        locks.as_ref(),
        job_dir,
        &job.id,
        &job.workspace_id,
        job.runnable_path.as_deref().unwrap_or_default(),
        job.runnable_id.map(|h| h.0),
        conn,
        client,
        &mut ctx,
        &envs,
        modules,
    )
    .await;
    // A preparation failure leaves the saved run alone: nothing up to here — the
    // bundle, `dbt deps`, the engine, the profile — touches a relation, so the
    // previous run's failures still describe the warehouse. Only an interrupted
    // BUILD invalidates that, which the save at the end of the job decides.
    let mut prepared = prepared?;

    // A `vars` override drives `enabled`, alias, schema, database and
    // materialization, so the run's models are not the deployed graph's: it
    // snapshots under its own job id. Re-asked below for a retry, which submits
    // only `dbt_command` until the failed run's arguments are restored.
    prepared.graph_refresh.add_caller_args(&descriptor, &args)?;

    let command = match arg_str(&args, "dbt_command")? {
        // An allowlist, not a passthrough: the value becomes the dbt subcommand,
        // and running a script needs weaker permission than editing it — so an
        // unchecked arg would let a runner invoke `clean` or `seed` on the
        // descriptor's warehouse.
        Some(c) if DBT_COMMANDS.contains(&c.as_str()) => c,
        Some(c) => {
            return Err(Error::BadRequest(format!(
                "`dbt_command` must be one of {}, got `{c}`",
                DBT_COMMANDS.join(", ")
            )))
        }
        None => windmill_parser_yaml::default_dbt_command(&descriptor).to_string(),
    };
    // A parse is the whole job: it resolves the project into a manifest, stores
    // the graph and stops. Handled before everything below because none of it
    // applies — nothing is built, so there is no test phase, no materialization,
    // no retry state and no ownership to publish.
    if command == "parse" {
        return run_parse_only(
            &prepared,
            &descriptor,
            // Tolerant of the `{{ }}` placeholders only a run can fill, exactly
            // as the deploy's own parse is: refreshing an editor buffer must not
            // require filling the run form in first, and the graph it produces is
            // the one the deploy would store.
            &Invocation { strict: false, ..inv },
            &mut ctx,
            job,
            conn,
        )
        .await;
    }

    // `dbt retry` resumes from the previous run's `run_results.json`, which is what
    // makes one-job-per-invocation defensible. Each attempt gets a fresh job dir, so
    // that state is restored along with the ARGUMENTS it ran with: dbt reuses the
    // failed invocation's selection and vars, so every phase must agree with them.
    let mut restored_results_digest: Option<String> = None;
    let inv = if command == "retry" {
        // Read BEFORE the restore replaces the arguments. A retry must name the run
        // it resumes: only the latest failure of this script is kept, so an unnamed
        // one would mean "whatever failed last" and quietly resume a different run
        // than the caller was looking at.
        let expected = arg_str(&inv.args, "dbt_retry_job")?.filter(|s| !s.trim().is_empty());
        let Some(expected) = expected else {
            return Err(Error::BadRequest(
                "a `retry` needs `dbt_retry_job`, the id of the run to resume. Open that run and \
                 use `Resume this run`, or pass its id: only the latest failure of this script is \
                 kept, so a retry names which one it means"
                    .to_string(),
            ));
        };
        // Parsed, not compared as text: the saved run is a `uuid`, and an id
        // that differs only in case or in braces names the same run.
        let expected = Uuid::parse_str(expected.trim()).map_err(|_| {
            Error::BadRequest(format!(
                "`dbt_retry_job` must be the id of the run to resume, got `{expected}`"
            ))
        })?;
        let restored = restore_run_state(
            &prepared,
            &job.workspace_id,
            &job.permissioned_as,
            &inv,
            expected,
            conn,
        )
        .await?;
        restored_results_digest = Some(restored.results_digest.clone());
        // Restored args are the ones SUBMITTED, so the references they carry are
        // resolved again now — against this caller's access, not the original's.
        let inv = Invocation {
            args: flatten_command(
                crate::common::transform_json(client, &job.workspace_id, &restored.args, job, conn)
                    .await?
                    .unwrap_or_else(|| restored.args.clone()),
            )?,
            raw_args: restored.args,
            ..inv
        };
        // Compared only now that they are resolved: a `$var:` whose value moved
        // selects a different node set, so the saved failures no longer describe
        // what a retry would build. Refused rather than resumed, since which
        // graph it would use depends on which worker it lands on.
        if let Some(saved) = restored.args_digest.as_deref() {
            if saved != inv.resolved_args_digest() {
                return Err(Error::BadRequest(
                    "the values this run's arguments resolve to have changed since the run \
                     being retried, so its failures no longer describe what a retry would \
                     build; run the script normally instead"
                        .to_string(),
                ));
            }
        }
        // The restored arguments decide this retry's graph, and RESOLVED ones: a
        // saved `select` spelled `$res:` is a string until resolved, so reading the
        // raw form refuses the retry as "must be a list of strings" for a reference
        // that resolves to the very list the failed run built with.
        prepared
            .graph_refresh
            .add_caller_args(&descriptor, &inv.args)?;
        if restored.needs_parse {
            // After the resolution above, so the manifest describes the project
            // the build is about to retry.
            run_dbt_parse(
                &prepared,
                &descriptor,
                &inv,
                &mut ctx,
                &job.id,
                &job.workspace_id,
                conn,
            )
            .await?;
        }
        inv
    } else {
        inv
    };

    // Ingested BEFORE the build, from a `dbt parse` with this run's vars, so the
    // models shown are the ones about to be built. Rows are keyed by path, version
    // AND job so no two runs collide; the path-keyed `asset` usage belongs to one
    // version, which `claim_graph_publication` arbitrates (docs/dbt-runtime.md).
    if prepared.graph_refresh.needed() && !windmill_parser_yaml::dbt::is_read_only_command(&command)
    {
        // A parse and an ingest write no relation either, so a failure in either
        // leaves the saved run as accurate as the preparation exits above do.
        if command != "retry" {
            run_dbt_parse(
                &prepared,
                &descriptor,
                &inv,
                &mut ctx,
                &job.id,
                &job.workspace_id,
                conn,
            )
            .await?;
        }
        // For a retry the restored manifest already describes the invocation
        // being resumed, so only the ingest runs — with that invocation's
        // arguments, which the selection resolver needs to interpolate.
        ingest_from_run(&prepared, &descriptor, &inv, &mut ctx, job, conn).await?;
    }

    // A read-only command prints rows to stdout, so it is captured rather than
    // streamed, and nothing below applies: nothing was built, so there is no graph
    // to publish, no materialization, no test phase and nothing to retry.
    if windmill_parser_yaml::dbt::is_read_only_command(&command) {
        return run_show(
            &prepared,
            &descriptor,
            &inv,
            &mut ctx,
            &job.id,
            &job.workspace_id,
            conn,
        )
        .await;
    }

    let mut run = run_dbt(
        &prepared,
        &command,
        &descriptor,
        &inv,
        job,
        conn,
        &mut ctx,
        true,
    )
    .await;

    // `after_all` is two invocations, models then tests, and each REWRITES
    // `run_results.json` — so the model results are read before the test phase
    // overwrites them, or the job reports tests alone and nothing settles the
    // models' materializations.
    let mut results = read_run_results(&prepared.project_dir).await;

    // In-job node retry: rebuilding only the failed and skipped nodes, while the
    // previous attempt's `run_results.json` is still in the job directory. Never on
    // an agent worker, which cannot read `v2_job_queue` — the wait below would be
    // uninterruptible, so a cancelled job would hold its slot and then start dbt.
    let node_retry = descriptor
        .retry_failed_nodes
        .filter(|_| matches!(conn, Connection::Sql(_)));
    let mut retries_left = node_retry.map(|p| p.attempts()).unwrap_or(0);
    if let Some(policy) = node_retry.filter(|_| run.is_err()) {
        retry_failed_nodes(
            policy,
            &prepared,
            &descriptor,
            &inv,
            job,
            conn,
            &mut ctx,
            &mut run,
            &mut results,
            &mut retries_left,
        )
        .await;
    }
    // A `retry` whose saved results were tests alone IS the test phase: dbt reran
    // exactly those tests, so running the suite after it would execute every test
    // a second time and report each one twice. `test_behavior: after_all` is how
    // `run_results.json` comes to hold tests alone.
    let retry_was_the_test_phase = command == "retry"
        && !results.is_empty()
        && results
            .iter()
            .all(|n| n.unique_id.starts_with("test.") || n.unique_id.starts_with("unit_test."));
    // `retry` counts as the model phase too: a run that failed midway and was
    // retried to success would otherwise return green having never tested.
    if run.is_ok()
        && matches!(descriptor.test_behavior, DbtTestBehavior::AfterAll)
        && matches!(command.as_str(), "build" | "retry")
        && !retry_was_the_test_phase
    {
        run = run_dbt(
            &prepared,
            "test",
            &descriptor,
            &inv,
            job,
            conn,
            &mut ctx,
            // The tests must be scoped exactly like the models were: testing
            // the whole project would assert against models this script never
            // builds, the same failure the ingest-side scoping fixes.
            true,
        )
        .await;
        // Merged, not appended: the model phase and the test phase can name the
        // same node, and a duplicate would double its totals and collide as a key
        // in the result table.
        merge_results(&mut results, read_run_results(&prepared.project_dir).await);
        // The same policy applies to a failing test. `dbt retry` redoes test
        // nodes, and the descriptor promises to retry the ones that failed —
        // retrying the model phase alone would exempt the failure mode
        // `test_behavior: after_all` exists to produce.
        if let Some(policy) = node_retry.filter(|_| run.is_err()) {
            retry_failed_nodes(
                policy,
                &prepared,
                &descriptor,
                &inv,
                job,
                conn,
                &mut ctx,
                &mut run,
                &mut results,
                &mut retries_left,
            )
            .await;
        }
    }

    // Best-effort: losing the state costs a retry, not the run that just
    // finished. Logged rather than dropped — without it the only symptom is
    // `dbt retry` reporting nothing to resume, which reads as a bug in retry.
    if let Err(e) = save_run_state(
        &prepared,
        &job.workspace_id,
        &job.permissioned_as,
        job.visible_to_owner,
        &job.id,
        &inv,
        restored_results_digest.as_deref(),
        conn,
    )
    .await
    {
        tracing::warn!("dbt: could not save retry state for job {}: {e:#}", job.id);
    }
    let reconciled = reconcile_materializations(&prepared, &results, job, conn, client).await;
    terminalize_running_relations(job, &reconciled, conn).await;

    let result = build_result(&prepared, &command, results, &inv);
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
pub(crate) async fn dbt_dep(
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
    // A DEPLOY writes a whole node set of its own, `raw_code` included, so it
    // has to reclaim as well: hung off runs alone, a project redeployed on every
    // push by CI and run nightly kept one full graph per push until the next
    // run, and one deployed but never run kept them for good.
    {
        let (pool, prune_w_id) = (db.clone(), w_id.to_string());
        let prune_path = script_path.to_string();
        tokio::spawn(async move {
            if let Err(e) =
                windmill_common::dbt_manifest::prune_dbt_run_graphs(&pool, &prune_path, &prune_w_id)
                    .await
            {
                tracing::warn!("pruning dbt graphs at deploy: {e:#}");
            }
        });
    }
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
    let mut ctx = JobCtx { mem_peak, canceled_by, occupancy_metrics, worker_name, deadline };
    let prepared = prepare_project(
        &descriptor,
        content,
        None,
        job_dir,
        job_id,
        w_id,
        script_path,
        deploying_script_hash(db, job_id).await,
        &conn,
        &client,
        &mut ctx,
        &envs,
        modules,
    )
    .await?;

    // A deploy has no job arguments, so it tolerates the `{{ }}` placeholders
    // only a run can fill (see `Invocation::strict`). Its environment is the
    // script's, matching what the run will parse with.
    let inv = Invocation { envs: envs.clone(), strict: false, ..Default::default() };
    run_dbt_parse(&prepared, &descriptor, &inv, &mut ctx, job_id, w_id, &conn).await?;

    let selected =
        resolve_selection(&prepared, &descriptor, &inv, &mut ctx, job_id, w_id, &conn).await?;
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
    // dependency jobs. The GRAPH is keyed by version so both may write theirs;
    // the path-keyed asset usages are claimed by the newest.
    let publisher = match deploying_script_hash(db, job_id).await {
        Some(hash) => GraphPublisher::Version(hash),
        None => GraphPublisher::Unversioned,
    };
    let superseded = if let Some(warehouse) = prepared.warehouse.as_deref() {
        let ingested = windmill_common::dbt_manifest::ingest_manifest(
            &manifest,
            warehouse,
            prepared.default_database.as_deref(),
            selected.as_ref(),
        );
        let published = persist_ingest(
            db,
            w_id,
            script_path,
            &ingested,
            &prepared.relation_root(),
            publisher,
            None,
            true,
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
        // No warehouse identity, so nothing can be ingested — but this version's
        // rows must still go, or a descriptor moved to its own profiles.yml keeps
        // claiming relations it no longer describes.
        let mut tx = db.begin().await?;
        // Clearing is a publication too: an older job that no longer describes the
        // script must not wipe a newer deploy's graph.
        let published = claim_graph_publication(&mut tx, w_id, script_path, publisher).await?;
        if published {
            // This VERSION's rows, never the path's: what is given up is the
            // path-keyed usage cleared below, while an older version's graph is
            // what its own finished runs still render.
            if let GraphPublisher::Version(hash) = publisher {
                windmill_common::dbt_manifest::clear_dbt_manifest_version(
                    &mut tx,
                    w_id,
                    script_path,
                    hash,
                )
                .await?;
            }
            windmill_common::assets::replace_static_asset_usage(&mut tx, w_id, script_path, &[])
                .await?;
            tx.commit().await?;
            append_logs(
                job_id,
                w_id,
                "\nNo asset-graph ingest: the descriptor names no `profile.warehouse` beside \
                 its own `profile.profiles_yml`, so there is no warehouse identity to key \
                 `dbt://` assets on. Any previously ingested nodes for this script have been \
                 cleared.\n"
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
        package_lock_digest: prepared.package_lock_digest.clone(),
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

/// Why a run re-ingests its graph instead of trusting the deployed version's —
/// and, since the reasons differ in WHOSE graph the result is, what becomes of
/// it.
///
/// A model set this RUN decides — a caller's override, or a descriptor dynamic by
/// construction — is stored under the job id so the run page shows what it built,
/// leaving the script's ownership alone: those schemas and aliases would otherwise
/// stand as the script's until the next deploy.
///
/// A moved PROFILE is the one that republishes, because every later run of this
/// version resolves there too. Publishing is also what ENDS a drift: the check
/// reads back the published root, so a run that saw a move and did not republish
/// leaves the next one seeing the same move.
#[derive(Clone, Copy, Default)]
pub struct GraphRefresh {
    /// This run's models are not the deployed descriptor's: a `{{ }}`
    /// placeholder in `vars` or a `$var:` in `env` (re-resolved every run), or
    /// an invocation that overrode `vars`. Vars steer `enabled`, alias, schema,
    /// database and materialization, so the deployed graph names another run's
    /// relations.
    per_run_models: bool,
    /// The profile resolves somewhere other than where the published usages
    /// point. The relations moved for the VERSION, not for one invocation.
    profile_drift: bool,
}

impl GraphRefresh {
    /// Whether this run parses and ingests a graph of its own at all.
    fn needed(&self) -> bool {
        self.per_run_models || self.profile_drift
    }

    /// The job to key this graph under, or `None` to write the version's own.
    ///
    /// Only a DRIFT alone writes the version's: the move is permanent, and
    /// storing it per run would leave every later run — which no longer detects
    /// a drift, because this one published the new root — reading the pre-move
    /// rows. A run whose models are its own goes under the job id, in both
    /// directions: written as the version's, a narrowing selection would drop
    /// every model this invocation left out, and a widening one would add models
    /// that version never had.
    fn snapshot_job(&self, job_id: uuid::Uuid) -> Option<uuid::Uuid> {
        self.per_run_models.then_some(job_id)
    }

    /// Whether this ingest also becomes what the script owns.
    ///
    /// Exactly when it wrote the VERSION's graph and the caller scoped nothing.
    /// The workspace graph takes an asset's relations from the `asset` rows and
    /// its models, SQL, tests and `ref()` lineage from that version's
    /// `dbt_node`/`dbt_edge`, so publishing relations the version's graph does
    /// not name leaves those assets with no model behind them — a placeholder
    /// that moves an alias would empty the current graph of everything dbt
    /// contributes to it.
    ///
    /// So a run that stored a snapshot of its own publishes nothing, which
    /// leaves two cases settled elsewhere and deliberately: an override's
    /// relations are a one-off and are meant not to stand as the script's, and a
    /// dynamic descriptor at a moved profile keeps its ownership at the deploy's
    /// relations until a redeploy — every run of it still shows its own models,
    /// and it re-parses regardless, so the undetected-forever drift costs it
    /// nothing it was not already paying.
    ///
    /// The exact complement of `snapshot_job`, which is what lets the agent
    /// worker's payload carry one `per_run` bit and no second flag: decouple the
    /// two and that wire format stops describing this decision.
    fn publishes_ownership(&self) -> bool {
        !self.per_run_models
    }

    /// Fold in what this invocation's own arguments say about its model set.
    fn add_caller_args(
        &mut self,
        descriptor: &DbtDescriptor,
        args: &HashMap<String, Box<RawValue>>,
    ) -> error::Result<()> {
        if has_vars_override(args) {
            self.per_run_models = true;
        }
        // A caller's selection is not necessarily a SUBSET of the deployed one:
        // deployed `select: ["tag:nightly"]`, overridden with `["*"]`, builds models
        // the deployed graph never had — and those are the ones whose progress, SQL
        // and lineage the run page would otherwise have nothing to draw.
        if selection_is_overridden(descriptor, args)? {
            self.per_run_models = true;
        }
        Ok(())
    }
}

pub struct PreparedProject {
    pub project_dir: PathBuf,
    pub profiles_dir: PathBuf,
    pub engine: ProvisionedEngine,
    /// Why this run's graph is its own rather than the deployed version's, if
    /// it is.
    pub graph_refresh: GraphRefresh,
    /// Digest of the project's own files: the identity of the code that runs.
    /// It keys the package cache (a `local:` dependency's content appears in no
    /// manifest) and gates retry state, so a project edited between attempts
    /// cannot resume the old one.
    pub project_digest: String,
    /// Resolution produced by `dbt deps` at deploy. The package cache is local
    /// to one worker; this makes a cache miss on another worker fail closed if
    /// an unlocked range or mutable Git revision has moved meanwhile.
    pub package_lock_digest: Option<String>,
    /// The workspace warehouse's NAME, the `<warehouse>` component of every
    /// `dbt://` asset this project produces. `None` when the project brings its
    /// own `profiles.yml` and names no warehouse, in which case there is no
    /// stable warehouse identity to key assets on.
    pub warehouse: Option<String>,
    /// The descriptor's `profile.target`, passed as `--target` so it applies to
    /// a project-owned `profiles.yml` as well as a rendered one.
    pub target: Option<String>,
    /// The profile target's database. Nodes that override it qualify their
    /// `dbt://` schema segment so two databases cannot collapse onto one node.
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

    /// The environment `dbt_project.yml`'s own `env_var()` calls render
    /// against: the two the run gives dbt, in the order the child receives
    /// them. `HOME` is left out on purpose — it is Windmill's, not the
    /// project's, and it differs on every attempt.
    fn template_env(&self) -> HashMap<String, String> {
        self.descriptor_env
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .chain(self.invocation_env.iter().cloned())
            .collect()
    }

    /// A digest of the DESCRIPTOR's resolved environment, for `run_identity`.
    /// Digested rather than listed: the values are resolved secrets.
    ///
    /// Only the descriptor's own entries. `env` additionally carries `HOME`,
    /// set to this job's directory, which differs on every attempt — hashing it
    /// would make a retry reject its own predecessor every time.
    fn env_digest(&self) -> String {
        stable_digest(
            self.descriptor_env
                .iter()
                .flat_map(|(k, v)| [k.as_str(), v.as_str()]),
        )
    }

    /// Everything that decides which relations a run produces, which is what a
    /// retry has to match before it may resume a saved `run_results.json`: same
    /// project files, same warehouse and target, same engine. Identity only,
    /// never credentials — the profile is digested, and the digest is one-way.
    ///
    /// Anything omitted here is something a redeploy could change while a stale
    /// `run_results.json` stays eligible, so `dbt retry` would resume one
    /// project's failures inside another. The descriptor's resolved environment
    /// is in it because `env_var()` can drive a model's schema, database, alias
    /// or `enabled`.
    fn run_identity(&self) -> String {
        // The descriptor whole, not field by field, or the next field added to it
        // is silently left out. And the RESOLVED engine and adapter versions: an
        // unchanged project redeployed after a release resolves a newer dbt, whose
        // retry would otherwise feed one version's `run_results.json` to another.
        format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}",
            self.project_digest,
            self.package_lock_digest.as_deref().unwrap_or(""),
            self.engine.engine.as_str(),
            self.engine.version,
            self.engine.adapter_version.as_deref().unwrap_or(""),
            digest(&self.descriptor_content),
            self.env_digest(),
            self.relation_root(),
            self.profile_digest,
        )
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn prepare_project(
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
    // The version this job runs, which is the one whose stored graph the drift
    // check below must read. `None` for a preview, which has no stored graph.
    script_hash: Option<i64>,
    conn: &Connection,
    client: &AuthedClient,
    ctx: &mut JobCtx<'_>,
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
    // Vars drive `enabled`, alias, schema, database and materialization, so a
    // placeholder var or a `$var:` env value (re-resolved every run) makes the
    // deploy-time graph a guess and each run re-ingests its own manifest.
    let has_placeholder = |v: &str| v.contains("{{");
    let graph_refresh = GraphRefresh {
        per_run_models: descriptor
            .vars
            .values()
            .flat_map(windmill_parser_yaml::dbt::string_leaves)
            .any(has_placeholder)
            || descriptor.env.values().any(|v| v.starts_with("$var:")),
        ..Default::default()
    };

    let resolved_env = resolve_env(descriptor, client).await?;
    reject_reserved_env(
        resolved_env.iter().map(|(k, _)| k),
        "the descriptor's `env`",
    )?;
    reject_reserved_env(invocation_env.keys(), "the script's environment variables")?;
    // `PreparedProject::template_env`, before there is one.
    let template_env: HashMap<String, String> = resolved_env
        .iter()
        .cloned()
        .chain(invocation_env.iter().map(|(k, v)| (k.clone(), v.clone())))
        .collect();

    let (profiles_dir, warehouse, adapter, default_database, default_schema, profile_digest) =
        write_profiles(descriptor, &project_dir, job_dir, client, &template_env).await?;
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
        &mut *ctx,
    )
    .await?;

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
        let nsjail_timeout = resolve_nsjail_timeout(conn, w_id, *job_id, ctx.timeout()).await;
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
        package_lock_digest: locks.and_then(|l| l.package_lock_digest.clone()),
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
        graph_refresh,
        warehouse,
        target: descriptor.profile.target.clone(),
        descriptor_content: descriptor_content.to_string(),
        descriptor_env,

        default_database,
        default_schema,
        script_path: script_path.to_string(),
        env,
    };
    // A moved profile relocates every relation, so the stored graph names ones that
    // no longer exist. Compared against the root recorded beside THIS VERSION's
    // graph: not the deploy lock, which A→B→A matches while the graph sits at B,
    // and not the newest ingest of any job, which a matching run never writes.
    match conn {
        Connection::Sql(db) => {
            if let Some(stored) = sqlx::query_scalar!(
                "SELECT relation_root_at_last_ingest FROM dbt_graph_snapshot
                  WHERE workspace_id = $1 AND script_path = $2 AND script_hash = $3
                    AND job_id = '00000000-0000-0000-0000-000000000000'",
                w_id,
                script_path,
                script_hash,
            )
            .fetch_optional(db)
            .await?
            .flatten()
            {
                if stored != prepared.relation_root() {
                    prepared.graph_refresh.profile_drift = true;
                }
            }
        }
        // An agent cannot READ the stored root, but publishing settles what the
        // comparison asks: it re-ingests what it parsed. Under its own job id,
        // since it cannot tell a moved profile from an unmoved one and must not
        // overwrite the version's graph on a guess.
        Connection::Http(_) => prepared.graph_refresh.per_run_models = true,
    }
    prepared.package_lock_digest =
        install_packages(&prepared, locks.is_some(), &mut *ctx, job_id, w_id, conn).await?;
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

async fn package_lock_digest(project_dir: &Path) -> error::Result<Option<String>> {
    match tokio::fs::read_to_string(project_dir.join("package-lock.yml")).await {
        Ok(lock) => Ok(Some(digest(&lock))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(Error::internal_err(format!(
            "reading the dbt package lock: {e}"
        ))),
    }
}

fn package_cache_key(base: &str, lock_digest: &str) -> String {
    digest(&format!("{base}\nresolved-lock\n{lock_digest}"))
}

/// Resolve or restore `dbt_packages/`, proving the tree matches the dependency
/// resolution recorded when this script version was deployed.
///
/// dbt re-resolves ranges and mutable git revisions on every `dbt deps`; the deploy
/// is the only place that happens here, so runs of one version cannot disagree about
/// what they installed. A project that wants a newer version deploys again — nothing
/// expires a tree by age, which would re-resolve mid-run and then be refused below
/// for differing from the pin.
async fn install_packages(
    p: &PreparedProject,
    require_pinned_resolution: bool,
    ctx: &mut JobCtx<'_>,
    job_id: &Uuid,
    w_id: &str,
    conn: &Connection,
) -> error::Result<Option<String>> {
    // A hit skips `dbt deps`, so the key covers what determines the tree: declared
    // packages, a committed lock, and a `local:` dependency's content (no manifest
    // holds it; the project digest stands in). Workspace too — `dbt deps` fetches
    // private git packages, and a shared tree would run one workspace's for another.
    let mut key = format!(
        "{w_id}\n{}\n{}\n{:x}\n",
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
        return Ok(None);
    }
    // A committed `package-lock.yml` keys the lookup but is not the answer: dbt
    // rewrites it when the `sha1_hash` it recorded for `packages.yml` stops
    // matching, so holding the deploy to it would refuse the first deploy after a
    // package is added, with no way out. The deploy records what dbt resolved.
    let project_lock_digest = package_lock_digest(&p.project_dir).await?;
    let expected_lock_digest = p
        .package_lock_digest
        .as_deref()
        .or(project_lock_digest.as_deref());
    if require_pinned_resolution && expected_lock_digest.is_none() {
        return Err(Error::BadRequest(
            "this script's lock predates deployment-pinned dbt dependencies; redeploy the project"
                .to_string(),
        ));
    }
    // Where `dbt deps` actually writes. `packages-install-path` is a project
    // setting, and assuming the default means a project that moved it gets no
    // cache at all: the publish finds nothing to copy and every job resolves
    // its dependencies over the network again.
    let target = p
        .project_dir
        .join(packages_install_path(&p.project_dir, &p.template_env()).await?);
    let cached = expected_lock_digest.map(|lock| {
        PathBuf::from(&*DBT_CACHE_DIR)
            .join("packages")
            .join(package_cache_key(&key, lock))
    });
    if let Some(cached) = cached.as_ref().filter(|cached| cached.exists()) {
        let restored = copy_dir_watched(
            cached,
            &target,
            "restoring cached dbt_packages",
            ctx,
            job_id,
            w_id,
            conn,
        )
        .await;
        // A tree that went missing between `exists()` and the copy is a cache MISS,
        // not a failed job: `dbt deps` resolves it again. Falling through costs a
        // fetch; failing costs the run.
        if restored.is_ok() {
            append_logs(
                job_id,
                w_id,
                "\nReusing cached dbt_packages\n".to_string(),
                conn,
            )
            .await;
            return Ok(expected_lock_digest.map(str::to_string));
        }
        tokio::fs::remove_dir_all(&target).await.ok();
        append_logs(
            job_id,
            w_id,
            "\nCached dbt_packages went away mid-restore; resolving them again\n".to_string(),
            conn,
        )
        .await;
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
    let resolved_lock_digest = package_lock_digest(&p.project_dir).await?.ok_or_else(|| {
        Error::ExecutionErr(
            "`dbt deps` completed without producing package-lock.yml; Windmill cannot pin this \
             dependency resolution across workers"
                .to_string(),
        )
    })?;
    // Against the PIN alone. Only a run has a resolution to be held to; the deploy
    // is where one is established, so it accepts what dbt resolved and records it.
    if let Some(pinned) = p.package_lock_digest.as_deref() {
        if pinned != resolved_lock_digest {
            return Err(Error::BadRequest(
                "this project's dbt dependencies resolve differently here from the resolution \
                 recorded when this script version was deployed; redeploy to accept the new one"
                    .to_string(),
            ));
        }
    }
    if target.exists() {
        let cached = PathBuf::from(&*DBT_CACHE_DIR)
            .join("packages")
            .join(package_cache_key(&key, &resolved_lock_digest));
        publish_to_cache(&target, &cached, ctx, job_id, w_id, conn).await;
    }
    Ok(Some(resolved_lock_digest))
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
    if copy_dir_watched(
        from,
        &staging,
        "caching dbt_packages",
        ctx,
        job_id,
        w_id,
        conn,
    )
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
/// the project itself. Both paths are supported (decision 8): the workspace
/// warehouse is the ergonomic one, the project's own file is what makes an
/// existing repo run unchanged.
async fn write_profiles(
    descriptor: &DbtDescriptor,
    project_dir: &Path,
    job_dir: &str,
    client: &AuthedClient,
    template_env: &HashMap<String, String>,
) -> error::Result<(
    PathBuf,
    Option<String>,
    DbtAdapter,
    Option<String>,
    Option<String>,
    String,
)> {
    // The workspace's warehouse, always: a descriptor names one by NAME or takes
    // `main`, and cannot name a resource at all. The NAME is what asset identity
    // keys on, so every project on one warehouse shares its nodes while the
    // credential stays a workspace setting only an admin writes.
    let warehouse = descriptor
        .profile
        .warehouse
        .as_deref()
        .unwrap_or(DBT_DEFAULT_WAREHOUSE);

    let declared = descriptor
        .profile
        .adapter
        .as_deref()
        .map(DbtAdapter::from_dbt_type)
        .transpose()?;

    if let Some(own) = descriptor.profile.profiles_yml.as_deref() {
        crate::common::validate_relative_path(own, "profile.profiles_yml")?;
        let path = project_dir.join(own);
        let dir = path
            .parent()
            .ok_or_else(|| Error::BadRequest("profile.profiles_yml has no parent".to_string()))?
            .to_path_buf();
        // Read from the project's own file even when the descriptor declares a type:
        // the file is what dbt connects with. Licensing is why it cannot be a hint —
        // the Rust engines carry every adapter, so a descriptor claiming `postgres`
        // over a `sqlserver` target would pass the CE check and connect anyway. A
        // templated `type` is refused outright rather than taken from the
        // descriptor, for the same reason: the claim cannot be checked against
        // what dbt will render.
        let target = adapter_from_profiles_yml(
            &path,
            &project_profile_name(project_dir, template_env).await,
            descriptor.profile.target.as_deref(),
        )
        .await?;
        let actual = target.adapter;
        if let Some(declared) = declared.filter(|d| *d != actual).as_ref() {
            return Err(Error::BadRequest(format!(
                "`profile.type: {}` disagrees with `{}`, whose target uses `{}`. dbt connects \
                 with the file, so remove `profile.type` or correct it",
                declared.name(),
                own,
                actual.name(),
            )));
        }
        let adapter = actual;
        ensure_adapter_licensed(&adapter)?;
        // The target's own database and schema, read from the file dbt connects
        // with. A relation that sits in them is then spelled plainly, exactly as
        // a workspace-warehouse project spells it, and one that overrides them
        // qualifies. Where the file leaves them implicit they stay `None` and
        // every relation qualifies, since assuming two share a database is what
        // would collapse distinct relations onto a single node.
        let profile_digest = digest(&tokio::fs::read_to_string(&path).await.unwrap_or_default());
        // Identity only when the descriptor NAMES a warehouse: defaulting to
        // `main` would key a self-hosted profile's assets onto the workspace
        // warehouse it never connected to.
        //
        // The name is still resolved, because a name that matches no configured
        // warehouse is not identity, it is a typo that would strand this
        // project's models on a node nothing else reaches.
        let identity = match descriptor.profile.warehouse.as_deref() {
            Some(named) => {
                windmill_common::workspaces::validate_dbt_warehouse_name(named)?;
                // Only that it EXISTS: this project connects through its own
                // file, so the connection behind the name is never opened and
                // pulling it here would decrypt a credential for a string
                // comparison.
                client.dbt_warehouse_exists(named).await.map_err(|e| {
                    Error::BadRequest(format!(
                        "`profile.warehouse: {named}` is where this project's assets belong, so \
                         it must name a warehouse this workspace configures: {e}"
                    ))
                })?;
                Some(named.to_string())
            }
            None => None,
        };
        return Ok((
            dir,
            identity,
            adapter,
            target.database,
            target.schema,
            profile_digest,
        ));
    }

    use windmill_common::workspaces::DBT_PROFILE_RESOURCE_TYPE;

    let resolved = resolve_warehouse(warehouse, client).await?;
    let workspace_target = resolved.target;
    let value = resolved.value;
    // From the resource's TYPE, not its shape: both kinds are objects with a `type`
    // (Windmill's bigquery resource is a service-account JSON), so the value cannot
    // say which it is.
    let is_dbt_profile = resolved.resource_type == DBT_PROFILE_RESOURCE_TYPE;
    // The block is written for the adapter it names, so a descriptor claiming another
    // is a mistake worth naming rather than a profile rendered under the wrong type.
    let stated = is_dbt_profile
        .then(|| DbtAdapter::stated_by_dbt_profile(&value))
        .transpose()?;
    if let (Some(declared), Some(stated)) = (declared.as_ref(), stated.as_ref()) {
        if declared != stated {
            return Err(Error::BadRequest(format!(
                "`profile.type: {}` disagrees with the `{warehouse}` warehouse, whose resource \
                 states `{}`. Remove `profile.type` or correct it",
                declared.name(),
                stated.name(),
            )));
        }
    }
    let adapter = declared
        .or(stated)
        // The resource TYPE: decision 9's authority, which the warehouse now carries.
        // Inference reads connection details and covers a workspace's own type.
        .or_else(|| KnownAdapter::from_resource_type(&resolved.resource_type).map(DbtAdapter::from))
        .or_else(|| KnownAdapter::infer_from_resource(&value).map(DbtAdapter::from))
        .ok_or_else(|| {
            Error::BadRequest(format!(
                "could not tell which dbt adapter the `{warehouse}` warehouse needs; \
                 set `profile.type` in the descriptor"
            ))
        })?;
    ensure_adapter_licensed(&adapter)?;
    let profile_name = project_profile_name(project_dir, template_env).await;
    // The workspace's warehouse may name the target too, so a project that carries
    // no connection still gets `{{ target }}` right.
    let target = descriptor
        .profile
        .target
        .as_deref()
        .or(workspace_target.as_deref())
        .unwrap_or("default");
    let dir = PathBuf::from(job_dir).join("dbt_profiles");
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| Error::internal_err(format!("creating the profiles dir: {e}")))?;
    let rendered = if is_dbt_profile {
        let block = value.as_object().ok_or_else(|| {
            Error::BadRequest(
                "a `dbt_profile` resource is a `profiles.yml` output block, so its value must be \
                 an object"
                    .to_string(),
            )
        })?;
        render_dbt_profile(
            &adapter,
            block,
            &profile_name,
            target,
            descriptor.threads,
            descriptor.profile.schema.as_deref(),
            &dir,
        )?
    } else {
        render_profile(
            &adapter,
            &value,
            &profile_name,
            target,
            descriptor.threads,
            descriptor.profile.schema.as_deref(),
            &dir,
        )?
    };
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
        &client.token,
    );
    Ok((
        dir,
        Some(warehouse.to_string()),
        adapter,
        rendered.database,
        rendered.schema,
        profile_digest,
    ))
}

/// Where a workspace warehouse name points: its resource path and, if the
/// workspace names one, its target.
async fn resolve_warehouse(
    warehouse: &str,
    client: &AuthedClient,
) -> error::Result<windmill_common::workspaces::DbtWarehouseConnection> {
    // The descriptor supplies this name, so it is checked HERE too, not only
    // where settings are written: it reaches the route as a URL path segment,
    // and `../../resources/get_value/...` would resolve to another route.
    windmill_common::workspaces::validate_dbt_warehouse_name(warehouse)?;
    // Through the API even when this worker holds the database, because the
    // route is where a resource is interpolated against the job — `$WM_TOKEN`
    // and its kin resolve there and nowhere the worker can reach.
    client
        .get_dbt_warehouse(warehouse)
        .await
        .map_err(|e| Error::BadRequest(format!("resolving the dbt warehouse `{warehouse}`: {e}")))
}

/// Identifies the connection a rendered profile describes, for run identity.
///
/// Two things in the rendered text belong to the ATTEMPT rather than the
/// connection, and hashing either as-is makes a retry reject its own
/// predecessor — it compares identities and finds a different one every time:
///
/// * the per-job profiles dir, spelled out when a private CA is configured
///   (`sslrootcert`). The certificate is part of the connection, so it is
///   hashed in place of its path.
/// * the job's own token, where the warehouse resource interpolates `$WM_TOKEN`
///   (a warehouse reached through an OIDC or on-behalf flow does). Every
///   attempt is a new job with a new token.
fn profile_identity_digest(
    yaml: &str,
    profiles_dir: &Path,
    root_cert_pem: Option<&str>,
    job_token: &str,
) -> String {
    let normalized = yaml.replace(profiles_dir.to_str().unwrap_or_default(), "$PROFILES_DIR");
    let normalized = if job_token.is_empty() {
        normalized
    } else {
        normalized.replace(job_token, "$WM_TOKEN")
    };
    digest(&format!(
        "{}\n{}",
        normalized,
        root_cert_pem.unwrap_or_default()
    ))
}

async fn adapter_from_profiles_yml(
    path: &Path,
    profile_name: &str,
    target: Option<&str>,
) -> error::Result<ProfileTarget> {
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
    // dbt renders the profile through Jinja before reading it; Windmill does not,
    // so a templated `target:` — `{{ env_var('DBT_TARGET', 'prod') }}`, which is
    // how a repo carries one profile across environments — is a name no output
    // has. Where the profile defines exactly one output there is nothing to
    // choose and it is taken; otherwise the descriptor has to say which.
    let templated_target = target.contains("{{");
    let out = if templated_target {
        let only = outputs
            .as_mapping()
            .filter(|m| m.len() == 1)
            .and_then(|m| m.values().next());
        only.ok_or_else(|| {
            Error::BadRequest(format!(
                "{} selects its target with a template (`{target}`), which dbt renders and \
                 Windmill does not, and defines several outputs. Set `profile.target` in the \
                 descriptor to say which one this script runs",
                path.display()
            ))
        })?
    } else {
        outputs.get(target).ok_or_else(|| {
            Error::BadRequest(format!(
                "{} has no `{target}` target under `{profile_name}`",
                path.display()
            ))
        })?
    };
    let declared_type = out
        .get("type")
        .and_then(|t| t.as_str())
        .filter(|t| !t.contains("{{"));
    let adapter = match declared_type {
        Some(t) => DbtAdapter::from_dbt_type(t)?,
        // REFUSED, not guessed. dbt renders the template and Windmill does not,
        // so the descriptor's word is the only thing left — and it is worth
        // nothing here: `profile.type: postgres` over a target resolving to
        // `sqlserver` would pass the CE check while dbt runs the licensed
        // adapter it bundles. A `target` may be templated (it only picks an
        // output); the `type` inside that output may not.
        None => {
            return Err(Error::BadRequest(format!(
                "{} does not state its adapter as a literal `type` — templated, or absent. \
                 Windmill cannot resolve it, and the adapter decides both the engine and the \
                 licence, so spell it literally in the target",
                path.display()
            )))
        }
    };
    // The target's own database and schema, read with the same keys the renderer
    // writes. A project that owns its profile then spells its `dbt://` paths
    // identically to one on a workspace warehouse, which is what lets the two
    // meet on the same node when they are on the same relation.
    let (database_key, schema_key) = adapter.target_identity_keys();
    let read = |k: &str| {
        out.get(k)
            .and_then(|v| v.as_str())
            .map(|v| v.to_string())
            .filter(|v| !v.is_empty() && !v.contains("{{"))
    };
    Ok(ProfileTarget { adapter, database: read(database_key), schema: read(schema_key) })
}

/// What a project-owned `profiles.yml` target says, for the two things Windmill
/// needs from a file it did not write: which adapter to provision, and how to
/// spell the relations it will produce.
#[derive(Debug)]
struct ProfileTarget {
    adapter: DbtAdapter,
    database: Option<String>,
    schema: Option<String>,
}

lazy_static::lazy_static! {
    /// `{{ env_var('NAME') }}` / `{{ env_var("NAME", "default") }}`.
    static ref ENV_VAR_CALL: regex::Regex = regex::Regex::new(
        r#"\{\{\s*env_var\(\s*['"]([^'"]+)['"]\s*(?:,\s*['"]([^'"]*)['"]\s*)?\)\s*\}\}"#
    )
    .unwrap();
}

/// dbt renders `env_var()` in `dbt_project.yml` as well, so a setting Windmill
/// reads out of that file has to be rendered against the environment the run
/// hands dbt. Left as written, Windmill acts on the template and dbt acts on the
/// value, and the two never name the same profile or the same directory.
///
/// Only `env_var` is rendered — the one Jinja call dbt documents for this file.
/// An expression that resolves to nothing is left verbatim so dbt reports it.
fn render_env_vars(value: &str, env: &HashMap<String, String>) -> String {
    ENV_VAR_CALL
        .replace_all(value, |caps: &regex::Captures| {
            env.get(&caps[1])
                .cloned()
                .or_else(|| caps.get(2).map(|d| d.as_str().to_string()))
                .unwrap_or_else(|| caps[0].to_string())
        })
        .into_owned()
}

/// dbt takes the profile to use from `dbt_project.yml`, so a rendered
/// `profiles.yml` has to answer to that name rather than one of our choosing.
async fn project_profile_name(project_dir: &Path, env: &HashMap<String, String>) -> String {
    let Ok(content) = tokio::fs::read_to_string(project_dir.join("dbt_project.yml")).await else {
        return FALLBACK_PROFILE_NAME.to_string();
    };
    serde_yml::from_str::<serde_yml::Value>(&content)
        .ok()
        .and_then(|v| {
            v.get("profile")
                .and_then(|p| p.as_str())
                .map(|s| render_env_vars(s, env))
        })
        .unwrap_or_else(|| FALLBACK_PROFILE_NAME.to_string())
}

/// Where the project has `dbt deps` install its packages, defaulting to dbt's
/// own `dbt_packages`.
///
/// REFUSED rather than replaced when it escapes the project: dbt reads
/// `dbt_project.yml` itself, so substituting a safe path here would only move
/// Windmill's cache — dbt would still install to the escaping one, writing
/// outside the job directory and leaving the cache watching a directory nothing
/// fills.
async fn packages_install_path(
    project_dir: &Path,
    env: &HashMap<String, String>,
) -> error::Result<String> {
    const DEFAULT: &str = "dbt_packages";
    let Ok(content) = tokio::fs::read_to_string(project_dir.join("dbt_project.yml")).await else {
        return Ok(DEFAULT.to_string());
    };
    let declared = serde_yml::from_str::<serde_yml::Value>(&content)
        .ok()
        .and_then(|v| {
            v.get("packages-install-path")
                .and_then(|p| p.as_str())
                .map(|s| {
                    render_env_vars(s, env)
                        .trim()
                        .trim_start_matches("./")
                        .to_string()
                })
        })
        .filter(|s| !s.is_empty());
    let Some(declared) = declared else {
        return Ok(DEFAULT.to_string());
    };
    if !Path::new(&declared)
        .components()
        .all(|c| matches!(c, std::path::Component::Normal(_)))
    {
        return Err(Error::BadRequest(format!(
            "`packages-install-path: {declared}` in dbt_project.yml points outside the \
             project; it must be a path within it, such as `dbt_packages`"
        )));
    }
    Ok(declared)
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
///
/// The invocation's environment must NOT reach the launcher under a sandbox:
/// what this returns is the process that execs nsjail, and those values come
/// from caller-controlled script metadata — an `LD_PRELOAD` naming a library
/// from the project bundle would be loaded by the dynamic linker as the worker,
/// before isolation exists. The jail profile carries them to the child instead
/// (see `sandbox_config`).
pub(crate) fn dbt_command(p: &PreparedProject, args: &[&str]) -> Command {
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
    // Both environments belong to the child. Under a sandbox they reach it through
    // the jail profile instead: set here, they would reach the dynamic loader that
    // execs nsjail itself, so an `LD_PRELOAD` from the project would run as the
    // worker, before any isolation exists.
    if p.sandbox_config.is_none() {
        cmd.envs(p.env.iter().map(|(k, v)| (k.as_str(), v.as_str())));
        cmd.envs(
            p.invocation_env
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str())),
        );
    }
    cmd.args(args)
        .arg("--profiles-dir")
        .arg(&p.profiles_dir)
        // Every artifact this runtime reads is found by path, so the location is
        // Windmill's to decide and not the project's `target-path`. An env var
        // because `dbt deps` rejects `--target-path`, and set last so neither
        // environment above can displace it.
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

/// Take one attempt from the job's budget, returning its 1-based number, or
/// `None` when nothing is left.
///
/// Claiming and counting are one operation because they are the retry loop's
/// only bound: separating them is how the budget stops being spent and the loop
/// reissues `dbt retry` until the job's deadline instead of `attempts` times.
/// Claimed before the guards that can decline an attempt, which is harmless —
/// each of those breaks out rather than looping again.
fn claim_attempt(remaining: &mut u32, total: u32) -> Option<u32> {
    if *remaining == 0 {
        return None;
    }
    let attempt = total - *remaining + 1;
    *remaining -= 1;
    Some(attempt)
}

/// The descriptor's `retry_failed_nodes` policy, applied to whichever phase just
/// failed. `dbt retry` rebuilds only the failed and skipped nodes, so a
/// transient warehouse error costs those rather than the whole project.
///
/// Called after the model phase and again after the `after_all` test phase: a
/// failing test is a failed node too. `remaining` is the budget for the WHOLE
/// job, not per phase — each attempt is a real dbt invocation holding a worker
/// slot, so a model phase that spent them all leaves none for the tests.
#[allow(clippy::too_many_arguments)]
async fn retry_failed_nodes(
    policy: windmill_parser_yaml::dbt::DbtNodeRetry,
    prepared: &PreparedProject,
    descriptor: &DbtDescriptor,
    inv: &Invocation,
    job: &MiniPulledJob,
    conn: &Connection,
    ctx: &mut JobCtx<'_>,
    run: &mut error::Result<()>,
    results: &mut Vec<DbtNodeResult>,
    remaining: &mut u32,
) {
    let total = policy.attempts();
    while let Some(attempt) = claim_attempt(remaining, total) {
        // A cancelled or timed-out job must not start another warehouse
        // write: its failure is not the transient kind this retries, and
        // the slot is supposed to be going away. The wait itself re-checks,
        // since a cancel most likely arrives during it.
        if !current_results_are_retryable(prepared).await {
            break;
        }
        if !sleep_before_retry(policy.delay_seconds, &job.id, conn, ctx.deadline).await {
            break;
        }
        append_logs(
            &job.id,
            &job.workspace_id,
            format!("\nRetrying the nodes that failed (attempt {attempt} of {total})\n"),
            conn,
        )
        .await;
        *run = run_dbt(
            prepared, "retry", descriptor, inv, job, conn, &mut *ctx,
            // `dbt retry` reuses the previous invocation's selection; adding
            // one would narrow what it resumes.
            false,
        )
        .await;
        // A retry's `run_results.json` describes only the nodes it redid, so
        // it OVERLAYS the previous attempt's rather than replacing it. The
        // job's result has to be every node this job touched.
        merge_results(results, read_run_results(&prepared.project_dir).await);
        if run.is_ok() {
            break;
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_dbt(
    p: &PreparedProject,
    command: &str,
    descriptor: &DbtDescriptor,
    inv: &Invocation,
    job: &MiniPulledJob,
    conn: &Connection,
    ctx: &mut JobCtx<'_>,
    with_selection: bool,
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
        ctx.mem_peak,
        ctx.canceled_by,
        child,
        false,
        ctx.worker_name,
        &job.workspace_id,
        &format!("dbt {command}"),
        // What is left of the job's wall clock: `dbt build` follows the whole
        // preparation sequence, and the `after_all` tests follow it.
        ctx.timeout(),
        false,
        &mut Some(&mut *ctx.occupancy_metrics),
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
        // Agent workers reach the DB only through the API, and tailing a log to
        // POST every event would spend a request per node. Their per-model state
        // is settled from `run_results.json` when the run ends instead: recorded
        // in full, but arriving at the end rather than live.
        return None;
    };
    if !p.engine.engine.emits_node_events() {
        // Nothing to read: those engines write a text file log, so tailing it
        // would burn a task per run for no events.
        return None;
    }
    let (db, w_id, job_id) = (db.clone(), job.workspace_id.clone(), job.id);
    let warehouse = p.warehouse.clone()?;
    let default_database = p.default_database.clone();
    Some(tokio::spawn(async move {
        use tokio::io::{AsyncReadExt, AsyncSeekExt};
        let mut offset = 0u64;
        let mut tail = LogTail::default();
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
                tail = LogTail::default();
            }
            if len == offset {
                continue;
            }
            // Seek rather than re-read: a long run's log grows without bound
            // and reading it whole every tick is quadratic in its size.
            if f.seek(std::io::SeekFrom::Start(offset)).await.is_err() {
                continue;
            }
            // BOUNDED, and heap-allocated: this reader runs in the worker
            // process, outside the jailed child's memory limit, so reading a
            // whole tail a macro can make arbitrarily large would take down
            // every job on the worker. What is left waits for the next tick.
            let mut buf = Vec::new();
            if (&mut f)
                .take(LOG_TICK_MAX_BYTES)
                .read_to_end(&mut buf)
                .await
                .is_err()
            {
                continue;
            }
            offset += buf.len() as u64;
            let chunk = tail.push(&String::from_utf8_lossy(&buf));
            for line in chunk.lines() {
                let Some(ev) = parse_node_event(line, &warehouse, default_database.as_deref())
                else {
                    continue;
                };
                windmill_common::dbt_manifest::record_run_progress(
                    &db,
                    &w_id,
                    &job_id,
                    &ev.asset_path,
                    ev.status,
                    ev.row_count,
                    ev.error.as_deref(),
                )
                .await;
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

/// How much of dbt's log one tick of the tailer takes, and how long one event
/// may be. Both bound what the WORKER process holds: the reader lives there,
/// not in the jailed child, and a macro can print anything into a log line.
const LOG_TICK_MAX_BYTES: u64 = 1 << 20;
const LOG_LINE_MAX_BYTES: usize = 256 * 1024;

/// Complete lines out of a log read in chunks.
///
/// A tick can land mid-write, leaving a trailing partial line; it is held over
/// rather than dropping the event it belongs to — unless it grows past
/// `LOG_LINE_MAX_BYTES`, which no node event does, and then it is discarded
/// through its next newline so the events after it still arrive.
#[derive(Default)]
struct LogTail {
    carry: String,
    /// Inside an over-long line: everything up to its end is dropped.
    skipping: bool,
}

impl LogTail {
    fn push(&mut self, chunk: &str) -> String {
        if self.skipping {
            match chunk.find('\n') {
                Some(i) => {
                    self.skipping = false;
                    self.carry.push_str(&chunk[i + 1..]);
                }
                None => return String::new(),
            }
        } else {
            // Appended BEFORE looking for the last newline: a line spanning
            // three reads would otherwise have its middle fragment replace the
            // first, and the reassembled line would be invalid JSON.
            self.carry.push_str(chunk);
        }
        let complete_upto = self.carry.rfind('\n').map(|i| i + 1).unwrap_or(0);
        let complete = self.carry[..complete_upto].to_string();
        self.carry.drain(..complete_upto);
        if self.carry.len() > LOG_LINE_MAX_BYTES {
            self.carry.clear();
            self.skipping = true;
        }
        complete
    }
}

/// One `node_info`-carrying dbt log event turned into the materialization
/// record the asset graph reads. `None` for events that are not per-node, and
/// for nodes with no physical relation (tests, ephemeral models).
fn parse_node_event(
    line: &str,
    warehouse: &str,
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
    let status = match classify_status(info.get("node_status")?.as_str()?) {
        DbtNodeOutcome::Started => MaterializationStatus::Running,
        DbtNodeOutcome::Passed => MaterializationStatus::Materialized,
        DbtNodeOutcome::Failed => MaterializationStatus::Failed,
        // `warn` is a passing test at reduced severity, and a node that never
        // built says nothing about the relation's state; none is a
        // materialization.
        DbtNodeOutcome::Warn
        | DbtNodeOutcome::Skipped
        | DbtNodeOutcome::NoOp
        | DbtNodeOutcome::Unknown => return None,
    };
    // `None` when the derived key would not fit `asset.path`; the node keeps its
    // manifest row and this run records no materialization for it, rather than
    // one swallowed `value too long` per node per run.
    let path = windmill_common::dbt_manifest::table_asset_path(
        warehouse,
        database,
        schema,
        alias,
        default_database,
    )?;
    Some(RecordMaterializationRequest {
        asset_kind: windmill_common::assets::AssetKind::Dbt,
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
///
/// Returns the relations it settled, and the ones the run reported but left
/// untouched (`no-op`, `warn`, `skipped`). The two need opposite treatment
/// afterwards, which is why they come back apart.
async fn reconcile_materializations(
    p: &PreparedProject,
    results: &[DbtNodeResult],
    job: &MiniPulledJob,
    conn: &Connection,
    client: &AuthedClient,
) -> Reconciled {
    let mut out = Reconciled::default();
    // One batch for an agent worker's per-model state; empty on a Sql worker,
    // which writes each row directly.
    let mut progress: Vec<windmill_common::dbt_manifest::DbtRunProgressRequest> = vec![];
    let Some(warehouse) = p.warehouse.as_deref() else {
        return out;
    };
    for r in results {
        let Some(path) = asset_path_of_relation(
            r.relation_name.as_deref(),
            warehouse,
            p.default_database.as_deref(),
        ) else {
            continue;
        };
        let status = match classify_status(&r.status) {
            DbtNodeOutcome::Passed => MaterializationStatus::Materialized,
            DbtNodeOutcome::Failed => MaterializationStatus::Failed,
            // Tests and nodes that built nothing say nothing about a relation's
            // state — but the tailer may already have written `running` for one,
            // so they are reported for the caller to clear rather than settle.
            _ => {
                out.untouched.push(path);
                continue;
            }
        };
        out.settled.push(path.clone());
        let error = (status == MaterializationStatus::Failed)
            .then(|| r.message.as_deref())
            .flatten();
        // An agent worker has no direct DB, so its outcomes go through the API —
        // otherwise a successful agent run leaves every model with no recorded
        // status or row count.
        let recorded = match conn {
            Connection::Sql(db) => {
                windmill_common::dbt_manifest::record_run_progress(
                    db,
                    &job.workspace_id,
                    &job.id,
                    &path,
                    status,
                    r.rows_affected,
                    error,
                )
                .await;
                record_materialization(
                    db,
                    &job.workspace_id,
                    windmill_common::assets::AssetKind::Dbt,
                    &path,
                    windmill_common::materialization::UNPARTITIONED,
                    status,
                    None,
                    r.rows_affected,
                    Some(job.id),
                    error,
                )
                .await
                .map_err(|e| e.to_string())
            }
            Connection::Http(http) => {
                // Progress too, not only the materialization: the live reporter
                // needs a database and does not run here, so these settled
                // outcomes are the only per-model state an agent's run page ever
                // gets. COLLECTED, not sent: one round trip per model would run
                // after dbt has already finished, and a large project has
                // hundreds. Posted once below.
                progress.push(windmill_common::dbt_manifest::DbtRunProgressRequest {
                    asset_path: path.clone(),
                    status,
                    row_count: r.rows_affected,
                    error: error.map(|e| e.to_string()),
                });
                crate::agent_workers::record_materialization_from_agent_http(
                    http,
                    &job.workspace_id,
                    &RecordMaterializationRequest {
                        asset_kind: windmill_common::assets::AssetKind::Dbt,
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
                .map_err(|e| e.to_string())
            }
        };
        if let Err(e) = recorded {
            tracing::warn!("recording the materialization of {path} failed: {e}");
        }
    }
    if !progress.is_empty() {
        if let Err(e) = client.record_dbt_run_progress(&progress).await {
            // A display, not the run: the models are built either way.
            tracing::warn!(
                "recording dbt run progress for {} nodes: {e:#}",
                progress.len()
            );
        }
    }
    out
}

/// What `reconcile_materializations` did, split by what has to happen next.
#[derive(Default)]
struct Reconciled {
    /// Given a terminal status by this run.
    settled: Vec<String>,
    /// Reported by this run but not built — `no-op`, `warn`, `skipped`.
    untouched: Vec<String>,
}

/// Leave no relation of this job's on `running`.
///
/// The live tailer writes `running` when a model starts, and two things can
/// leave it there. A node that ends `no-op`, `warn` or `skipped` built nothing,
/// so there is no outcome to record — its row is DELETED, which is what the
/// finished run's own result says about it too (`relationOutcome` colours it
/// nothing), so the live view and the settled view agree. And a cancellation or
/// a timeout means dbt never wrote `run_results.json` for the node in flight, so
/// it is reported nowhere — that one is FAILED, because the run ended without
/// finishing it.
///
/// A killed worker reaches none of this and leaves its rows `running` in both
/// tables until the retention sweep takes them; recovering those belongs to
/// whatever reclaims the job, not here.
///
/// Only the SQL path can strand a row: `spawn_progress_reporter` returns `None`
/// for an agent worker and for the engines that emit no node events, so nothing
/// there writes `running` in the first place.
async fn terminalize_running_relations(
    job: &MiniPulledJob,
    reconciled: &Reconciled,
    conn: &Connection,
) {
    let Connection::Sql(db) = conn else {
        return;
    };
    if let Err(e) = sqlx::query!(
        "DELETE FROM materialized_partition
          WHERE workspace_id = $1 AND job_id = $2 AND status = 'running'
            AND asset_path = ANY($3)",
        job.workspace_id,
        job.id,
        &reconciled.untouched
    )
    .execute(db)
    .await
    {
        tracing::warn!("clearing the models {} left untouched: {e}", job.id);
    }
    let accounted: Vec<String> = reconciled
        .settled
        .iter()
        .chain(reconciled.untouched.iter())
        .cloned()
        .collect();
    if let Err(e) = sqlx::query!(
        "UPDATE materialized_partition
            SET status = 'failed',
                error = COALESCE(error, 'the run ended before this model finished')
          WHERE workspace_id = $1 AND job_id = $2 AND status = 'running'
            AND NOT (asset_path = ANY($3))",
        job.workspace_id,
        job.id,
        &accounted
    )
    .execute(db)
    .await
    {
        tracing::warn!("settling the models left running by {}: {e}", job.id);
    }
    // The run page reads `dbt_run_progress`, so both closing writes have to land
    // there too. Without them a cancelled or killed run — which leaves no
    // `run_results.json`, so the page falls back to this poll — shows every
    // in-flight model still spinning for as long as the row is retained.
    if let Err(e) = sqlx::query!(
        "DELETE FROM dbt_run_progress
          WHERE workspace_id = $1 AND job_id = $2 AND status = 'running'
            AND asset_path = ANY($3)",
        job.workspace_id,
        job.id,
        &reconciled.untouched
    )
    .execute(db)
    .await
    {
        tracing::warn!("clearing the progress rows {} left untouched: {e}", job.id);
    }
    if let Err(e) = sqlx::query!(
        "UPDATE dbt_run_progress
            SET status = 'failed',
                error = COALESCE(error, 'the run ended before this model finished')
          WHERE workspace_id = $1 AND job_id = $2 AND status = 'running'
            AND NOT (asset_path = ANY($3))",
        job.workspace_id,
        job.id,
        &accounted
    )
    .execute(db)
    .await
    {
        tracing::warn!("settling the progress rows left running by {}: {e}", job.id);
    }
}

/// `"db"."schema"."name"` from dbt into the `dbt://` path of the relation,
/// through the same derivation the manifest ingest and the live events use.
fn asset_path_of_relation(
    relation_name: Option<&str>,
    warehouse: &str,
    default_database: Option<&str>,
) -> Option<String> {
    let parts = split_relation(relation_name?);
    let (database, schema, name) = match parts.as_slice() {
        [db, schema, name] => (Some(db.as_str()), schema.as_str(), name.as_str()),
        [schema, name] => (None, schema.as_str(), name.as_str()),
        _ => return None,
    };
    windmill_common::dbt_manifest::table_asset_path(
        warehouse,
        database,
        schema,
        name,
        default_database,
    )
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
    let mut chars = rel.chars().peekable();
    while let Some(c) = chars.next() {
        match quote {
            Some(q) => {
                let close = if q == '[' { ']' } else { q };
                if c == close {
                    // Doubled is how these dialects escape their delimiter: one
                    // literal character, not the end of the identifier. Dropping
                    // it renames the relation, so the run records progress
                    // against a key no node has.
                    if chars.peek() == Some(&close) {
                        chars.next();
                        current.push(close);
                    } else {
                        quote = None;
                    }
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

/// Ceiling on a preview's captured output, enforced as `run_capturing` reads.
/// `--limit` bounds how many rows dbt returns, not how big they are — a single
/// column can hold a megabyte — so the row clamp is not a memory bound on its
/// own. This also bounds what a preview can store in `v2_job_completed.result`.
const SHOW_MAX_OUTPUT_BYTES: usize = 8 * 1024 * 1024;

/// Ceiling on `dbt ls`, whose output is one line per selected node. Generous
/// against the largest real projects, and there only so a runaway cannot be
/// unbounded.
const LS_MAX_OUTPUT_BYTES: usize = 8 * 1024 * 1024;

/// The `--limit` a `show` runs with, from the run's argument.
///
/// Clamped, not merely defaulted: the rows are read out of dbt's stdout, so this
/// argument decides how much a caller can make the worker hold — and running a
/// script needs only run permission. `SHOW_MAX_OUTPUT_BYTES` is the backstop for
/// rows that are individually large; this keeps an ordinary preview from
/// reaching it. Zero and negatives fall back to the default rather than reaching
/// dbt, where `--limit 0` means something else.
/// Whether a `show` selector can name at most one node, syntactically.
///
/// dbt splits a selector on WHITESPACE into a union and applies commas within
/// each member, so the `,resource_type:model` intersection `run_show` appends
/// binds to the last member only. More than one member therefore leaves the
/// others unconstrained — and a seed among them is dispatched through dbt's
/// seed runner, which writes its relation from the one command this runtime
/// calls read-only.
///
/// Graph operators and wildcards are refused for the plainer reason: they
/// resolve to a set, and `show` previews one relation.
fn show_selects_one_node(model: &str) -> bool {
    model.split_whitespace().count() == 1 && !model.contains(['+', '*', '@'])
}

fn show_limit(requested: Option<i64>) -> i64 {
    let max = windmill_parser_yaml::dbt::DBT_SHOW_MAX_LIMIT as i64;
    requested
        .filter(|l| *l > 0)
        .map(|l| l.min(max))
        .unwrap_or(windmill_parser_yaml::dbt::DBT_SHOW_DEFAULT_LIMIT as i64)
}

/// `dbt show`: SELECT from the selected node and return its rows. dbt prints one
/// document per selected node and the result is a single preview, so a selection
/// naming several returns the first — which is what the argument's description
/// tells the caller.
///
/// Captured rather than streamed, for the same reason `dbt ls` is: the job-log
/// writer is what `NO_LOGS_AT_ALL` discards, and these rows ARE the result, not
/// commentary about it.
async fn run_show(
    p: &PreparedProject,
    descriptor: &DbtDescriptor,
    inv: &Invocation,
    ctx: &mut JobCtx<'_>,
    job_id: &Uuid,
    w_id: &str,
    conn: &Connection,
) -> error::Result<Box<RawValue>> {
    // ONE node, named by the run. Not `add_selection`: the descriptor's selectors
    // narrow a BUILD, and applying them here would preview whatever they happen
    // to resolve to first. dbt's own failure for an empty selection carries no
    // message at all, so the refusal has to be this one.
    let Some(model) = arg_str(&inv.args, "model")? else {
        return Err(Error::BadRequest(
            "`show` previews one model's rows, so it needs `model` to name one — `stg_orders`, \
             or any dbt selector that resolves to a single MODEL (a seed, test or \
             snapshot is not previewable)"
                .to_string(),
        ));
    };
    let model = model.trim();
    // ONE union member, because the intersection below binds to the LAST one:
    // dbt splits a selector on whitespace into a union and applies commas within
    // each member, so `my_seed safe_model,resource_type:model` still selects the
    // seed — and dbt dispatches a selected SEED through its seed runner, which
    // LOADS it. That writes a relation through a path recording no
    // materialization, no graph and no retry state, from the one command this
    // runtime advertises as read-only. Graph operators are refused for the same
    // reason they are refused above: they resolve to a set, and `show` previews
    // one relation.
    if !show_selects_one_node(model) {
        return Err(Error::BadRequest(format!(
            "`show` previews ONE model's rows, so `model` must name exactly one — got \
             `{model}`. A selector naming several (spaces), or a graph operator (`+`, \
             `@`) or wildcard (`*`), resolves to a set: run `build` with it instead"
        )));
    }
    let mut cmd = dbt_command(p, &["show"]);
    add_vars(&mut cmd, descriptor, inv)?;
    // Intersected with `resource_type:model`, because `show` is only read-only
    // for models: dbt dispatches a selected SEED through its seed runner and
    // loads it, which would write a relation through a path that records no
    // materialization and no graph. A comma is dbt's own intersection, the same
    // one `package:` already uses here, so a seed simply selects nothing. Sound
    // only because the check above left exactly one union member for it to bind.
    cmd.args(["--select", &format!("{model},resource_type:model")]);
    let limit = show_limit(arg_i64(&inv.args, "limit")?);
    cmd.args(["--output", "json", "--limit", &limit.to_string()]);
    let stdout = run_capturing(
        cmd,
        "dbt show",
        ctx,
        job_id,
        w_id,
        conn,
        SHOW_MAX_OUTPUT_BYTES,
    )
    .await?;
    // dbt frames the rows as `{"node": …, "show": [ … ]}`, pretty-printed, with a
    // banner before and a deprecation summary after — so neither "the line starting
    // with `{`" nor "first `{` to the end" parses. A streaming deserializer stops at
    // the first complete document and ignores the rest.
    let mut from = 0;
    while let Some(rel) = stdout[from..].find('{') {
        let at = from + rel;
        let mut docs =
            serde_json::Deserializer::from_str(&stdout[at..]).into_iter::<serde_json::Value>();
        if let Some(Ok(v)) = docs.next() {
            if v.get("show").is_some() {
                return Ok(to_raw_value(&v));
            }
        }
        from = at + 1;
    }
    Err(Error::ExecutionErr(format!(
        "dbt show returned no rows to parse. Output was:\n{}",
        stdout.chars().take(2000).collect::<String>()
    )))
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
            outcome: classify_status(&r.status).as_result_word(),
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

fn build_result(
    p: &PreparedProject,
    command: &str,
    nodes: Vec<DbtNodeResult>,
    inv: &Invocation,
) -> DbtRunResult {
    let mut totals = DbtTotals { total: nodes.len(), ..Default::default() };
    for n in &nodes {
        match classify_status(&n.status) {
            DbtNodeOutcome::Passed => totals.success += 1,
            DbtNodeOutcome::Warn => totals.warn += 1,
            DbtNodeOutcome::Skipped | DbtNodeOutcome::NoOp => totals.skipped += 1,
            _ => totals.error += 1,
        }
    }
    DbtRunResult {
        engine: p.engine.engine.as_str().to_string(),
        engine_version: p.engine.version.clone(),
        command: command.to_string(),
        totals,
        nodes,
        invocation_args: inv.raw_args.clone(),
    }
}

fn render_failures(r: &DbtRunResult) -> String {
    let failed: Vec<&DbtNodeResult> = r
        .nodes
        .iter()
        .filter(|n| {
            matches!(
                classify_status(&n.status),
                DbtNodeOutcome::Failed | DbtNodeOutcome::Warn | DbtNodeOutcome::Unknown
            )
        })
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

/// What a `parse` returns. Deliberately not a `DbtRunResult`: nothing ran, so
/// there are no per-node outcomes to report and a result shaped like a build's
/// would invite a caller to read totals that describe nothing.
#[derive(Serialize, Debug)]
pub struct DbtParseResult {
    pub engine: String,
    pub engine_version: String,
    pub command: &'static str,
    /// The workspace warehouse the relations are keyed on. Absent for a project
    /// that brings its own `profiles.yml` and names none, which is exactly the
    /// case that stores no graph.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warehouse: Option<String>,
    pub nodes: usize,
    pub edges: usize,
    /// Nodes by dbt `resource_type` (`model`, `source`, `test`, `seed`,
    /// `snapshot`, …). A map rather than a field each, so a resource type dbt
    /// adds later is reported instead of silently dropped.
    pub by_resource_type: std::collections::BTreeMap<String, usize>,
    /// The job to read this graph back through, with
    /// `GET /jobs/dbt_graph/{id}`. Its own, always — a version-less parse is
    /// reachable no other way, and a versioned one resolves to its snapshot or
    /// falls back to the version's graph, which is what it agreed with.
    ///
    /// Present once the write was ACCEPTED, which is the half a caller can act
    /// on; absent when there was nothing to store (no warehouse identity to key
    /// relations on) or nothing to store it against (a deleted script).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph_job: Option<Uuid>,
}

/// `dbt parse` over this job's project, stored as a graph and nothing else.
///
/// Three provenances end up here, and they differ only in what the graph is
/// keyed to:
///
/// * a PREVIEW job — the editor refreshing its buffer — keys it to the job
///   alone, with no version. Those rows are readable only back through that job
///   id, never through the path, so a caller who needs no more than `jobs:run`
///   cannot restate what a deployed project's graph says.
/// * a job that names a deployed VERSION stores an ordinary per-run snapshot of
///   it, suppressed when it matches what the deploy stored.
/// * an agent worker posts either to the API, which decides the same way from
///   the job it verified.
///
/// None of them publishes the path-keyed `asset` usages: a parse is a question
/// about a project, and answering it must not move what the script owns.
async fn run_parse_only(
    p: &PreparedProject,
    descriptor: &DbtDescriptor,
    inv: &Invocation,
    ctx: &mut JobCtx<'_>,
    job: &MiniPulledJob,
    conn: &Connection,
) -> error::Result<Box<RawValue>> {
    run_dbt_parse(p, descriptor, inv, ctx, &job.id, &job.workspace_id, conn).await?;
    let manifest = read_manifest(&p.project_dir).await?;
    let selected =
        resolve_selection(p, descriptor, inv, ctx, &job.id, &job.workspace_id, conn).await?;
    let mut result = DbtParseResult {
        engine: p.engine.engine.as_str().to_string(),
        engine_version: p.engine.version.clone(),
        command: "parse",
        warehouse: p.warehouse.clone(),
        nodes: 0,
        edges: 0,
        by_resource_type: Default::default(),
        graph_job: None,
    };
    // Counted before the guard, because the node and edge SETS come from the
    // manifest and the selection while the warehouse only keys them — so a project
    // with no warehouse identity still reports what dbt found. The placeholder
    // reaches no row: the guard below returns before anything is written.
    let ingested = windmill_common::dbt_manifest::ingest_manifest(
        &manifest,
        p.warehouse.as_deref().unwrap_or("unkeyed"),
        p.default_database.as_deref(),
        selected.as_ref(),
    );
    result.nodes = ingested.nodes.len();
    result.edges = ingested.edges.len();
    for n in &ingested.nodes {
        *result
            .by_resource_type
            .entry(n.resource_type.clone())
            .or_default() += 1;
    }
    // No warehouse identity means no `dbt://` key to store the relations under, so
    // the parse reports what it found and stores nothing.
    let (Some(_), Some(script_path)) = (p.warehouse.as_deref(), job.runnable_path.as_deref())
    else {
        return Ok(to_raw_value(&result));
    };
    match conn {
        Connection::Sql(db) => match job.runnable_id.map(|h| h.0) {
            Some(script_hash) => {
                let stored = persist_ingest(
                    db,
                    &job.workspace_id,
                    script_path,
                    &ingested,
                    &p.relation_root(),
                    GraphPublisher::Version(script_hash),
                    Some(job.id),
                    // A parse answers for the arguments IT was given, so it can
                    // no more stand as what the script owns than an overriding
                    // run can. Ownership stays the deploy's.
                    false,
                )
                .await?;
                result.graph_job = stored.then_some(job.id);
            }
            None => {
                let mut tx = db.begin().await?;
                windmill_common::dbt_manifest::replace_dbt_editor_graph(
                    &mut tx,
                    &job.workspace_id,
                    script_path,
                    job.id,
                    &job.permissioned_as,
                    &ingested,
                    &p.relation_root(),
                )
                .await?;
                tx.commit().await?;
                result.graph_job = Some(job.id);
            }
        },
        Connection::Http(client) => {
            client
                .post::<_, serde_json::Value>(
                    &format!("/api/agent_workers/dbt_graph/{}", job.workspace_id),
                    None,
                    // `per_run`, because a parse never writes the version's own
                    // graph: it is a question about one invocation's project.
                    &serde_json::json!({
                        "job_id": job.id,
                        "per_run": true,
                        "relation_root": p.relation_root(),
                        "manifest": ingested,
                    }),
                )
                .await
                .map_err(|e| {
                    Error::internal_err(format!("publishing dbt graph from an agent worker: {e:#}"))
                })?;
            result.graph_job = Some(job.id);
        }
    }
    Ok(to_raw_value(&result))
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
    let Some(warehouse) = p.warehouse.as_deref() else {
        return Ok(());
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
        warehouse,
        p.default_database.as_deref(),
        selected.as_ref(),
    );
    // Only a run whose models are its own snapshots per run. A static
    // descriptor at a moved profile re-ingests the VERSION's graph, since the
    // move outlives the run; one that neither drifted nor overrode anything
    // re-ingests the same graph its deploy wrote and stores nothing.
    let snapshot_job = p.graph_refresh.snapshot_job(job.id);
    match conn {
        Connection::Sql(db) => {
            persist_ingest(
                db,
                &job.workspace_id,
                script_path,
                &ingested,
                &p.relation_root(),
                job.runnable_id
                    .map(|h| GraphPublisher::Version(h.0))
                    .unwrap_or(GraphPublisher::Unversioned),
                snapshot_job,
                p.graph_refresh.publishes_ownership(),
            )
            .await?;
        }
        // An agent worker reaches these tables only through the API. Publishing
        // is the whole of what it needs: a worker that can replace the graph
        // never has to establish that the stored one still describes its
        // profile, because it stores what it just parsed.
        Connection::Http(client) => {
            client
                .post::<_, serde_json::Value>(
                    &format!("/api/agent_workers/dbt_graph/{}", job.workspace_id),
                    None,
                    // Only the job id: the server reads the path and version
                    // from the job it verified, so this cannot name another
                    // script's graph.
                    &serde_json::json!({
                        "job_id": job.id,
                        "per_run": snapshot_job.is_some(),
                        "relation_root": p.relation_root(),
                        "manifest": ingested,
                    }),
                )
                .await
                .map_err(|e| {
                    Error::internal_err(format!("publishing dbt graph from an agent worker: {e:#}"))
                })?;
        }
    }
    // Synchronously, not through the notify poller: the ingest just rewrote this
    // script's `asset` rows and the poll is seconds away, so anything in this
    // process reading the producer map meanwhile would see the pre-refresh copy.
    // The `notify_event` the transaction emitted still reaches every other process.
    windmill_queue::asset_dispatch::ASSET_PRODUCER_WRITES_CACHE.remove(&job.workspace_id);
    Ok(())
}

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
/// Write one ingest: the sidecar rows and the `asset` usages the manifest
/// implies. No subscriptions — a `dbt://` one could never fire.
///
/// Returns whether this job was still the one entitled to the path-keyed half —
/// false once a newer version has superseded it, or once the version is gone.
async fn persist_ingest(
    db: &sqlx::Pool<sqlx::Postgres>,
    w_id: &str,
    script_path: &str,
    ingested: &windmill_common::dbt_manifest::IngestedManifest,
    relation_root: &str,
    publisher: GraphPublisher,
    // Set for a RUN whose model set depends on its own arguments: that graph is
    // a snapshot of this run and must not overwrite what another run of the same
    // version saw. A deploy passes `None` and writes the version's own graph, as
    // does a run that found the profile moved — that move is the version's.
    run_snapshot: Option<uuid::Uuid>,
    // Whether this ingest also becomes what the SCRIPT owns: the path-keyed
    // `asset` usages, and the relation root the drift check reads back. False
    // for a graph the caller's own arguments scoped (`GraphRefresh`).
    publish_ownership: bool,
) -> error::Result<bool> {
    let GraphPublisher::Version(script_hash) = publisher else {
        // No version to attribute the graph to — an inline or preview run, which
        // deploys nothing and must not touch what a deploy wrote.
        return Ok(false);
    };
    let mut tx = db.begin().await?;
    // Deletion only soft-updates `script`, so re-inserting would republish model
    // SQL a user deleted. NOT `archived`: every redeploy archives the parent, and
    // treating that as deletion would deny v1 the graph its own runs render. An
    // explicit archive still waits on `FOR UPDATE` and clears afterwards.
    let deleted = sqlx::query_scalar!(
        "SELECT deleted FROM script WHERE workspace_id = $1 AND hash = $2 FOR UPDATE",
        w_id,
        script_hash,
    )
    .fetch_optional(&mut *tx)
    .await?
    .unwrap_or(true);
    if deleted {
        return Ok(false);
    }
    // Written without regard to NEWER versions: the rows are keyed by this one, so
    // an older deploy finishing late overwrites nothing and its own runs still
    // render.
    windmill_common::dbt_manifest::replace_dbt_manifest(
        &mut tx,
        w_id,
        script_path,
        script_hash,
        run_snapshot,
        ingested,
        relation_root,
    )
    .await?;
    // Beside the graph just written, not the ownership below: this is the root the
    // STORED VERSION GRAPH describes. Hung off publication, a version that cannot
    // claim the path records nothing and compares against a stale root forever.
    // Only the DEPLOYED row: a run's own snapshot left its root as it was.
    if run_snapshot.is_none() {
        sqlx::query!(
            "UPDATE dbt_graph_snapshot SET relation_root_at_last_ingest = $4
          WHERE workspace_id = $1 AND script_path = $2 AND script_hash = $3
            AND job_id = '00000000-0000-0000-0000-000000000000'",
            w_id,
            script_path,
            script_hash,
            relation_root,
        )
        .execute(&mut *tx)
        .await?;
    }
    // What follows is keyed by PATH, so it belongs to the newest version alone: an
    // older deploy, or a run whose `vars`/`select` describe one invocation, stops
    // here. Publishing either as ownership would leave the workspace graph wrong
    // until the next deploy, since a static descriptor never ingests again.
    if !publish_ownership {
        tx.commit().await?;
        return Ok(true);
    }
    if !claim_graph_publication(&mut tx, w_id, script_path, publisher).await? {
        tx.commit().await?;
        return Ok(false);
    }
    windmill_common::assets::replace_static_asset_usage(
        &mut tx,
        w_id,
        script_path,
        &ingested.assets,
    )
    .await?;
    // A `dbt://` subscription can never fire, so none are derived from the
    // manifest. The delete stays to clear what earlier versions wrote, which would
    // otherwise keep drawing cascade arrows that wake nothing.
    sqlx::query!(
        "DELETE FROM script_trigger
          WHERE workspace_id = $1 AND runnable_kind = 'script' AND runnable_path = $2
            AND trigger_kind = 'asset' AND trigger_ref LIKE 'dbt://%'",
        w_id,
        script_path,
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(true)
}

/// Serialize publishers for one script path and confirm this job's version is
/// still the newest. Both happen inside the caller's transaction, so a newer
/// publisher either commits before this check sees it, or waits behind it and
/// overwrites afterwards — which is the correct order either way.
///
/// Only the PATH-keyed writes need this. The graph itself is keyed by version,
/// so two deploys of one path write disjoint rows and neither can lose.
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
        "SELECT hash FROM script WHERE workspace_id = $1 AND path = $2 \
           AND deleted = false AND archived = false \
         ORDER BY created_at DESC LIMIT 1",
        w_id,
        script_path
    )
    .fetch_optional(&mut **tx)
    .await?;
    // No live version left means the script was archived or deleted while this
    // job ran, and the deploy path has already cleared its graph. Publishing now
    // would put the asset, provenance and subscription rows back with nothing
    // left to remove them, so a missing row is a refusal, not a free pass.
    Ok(latest.is_some_and(|latest| latest == mine))
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
    // Spelled out rather than `all`, which dbt-core 2.x rejects — and `unit_test`
    // is its own type, so omitting it resolves a unit-test selection to the empty
    // set. All three engines accept every value here.
    for t in ["model", "source", "seed", "snapshot", "test", "unit_test"] {
        cmd.args(["--resource-type", t]);
    }
    cmd.args(["--output", "json", "--quiet"]);
    add_selection(&mut cmd, descriptor, inv)?;
    // Captured directly, not through `handle_child`: its `pipe_stdout` path goes
    // through the job-log writer, which `NO_LOGS_AT_ALL` discards — the selection
    // would resolve to the empty set and the ingest would wipe the script's assets
    // while dbt went on building the descriptor's models.
    let stdout = run_capturing(cmd, "dbt ls", ctx, job_id, w_id, conn, LS_MAX_OUTPUT_BYTES).await?;
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

/// A failed command's stderr is quoted back to the user, so it is bounded — and
/// what is kept is the TAIL, because dbt prints its error summary last.
const CAPTURE_MAX_STDERR_BYTES: usize = 64 * 1024;

/// Run a command for its stdout under the job's cancellation and timeout.
/// The same poller `handle_child` uses drives them, so a cancel or a deadline
/// drops the wait future — which owns the child, and `kill_on_drop` then
/// terminates it. Dropping a wait future does NOT by itself kill a process, so
/// without that flag the child would outlive the job.
///
/// Reads the pipes incrementally against `max_stdout_bytes` rather than
/// `wait_with_output`, which would buffer whatever the child chose to write
/// before any ceiling could apply: the point of the ceiling is that the worker
/// never holds more than it, so it has to be enforced while reading. Both pipes
/// are drained concurrently because a child that fills the one nobody reads
/// blocks forever.
async fn run_capturing(
    mut cmd: Command,
    name: &str,
    ctx: &mut JobCtx<'_>,
    job_id: &Uuid,
    w_id: &str,
    conn: &Connection,
    max_stdout_bytes: usize,
) -> error::Result<String> {
    use tokio::io::AsyncReadExt;

    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| Error::internal_err(format!("{name} could not be started: {e}")))?;
    let pid = child.id();
    let mut stdout_pipe = child
        .stdout
        .take()
        .ok_or_else(|| Error::internal_err(format!("{name} has no stdout")))?;
    let mut stderr_pipe = child
        .stderr
        .take()
        .ok_or_else(|| Error::internal_err(format!("{name} has no stderr")))?;

    let out = run_future_with_polling_update_job_poller(
        *job_id,
        ctx.timeout(),
        conn,
        ctx.mem_peak,
        ctx.canceled_by,
        async move {
            let mut stdout: Vec<u8> = Vec::new();
            let mut stderr: Vec<u8> = Vec::new();
            // On the heap, not the stack: these live across the `select!`, so an
            // array would be baked into this future's state, and the future is
            // then moved into the job poller and boxed several layers deep. Two
            // 16 KB arrays there overflow the worker thread's stack.
            let mut out_buf = vec![0u8; 16 * 1024];
            let mut err_buf = vec![0u8; 16 * 1024];
            let (mut out_open, mut err_open) = (true, true);
            while out_open || err_open {
                tokio::select! {
                    r = stdout_pipe.read(&mut out_buf[..]), if out_open => match r {
                        Ok(0) => out_open = false,
                        Ok(n) => {
                            if stdout.len() + n > max_stdout_bytes {
                                // Killed here rather than left to `kill_on_drop`
                                // so the child is gone before the error unwinds,
                                // not merely once this future is dropped.
                                let _ = child.kill().await;
                                return Err(Error::ExecutionErr(format!(
                                    "{name} produced more than {} MB of output. Narrow the \
                                     selection, or query the relation from a SQL script.",
                                    max_stdout_bytes / 1024 / 1024
                                )));
                            }
                            stdout.extend_from_slice(&out_buf[..n]);
                        }
                        Err(e) => return Err(Error::internal_err(format!("{name} failed: {e}"))),
                    },
                    r = stderr_pipe.read(&mut err_buf[..]), if err_open => match r {
                        Ok(0) => err_open = false,
                        Ok(n) => {
                            stderr.extend_from_slice(&err_buf[..n]);
                            if stderr.len() > CAPTURE_MAX_STDERR_BYTES {
                                let excess = stderr.len() - CAPTURE_MAX_STDERR_BYTES;
                                stderr.drain(..excess);
                            }
                        }
                        Err(_) => err_open = false,
                    },
                }
            }
            let status = child
                .wait()
                .await
                .map_err(|e| Error::internal_err(format!("{name} failed: {e}")))?;
            Ok((status, stdout, stderr))
        },
        ctx.worker_name,
        w_id,
        &mut Some(ctx.occupancy_metrics),
        Box::pin(futures::stream::unfold((), move |_| async move {
            Some((get_mem_peak(pid, false).await, ()))
        })),
    )
    .await?;
    let (status, stdout, stderr) = out;
    if !status.success() {
        return Err(Error::ExecutionErr(format!(
            "{name} failed: {}",
            String::from_utf8_lossy(&stderr)
        )));
    }
    Ok(String::from_utf8_lossy(&stdout).to_string())
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

pub(crate) async fn read_manifest(
    project_dir: &Path,
) -> error::Result<windmill_common::dbt_manifest::Manifest> {
    let path = project_dir.join(ARTIFACTS_DIR).join("manifest.json");
    let content = tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| Error::internal_err(format!("dbt produced no manifest.json: {e}")))?;
    serde_json::from_str(&content)
        .map_err(|e| Error::internal_err(format!("could not parse manifest.json: {e}")))
}

/// `dbt retry` reads `run_results.json` from the previous invocation, and
/// Windmill gives each attempt a fresh job dir — so this is where the last one
/// is kept. It is a fast path over `dbt_run_state`, which any worker of the
/// group can read; only an agent worker, which cannot reach that table, is
/// limited to what its own disk holds.
///
/// Keyed by principal as well, matching `dbt_run_state`. An agent worker never
/// reads that table, so this cache is the whole boundary there: without it one
/// principal's saved `select` and `vars` are restorable by the next.
fn state_dir(w_id: &str, script_path: &str, permissioned_as: &str) -> PathBuf {
    PathBuf::from(&*DBT_CACHE_DIR)
        .join("state")
        .join(digest(&format!("{w_id}/{script_path}/{permissioned_as}")))
}

/// Forget any saved retry state for this principal and script.
///
/// Called where an invocation ran the BUILD and left nothing resumable —
/// cancelled, timed out, dead before dbt wrote `run_results.json` — because the
/// warehouse is then no longer what the previous run's failures describe, and a
/// `dbt retry` would rebuild against a state that moved under it. Also where the
/// run is hidden from the script's owners, which keeps no state at all. A
/// failure BEFORE the build touches no relation and leaves the saved run alone.
async fn invalidate_run_state(
    w_id: &str,
    script_path: &str,
    permissioned_as: &str,
    conn: &Connection,
) {
    if script_path.is_empty() {
        return;
    }
    if let Connection::Sql(db) = conn {
        let _ = sqlx::query!(
            "DELETE FROM dbt_run_state
              WHERE workspace_id = $1 AND script_path = $2 AND permissioned_as = $3",
            w_id,
            script_path,
            permissioned_as,
        )
        .execute(db)
        .await
        .inspect_err(|e| {
            tracing::warn!("dbt: could not clear retry state for {script_path}: {e:#}")
        });
    }
    let dir = state_dir(w_id, script_path, permissioned_as);
    if let Err(e) = tokio::fs::remove_file(dir.join(CURRENT_GENERATION)).await {
        if e.kind() != std::io::ErrorKind::NotFound {
            tracing::warn!("dbt: could not drop local retry state for {script_path}: {e:#}");
        }
    }
    // The generations too, not only the pointer. `prune_old_generations` otherwise
    // runs at the tail of a successful SAVE, so a script whose last run went green
    // stranded the failed run's directory, each holding a `manifest.json` that
    // grows with the project. Keeps only what a retry may already be copying.
    prune_old_generations(&dir, "").await;
}

async fn save_run_state(
    p: &PreparedProject,
    w_id: &str,
    // Part of the state's key: a retry replaces the caller's arguments with
    // these, so another principal must not be able to restore them.
    permissioned_as: &str,
    // A HIDDEN run keeps no state: the key is the principal, which every caller of
    // an `on_behalf_of` script shares, and a retry publishes the arguments it
    // restored — so that retry would be the one way to see a run they cannot read.
    // A visible run discloses nothing they could not already read.
    visible_to_owner: bool,
    // Names this run's generation directory, so two runs of one script stage into
    // their own and cannot publish a mixture of each other's artifacts.
    job_id: &Uuid,
    inv: &Invocation,
    // Digest of the `run_results.json` a retry restored, when this run is one.
    restored_results_digest: Option<&str>,
    conn: &Connection,
) -> error::Result<()> {
    if p.script_path.is_empty() {
        return Ok(());
    }
    // The previous run's state goes with it: this invocation happened, so those
    // failures are no longer what last ran here.
    if !visible_to_owner {
        invalidate_run_state(w_id, &p.script_path, permissioned_as, conn).await;
        return Ok(());
    }
    let identity = format!(
        "{}|{}{ARGS_DIGEST_TAG}{}",
        p.run_identity(),
        inv.env_digest(),
        inv.resolved_args_digest()
    );
    // As SUBMITTED, not resolved: `build_args_map` turns `$var:` and `$res:` into
    // plaintext and this row outlives the job, so persisting the resolved value
    // would leave a secret in the database and let a retry replay it after the
    // grant was revoked. The restore resolves again, under whoever retries.
    let args: HashMap<String, String> = inv
        .raw_args
        .iter()
        .map(|(k, v)| (k.clone(), v.get().to_string()))
        .collect();
    // The durable copy, so a retry works from any worker of the group. Only
    // `run_results.json`: the manifest is a pure function of what `identity`
    // already pins, so the resuming worker re-derives it with a `dbt parse`.
    let results =
        tokio::fs::read_to_string(p.project_dir.join(ARTIFACTS_DIR).join("run_results.json"))
            .await
            .ok();
    // No `run_results.json` means nothing resumable happened — cancelled, timed
    // out, dead before dbt wrote one. The previous run's state must not stay
    // authoritative, or `dbt retry` resumes ITS failed nodes, which are not what
    // last ran here. Both copies go, so neither answers for the other.
    let Some(results) = results else {
        invalidate_run_state(w_id, &p.script_path, permissioned_as, conn).await;
        return Ok(());
    };
    // A retry that ended before dbt rewrote the file leaves what the restore put
    // there. Republishing dates the PREVIOUS attempt's failures to this job, so the
    // next retry rebuilds nodes this one already redid — appending to an
    // incremental model twice. As above: nothing resumable happened here.
    if restored_results_digest == Some(digest(&results).as_str()) {
        invalidate_run_state(w_id, &p.script_path, permissioned_as, conn).await;
        return Ok(());
    }
    let mut durable_err = None;
    if let Connection::Sql(db) = conn {
        {
            // Only while a live dbt version stays at this path — the test
            // `clear_dbt_run_state_if_path_retired` retires state by, plus the
            // language, since a rename leaves the old path archived rather than
            // deleted and a path can come back as another language. A job already
            // running finishes after those move or clear the row: writing then
            // strands a failure where the script no longer is.
            durable_err = sqlx::query!(
                "INSERT INTO dbt_run_state (workspace_id, script_path, permissioned_as, identity, args, run_results, job_id, retryable, updated_at)
                 SELECT $1::varchar, $2::varchar, $7::varchar, $3::text, $4::jsonb, $5::text, $6::uuid, $8::boolean, now()
                  WHERE EXISTS (SELECT 1 FROM script
                                 WHERE workspace_id = $1 AND path = $2
                                   AND deleted = false AND archived = false
                                   AND language = 'dbt')
                 ON CONFLICT (workspace_id, script_path, permissioned_as) DO UPDATE SET
                   identity = EXCLUDED.identity, args = EXCLUDED.args,
                   run_results = EXCLUDED.run_results, job_id = EXCLUDED.job_id,
                   retryable = EXCLUDED.retryable, updated_at = now()",
                w_id,
                &p.script_path,
                identity,
                serde_json::to_value(&args).unwrap_or_default(),
                results,
                job_id,
                permissioned_as,
                // Read back by the API to decide whether anything may offer a
                // resume; the restore re-checks the results themselves, which is
                // what tells a retry the run succeeded rather than that no state
                // exists.
                has_retryable_node(&results),
            )
            .execute(db)
            .await
            .err();
        }
    }
    // A failed durable write leaves this generation unpublished locally too:
    // `restore` takes a local one only when the row names it, so publishing what no
    // row records would have this worker reject its own newest state. The previous
    // run's goes with it, since resuming it would rebuild the wrong nodes.
    if let Some(e) = durable_err {
        invalidate_run_state(w_id, &p.script_path, permissioned_as, conn).await;
        return Err(e.into());
    }
    let dir = state_dir(w_id, &p.script_path, permissioned_as);
    let generation = format!("gen-{job_id}");
    // Giving up leaves the PREVIOUS generation's pointer in place, which is
    // harmless where a durable row exists — `restore` falls back to it. An agent
    // worker has no row, so that pointer is all a retry reads, and it would answer
    // for a run that is not the last one to have happened here.
    let abandon_local = || async {
        if matches!(conn, Connection::Http(_)) {
            invalidate_run_state(w_id, &p.script_path, permissioned_as, conn).await;
        }
        Ok(())
    };
    let staging = dir.join(&generation);
    tokio::fs::remove_dir_all(&staging).await.ok();
    if tokio::fs::create_dir_all(&staging).await.is_err() {
        return abandon_local().await;
    }
    for f in ["run_results.json", "manifest.json"] {
        if tokio::fs::copy(p.project_dir.join(ARTIFACTS_DIR).join(f), staging.join(f))
            .await
            .is_err()
        {
            tokio::fs::remove_dir_all(&staging).await.ok();
            return abandon_local().await;
        }
    }
    // What produced it, environment included: a moved ref, a repointed profile or
    // a changed script variable makes the saved results describe relations a retry
    // would not produce. The arguments come back too, since `dbt retry` reuses the
    // original invocation's selection and vars rather than this job's.
    let state = SavedRunState { identity, args };
    if tokio::fs::write(
        staging.join("state.json"),
        serde_json::to_vec(&state).unwrap_or_default(),
    )
    .await
    .is_err()
    {
        tokio::fs::remove_dir_all(&staging).await.ok();
        return abandon_local().await;
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
        return abandon_local().await;
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

/// How many superseded generations may sit inside the grace period at once.
/// Each holds a manifest and a results copy, and the grace is a whole hour: a
/// burst of runs would otherwise accumulate that many copies, and nothing after
/// the burst comes back to remove them.
const GENERATION_KEEP: usize = 4;

async fn prune_old_generations(dir: &Path, keep: &str) {
    let Ok(mut entries) = tokio::fs::read_dir(dir).await else {
        return;
    };
    let now = std::time::SystemTime::now();
    let mut young: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();
    while let Ok(Some(e)) = entries.next_entry().await {
        let name = e.file_name().to_string_lossy().to_string();
        if !name.starts_with("gen-") || name == keep {
            continue;
        }
        let modified = e.metadata().await.ok().and_then(|m| m.modified().ok());
        let stale = modified
            .and_then(|t| now.duration_since(t).ok())
            .is_some_and(|age| age.as_secs() > GENERATION_GRACE_SECS);
        if stale {
            tokio::fs::remove_dir_all(e.path()).await.ok();
        } else if let Some(t) = modified {
            young.push((t, e.path()));
        }
    }
    // Oldest first, so what goes is what a retry is least likely to still be
    // copying out of.
    young.sort_by_key(|(t, _)| *t);
    let excess = young.len().saturating_sub(GENERATION_KEEP);
    for (_, path) in young.into_iter().take(excess) {
        tokio::fs::remove_dir_all(path).await.ok();
    }
}

/// Everything a dbt invocation is parameterized by. One struct because every
/// command in a run — `parse`, `ls`, `build` — must see the SAME arguments and
/// environment: a difference between any two of them means the graph describes
/// something other than what was built, silently.
#[derive(Clone, Default)]
pub struct Invocation {
    pub args: HashMap<String, Box<RawValue>>,
    /// The args as SUBMITTED, before `$var:` / `$res:` / `$encrypted:` were
    /// resolved. This is what run state persists: saving `args` would write the
    /// resolved plaintext into `dbt_run_state` and the worker's `state.json`,
    /// and a later `retry` would replay another caller's secret — after the
    /// grant was revoked or the value rotated. The reference outlives the run;
    /// what it pointed at must not.
    pub raw_args: HashMap<String, Box<RawValue>>,
    pub envs: HashMap<String, String>,
    /// A run must fail on a `{{ }}` placeholder it cannot fill; a deploy, which
    /// has no arguments at all, tolerates them. Declared rather than inferred
    /// from the argument count: a run submitted with `{}` is still a run, and
    /// treating it as a deploy would blank its placeholders and build against
    /// an unintended schema or alias.
    pub strict: bool,
}

impl Invocation {
    /// Digest of the RESOLVED arguments, for retry identity.
    ///
    /// The saved arguments are the ones submitted, so a `$var:` in them is
    /// re-resolved on retry — and a value that changed since the failed run
    /// selects a different set of nodes, or an `enabled`/`schema`/`alias` that
    /// moves the relations. Without this in the identity that retry is accepted,
    /// and which graph it uses then depends on WHERE it lands: a worker holding
    /// the local snapshot replays the saved manifest, while a database restore
    /// reparses with the new value. Placement must not decide that.
    ///
    /// Values are the user's, and can be secrets, so they are hashed.
    fn resolved_args_digest(&self) -> String {
        let mut sorted: Vec<(&String, &Box<RawValue>)> = self.args.iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(b.0));
        stable_digest(sorted.iter().flat_map(|(k, v)| [k.as_str(), v.get()]))
    }

    /// Digest of the script-level environment, for retry identity. Values are
    /// secrets, so they are hashed rather than stored.
    fn env_digest(&self) -> String {
        let mut sorted: Vec<(&String, &String)> = self.envs.iter().collect();
        sorted.sort();
        stable_digest(sorted.iter().flat_map(|(k, v)| [k.as_str(), v.as_str()]))
    }
}

/// A digest that survives a toolchain upgrade.
///
/// These values are written into `dbt_run_state.identity` and compared by a
/// later worker, possibly built with a different Rust release —
/// `DefaultHasher`'s output is explicitly not stable across those, so a bump
/// would refuse every saved failure as a different project. Each part is
/// length-prefixed so no split of the same bytes can collide.
fn stable_digest<'a>(parts: impl Iterator<Item = &'a str>) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    for p in parts {
        h.update((p.len() as u64).to_be_bytes());
        h.update(p.as_bytes());
    }
    format!("{:x}", h.finalize())
}

/// Whether the invocation overrides the descriptor's `vars`, which decides
/// whether its graph is its own rather than the deployed one.
fn has_vars_override(args: &HashMap<String, Box<RawValue>>) -> bool {
    args.get("vars")
        .is_some_and(|r| !matches!(r.get().trim(), "" | "null" | "{}"))
}

/// The saved identity, split at the point resolution happens.
///
/// The project, warehouse, engine and env can be checked before anything is
/// restored. The resolved-arguments digest cannot: the retry REQUEST carries
/// only `dbt_command`, and the arguments to compare are the saved ones after
/// this caller has re-resolved them — which happens later, in `handle_dbt_job`.
/// Comparing the whole string up front would refuse every retry.
fn split_identity(identity: &str) -> (&str, Option<&str>) {
    // Tagged, not positional. The previous format was `<identity>|<env>`, so
    // taking "everything after the last `|`" reads a pre-upgrade row's env
    // digest as an arguments digest, leaves `<identity>` as the prefix, and
    // rejects every saved failure on the instance as a different project.
    match identity.rsplit_once(ARGS_DIGEST_TAG) {
        Some((prefix, args)) => (prefix, Some(args)),
        None => (identity, None),
    }
}

/// Separates the resolved-arguments digest from the rest of a saved identity.
/// A row written before it existed simply does not contain this.
const ARGS_DIGEST_TAG: &str = "|args=";

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
/// `manifest.json`, which is far larger and grows with the project, so it has to
/// be re-derived with a `dbt parse` — sound because `identity` pins the project
/// digest, the warehouse and the engine, which is everything the manifest is a
/// function of. That parse is the caller's, on resolved arguments.
async fn restore_from_db(
    p: &PreparedProject,
    w_id: &str,
    permissioned_as: &str,
    inv: &Invocation,
    // The run the caller named. Re-checked here rather than trusted from the
    // caller's earlier read: another invocation can replace the row in between,
    // and the row this restore actually reads must still be that run.
    expected_job: Uuid,
    conn: &Connection,
    no_state: Error,
) -> error::Result<RestoredRun> {
    let Connection::Sql(db) = conn else {
        // An agent worker reaches the database only through the API, and this
        // state is not exposed there.
        return Err(no_state);
    };
    // The principal IS the boundary, and deliberately: a retry resumes what last
    // ran as this identity, not what this caller ran. Adding a caller check needs an
    // identity the worker does not have — `created_by` is a display name a token
    // label supplies — and resuming grants nothing a caller entitled to run the
    // script lacks. What it does add, the resumed arguments echoed in the result,
    // and the one sharing shape where that crosses a line, are in docs/dbt-runtime.md.
    let Some(row) = sqlx::query!(
        "SELECT identity, args, run_results, job_id FROM dbt_run_state
         WHERE workspace_id = $1 AND script_path = $2 AND permissioned_as = $3",
        w_id,
        &p.script_path,
        permissioned_as
    )
    .fetch_optional(db)
    .await?
    else {
        return Err(no_state);
    };
    if row.job_id != Some(expected_job) {
        return Err(wrong_run(
            expected_job,
            row.job_id.map(|j| j.to_string()).as_deref(),
            "for this script",
        ));
    }
    let (saved_prefix, saved_args_digest) = split_identity(&row.identity);
    if saved_prefix != format!("{}|{}", p.run_identity(), inv.env_digest()) {
        return Err(different_project());
    }
    let saved_args_digest = saved_args_digest.map(str::to_string);
    if !has_retryable_node(&row.run_results) {
        return Err(nothing_to_retry());
    }
    let target = p.project_dir.join(ARTIFACTS_DIR);
    tokio::fs::create_dir_all(&target).await.ok();
    tokio::fs::write(target.join("run_results.json"), &row.run_results)
        .await
        .map_err(|e| Error::internal_err(format!("restoring run_results.json: {e}")))?;
    // No manifest came with the row, so one has to be re-derived — but not here:
    // these arguments are as SUBMITTED, and a `$var:` in them shapes the graph
    // only once resolved. The caller resolves, then parses.
    Ok(RestoredRun {
        args: restored_args(row.args),
        needs_parse: true,
        args_digest: saved_args_digest,
        results_digest: digest(&row.run_results),
    })
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

/// A retry named a run other than the one whose failure is held. Both ids are
/// spelled out: only one failure is kept per script per principal, so a later run
/// replaces it, and without the held id "that is not the one" reads as a bug.
fn wrong_run(asked: Uuid, held: Option<&str>, held_where: &str) -> Error {
    Error::BadRequest(match held {
        Some(held) => format!(
            "you asked to resume run {asked}, but the failure saved {held_where} is run {held}; \
             only the most recent one is kept. Open {held} to retry that one, or run the script \
             normally"
        ),
        None => format!(
            "you asked to resume run {asked}, but no failure is saved {held_where}; run the \
             script normally to rebuild"
        ),
    })
}

fn nothing_to_retry() -> Error {
    Error::BadRequest(
        "the last dbt run succeeded, so there is nothing to retry: `dbt retry` resumes the \
         previous run's failed and skipped nodes. Run the script normally to rebuild"
            .to_string(),
    )
}

/// The previous invocation, restored.
pub struct RestoredRun {
    /// ITS arguments, as submitted, which is what the graph refresh for a retry
    /// must use — still unresolved, so the caller resolves before using them.
    pub args: HashMap<String, Box<RawValue>>,
    /// Whether a `dbt parse` still owes a `manifest.json`. The local snapshot
    /// carries one — unless it was pruned mid-restore; the database row never
    /// does.
    pub needs_parse: bool,
    /// The resolved-arguments digest the saved run had, checked once the caller
    /// has re-resolved those arguments.
    pub args_digest: Option<String>,
    /// Digest of the `run_results.json` this restore put in the job directory.
    /// A retry that ends before dbt rewrites that file — cancelled, timed out —
    /// leaves it there unchanged, and saving it would republish the PREVIOUS
    /// attempt's failures as the newest state, so the retry after this one would
    /// redo nodes this one already rebuilt.
    pub results_digest: String,
}

/// Which worker-local generation a restore may use, if any.
///
/// The pointer is a fast path over the database row and is accepted only when it
/// names the run that row does. An agent has no row — so for it the pointer is
/// all there is, and its own name is the only thing that can answer "is this the
/// run you asked to resume", which is why the check below is not the caller's.
fn chosen_generation(
    local: Option<String>,
    conn: &Connection,
    latest_job: Option<Uuid>,
    expected_job: Uuid,
) -> error::Result<Option<String>> {
    let generation = match (local, conn, latest_job) {
        (Some(g), Connection::Http(_), _) => Some(g),
        (Some(g), _, Some(id)) if g.trim() == format!("gen-{id}") => Some(g),
        _ => None,
    };
    if let Some(g) = generation.as_ref() {
        if g.trim() != format!("gen-{expected_job}") {
            return Err(wrong_run(
                expected_job,
                Some(g.trim().trim_start_matches("gen-")),
                "on this worker",
            ));
        }
    }
    Ok(generation)
}

/// Restore the previous invocation. The `dbt parse` a database restore needs is
/// left to the caller so it runs on RESOLVED arguments: a `$var:` reference
/// shapes the graph only once it has a value, and parsing with the reference
/// verbatim would hand the build a manifest of a different project than the one
/// it goes on to build.
async fn restore_run_state(
    p: &PreparedProject,
    w_id: &str,
    permissioned_as: &str,
    inv: &Invocation,
    // The run the caller means to resume, which a retry always names.
    expected_job: Uuid,
    conn: &Connection,
) -> error::Result<RestoredRun> {
    if p.script_path.is_empty() {
        // A preview has no path to key state on, and an empty key is the one
        // that used to be shared by every dbt script in the workspace.
        return Err(Error::BadRequest(
            "`dbt_command: retry` needs a deployed script; a preview run has no state to \
             resume from"
                .to_string(),
        ));
    }
    let dir = state_dir(w_id, &p.script_path, permissioned_as);
    // Resolved ONCE, with everything read out of the generation it names.
    // Generations are immutable, so arguments, manifest and results describe one
    // invocation; resolving per file could pair one run's arguments with another's
    // results.
    let no_state = || {
        Error::BadRequest(
            "no previous dbt run to retry from. `dbt retry` resumes from the \
             `run_results.json` the failed run left behind; run the script normally to rebuild"
                .to_string(),
        )
    };
    // The row is the authoritative latest state, written by whichever worker ran
    // last, while `current` names only what THIS one saw — preferring local would
    // let a retry on an idle worker resume an older invocation. The local snapshot
    // is a fast path only when it names that same run: it already holds a manifest.
    let saved_state = match conn {
        Connection::Sql(db) => {
            sqlx::query_scalar!(
                "SELECT job_id FROM dbt_run_state
              WHERE workspace_id = $1 AND script_path = $2 AND permissioned_as = $3",
                w_id,
                &p.script_path,
                permissioned_as
            )
            .fetch_optional(db)
            .await?
        }
        // An agent worker cannot read it; its local copy is all there is.
        Connection::Http(_) => None,
    };
    let latest_job = saved_state.flatten();
    if let Some(saved) = latest_job {
        if expected_job != saved {
            return Err(wrong_run(
                expected_job,
                Some(&saved.to_string()),
                "for this script",
            ));
        }
    }
    // Authoritative including when it says nothing: no row means the last
    // invocation left nothing resumable, and a local generation that outlived it
    // would resurrect a run the newer one replaced. An agent worker has no such
    // authority to consult, so its local copy stands.
    let local = tokio::fs::read_to_string(dir.join(CURRENT_GENERATION))
        .await
        .ok();
    let generation = chosen_generation(local, conn, latest_job, expected_job)?;
    let Some(generation) = generation else {
        return restore_from_db(
            p,
            w_id,
            permissioned_as,
            inv,
            expected_job,
            conn,
            no_state(),
        )
        .await;
    };
    let snapshot = dir.join(generation.trim());
    // The local generation is a fast path over the row for this same run, so one
    // pruned out from under this restore falls back to the row rather than
    // reporting nothing to resume. An agent worker has no row and gets that
    // report, which is then true.
    let Ok(saved_results) = tokio::fs::read_to_string(snapshot.join("run_results.json")).await
    else {
        return restore_from_db(
            p,
            w_id,
            permissioned_as,
            inv,
            expected_job,
            conn,
            no_state(),
        )
        .await;
    };
    // dbt builds a retry's graph from the error, fail and skipped nodes alone, so
    // retrying an all-green run selects nothing and builds nothing. Refused
    // rather than reported as a successful run of nothing.
    if !has_retryable_node(&saved_results) {
        return Err(nothing_to_retry());
    }
    let Some(saved) = tokio::fs::read_to_string(snapshot.join("state.json"))
        .await
        .ok()
        .and_then(|s| serde_json::from_str::<SavedRunState>(&s).ok())
    else {
        return restore_from_db(
            p,
            w_id,
            permissioned_as,
            inv,
            expected_job,
            conn,
            no_state(),
        )
        .await;
    };
    let (saved_prefix, saved_args_digest) = split_identity(&saved.identity);
    if saved_prefix != format!("{}|{}", p.run_identity(), inv.env_digest()) {
        return Err(different_project());
    }
    let saved_args_digest = saved_args_digest.map(str::to_string);
    let target = p.project_dir.join(ARTIFACTS_DIR);
    tokio::fs::create_dir_all(&target).await.ok();
    // From the bytes already read, not by copying the file again: a burst of saves
    // can prune this generation mid-restore, and a `dbt retry` whose
    // `run_results.json` went missing rebuilds nothing and reports success. The
    // manifest has no such copy, so a failure there falls back to a `dbt parse`.
    tokio::fs::write(target.join("run_results.json"), &saved_results)
        .await
        .map_err(|e| {
            Error::internal_err(format!("could not restore the previous run's results: {e}"))
        })?;
    let needs_parse = tokio::fs::copy(snapshot.join("manifest.json"), target.join("manifest.json"))
        .await
        .is_err();
    // The generation was chosen from a row read before the file work above. A run
    // finishing in that window publishes a newer one, and resuming the superseded
    // generation redoes nodes it has already rebuilt — appending to an incremental
    // model twice. Re-read and refuse rather than resume what is no longer the last
    // failure here.
    if let Connection::Sql(db) = conn {
        let still = sqlx::query_scalar!(
            "SELECT job_id FROM dbt_run_state
              WHERE workspace_id = $1 AND script_path = $2 AND permissioned_as = $3",
            w_id,
            &p.script_path,
            permissioned_as
        )
        .fetch_optional(db)
        .await?
        .flatten();
        if still != latest_job {
            return Err(Error::BadRequest(
                "another run of this script finished while this retry was starting, so its saved \
                 failures are no longer the last ones; retry again to resume those"
                    .to_string(),
            ));
        }
    }
    Ok(RestoredRun {
        args: saved
            .args
            .into_iter()
            .filter_map(|(k, v)| Some((k, RawValue::from_string(v).ok()?)))
            .collect(),
        needs_parse,
        args_digest: saved_args_digest,
        results_digest: digest(&saved_results),
    })
}

/// Wait between retries, giving up if the job is cancelled or runs out of time.
///
/// Returns whether the retry should still happen. A plain sleep would hold the
/// worker slot for the whole delay after a cancel and then start another dbt
/// process on the far side of it.
///
/// The cancellation is READ FROM THE DATABASE each second rather than from
/// `canceled_by`: that is only written by the job poller, which runs alongside a
/// child process and so is not running here. Re-reading it would report the
/// state as of the failed attempt and miss every cancel issued during the wait,
/// which is the whole window this exists to cover.
async fn sleep_before_retry(
    delay_seconds: u64,
    job_id: &Uuid,
    conn: &Connection,
    deadline: JobDeadline,
) -> bool {
    let mut left = delay_seconds;
    loop {
        if deadline.is_expired() || job_is_canceled(job_id, conn).await {
            return false;
        }
        if left == 0 {
            return true;
        }
        let step = left.min(1);
        tokio::time::sleep(std::time::Duration::from_secs(step)).await;
        left -= step;
    }
}

/// Whether the job has been cancelled, as of now.
///
/// Only reachable with a database: the automatic retry that calls this is
/// refused on an agent worker precisely because it could not answer here.
async fn job_is_canceled(job_id: &Uuid, conn: &Connection) -> bool {
    let Connection::Sql(db) = conn else {
        return false;
    };
    sqlx::query_scalar!(
        "SELECT canceled_by IS NOT NULL AS \"canceled!\" FROM v2_job_queue WHERE id = $1",
        job_id
    )
    .fetch_optional(db)
    .await
    .map(|v| v == Some(true))
    .unwrap_or(false)
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

/// What a dbt node's status means, in the three terms this runtime acts on.
///
/// Every site classifies through here: dbt-core 1.x echoes the author's casing
/// where 2.x uppercases, so comparing statuses inline gives each site its own
/// answer for the same node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DbtNodeOutcome {
    Started,
    Passed,
    Failed,
    /// `warn`: a test failed under a severity that does not fail the run. The
    /// relation is untouched, but the node is worth showing.
    Warn,
    /// `skipped`: dbt did not run the node, usually because an upstream one
    /// failed. Says nothing about the relation, and `dbt retry` redoes it.
    Skipped,
    /// `no-op`: the node ran and had nothing to do (an empty microbatch, a model
    /// with no rows to build). Tallied with `skipped` — nothing was built — but
    /// NOT retryable: dbt's own retry set is error / fail / skipped.
    NoOp,
    /// A status this dbt version spells some way we do not know. Counted as an
    /// error rather than silently passing, but never used to settle a relation.
    Unknown,
}

impl DbtNodeOutcome {
    /// The word this outcome is published as in a job's result. Stable by
    /// contract: a dbt release may rename its own status, and this must not
    /// move with it.
    fn as_result_word(&self) -> &'static str {
        match self {
            // A finished node is never `Started`; it is spelled here so the
            // match stays exhaustive rather than falling into `unknown`.
            DbtNodeOutcome::Started => "started",
            DbtNodeOutcome::Passed => "passed",
            DbtNodeOutcome::Failed => "failed",
            DbtNodeOutcome::Warn => "warned",
            DbtNodeOutcome::Skipped => "skipped",
            DbtNodeOutcome::NoOp => "no_op",
            DbtNodeOutcome::Unknown => "unknown",
        }
    }
}

fn classify_status(status: &str) -> DbtNodeOutcome {
    match status.trim().to_ascii_lowercase().as_str() {
        "started" => DbtNodeOutcome::Started,
        "success" | "pass" => DbtNodeOutcome::Passed,
        // `partial success` builds the relation and then fails its tests: the
        // node is a failure, and the relation it wrote is real but suspect.
        "error" | "fail" | "runtime error" | "partial success" => DbtNodeOutcome::Failed,
        "warn" => DbtNodeOutcome::Warn,
        "skipped" => DbtNodeOutcome::Skipped,
        "no-op" => DbtNodeOutcome::NoOp,
        _ => DbtNodeOutcome::Unknown,
    }
}

/// Whether a saved `run_results.json` holds anything `dbt retry` would redo.
///
/// The rule is dbt's own: `error`, `fail` and `skipped`. A `partial success`
/// counts too — dbt spells that for a node that built but whose tests failed,
/// and its retry redoes the node.
///
/// Tests count too, and they are the common case: with
/// `test_behavior: after_all` a failing test is the whole of
/// `run_results.json`, so requiring a relation-writing node would refuse the
/// retry exactly when it is wanted.
fn has_retryable_node(run_results: &str) -> bool {
    serde_json::from_str::<RunResults>(run_results)
        .map(|r| {
            r.results.iter().any(|n| {
                matches!(
                    classify_status(&n.status),
                    DbtNodeOutcome::Failed | DbtNodeOutcome::Skipped
                )
            })
        })
        // Unreadable results are not "nothing to retry": let dbt decide rather
        // than refusing a retry the user may well need.
        .unwrap_or(true)
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
                // argument's own type: `"{{ strict }}"` given `false` must reach
                // dbt as a boolean, since "false" is truthy in Jinja. Embedded in
                // text it stays a string, which is what interpolation means.
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

/// One job argument, absent when unset or JSON `null` — a schema-less run sends
/// the key with a null rather than omitting it, and both mean "not given".
///
/// A wrong TYPE is an error rather than an absence: argument-schema validation
/// is opt-in, so `dbt_command: 1` read as unset would run the default command
/// and `full_refresh: "false"` would still full-refresh, the caller silently
/// getting something other than what they asked for.
fn arg<T: serde::de::DeserializeOwned>(
    args: &HashMap<String, Box<RawValue>>,
    k: &str,
    expected: &str,
) -> error::Result<Option<T>> {
    let Some(raw) = args.get(k).filter(|r| r.get().trim() != "null") else {
        return Ok(None);
    };
    serde_json::from_str::<T>(raw.get())
        .map(Some)
        .map_err(|e| Error::BadRequest(format!("`{k}` must be {expected}: {e}")))
}

/// The run's arguments with the command block spread over them, its variant
/// `label` under `dbt_command`.
///
/// A run submits one `command` argument whose `oneOf` variant IS the command, so
/// an override belongs to the command that takes it. Every reader below wants a
/// single map, and the block is spread here rather than at each of them so the
/// two shapes never both reach one. `raw_args` keeps the submitted shape: it is
/// what the state saves and the result publishes.
///
/// A block that is present must NAME a command: dropping a malformed one would
/// leave no command at all, which reads as "the descriptor's default" — so
/// `{"command": "show"}` would build the project the caller meant to preview.
/// Argument-schema validation is opt-in, so this is the only check a direct
/// request passes through.
fn flatten_command(
    mut args: HashMap<String, Box<RawValue>>,
) -> error::Result<HashMap<String, Box<RawValue>>> {
    let Some(block) = args.remove(DBT_COMMAND_ARG) else {
        return Ok(args);
    };
    // `null` is how a schema-less run sends "not given", like every other
    // argument, and means the descriptor's own command.
    if block.get().trim() == "null" {
        return Ok(args);
    }
    let malformed = || {
        Error::BadRequest(format!(
            "`{DBT_COMMAND_ARG}` must be an object naming what to run, e.g. \
             `{{\"{DBT_COMMAND_LABEL}\": \"{}\"}}` — one of {}",
            DBT_COMMANDS[0],
            DBT_COMMANDS.join(", ")
        ))
    };
    let mut fields = serde_json::from_str::<HashMap<String, Box<RawValue>>>(block.get())
        .map_err(|_| malformed())?;
    if arg_str(&fields, DBT_COMMAND_LABEL)?.is_none() {
        return Err(malformed());
    }
    let label = fields.remove(DBT_COMMAND_LABEL);
    for (k, v) in fields {
        // A placeholder of the same name would otherwise be overwritten by the
        // block — they are reserved for exactly that reason.
        args.insert(k, v);
    }
    // LAST, and from the variant's label alone: a block carrying a `dbt_command`
    // of its own would otherwise land on the same key, and which command ran
    // would be map iteration order — the malformed check above, arrived at from
    // the other side.
    if let Some(label) = label {
        args.insert("dbt_command".to_string(), label);
    }
    Ok(args)
}

/// Empty reads as absent: it is what an untouched text field sends.
fn arg_str(args: &HashMap<String, Box<RawValue>>, k: &str) -> error::Result<Option<String>> {
    Ok(arg::<String>(args, k, "a string")?.filter(|s| !s.is_empty()))
}

fn arg_bool(args: &HashMap<String, Box<RawValue>>, k: &str) -> error::Result<Option<bool>> {
    arg(args, k, "a boolean")
}

fn arg_i64(args: &HashMap<String, Box<RawValue>>, k: &str) -> error::Result<Option<i64>> {
    arg(args, k, "a whole number")
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
    if let Some(sel) = effective_selector(descriptor, inv)? {
        cmd.args(["--selector", sel]);
    }
    Ok(())
}

/// Whether this invocation chose its own `select`/`exclude`.
///
/// DIFFERENT from the descriptor's, not merely present: `parse_dbt_sig` gives
/// both fields the descriptor's own value as their default and the generated
/// run form posts a default back for every field the caller left untouched, so
/// every run from the UI, a schedule, a webhook or a flow step carries them.
/// Reading that echo as a choice is wrong in two ways at once — it drops the
/// descriptor's `--selector` from those runs (building the whole project), and
/// it marks their graph caller-scoped so a moved profile never republishes and
/// never settles. Both decisions ask this one question.
///
/// A run that wants the whole project despite a descriptor selector names a
/// selection that differs — `["*"]`.
fn selection_is_overridden(
    descriptor: &DbtDescriptor,
    args: &HashMap<String, Box<RawValue>>,
) -> error::Result<bool> {
    let differs = |key: &str, from: &Vec<String>| -> error::Result<bool> {
        Ok(arg_list(args, key)?.is_some_and(|v| &v != from))
    };
    Ok(differs("select", &descriptor.select)? || differs("exclude", &descriptor.exclude)?)
}

/// The descriptor's named selector, unless this run named its own selection.
///
/// dbt resolves `--selector` INSTEAD of `--select`, so passing both makes the
/// descriptor win: a preview asked for one model would return the descriptor's
/// nodes, and a run asked for a subset would build something else. A run naming
/// its own selection therefore replaces the descriptor's selector entirely.
fn effective_selector<'a>(
    descriptor: &'a DbtDescriptor,
    inv: &Invocation,
) -> error::Result<Option<&'a str>> {
    if selection_is_overridden(descriptor, &inv.args)? {
        return Ok(None);
    }
    Ok(descriptor.selector.as_deref())
}

fn effective_select(descriptor: &DbtDescriptor, inv: &Invocation) -> error::Result<Vec<String>> {
    Ok(arg_list(&inv.args, "select")?.unwrap_or_else(|| descriptor.select.clone()))
}

fn effective_exclude(descriptor: &DbtDescriptor, inv: &Invocation) -> error::Result<Vec<String>> {
    Ok(arg_list(&inv.args, "exclude")?.unwrap_or_else(|| descriptor.exclude.clone()))
}

/// Whether an invocation selects a subset at all, from whichever of the
/// descriptor's fields and the run's arguments end up in force.
fn has_selection(descriptor: &DbtDescriptor, inv: &Invocation) -> error::Result<bool> {
    Ok(!effective_select(descriptor, inv)?.is_empty()
        || !effective_exclude(descriptor, inv)?.is_empty()
        || effective_selector(descriptor, inv)?.is_some())
}

/// An explicitly supplied list, including an empty one — `[]` is how a run asks
/// for the whole project where the descriptor named a selection, so it must not
/// read as "absent" and fall back to that selection.
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
    let mut cmd = Command::new("cp");
    cmd.arg("-a").arg(format!("{}/.", from.display())).arg(to);
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

    // `[]` against a descriptor that names a selection is how a run asks for the
    // whole project. Reading it as "absent" would fall back to the descriptor's
    // and build a different model set than the run asked for.
    #[test]
    fn an_empty_override_widens_a_descriptor_selection() {
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

    // Compared in two halves because resolution happens between them: the prefix
    // is checkable up front, the digest only once the caller has re-resolved the
    // saved arguments. Comparing the whole string up front refuses every retry,
    // whose request carries only `dbt_command`.
    #[test]
    fn the_identity_splits_at_the_resolved_arguments() {
        let (prefix, args) = split_identity("proj|wh|engine|deadbeef|args=c0ffee");
        assert_eq!(prefix, "proj|wh|engine|deadbeef");
        assert_eq!(args, Some("c0ffee"));
        // Profile targets and other identity inputs are user-controlled. A
        // literal `args=` inside one must stay in the identity prefix.
        let (prefix, args) = split_identity("proj|target=blue|args=warehouse|deadbeef|args=c0ffee");
        assert_eq!(prefix, "proj|target=blue|args=warehouse|deadbeef");
        assert_eq!(args, Some("c0ffee"));
        // A PRE-UPGRADE row, which ends in the env digest and has plenty of
        // `|` in it. Splitting on the last one would take that digest for an
        // arguments digest and make every saved failure unretryable.
        let (prefix, args) = split_identity("proj|wh|engine|deadbeef");
        assert_eq!(prefix, "proj|wh|engine|deadbeef");
        assert_eq!(args, None);
    }

    #[test]
    // dbt resolves `--selector` INSTEAD of `--select`, so a descriptor selector
    // left on alongside an explicit selection makes the descriptor win: a preview
    // of one model returns another's rows.
    fn a_runs_own_selection_replaces_the_descriptor_selector() {
        let descriptor =
            DbtDescriptor { selector: Some("nightly".to_string()), ..Default::default() };
        let selects = |v: &str| Invocation {
            args: [(
                "select".to_string(),
                serde_json::value::RawValue::from_string(v.to_string()).unwrap(),
            )]
            .into_iter()
            .collect(),
            ..Default::default()
        };
        assert_eq!(
            effective_selector(&descriptor, &Invocation::default()).unwrap(),
            Some("nightly")
        );
        assert_eq!(
            effective_selector(&descriptor, &selects(r#"["stg_orders"]"#)).unwrap(),
            None
        );
        // The generated form posts the descriptor's own `[]` back for a field nobody
        // touched, so that is not an override: reading it as one drops the selector
        // from every UI, schedule and webhook run and builds the whole project.
        // `["*"]` is how a run asks for that on purpose.
        assert_eq!(
            effective_selector(&descriptor, &selects("[]")).unwrap(),
            Some("nightly")
        );
        assert!(has_selection(&descriptor, &selects("[]")).unwrap());
        assert_eq!(
            effective_selector(&descriptor, &selects(r#"["*"]"#)).unwrap(),
            None
        );
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
            "",
        );
        let retry = profile_identity_digest(
            &yaml("/tmp/windmill/w/job-2/profiles", "wh.internal"),
            Path::new("/tmp/windmill/w/job-2/profiles"),
            Some("PEM"),
            "",
        );
        assert_eq!(first, retry);

        let repointed = profile_identity_digest(
            &yaml("/tmp/windmill/w/job-2/profiles", "other.internal"),
            Path::new("/tmp/windmill/w/job-2/profiles"),
            Some("PEM"),
            "",
        );
        let recerted = profile_identity_digest(
            &yaml("/tmp/windmill/w/job-2/profiles", "wh.internal"),
            Path::new("/tmp/windmill/w/job-2/profiles"),
            Some("OTHER PEM"),
            "",
        );
        assert_ne!(first, repointed);
        assert_ne!(first, recerted);
    }

    // The warehouse's resource may interpolate `$WM_TOKEN`, so the rendered
    // profile carries the ATTEMPT's token. A retry is a new job with a new one,
    // and without normalizing it the saved run is never recognized as its own.
    #[test]
    fn profile_identity_ignores_the_attempts_token() {
        let yaml = |tok: &str| format!("host: \"wh\"\npassword: \"{tok}\"\n");
        let dir = Path::new("/tmp/windmill/w/job-1/profiles");
        assert_eq!(
            profile_identity_digest(&yaml("tok-first"), dir, None, "tok-first"),
            profile_identity_digest(&yaml("tok-retry"), dir, None, "tok-retry")
        );
        // A password that is NOT the job's token is the connection, and changing
        // it must still read as a different warehouse.
        assert_ne!(
            profile_identity_digest(&yaml("static-a"), dir, None, "tok-first"),
            profile_identity_digest(&yaml("static-b"), dir, None, "tok-retry")
        );
    }

    // The jail profile is protobuf text format, and the project path and the
    // descriptor's environment land inside string literals. An unescaped quote or
    // newline closes the literal and lets the rest be read as further directives —
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

    // THREE sites derive a `dbt://` key: the manifest ingest, the live events and
    // the end-of-run settlement. One deriving it differently records progress
    // against a path no node has — the run still succeeds and the graph never
    // moves. Nothing else catches that.
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
        // A doubled delimiter is that delimiter, literally: the manifest keeps
        // the real spelling, so dropping the pair here records the run's
        // progress against a key no node has.
        assert_eq!(
            split_relation("\"wh\".\"schema\".\"a\"\"b\""),
            vec!["wh", "schema", "a\"b"]
        );
        assert_eq!(
            split_relation("[db].[my]]schema].[t]"),
            vec!["db", "my]schema", "t"]
        );
    }

    // dbt vars are typed and Jinja treats the string "false" as truthy, so a var
    // that IS a placeholder carries the argument's own type through, while one
    // embedded in text stays the string it interpolates to.
    #[test]
    fn placeholder_vars_keep_the_arguments_type() {
        use windmill_parser_yaml::parse_dbt_descriptor;
        let d = parse_dbt_descriptor(
            "vars:\n  strict: \"{{ strict }}\"\n  n: \"{{ n }}\"\n  label: \"run-{{ name }}\"\n",
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

    // A project that owns its `profiles.yml` has to spell its relations the way
    // a workspace-warehouse project does, or the same physical table becomes two
    // nodes — `main/analytics/orders` for one and `main/prod.analytics/orders`
    // for the other — and the lineage they exist to share never connects.
    #[tokio::test]
    async fn a_project_owned_profile_reports_its_target_database() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profiles.yml");
        std::fs::write(
            &path,
            "jaffle:\n  target: prod\n  outputs:\n    prod:\n      type: snowflake\n\
             \x20     database: prod\n      schema: analytics\n",
        )
        .unwrap();
        let t = adapter_from_profiles_yml(&path, "jaffle", None)
            .await
            .unwrap();
        assert_eq!(t.adapter, DbtAdapter::from(KnownAdapter::Snowflake));
        assert_eq!(t.database.as_deref(), Some("prod"));
        assert_eq!(t.schema.as_deref(), Some("analytics"));
        // Spelled plainly, exactly as a rendered profile on the same relation.
        assert_eq!(
            windmill_common::dbt_manifest::table_asset_path(
                "main",
                Some("prod"),
                "analytics",
                "orders",
                t.database.as_deref(),
            )
            .as_deref(),
            Some("main/analytics/orders")
        );
    }

    // dbt renders `profiles.yml` through Jinja; Windmill reads it raw. A repo that
    // carries one profile across environments selects its target with `env_var`,
    // and refusing that would refuse the unmodified-project path this exists for.
    #[tokio::test]
    async fn a_templated_target_takes_the_only_output() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profiles.yml");
        std::fs::write(
            &path,
            "jaffle:\n  target: \"{{ env_var('DBT_TARGET', 'prod') }}\"\n  outputs:\n\
             \x20   prod:\n      type: snowflake\n      database: prod\n      schema: analytics\n",
        )
        .unwrap();
        let t = adapter_from_profiles_yml(&path, "jaffle", None)
            .await
            .unwrap();
        assert_eq!(t.adapter, DbtAdapter::from(KnownAdapter::Snowflake));
        assert_eq!(t.database.as_deref(), Some("prod"));
    }

    // With several outputs the template names none of them, so the descriptor
    // has to choose rather than Windmill guessing which environment to run.
    #[tokio::test]
    async fn a_templated_target_with_several_outputs_asks_for_one() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profiles.yml");
        std::fs::write(
            &path,
            "jaffle:\n  target: \"{{ env_var('DBT_TARGET') }}\"\n  outputs:\n\
             \x20   prod:\n      type: snowflake\n    dev:\n      type: snowflake\n",
        )
        .unwrap();
        let e = adapter_from_profiles_yml(&path, "jaffle", None)
            .await
            .unwrap_err()
            .to_string();
        assert!(e.contains("profile.target"), "{e}");
    }

    // A target that leaves its database to dbt's own defaults says nothing, and
    // guessing one would collapse relations that are genuinely apart.
    #[tokio::test]
    async fn an_implicit_database_stays_unknown() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profiles.yml");
        std::fs::write(
            &path,
            "jaffle:\n  target: dev\n  outputs:\n    dev:\n      type: postgres\n\
             \x20     dbname: \"{{ env_var('DB') }}\"\n",
        )
        .unwrap();
        let t = adapter_from_profiles_yml(&path, "jaffle", None)
            .await
            .unwrap();
        assert_eq!(t.database, None);
    }

    #[tokio::test]
    async fn package_cache_identity_includes_the_resolved_lock() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("package-lock.yml"),
            "packages:\n  - package: dbt-labs/dbt_utils\n    version: 1.3.0\n",
        )
        .unwrap();
        let first = package_lock_digest(dir.path()).await.unwrap().unwrap();
        std::fs::write(
            dir.path().join("package-lock.yml"),
            "packages:\n  - package: dbt-labs/dbt_utils\n    version: 1.4.0\n",
        )
        .unwrap();
        let second = package_lock_digest(dir.path()).await.unwrap().unwrap();

        assert_ne!(first, second);
        assert_ne!(
            package_cache_key("same project and environment", &first),
            package_cache_key("same project and environment", &second)
        );
    }

    // The path is project-controlled and both cache copies are rooted at it, so
    // an absolute or `..`-bearing value would read and write outside the job.
    #[tokio::test]
    async fn a_projects_packages_path_cannot_escape_the_project() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let write = |yml: &str| std::fs::write(root.join("dbt_project.yml"), yml).unwrap();
        let none = HashMap::new();

        write("name: p\n");
        assert_eq!(
            packages_install_path(root, &none).await.unwrap(),
            "dbt_packages"
        );
        write("name: p\npackages-install-path: ./vendor\n");
        assert_eq!(packages_install_path(root, &none).await.unwrap(), "vendor");
        // Unset and empty are the default; an escaping one is refused rather
        // than replaced, since dbt reads the file itself and would honour it.
        write("name: p\npackages-install-path: \"\"\n");
        assert_eq!(
            packages_install_path(root, &none).await.unwrap(),
            "dbt_packages"
        );
        for escape in ["/etc", "../../etc", "a/../../b"] {
            write(&format!("name: p\npackages-install-path: \"{escape}\"\n"));
            assert!(
                packages_install_path(root, &none).await.is_err(),
                "{escape} must not be honoured"
            );
        }
        // An escaping value the ENVIRONMENT supplies is refused just the same:
        // rendering happens before the check, not after it.
        write("name: p\npackages-install-path: \"{{ env_var('D') }}\"\n");
        let escaping = HashMap::from([("D".to_string(), "../out".to_string())]);
        assert!(packages_install_path(root, &escaping).await.is_err());
    }

    // dbt renders `env_var()` in `dbt_project.yml`, so the profile it looks up
    // and the directory `dbt deps` fills are the RENDERED ones. Reading the
    // template instead leaves a project that runs everywhere else unable to find
    // its profile, and its package cache watching a directory nothing fills.
    #[tokio::test]
    async fn a_projects_settings_are_rendered_with_the_runs_environment() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(
            root.join("dbt_project.yml"),
            "name: p\nprofile: \"{{ env_var('DBT_PROFILE', 'analytics') }}\"\n\
             packages-install-path: \"{{ env_var('PKGS', 'vendor') }}\"\n",
        )
        .unwrap();

        let none = HashMap::new();
        assert_eq!(project_profile_name(root, &none).await, "analytics");
        assert_eq!(packages_install_path(root, &none).await.unwrap(), "vendor");

        let set = HashMap::from([
            ("DBT_PROFILE".to_string(), "prod".to_string()),
            ("PKGS".to_string(), "deps".to_string()),
        ]);
        assert_eq!(project_profile_name(root, &set).await, "prod");
        assert_eq!(packages_install_path(root, &set).await.unwrap(), "deps");

        // Neither set nor defaulted: left verbatim, so dbt reports it rather
        // than Windmill inventing a name.
        std::fs::write(
            root.join("dbt_project.yml"),
            "name: p\nprofile: \"{{ env_var('DBT_PROFILE') }}\"\n",
        )
        .unwrap();
        assert_eq!(
            project_profile_name(root, &none).await,
            "{{ env_var('DBT_PROFILE') }}"
        );
    }

    // A retry rewrites `run_results.json` with only the nodes it redid, so the
    // job's own result has to be the union: replacing would drop every node that
    // succeeded before it, and nothing would settle their materializations.
    #[test]
    fn a_retrys_results_overlay_the_attempt_they_resumed() {
        let node = |id: &str, status: &str| DbtNodeResult {
            outcome: classify_status(status).as_result_word(),
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

    // `--limit` decides how much of dbt's stdout the worker buffers, and any
    // caller who may run the script may set it.
    #[test]
    fn only_a_single_node_may_be_shown() {
        // What the run page sends: one model, package-qualified.
        assert!(show_selects_one_node("stg_orders,package:slow_shop"));
        assert!(show_selects_one_node("stg_orders"));
        // A union: the intersection would bind to `safe_model` alone, leaving
        // `my_seed` selected — and a selected seed is LOADED, not shown.
        assert!(!show_selects_one_node("my_seed safe_model"));
        assert!(!show_selects_one_node(
            "my_seed  safe_model,resource_type:model"
        ));
        // Resolve to a set rather than to one relation.
        assert!(!show_selects_one_node("stg_orders+"));
        assert!(!show_selects_one_node("+stg_orders"));
        assert!(!show_selects_one_node("stg_*"));
        assert!(!show_selects_one_node("@stg_orders"));
    }

    #[test]
    fn a_show_limit_is_clamped_to_the_ceiling() {
        let max = windmill_parser_yaml::dbt::DBT_SHOW_MAX_LIMIT as i64;
        let default = windmill_parser_yaml::dbt::DBT_SHOW_DEFAULT_LIMIT as i64;
        assert_eq!(show_limit(Some(i64::MAX)), max, "an enormous ask is capped");
        assert_eq!(show_limit(Some(max + 1)), max);
        assert_eq!(show_limit(Some(5)), 5, "a modest ask is honoured");
        assert_eq!(show_limit(None), default);
        // `--limit 0` means something else to dbt, and a negative is nonsense.
        assert_eq!(show_limit(Some(0)), default);
        assert_eq!(show_limit(Some(-1)), default);
    }

    // The loop this feeds is bounded by nothing else: stop spending the budget
    // and a failing job reissues `dbt retry` until its deadline.
    #[test]
    fn an_attempt_is_claimed_from_the_budget_exactly_once() {
        let mut remaining = 3;
        let mut claimed = vec![];
        // Bounded by this `for`, never by `claim_attempt`: a regression that
        // stops spending the budget has to fail an assertion here, and must not
        // be able to allocate until the machine dies. `from_fn(..).collect()`
        // would do the latter.
        for _ in 0..10 {
            match claim_attempt(&mut remaining, 3) {
                Some(n) => claimed.push(n),
                None => break,
            }
        }
        assert_eq!(claimed, vec![1, 2, 3], "numbered in order, one per attempt");
        assert_eq!(remaining, 0);
        assert_eq!(
            claim_attempt(&mut remaining, 3),
            None,
            "a spent budget grants no more"
        );
        let mut none = 0;
        assert_eq!(claim_attempt(&mut none, 0), None);
    }

    // A second `dbt test` after a tests-only retry runs every test twice and
    // reports each one twice — duplicate ids in the result table, doubled totals.
    #[test]
    fn merging_two_phases_keeps_one_row_per_node() {
        let n = |id: &str, status: &str| DbtNodeResult {
            outcome: classify_status(status).as_result_word(),
            unique_id: id.to_string(),
            status: status.to_string(),
            execution_time: None,
            rows_affected: None,
            relation_name: None,
            message: None,
            failures: None,
        };
        let mut results = vec![n("model.p.m", "success"), n("test.p.t", "fail")];
        // The test phase re-reports the same test, now passing.
        merge_results(&mut results, vec![n("test.p.t", "pass")]);
        assert_eq!(results.len(), 2, "a node re-reported must not duplicate");
        assert_eq!(
            results
                .iter()
                .find(|r| r.unique_id == "test.p.t")
                .unwrap()
                .status,
            "pass",
            "the later phase's outcome wins"
        );
    }

    // `partial success` is a node that built and then failed its tests. Read as
    // "says nothing about the relation", it left the model on `Running` — the
    // tailer writes that when the node starts and nothing after it moves the
    // record, so a finished job showed a model still building.
    #[test]
    fn partial_success_settles_the_relation_as_failed() {
        assert_eq!(classify_status("partial success"), DbtNodeOutcome::Failed);
        // The engines disagree on casing: 1.x echoes the author's, 2.x
        // uppercases. Every classifier folds, or they disagree with each other.
        for spelling in ["PARTIAL SUCCESS", " Partial Success ", "ERROR", "Pass"] {
            assert_ne!(
                classify_status(spelling),
                DbtNodeOutcome::Unknown,
                "{spelling} must classify"
            );
        }
        assert_eq!(classify_status("success"), DbtNodeOutcome::Passed);
        assert_eq!(classify_status("started"), DbtNodeOutcome::Started);
        assert_eq!(classify_status("warn"), DbtNodeOutcome::Warn);
        assert_eq!(classify_status("skipped"), DbtNodeOutcome::Skipped);
        // `no-op` is not `skipped`: nothing was built either way, but dbt's
        // retry set is error / fail / skipped, so a retry must not redo it.
        assert_eq!(classify_status("no-op"), DbtNodeOutcome::NoOp);
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
        // Failed TESTS alone are retryable: `test_behavior: after_all` is exactly
        // how `run_results.json` comes to describe tests alone.
        let tests_only = r#"{"results":[
            {"unique_id":"test.p.not_null_orders_id.ab","status":"fail"},
            {"unique_id":"test.p.unique_orders_id.cd","status":"error"}]}"#;
        assert!(has_retryable_node(tests_only));
        // A passing test-only run still has nothing to retry.
        let tests_passed = r#"{"results":[
            {"unique_id":"test.p.not_null_orders_id.ab","status":"pass"}]}"#;
        assert!(!has_retryable_node(tests_passed));
        // Unreadable results let dbt decide rather than refusing a retry the
        // user may well need.
        assert!(has_retryable_node("not json"));
    }

    /// An interrupted BUILD leaves the warehouse in a state the previous run's
    /// failures no longer describe, so that state has to go — including its local
    /// generation, which a retry landing back on this worker reads first.
    #[tokio::test]
    async fn invalidating_run_state_drops_the_local_generation() {
        let dir = state_dir("ws", "f/a/one", "u/alice");
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::write(dir.join(CURRENT_GENERATION), b"gen-old")
            .await
            .unwrap();

        let http = Connection::Http(windmill_common::worker::HttpClient {
            client: reqwest_middleware::ClientBuilder::new(reqwest::Client::new()).build(),
            base_internal_url: String::new(),
        });
        invalidate_run_state("ws", "f/a/one", "u/alice", &http).await;

        assert!(
            !dir.join(CURRENT_GENERATION).exists(),
            "the pointer a retry reads must be gone"
        );
    }

    /// The tailer runs in the WORKER process, outside the jailed child's memory
    /// limit, and a dbt macro decides how long a log line is. An unbounded line
    /// must be dropped rather than held, and the events after it must still
    /// arrive — a tailer that stops reporting is a run with a blank graph.
    #[test]
    fn an_oversized_log_line_is_dropped_without_stopping_the_tailer() {
        let mut tail = LogTail::default();
        let event = |name: &str| format!(r#"{{"info":{{"name":"{name}"}}}}"#);

        // A line split across reads is reassembled, not dropped.
        assert_eq!(tail.push("{\"a\":1"), "");
        assert_eq!(tail.push(",\"b\":2}\n"), "{\"a\":1,\"b\":2}\n");

        // One that never ends is discarded, in bounded memory, ...
        let huge = "x".repeat(LOG_LINE_MAX_BYTES / 2 + 1);
        assert_eq!(tail.push(&huge), "");
        assert_eq!(tail.push(&huge), "");
        assert!(tail.carry.is_empty(), "the over-long line is not held");
        assert_eq!(tail.push(&huge), "", "still inside that line");

        // ... through its end, after which the next events come through.
        let resumed = tail.push(&format!("tail-of-the-huge-line\n{}\n", event("ok")));
        assert_eq!(resumed, format!("{}\n", event("ok")));
        assert_eq!(
            tail.push(&format!("{}\n", event("next"))),
            format!("{}\n", event("next"))
        );
    }

    /// A command block that names nothing must not read as "no command given":
    /// that is the descriptor's default, so a mistyped `show` would BUILD the
    /// project — the one direction a read-only command must never fail in.
    /// Argument-schema validation is opt-in, so this is the only check between a
    /// direct request and the engine.
    #[test]
    fn a_command_block_that_names_no_command_is_refused() {
        let args = |json: &str| {
            serde_json::from_str::<HashMap<String, Box<RawValue>>>(json).expect("test payload")
        };
        for payload in [
            r#"{"command": "show"}"#,
            r#"{"command": {}}"#,
            r#"{"command": {"select": ["a"]}}"#,
            r#"{"command": {"label": null}}"#,
            r#"{"command": {"label": ""}}"#,
            r#"{"command": []}"#,
        ] {
            let err = flatten_command(args(payload))
                .expect_err("a block naming no command must not fall back to the default");
            assert!(
                err.to_string().contains("naming what to run"),
                "{payload}: {err}"
            );
        }
        // Absent and `null` both mean "the descriptor's own command", which is
        // what an empty run submits.
        for payload in ["{}", r#"{"command": null}"#] {
            let out = flatten_command(args(payload)).expect(payload);
            assert!(!out.contains_key("dbt_command"), "{payload}");
        }
        // A named one spreads, its label under the name every reader uses.
        let out = flatten_command(args(r#"{"command": {"label": "show", "limit": 3}}"#)).unwrap();
        assert_eq!(
            arg_str(&out, "dbt_command").unwrap().as_deref(),
            Some("show")
        );
        assert_eq!(arg_i64(&out, "limit").unwrap(), Some(3));
        assert!(!out.contains_key(DBT_COMMAND_ARG));
        // The variant's label decides, not map iteration order: a block carrying a
        // `dbt_command` of its own lands on the same key.
        let both = flatten_command(args(
            r#"{"command": {"label": "show", "dbt_command": "build"}}"#,
        ))
        .unwrap();
        assert_eq!(
            arg_str(&both, "dbt_command").unwrap().as_deref(),
            Some("show")
        );
    }

    /// An agent reaches no database, so the row that answers "is this the run you
    /// asked to resume" is not there — only the generation the worker itself
    /// wrote. Unchecked, a retry naming one failed run resumes whichever failed
    /// last on that worker, which is what the run page's two retry actions would
    /// otherwise do from any older run.
    #[test]
    fn an_agent_refuses_a_generation_the_caller_did_not_name() {
        let http = Connection::Http(windmill_common::worker::HttpClient {
            client: reqwest_middleware::ClientBuilder::new(reqwest::Client::new()).build(),
            base_internal_url: String::new(),
        });
        let saved = uuid::Uuid::new_v4();
        let asked_for = uuid::Uuid::new_v4();
        let local = || Some(format!("gen-{saved}"));

        let err = chosen_generation(local(), &http, None, asked_for)
            .expect_err("a generation the caller did not name must not be resumed");
        assert!(
            err.to_string().contains(&saved.to_string())
                && err.to_string().contains(&asked_for.to_string()),
            "the refusal names both the run asked for and the one held, got: {err}"
        );

        assert_eq!(
            chosen_generation(local(), &http, None, saved).unwrap(),
            local(),
            "the run it does hold still resumes"
        );
    }

    /// Why a run re-ingests decides what becomes of the result. Drift is the one
    /// that MUST publish: the check reads back the published root, so a drifted
    /// run that only snapshots leaves the next run detecting the same move, and
    /// the asset rows naming the schema the project no longer builds into.
    #[test]
    fn what_a_refresh_publishes_depends_on_why_it_happened() {
        let job = uuid::Uuid::new_v4();
        let arg = |k: &str, v: &str| {
            HashMap::from([(k.to_string(), RawValue::from_string(v.to_string()).unwrap())])
        };
        let descriptor = DbtDescriptor::default();

        // A moved profile relocates the VERSION's relations: its own graph, and
        // the ownership that answers the next drift check.
        let drift = GraphRefresh { profile_drift: true, ..Default::default() };
        assert!(drift.needed());
        assert_eq!(drift.snapshot_job(job), None);
        assert!(drift.publishes_ownership());

        // A dynamic descriptor: this run's graph, under its own job id — and
        // nothing published, because the `asset` rows and the version's nodes
        // must describe one picture, and the version's are the deploy's.
        let dynamic = GraphRefresh { per_run_models: true, ..Default::default() };
        assert_eq!(dynamic.snapshot_job(job), Some(job));
        assert!(!dynamic.publishes_ownership());

        // A `vars` override: this run's graph, and only this run's.
        let mut overridden = GraphRefresh::default();
        overridden
            .add_caller_args(&descriptor, &arg("vars", r#"{"day":"2026-07-29"}"#))
            .unwrap();
        assert!(overridden.needed());
        assert_eq!(overridden.snapshot_job(job), Some(job));
        assert!(!overridden.publishes_ownership());

        // A caller's selection is not necessarily a subset of the deployed one —
        // `["*"]` against `tag:nightly` builds models the deployed graph never had,
        // which the run page would have nothing to draw. Under its own job id, so it
        // neither becomes what the script owns nor replaces the version's graph.
        let mut narrowed = GraphRefresh::default();
        narrowed
            .add_caller_args(&descriptor, &arg("select", r#"["stg_orders"]"#))
            .unwrap();
        assert!(narrowed.needed());
        assert_eq!(narrowed.snapshot_job(job), Some(job));
        assert!(!narrowed.publishes_ownership());

        // The form posts the descriptor's own `select` back for every UI, schedule
        // and webhook run. Reading that echo as a narrowing marks all of them
        // caller-scoped, leaving a moved profile no run that could republish: it
        // re-detects the same drift, and pays a `dbt parse` for it, forever.
        let echoed =
            DbtDescriptor { select: vec!["tag:nightly".to_string()], ..Default::default() };
        let mut untouched = GraphRefresh { profile_drift: true, ..Default::default() };
        untouched
            .add_caller_args(&echoed, &arg("select", r#"["tag:nightly"]"#))
            .unwrap();
        assert!(untouched.publishes_ownership());
        assert_eq!(untouched.snapshot_job(job), None);
    }

    // `dbt retry` restores the previous run's target/ from this directory, so two
    // dbt scripts must not share one — a retry would resume another project's
    // `run_results.json`, which an empty script_path is exactly how it happened.
    #[test]
    fn retry_state_is_per_script_and_principal() {
        assert_ne!(
            state_dir("ws", "f/a/one", "u"),
            state_dir("ws", "f/a/two", "u")
        );
        assert_ne!(
            state_dir("ws1", "f/a/one", "u"),
            state_dir("ws2", "f/a/one", "u")
        );
        assert_eq!(
            state_dir("ws", "f/a/one", "u"),
            state_dir("ws", "f/a/one", "u")
        );
        // A retry replaces the caller's arguments with the saved ones, and an
        // agent worker has no database row to fall back on: this separation is
        // the only thing keeping one principal's `select`/`vars` from another.
        assert_ne!(
            state_dir("ws", "f/a/one", "u/alice"),
            state_dir("ws", "f/a/one", "u/bob")
        );
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
