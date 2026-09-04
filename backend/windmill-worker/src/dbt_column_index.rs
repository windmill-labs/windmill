//! Column-level lineage and real column schemas, from the engine's own static
//! analysis.
//!
//! `manifest.json` carries neither. What does is the parquet index an engine
//! writes under `dbt compile --static-analysis strict --write-index`:
//! `dbt.column_lineage.parquet` (column-to-column edges, each labelled `copy`,
//! `mod` or `scan`) and `dbt.node_columns.parquet` (every column of every node,
//! typed and ordered, rather than only the ones an author documented).
//!
//! Three properties shape everything here, all of them measured against the real
//! engines rather than assumed:
//!
//! - **Strict analysis rejects SQL the default accepts.** An unresolvable
//!   identifier is an error under `strict` and compiles fine otherwise, so this
//!   is a SEPARATE pass with its own `--target-path`, never a flag on the build,
//!   and it is opt-in per project.
//! - **A failed pass still writes the index**, holding every edge of the models
//!   that did analyze. So the artifact is read whatever the exit status.
//! - **The flag is not the capability.** `dbt-core` 2.0.0-alpha.5 accepts
//!   `--write-index`, declares the views over these two tables in its own
//!   `views.sql`, and writes neither file; only Fusion does today. Nothing here
//!   asks which engine it is beyond "has the flag" — a release that starts
//!   writing them is picked up with no change.

use std::collections::HashSet;
use std::path::Path;
use std::time::Duration;

use parquet::file::reader::{FileReader, SerializedFileReader};
use parquet::record::{Field, Row};
use uuid::Uuid;
use windmill_common::dbt_manifest::{
    is_direct, ColumnIndex, IndexedColumn, IngestedColumnEdge, MAX_COLUMN_EDGES,
};
use windmill_common::error;
use windmill_common::worker::Connection;
use windmill_parser_yaml::dbt::DbtDescriptor;
use windmill_queue::append_logs;

use crate::dbt_executor::{dbt_command, Invocation, PreparedProject};
use crate::handle_child::JobCtx;

/// Where the lineage pass writes, relative to the project directory.
///
/// Its own tree, not the runtime's `wm_target`: a `dbt compile` writes
/// `manifest.json` and `run_results.json` like any other invocation, and after a
/// build those two are what the graph ingest and `dbt retry` read.
const CLL_ARTIFACTS_DIR: &str = "wm_target_cll";

const COLUMN_LINEAGE_PARQUET: &str = "dbt.column_lineage.parquet";
const NODE_COLUMNS_PARQUET: &str = "dbt.node_columns.parquet";

/// Run the lineage pass and read what it produced.
///
/// Best-effort about the COMPILE and about the artifact: a wrong engine, a
/// failed analysis, a missing or unreadable parquet, or this phase outrunning
/// its budget all return `None` with a line in the job log saying which, because
/// the graph without column lineage is exactly the graph this project had before
/// it asked for any.
///
/// NOT best-effort about the job: a cancellation, the job's own deadline or the
/// output ceiling are returned as `Err` and fail it. Swallowing those would let
/// a run that blew its timeout inside an optional annotation go on to publish a
/// graph and report success.
pub(crate) async fn collect(
    p: &PreparedProject,
    descriptor: &DbtDescriptor,
    inv: &Invocation,
    ctx: &mut JobCtx<'_>,
    job_id: &Uuid,
    w_id: &str,
    conn: &Connection,
    kept: &HashSet<&str>,
) -> error::Result<Option<ColumnIndex>> {
    let Some(budget) = phase_budget(ctx) else {
        return run_pass(p, descriptor, inv, ctx, job_id, w_id, conn, kept).await;
    };
    match tokio::time::timeout(
        budget,
        run_pass(p, descriptor, inv, ctx, job_id, w_id, conn, kept),
    )
    .await
    {
        Ok(r) => r,
        Err(_) => {
            append_logs(
                job_id,
                w_id,
                format!(
                    "\nNo column lineage: the analysis pass did not finish within {}s, half of \
                     what was left of this job's time. The build below gets the rest.\n",
                    budget.as_secs()
                ),
                conn,
            )
            .await;
            Ok(None)
        }
    }
}

async fn run_pass(
    p: &PreparedProject,
    descriptor: &DbtDescriptor,
    inv: &Invocation,
    ctx: &mut JobCtx<'_>,
    job_id: &Uuid,
    w_id: &str,
    conn: &Connection,
    kept: &HashSet<&str>,
) -> error::Result<Option<ColumnIndex>> {
    if !descriptor.column_lineage {
        return Ok(None);
    }
    if !p.engine.engine.writes_column_index() {
        append_logs(
            job_id,
            w_id,
            format!(
                "\n`column_lineage` is set, but the {} engine has no `--write-index`: column \
                 lineage needs an engine that does static analysis. The rest of the graph is \
                 unaffected.\n",
                p.engine.engine.as_str()
            ),
            conn,
        )
        .await;
        return Ok(None);
    }

    let index_dir = p.project_dir.join(CLL_ARTIFACTS_DIR).join("index");
    // A previous pass in the same job directory — a retry's second attempt —
    // would otherwise be read back as this one's answer.
    tokio::fs::remove_dir_all(p.project_dir.join(CLL_ARTIFACTS_DIR))
        .await
        .ok();

    let mut cmd = dbt_command(
        p,
        &[
            "compile",
            "--static-analysis",
            "strict",
            "--write-index",
            // Documented as what builds the CLL graph, and `--write-index` alone
            // happens to imply it on the engine probed. Passed explicitly so the
            // pass does not depend on which of the two is doing the work.
            "--write-lineage",
            "--target-path",
            CLL_ARTIFACTS_DIR,
        ],
    );
    // The flag already wins over the env var dbt_command sets, but setting both
    // means this pass cannot write into the runtime's artifacts even if that
    // precedence ever changes — and what is in there after a build is the
    // `run_results.json` a `dbt retry` resumes from.
    cmd.env("DBT_TARGET_PATH", CLL_ARTIFACTS_DIR);
    crate::dbt_executor::add_vars(&mut cmd, descriptor, inv)?;
    // Captured rather than streamed: a strict-analysis failure is a wall of
    // diagnostics about SQL the build itself accepts, and this pass decides
    // nothing about whether that build runs.
    //
    // `run_captured`, so a failed COMPILE is `success == false` and gets
    // downgraded below, while an `Err` — a cancellation, the job's deadline, the
    // output ceiling — still fails the job. Swallowing those would let a run
    // that blew its timeout inside this pass go on to publish a graph and report
    // success.
    let outcome = crate::dbt_executor::run_captured(
        cmd,
        "dbt compile (column lineage)",
        ctx,
        job_id,
        w_id,
        conn,
        CLL_MAX_OUTPUT_BYTES,
    )
    .await?;

    let index = read_index(&index_dir, kept).await;
    // What the caller cannot say for itself. The COUNTS are logged where the
    // index is folded into the graph, since the graph is what decides how much
    // of it is kept; this is the part only the pass knows.
    match (&index, outcome.success) {
        (Some(_), true) => {}
        // The ordinary partial outcome: some models did not analyze, the rest
        // did, and their lineage is real.
        (Some(_), false) => {
            let note = "Column lineage: `--static-analysis strict` rejected part of the project, \
                        so the lineage covers only the models it could analyze.";
            append_logs(
                job_id,
                w_id,
                format!("\n{note}\n{}", diagnostics(&outcome.stderr)),
                conn,
            )
            .await;
        }
        (None, _) => {
            let note = format!(
                "No column lineage: the analysis pass wrote no `{COLUMN_LINEAGE_PARQUET}`. Only \
                 an engine that computes it does, and only for the warehouses it analyzes \
                 natively — the flag alone is not the capability."
            );
            // The engine's own diagnostics. They are how a reader learns that
            // this adapter turned static analysis off, which it reports as a
            // warning on a SUCCESSFUL compile that nothing else would show.
            append_logs(
                job_id,
                w_id,
                format!("\n{note}\n{}", diagnostics(&outcome.stderr)),
                conn,
            )
            .await;
        }
    }
    Ok(index)
}

/// stdout the pass may produce. It is a compile, so this is diagnostics rather
/// than data.
const CLL_MAX_OUTPUT_BYTES: usize = 1 << 20;

/// The share of the job's remaining wall clock this pass may spend.
///
/// A per-run refresh ingests BEFORE the build and shares the job's one deadline,
/// so an unbounded pass on a slow project would hand `dbt build` an expired
/// budget and fail the run it exists only to annotate. Half leaves the build at
/// least as long as the annotation was allowed to take.
///
/// Spent as a race around the whole pass rather than as a shortened deadline
/// handed to the runner: the runner reports its expiry as an `Err`, which is
/// indistinguishable from a cancellation or the job's own deadline, and those
/// two MUST fail the job. Expiring here is this budget and nothing else, so it
/// degrades to no lineage. Dropping the future kills the child, which
/// `run_captured` spawns with `kill_on_drop`.
fn phase_budget(ctx: &JobCtx<'_>) -> Option<Duration> {
    ctx.timeout()
        .map(|left| Duration::from_secs((left.max(0) as u64 / 2).max(1)))
}

/// The tail of what the engine said, bounded. The whole of it is every rendered
/// model on a large project, which is not what a job log is for.
const DIAGNOSTIC_LINES: usize = 40;

fn diagnostics(out: &str) -> String {
    let lines: Vec<&str> = out.lines().filter(|l| !l.trim().is_empty()).collect();
    let tail = &lines[lines.len().saturating_sub(DIAGNOSTIC_LINES)..];
    match tail.is_empty() {
        true => String::new(),
        false => format!("{}\n", tail.join("\n")),
    }
}

/// Read both parquets, if the lineage one is there.
///
/// The column schemas alone are not worth a graph: they arrive with the lineage
/// or not at all, and a node's declared columns already answer for the case
/// where the pass never ran.
async fn read_index(index_dir: &Path, kept: &HashSet<&str>) -> Option<ColumnIndex> {
    let lineage = index_dir.join(COLUMN_LINEAGE_PARQUET);
    if !tokio::fs::try_exists(&lineage).await.unwrap_or(false) {
        return None;
    }
    let columns = index_dir.join(NODE_COLUMNS_PARQUET);
    // Owned, because the decode moves to a blocking thread. The index describes
    // the whole project while this graph describes one selection of it, so
    // scoping HERE is what keeps the cap below from being spent on rows the
    // graph would discard anyway.
    let kept: HashSet<String> = kept.iter().map(|s| (*s).to_string()).collect();
    // Decompressing and decoding a parquet is CPU work on a file the engine just
    // wrote, so it does not belong on the runtime's poll thread.
    tokio::task::spawn_blocking(move || read_index_blocking(&lineage, &columns, &kept))
        .await
        .map_err(|e| tracing::warn!("reading the dbt column index: {e:#}"))
        .ok()?
        .map_err(|e| tracing::warn!("reading the dbt column index: {e:#}"))
        .ok()
}

fn read_index_blocking(
    lineage: &Path,
    columns: &Path,
    kept: &HashSet<String>,
) -> error::Result<ColumnIndex> {
    let mut out = ColumnIndex::default();
    // Two passes, direct kinds first. The cap is a memory bound, so it has to
    // apply while decoding — but applied to the file's own row order it would
    // let `scan` edges, which are the bulk of a wide project's index and which
    // nothing renders, fill the budget before a single `copy` edge is read.
    // Reading a row and dropping it costs no memory, so the second pass is only
    // time, on a file the engine just wrote.
    for direct in [true, false] {
        let remaining = MAX_COLUMN_EDGES - out.edges.len();
        if remaining == 0 {
            break;
        }
        for_each_row(lineage, remaining, |row| {
            let lineage_kind = string(row, "lineage_kind");
            if is_direct(&lineage_kind) != direct {
                return Kept::No;
            }
            let parent_unique_id = string(row, "from_node_unique_id");
            let child_unique_id = string(row, "to_node_unique_id");
            let parent_column = string(row, "from_column_name");
            let child_column = string(row, "to_column_name");
            // A column of a node the analysis could not name is not an endpoint
            // the graph can draw, and neither is one outside this graph's nodes.
            if parent_column.is_empty()
                || child_column.is_empty()
                || !kept.contains(&parent_unique_id)
                || !kept.contains(&child_unique_id)
            {
                return Kept::No;
            }
            out.edges.push(IngestedColumnEdge {
                parent_unique_id,
                parent_column,
                child_unique_id,
                child_column,
                lineage_kind,
            });
            Kept::Yes
        })?;
    }
    // Absent is normal — an engine can write the lineage table and not this one —
    // and unreadable is not worth losing the lineage over.
    let _ = for_each_row(columns, MAX_INDEXED_COLUMNS, |row| {
        let unique_id = string(row, "unique_id");
        let name = string(row, "column_name");
        if name.is_empty() || !kept.contains(&unique_id) {
            return Kept::No;
        }
        // The author's `data_type` where `schema.yml` gives one, since that is
        // what the project calls the column; the analysis's own inference
        // otherwise.
        let column_type = match string(row, "declared_type") {
            t if !t.is_empty() => t,
            _ => string(row, "inferred_type"),
        };
        out.columns
            .entry(unique_id)
            .or_default()
            .push(IndexedColumn {
                name,
                column_type,
                index: int(row, "column_index").unwrap_or(i64::MAX),
            });
        Kept::Yes
    });
    Ok(out)
}

/// The most rows of `dbt.node_columns.parquet` one pass keeps. One per column of
/// the project, so the same bound as the edges is far more than any project
/// reaches; it exists for the same reason.
const MAX_INDEXED_COLUMNS: usize = MAX_COLUMN_EDGES;

/// Decode a parquet row at a time, stopping once `f` has ACCEPTED `limit` rows.
///
/// Counted on what the caller keeps, not on what the file holds, because the
/// input this defends against is the one that cannot be collected: `scan`
/// lineage is emitted from every predicate and join column to every output
/// column, so a project shaped that way writes an index whose row count is
/// quadratic in its widest model. Materializing that into a `Vec<Row>` first —
/// each row holding its own copy of every column NAME — is what would take the
/// worker process down, and this module's whole contract is that it cannot fail
/// a deploy or a run. A row the caller skips costs nothing, so skipping is free
/// and only keeping is budgeted.
fn for_each_row(path: &Path, limit: usize, mut f: impl FnMut(&Row) -> Kept) -> error::Result<()> {
    let fail = |e: parquet::errors::ParquetError| {
        error::Error::internal_err(format!("reading {}: {e}", path.display()))
    };
    let file = std::fs::File::open(path)
        .map_err(|e| error::Error::internal_err(format!("opening {}: {e}", path.display())))?;
    let reader = SerializedFileReader::new(file).map_err(fail)?;
    let mut budget = limit;
    for row in reader.get_row_iter(None).map_err(fail)? {
        if budget == 0 {
            tracing::warn!(
                "dbt column index: {} yielded more than {limit} usable rows; the rest is dropped",
                path.display()
            );
            break;
        }
        if f(&row.map_err(fail)?) == Kept::Yes {
            budget -= 1;
        }
    }
    Ok(())
}

/// Whether the row the closure just saw was retained, which is what the budget
/// counts.
#[derive(PartialEq, Eq)]
enum Kept {
    Yes,
    No,
}

/// By NAME, not by position: these tables are the engine's own schema and it
/// adds columns to them between releases.
fn field<'a>(row: &'a Row, name: &str) -> Option<&'a Field> {
    row.get_column_iter()
        .find(|(k, _)| k.as_str() == name)
        .map(|(_, v)| v)
}

fn string(row: &Row, name: &str) -> String {
    match field(row, name) {
        Some(Field::Str(s)) => s.clone(),
        Some(Field::Bytes(b)) => String::from_utf8_lossy(b.data()).into_owned(),
        _ => String::new(),
    }
}

fn int(row: &Row, name: &str) -> Option<i64> {
    match field(row, name) {
        Some(Field::Long(v)) => Some(*v),
        Some(Field::Int(v)) => Some(*v as i64),
        Some(Field::Short(v)) => Some(*v as i64),
        Some(Field::UInt(v)) => Some(*v as i64),
        Some(Field::ULong(v)) => i64::try_from(*v).ok(),
        _ => None,
    }
}
