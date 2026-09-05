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
use windmill_common::error::{self, Error};
use windmill_common::worker::Connection;

use crate::dbt_executor::{digest, PreparedProject, ARTIFACTS_DIR};

lazy_static::lazy_static! {
    /// Above this, an artifact goes to the instance's object storage instead of
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
        // The target dbt RUNS, not the descriptor's: it falls back to the
        // workspace warehouse's and to the project's own default, so reading the
        // descriptor's would put two inherited targets under one empty name.
        p.effective_target.as_deref(),
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
///
/// **What the artifacts may carry follows from that condition.** A publishing run
/// added nothing of its own — no `select` or `vars` override, and a descriptor
/// interpolating a `{{ }}` placeholder into `vars` never publishes at all — so
/// dbt's `run_results.json` records the descriptor's own arguments, which are the
/// script's content. That is why this is keyed by environment where
/// `dbt_run_state` is keyed by principal: the retry state holds whatever a caller
/// submitted, this holds what the script says. Widen the publish condition and
/// that stops being true.
pub(crate) async fn publish(
    p: &PreparedProject,
    w_id: &str,
    job_id: &Uuid,
    // The version this job ran. `None` for a preview, which publishes nothing.
    script_hash: Option<i64>,
    // A build recovered by the automatic in-job node retry has a
    // `run_results.json` naming only the nodes that retry redid. The manifest is
    // unaffected — it is a function of the project, not of what ran — so the
    // state is published without results rather than with a set describing some
    // other slice of the build.
    results_are_partial: bool,
    conn: &Connection,
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
    if p.templated_location {
        // Refused on this side too, not only where a deferral reads. A template
        // renders to one location per environment while the key sees the
        // template, so publishing would file this run's manifest under a key a
        // literal profile shares — and de-templating later would make that stale
        // manifest readable as the new location's.
        return Ok(());
    }
    let artifacts = p.project_dir.join(ARTIFACTS_DIR);
    // The manifest is what a deferral resolves through, so there is no state
    // without one. Every engine writes it beside the results of a build, so this
    // is the invocation that built nothing rather than a case to report.
    let Ok(manifest) = tokio::fs::read_to_string(artifacts.join("manifest.json")).await else {
        return Ok(());
    };
    let run_results = match results_are_partial {
        true => None,
        false => tokio::fs::read_to_string(artifacts.join("run_results.json"))
            .await
            .ok(),
    };
    let environment = environment(p);
    // Uploaded BEFORE the transaction, and to this publication's own keys, so two
    // publishers cannot collide on them and nothing here can overwrite an
    // artifact a committed row still names. A failure below has only its own
    // objects to drop.
    let nonce = Uuid::new_v4();
    let (manifest, manifest_key) = store(
        manifest,
        "manifest.json",
        &environment,
        &p.script_path,
        w_id,
        job_id,
        &nonce,
    )
    .await?;
    let (run_results, run_results_key) = match run_results {
        Some(r) => match store(
            r,
            "run_results.json",
            &environment,
            &p.script_path,
            w_id,
            job_id,
            &nonce,
        )
        .await
        {
            Ok(stored) => stored,
            Err(e) => {
                forget_objects(&[manifest_key, None]).await;
                return Err(e);
            }
        },
        None => (None, None),
    };
    let mine = [manifest_key.clone(), run_results_key.clone()];
    // One publisher per environment at a time, so the row and the objects it
    // displaces are settled by one of them at a time. An advisory lock rather
    // than the row's, because the first publish of an environment has no row to
    // lock and is exactly when two runs of a newly deployed script are most
    // likely to race.
    let mut tx = match db.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            forget_objects(&mine).await;
            return Err(e.into());
        }
    };
    let staged = async {
        sqlx::query_scalar!(
            "SELECT pg_advisory_xact_lock($1)",
            publication_lock(w_id, &p.script_path, &environment)
        )
        .execute(&mut *tx)
        .await?;
        // The script row FIRST, and held, so a rename, archive or delete of this
        // path either waits for this publication or is seen by it. Reading it
        // unlocked leaves a window where lifecycle cleanup finds no row to clear,
        // finishes, and this transaction then commits state at a path a new
        // script goes on to occupy. Script row before sidecar is also the order
        // every other dbt writer takes, which is what keeps the two off a
        // deadlock.
        //
        // The version, not just the path: "some live dbt script is here" is also
        // satisfied by a script created at a path this one was renamed away from.
        // A preview names no version, so `script_hash` is NULL and nothing
        // matches — right for a run of content that was never deployed.
        let owns_path = sqlx::query_scalar!(
            "SELECT 1 FROM script
              WHERE workspace_id = $1 AND path = $2
                AND deleted = false AND archived = false AND language = 'dbt'
                AND (hash = $3 OR $3 = ANY(parent_hashes))
              FOR SHARE",
            w_id,
            &p.script_path,
            script_hash,
        )
        .fetch_optional(&mut *tx)
        .await?
        .is_some();
        if !owns_path {
            return error::Result::Ok(None);
        }
        // What the row points at NOW, so those objects can go once this one is
        // committed in their place — never before, since a reader that has
        // already read the row is about to fetch them.
        let displaced = sqlx::query!(
            "SELECT manifest_key, run_results_key FROM dbt_environment_state
              WHERE workspace_id = $1 AND script_path = $2 AND environment = $3",
            w_id,
            &p.script_path,
            environment
        )
        .fetch_optional(&mut *tx)
        .await?
        .map(|r| [r.manifest_key, r.run_results_key])
        .unwrap_or_default()
        // Never a key this publication is about to commit. The keys carry a
        // per-execution nonce so the two cannot coincide, and this is what says
        // so rather than leaving it to be re-derived.
        .map(|k| k.filter(|k| !mine.iter().flatten().any(|m| m == k)));
        sqlx::query!(
            "INSERT INTO dbt_environment_state (workspace_id, script_path, environment, job_id,
                                                manifest, manifest_key, run_results,
                                                run_results_key, updated_at)
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
        .execute(&mut *tx)
        .await?;
        error::Result::Ok(Some(displaced))
    }
    .await;
    let displaced = match staged {
        // Refused by the guard, or the write failed: nothing is committed and
        // what was uploaded above has no row naming it.
        Ok(None) | Err(_) => {
            forget_objects(&mine).await;
            return staged.map(|_| ());
        }
        Ok(Some(displaced)) => displaced,
    };
    // A commit that reports an error may still have committed — what was lost can
    // be the acknowledgement. Dropping this run's objects would then leave the
    // committed row naming objects that are gone, and every deferral would fail
    // until the next publication; an orphan costs storage instead.
    tx.commit().await?;
    forget_objects(&displaced).await;
    Ok(())
}

/// The environment's state, or `None` where nothing has published one.
pub(crate) async fn load(
    p: &PreparedProject,
    w_id: &str,
    conn: &Connection,
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
    // Twice at most: a publish committing between the row and the objects it
    // named drops those objects, so a miss is one re-read rather than a run
    // refused for a state that is there.
    let mut attempts = 0;
    loop {
        attempts += 1;
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
        let job_id = row.job_id;
        let fetched = async {
            let manifest = fetch(row.manifest, row.manifest_key).await?;
            let run_results = fetch(row.run_results, row.run_results_key).await?;
            error::Result::Ok((manifest, run_results))
        }
        .await;
        match fetched {
            Ok((Some(manifest), run_results)) => {
                return Ok(Some(StoredState { manifest, run_results, job_id }))
            }
            Ok((None, _)) => return Ok(None),
            Err(_) if attempts < 2 => continue,
            Err(e) => return Err(e),
        }
    }
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

/// The advisory lock one environment's publishers take, so only one of them
/// settles the row and the objects it displaces at a time.
///
/// Derived from the same three components as the row's key. Two environments
/// whose digests collide wait for each other, which costs a moment and nothing
/// else.
fn publication_lock(w_id: &str, script_path: &str, environment: &str) -> i64 {
    let d = digest(&format!("{w_id}|{script_path}|{environment}"));
    // Parsed unsigned and reinterpreted: half of all digests set the top bit,
    // and read as `i64` those overflow and would collapse onto one key.
    u64::from_str_radix(&d[..16], 16).unwrap_or_default() as i64
}

/// The object-storage key an artifact takes.
///
/// One key per PUBLICATION, so an upload never overwrites an artifact the
/// committed row still names: a run that fails between its two uploads, or
/// between them and its row, leaves the state pointing at the pair it already
/// had. The row switches to these in one statement and the objects it displaced
/// are dropped afterwards. The path and environment are only a prefix — the row
/// is what says where an artifact is, so state that moves with a renamed script
/// keeps naming objects under the old one. Digested because a Windmill path and a
/// schema name may both carry characters an object key gives meaning to.
///
/// The `nonce` is per EXECUTION rather than per job, because zombie recovery
/// re-runs a job under its own id: keyed on that alone, the second attempt would
/// overwrite the objects the first attempt's committed row still names, and then
/// read those same keys back as displaced and drop them.
fn object_key(
    w_id: &str,
    script_path: &str,
    environment: &str,
    job_id: &Uuid,
    nonce: &Uuid,
    artifact: &str,
) -> String {
    format!(
        "wmill_dbt_state/{w_id}/{}/{job_id}.{nonce}/{artifact}",
        digest(&format!("{script_path}|{environment}"))
    )
}

/// Put an artifact where its size says it belongs: `(inline, key)`, exactly one
/// of which is set.
#[allow(clippy::too_many_arguments)]
async fn store(
    value: String,
    artifact: &str,
    environment: &str,
    script_path: &str,
    w_id: &str,
    job_id: &Uuid,
    nonce: &Uuid,
) -> error::Result<(Option<String>, Option<String>)> {
    if value.len() <= *DBT_STATE_INLINE_MAX_BYTES {
        return Ok((Some(value), None));
    }
    let key = object_key(w_id, script_path, environment, job_id, nonce, artifact);
    let size = value.len();
    if put_object(&key, value).await? {
        return Ok((None, Some(key)));
    }
    Err(Error::BadRequest(format!(
        "this project's {artifact} is {}, past the {} this instance keeps in the database, and \
         this instance has no object storage configured to hold it. Configure instance object \
         storage, or raise DBT_STATE_INLINE_MAX_BYTES",
        mib(size),
        mib(*DBT_STATE_INLINE_MAX_BYTES),
    )))
}

fn mib(bytes: usize) -> String {
    format!("{:.1} MiB", bytes as f64 / (1024.0 * 1024.0))
}

/// Read an artifact back from whichever home the row names.
async fn fetch(inline: Option<String>, key: Option<String>) -> error::Result<Option<String>> {
    match (inline, key) {
        (Some(inline), _) => Ok(Some(inline)),
        (None, Some(key)) => get_object(&key).await.map(Some),
        (None, None) => Ok(None),
    }
}

/// Drop the objects nothing points at any more. Best-effort: an object left
/// behind costs storage, and there is nothing useful to do about it in the path
/// of a run that has already finished.
async fn forget_objects(keys: &[Option<String>; 2]) {
    for key in keys.iter().flatten() {
        delete_object(key).await;
    }
}

/// Whether the artifact was stored. `false` means this instance has no object
/// storage to put it in.
///
/// The INSTANCE store, where every other internal worker artifact lives — bun
/// bundles, python wheels, job logs, the global cache. Not the workspace's:
/// that bucket is the one workspace members read and write through
/// `job_helpers/*` and `wmill.write_s3_file`, so a manifest there is one any
/// member could replace, and the next deferring run would hand dbt an
/// attacker-chosen `defer_relation` for every unbuilt `ref()` while holding the
/// script's warehouse credentials. Its compiled SQL would be readable there too,
/// for a project the reader may have no access to.
#[cfg(all(feature = "enterprise", feature = "parquet"))]
async fn put_object(key: &str, value: String) -> error::Result<bool> {
    use windmill_object_store::object_store_reexports::Path as ObjectPath;
    let Some(store) = windmill_object_store::get_object_store().await else {
        return Ok(false);
    };
    store
        .put(&ObjectPath::from(key), bytes::Bytes::from(value).into())
        .await
        .map_err(|e| Error::internal_err(format!("storing the dbt state at {key}: {e:#}")))?;
    Ok(true)
}

#[cfg(all(feature = "enterprise", feature = "parquet"))]
async fn get_object(key: &str) -> error::Result<String> {
    let Some(store) = windmill_object_store::get_object_store().await else {
        return Err(missing_storage());
    };
    let bytes = windmill_object_store::attempt_fetch_bytes(store, key).await?;
    String::from_utf8(bytes.to_vec())
        .map_err(|e| Error::internal_err(format!("the stored dbt state is not valid UTF-8: {e}")))
}

#[cfg(all(feature = "enterprise", feature = "parquet"))]
async fn delete_object(key: &str) {
    use windmill_object_store::object_store_reexports::Path as ObjectPath;
    let Some(store) = windmill_object_store::get_object_store().await else {
        return;
    };
    if let Err(e) = store.delete(&ObjectPath::from(key)).await {
        tracing::warn!("dbt: could not drop the superseded state object {key}: {e:#}");
    }
}

#[cfg(not(all(feature = "enterprise", feature = "parquet")))]
async fn delete_object(_key: &str) {}

/// A build without the instance store carries no client at all, so an oversized
/// artifact has nowhere but the row, and a row naming a key was written by a
/// worker that did have one.
#[cfg(not(all(feature = "enterprise", feature = "parquet")))]
async fn put_object(_key: &str, _value: String) -> error::Result<bool> {
    Ok(false)
}

#[cfg(not(all(feature = "enterprise", feature = "parquet")))]
async fn get_object(_key: &str) -> error::Result<String> {
    Err(missing_storage())
}

fn missing_storage() -> Error {
    Error::BadRequest(
        "the dbt state for this environment is in the instance's object storage, which this \
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
    job_dir: &str,
    conn: &Connection,
) -> error::Result<Deferral> {
    if p.script_path.is_empty() {
        return Err(Error::BadRequest(
            "`defer` resolves a `ref()` through the state a previous run of this script \
             published, so it needs a deployed script; a preview run has no environment to have \
             published one"
                .to_string(),
        ));
    }
    // An environment is the warehouse, the target and where they RESOLVE to, and
    // a `profiles.yml` that templates its schema or database resolves somewhere
    // this runtime does not render. Two renderings would then share one
    // environment, and a deferral after the value changed would resolve every
    // unbuilt `ref()` through the previous location's manifest.
    if p.templated_location {
        return Err(Error::BadRequest(
            "this project's profile selects its schema or database with a template, which dbt \
             renders and Windmill does not — so two environments cannot be told apart and a \
             deferral could resolve through the wrong one's manifest. Spell the target's schema \
             and database literally to use `defer`"
                .to_string(),
        ));
    }
    let Some(state) = load(p, w_id, conn).await? else {
        return Err(Error::BadRequest(format!(
            "no dbt state is stored for this environment ({}), so a `ref()` this run does not \
             build has no relation to resolve to. It is published by a successful run that adds \
             nothing of its own: one overriding `select` or `vars` does not publish, and neither \
             does any run of a descriptor that interpolates a `{{{{ }}}}` placeholder into `vars` \
             or a `$var:` into `env` — those describe a model set the caller's arguments decided. \
             Run this script once without `defer` and without overrides",
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

    // Two environments must not queue behind one advisory lock, which is what a
    // digest folded through a signed parse did for every one whose top bit is
    // set — half of them.
    #[test]
    fn each_environment_gets_its_own_publication_lock() {
        let mut seen = std::collections::HashSet::new();
        for i in 0..64 {
            seen.insert(publication_lock(
                "ws",
                "f/a/p",
                &format!("main|prod|s{i}|db"),
            ));
        }
        assert_eq!(seen.len(), 64);
        assert_eq!(
            publication_lock("ws", "f/a/p", "e"),
            publication_lock("ws", "f/a/p", "e")
        );
    }
}
