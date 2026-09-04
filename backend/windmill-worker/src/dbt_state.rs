//! The dbt state a project last built into one environment, and the state
//! directory a run reads it back through.
//!
//! `dbt --defer --state <dir>` resolves a `ref()` the run does not build to the
//! relation the manifest in `<dir>` names, instead of to the schema this run
//! writes into. That makes the state a durable, per-environment artifact rather
//! than a cache: the next run of a project usually lands on a worker holding
//! neither the manifest nor the results, so anything worker-local answers for
//! one machine's history rather than for the environment.
//!
//! Two artifacts live in that directory and both are stored: `manifest.json`,
//! which is what a deferral resolves through, and `run_results.json`, which
//! `select`'s `result:` selectors read — and `select` reaches dbt verbatim, so a
//! state directory missing it fails a selection a user may legitimately write.

use std::path::{Path, PathBuf};

use uuid::Uuid;
use windmill_common::client::AuthedClient;
use windmill_common::error::{self, Error};
use windmill_common::worker::Connection;

use crate::dbt_executor::{digest, PreparedProject, ARTIFACTS_DIR};

lazy_static::lazy_static! {
    /// Above this, an artifact goes to the workspace's object storage instead of
    /// into the row. A manifest passes a few hundred KB on a handful of models
    /// and grows with the project, so this ceiling is what decides whether a
    /// large project needs storage configured at all; a small one stays in the
    /// database, where it costs no round trip and needs nothing configured.
    static ref DBT_STATE_INLINE_MAX_BYTES: usize = std::env::var("DBT_STATE_INLINE_MAX_BYTES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8 * 1024 * 1024);
}

/// The directory `--state` points at. Inside the job directory, so it sits in
/// the sandbox's one writable bind and goes away with the job, and prefixed like
/// the artifacts directory beside it so a project carrying a directory of this
/// name is not overwritten.
///
/// Passed to dbt RELATIVE, and that is load-bearing rather than tidiness. dbt
/// records the invocation's flags into `run_results.json` and a later
/// `dbt retry` restores them, so an absolute path would name the job directory
/// of the run being resumed — gone by then, leaving the retry to resolve a
/// deferred `ref()` against nothing. Relative, it resolves against the project
/// root, which is whichever job directory the retry landed in.
pub(crate) const STATE_DIR: &str = "wm_dbt_state";

/// Where this run's relations live, which is the only thing a deferral is about.
pub(crate) fn environment(p: &PreparedProject) -> String {
    environment_key(
        p.warehouse.as_deref(),
        p.target.as_deref(),
        &p.relation_root(),
    )
}

/// The warehouse and the target name the environment; the database and schema
/// they resolve to are in the key because a repointed warehouse resource or a
/// moved schema keeps both names while putting the relations somewhere else —
/// and a manifest is a list of relation names, so a deferral has no other way to
/// notice. A move therefore reads as an environment nothing has published yet.
///
/// `relation_root` rather than the two fields, so the one definition of where a
/// run's relations live serves both this and the graph's drift check.
fn environment_key(warehouse: Option<&str>, target: Option<&str>, relation_root: &str) -> String {
    format!(
        "{}|{}|{}",
        warehouse.unwrap_or(""),
        target.unwrap_or(""),
        relation_root,
    )
}

/// The state one environment last published.
pub(crate) struct StoredState {
    pub manifest: String,
    pub run_results: Option<String>,
    /// The run that published it, so a deferring run can say what it deferred to.
    pub job_id: Uuid,
}

/// Publish this run's artifacts as the environment's state.
///
/// Called for a run that BUILT what the script's own descriptor selects and
/// succeeded (see `handle_dbt_job`). Best-effort in the same sense as the retry
/// state: losing it costs the next deferral, not the run that just finished.
pub(crate) async fn publish(
    p: &PreparedProject,
    w_id: &str,
    job_id: &Uuid,
    conn: &Connection,
    client: &AuthedClient,
) -> error::Result<()> {
    let Connection::Sql(db) = conn else {
        // An agent worker reaches the database only through the API, which does
        // not expose this table.
        return Ok(());
    };
    if p.script_path.is_empty() {
        // A preview has no path to key state on, and an empty one would be
        // shared by every dbt script in the workspace.
        return Ok(());
    }
    let artifacts = p.project_dir.join(ARTIFACTS_DIR);
    // The manifest is what a deferral resolves through, so there is no state
    // without one. Every engine writes it beside the results of a build, so this
    // is the invocation that built nothing rather than a case to report.
    let Ok(manifest) = tokio::fs::read_to_string(artifacts.join("manifest.json")).await else {
        return Ok(());
    };
    let run_results = tokio::fs::read_to_string(artifacts.join("run_results.json"))
        .await
        .ok();
    let environment = environment(p);
    let manifest = store(
        manifest,
        "manifest.json",
        &environment,
        &p.script_path,
        w_id,
        job_id,
        client,
        conn,
    )
    .await?;
    let run_results = match run_results {
        Some(r) => Some(
            store(
                r,
                "run_results.json",
                &environment,
                &p.script_path,
                w_id,
                job_id,
                client,
                conn,
            )
            .await?,
        ),
        None => None,
    };
    let (manifest, manifest_key) = split(Some(manifest));
    let (run_results, run_results_key) = split(run_results);
    sqlx::query!(
        "INSERT INTO dbt_environment_state (workspace_id, script_path, environment, job_id,
                                            manifest, manifest_key, run_results, run_results_key,
                                            updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, now())
         ON CONFLICT (workspace_id, script_path, environment) DO UPDATE SET
           job_id = EXCLUDED.job_id, manifest = EXCLUDED.manifest,
           manifest_key = EXCLUDED.manifest_key, run_results = EXCLUDED.run_results,
           run_results_key = EXCLUDED.run_results_key, updated_at = now()",
        w_id,
        &p.script_path,
        environment,
        job_id,
        manifest,
        manifest_key,
        run_results,
        run_results_key,
    )
    .execute(db)
    .await?;
    Ok(())
}

/// The environment's state, or `None` where nothing has published one.
pub(crate) async fn load(
    p: &PreparedProject,
    w_id: &str,
    job_id: &Uuid,
    conn: &Connection,
    client: &AuthedClient,
) -> error::Result<Option<StoredState>> {
    let Connection::Sql(db) = conn else {
        return Err(Error::BadRequest(
            "`defer` resolves a `ref()` through the dbt state stored for this environment, which \
             an agent worker cannot read: it reaches the database only through the API. Run this \
             script on a worker of the main group, or without `defer`"
                .to_string(),
        ));
    };
    let environment = environment(p);
    let Some(row) = sqlx::query!(
        "SELECT job_id, manifest, manifest_key, run_results, run_results_key
           FROM dbt_environment_state
          WHERE workspace_id = $1 AND script_path = $2 AND environment = $3",
        w_id,
        &p.script_path,
        environment
    )
    .fetch_optional(db)
    .await?
    else {
        return Ok(None);
    };
    let Some(manifest) = fetch(row.manifest, row.manifest_key, w_id, job_id, client, conn).await?
    else {
        return Ok(None);
    };
    let run_results = fetch(
        row.run_results,
        row.run_results_key,
        w_id,
        job_id,
        client,
        conn,
    )
    .await?;
    Ok(Some(StoredState {
        manifest,
        run_results,
        job_id: row.job_id,
    }))
}

/// A `manifest.json` for a state directory, whichever side it comes from.
///
/// One enum because the three restores — a deferral's stored state, a retry's
/// worker-local generation, a retry's database row — differ only in where the
/// bytes are, and a second copy of the directory layout is a second chance for
/// one of them to write a directory dbt reads differently.
pub(crate) enum StateManifest {
    Bytes(String),
    /// A file on this worker, copied rather than read into memory: a manifest
    /// grows with the project.
    CopyOf(PathBuf),
    None,
}

/// Write the artifacts a dbt state directory holds, creating it if needed.
///
/// Returns whether a `manifest.json` ended up there — a worker-local generation
/// can be pruned out from under a restore, and the caller then owes a
/// `dbt parse` for one.
pub(crate) async fn write_state_dir(
    dir: &Path,
    run_results: Option<&str>,
    manifest: StateManifest,
) -> error::Result<bool> {
    tokio::fs::create_dir_all(dir)
        .await
        .map_err(|e| Error::internal_err(format!("preparing the dbt state directory: {e}")))?;
    if let Some(run_results) = run_results {
        tokio::fs::write(dir.join("run_results.json"), run_results)
            .await
            .map_err(|e| Error::internal_err(format!("writing run_results.json: {e}")))?;
    }
    Ok(match manifest {
        StateManifest::Bytes(m) => {
            tokio::fs::write(dir.join("manifest.json"), m)
                .await
                .map_err(|e| Error::internal_err(format!("writing manifest.json: {e}")))?;
            true
        }
        StateManifest::CopyOf(from) => tokio::fs::copy(from, dir.join("manifest.json"))
            .await
            .is_ok(),
        StateManifest::None => false,
    })
}

/// Where a stored artifact lives.
enum Home {
    /// Small enough to sit in the row.
    Inline(String),
    /// In the workspace's object storage, under this key.
    Stored(String),
}

fn split(home: Option<Home>) -> (Option<String>, Option<String>) {
    match home {
        Some(Home::Inline(v)) => (Some(v), None),
        Some(Home::Stored(k)) => (None, Some(k)),
        None => (None, None),
    }
}

/// The object-storage key an artifact takes.
///
/// Derived from the row's own key rather than randomly, so a republish
/// overwrites in place and the store holds one object per artifact per
/// environment however many times a project runs. Digested because a Windmill
/// path and a schema name may both carry characters an object key gives meaning
/// to.
fn object_key(w_id: &str, script_path: &str, environment: &str, artifact: &str) -> String {
    format!(
        "wmill_dbt_state/{w_id}/{}/{artifact}",
        digest(&format!("{script_path}|{environment}"))
    )
}

/// Put an artifact where its size says it belongs.
#[allow(clippy::too_many_arguments)]
async fn store(
    value: String,
    artifact: &str,
    environment: &str,
    script_path: &str,
    w_id: &str,
    job_id: &Uuid,
    client: &AuthedClient,
    conn: &Connection,
) -> error::Result<Home> {
    if value.len() <= *DBT_STATE_INLINE_MAX_BYTES {
        return Ok(Home::Inline(value));
    }
    let key = object_key(w_id, script_path, environment, artifact);
    let size = value.len();
    if put_object(&key, value, w_id, job_id, client, conn).await? {
        return Ok(Home::Stored(key));
    }
    Err(Error::BadRequest(format!(
        "this project's {artifact} is {}, past the {} this instance keeps in the database, and \
         the workspace has no object storage configured to hold it. Configure workspace object \
         storage, or raise DBT_STATE_INLINE_MAX_BYTES",
        mib(size),
        mib(*DBT_STATE_INLINE_MAX_BYTES),
    )))
}

fn mib(bytes: usize) -> String {
    format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
}

/// Read an artifact back from whichever home the row names.
async fn fetch(
    inline: Option<String>,
    key: Option<String>,
    w_id: &str,
    job_id: &Uuid,
    client: &AuthedClient,
    conn: &Connection,
) -> error::Result<Option<String>> {
    match (inline, key) {
        (Some(inline), _) => Ok(Some(inline)),
        (None, Some(key)) => get_object(&key, w_id, job_id, client, conn).await.map(Some),
        (None, None) => Ok(None),
    }
}

/// The workspace's object storage, or `None` where it is not configured or not
/// reachable. Failing to reach it is reported here and answered by the caller:
/// a store falls back to the size error, and a fetch to "the state is no longer
/// where the row says it is".
#[cfg(feature = "parquet")]
async fn workspace_store(
    w_id: &str,
    job_id: &Uuid,
    client: &AuthedClient,
    conn: &Connection,
) -> Option<std::sync::Arc<dyn windmill_object_store::object_store_reexports::ObjectStore>> {
    let Connection::Sql(db) = conn else {
        return None;
    };
    let resource = crate::common::get_workspace_s3_resource_path(db, client, w_id, None, job_id)
        .await
        .inspect_err(|e| tracing::warn!("dbt: resolving the workspace object storage: {e:#}"))
        .ok()
        .flatten()?;
    windmill_object_store::build_object_store_client(&resource)
        .await
        .inspect_err(|e| tracing::warn!("dbt: reaching the workspace object storage: {e:#}"))
        .ok()
}

/// Whether the artifact was stored. `false` means the workspace has no object
/// storage this worker can write to.
#[cfg(feature = "parquet")]
async fn put_object(
    key: &str,
    value: String,
    w_id: &str,
    job_id: &Uuid,
    client: &AuthedClient,
    conn: &Connection,
) -> error::Result<bool> {
    use windmill_object_store::object_store_reexports::Path as ObjectPath;
    let Some(store) = workspace_store(w_id, job_id, client, conn).await else {
        return Ok(false);
    };
    store
        .put(&ObjectPath::from(key), bytes::Bytes::from(value).into())
        .await
        .map_err(|e| Error::internal_err(format!("storing the dbt state at {key}: {e:#}")))?;
    Ok(true)
}

#[cfg(feature = "parquet")]
async fn get_object(
    key: &str,
    w_id: &str,
    job_id: &Uuid,
    client: &AuthedClient,
    conn: &Connection,
) -> error::Result<String> {
    let Some(store) = workspace_store(w_id, job_id, client, conn).await else {
        return Err(missing_storage());
    };
    let bytes = windmill_object_store::attempt_fetch_bytes(store, key).await?;
    String::from_utf8(bytes.to_vec())
        .map_err(|e| Error::internal_err(format!("the stored dbt state is not valid UTF-8: {e}")))
}

/// A build without `parquet` carries no object-store client at all, so an
/// oversized artifact has nowhere but the row and a row naming a key was written
/// by a worker that did.
#[cfg(not(feature = "parquet"))]
async fn put_object(
    _key: &str,
    _value: String,
    _w_id: &str,
    _job_id: &Uuid,
    _client: &AuthedClient,
    _conn: &Connection,
) -> error::Result<bool> {
    Ok(false)
}

#[cfg(not(feature = "parquet"))]
async fn get_object(
    _key: &str,
    _w_id: &str,
    _job_id: &Uuid,
    _client: &AuthedClient,
    _conn: &Connection,
) -> error::Result<String> {
    Err(missing_storage())
}

fn missing_storage() -> Error {
    Error::BadRequest(
        "the dbt state for this environment is in the workspace's object storage, which this \
         worker cannot reach: it is no longer configured, or this worker was built without \
         object-storage support"
            .to_string(),
    )
}

/// The stored state this run resolves its unbuilt `ref()`s through, materialised
/// into the job directory at `STATE_DIR`.
#[derive(Clone, Debug)]
pub(crate) struct Deferral {
    /// The run that published the state, so the job log and the result can say
    /// what this one deferred to.
    pub published_by: Uuid,
}

/// Materialise the environment's state so `--state` has a directory to read.
///
/// Refused rather than run without deferral where nothing is published: the run
/// would build against a `ref()` resolving into the schema it writes, and fail
/// deep inside dbt with a relation-not-found the caller has no way to connect
/// back to a missing state.
pub(crate) async fn prepare_deferral(
    p: &PreparedProject,
    w_id: &str,
    job_id: &Uuid,
    job_dir: &str,
    conn: &Connection,
    client: &AuthedClient,
) -> error::Result<Deferral> {
    if p.script_path.is_empty() {
        return Err(Error::BadRequest(
            "`defer` resolves a `ref()` through the state a previous run of this script \
             published, so it needs a deployed script; a preview run has no environment to have \
             published one"
                .to_string(),
        ));
    }
    let Some(state) = load(p, w_id, job_id, conn, client).await? else {
        return Err(Error::BadRequest(format!(
            "no dbt state is stored for this environment ({}), so a `ref()` this run does not \
             build has no relation to resolve to. Run this script once without `defer`: a \
             successful run of the descriptor's own selection publishes it",
            environment(p)
        )));
    };
    write_state_dir(
        &PathBuf::from(job_dir).join(STATE_DIR),
        state.run_results.as_deref(),
        StateManifest::Bytes(state.manifest),
    )
    .await?;
    Ok(Deferral { published_by: state.job_id })
}

#[cfg(test)]
mod tests {
    use super::*;

    // Every component earns its place: a deferral resolves relation NAMES, so
    // state published where those names meant something else has to read as no
    // state at all rather than as state that silently no longer fits.
    #[test]
    fn a_moved_profile_is_another_environment() {
        let here = environment_key(Some("main"), Some("prod"), "analytics|warehouse");
        assert_eq!(
            here,
            environment_key(Some("main"), Some("prod"), "analytics|warehouse")
        );
        assert_ne!(
            here,
            environment_key(Some("other"), Some("prod"), "analytics|warehouse")
        );
        assert_ne!(
            here,
            environment_key(Some("main"), Some("dev"), "analytics|warehouse")
        );
        assert_ne!(
            here,
            environment_key(Some("main"), Some("prod"), "marts|warehouse")
        );
        assert_ne!(
            here,
            environment_key(Some("main"), Some("prod"), "analytics|other_db")
        );
    }
}
