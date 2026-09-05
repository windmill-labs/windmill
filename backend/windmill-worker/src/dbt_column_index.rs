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
/// Two steps with deliberately different contracts, because conflating them is
/// what made a best-effort annotation able to fail the job it annotates:
///
/// - [`compile_index`] runs a subprocess and owns the JOB's semantics. Only a
///   cancellation or the job's own deadline can `Err` out of it; a non-zero exit
///   and an over-long output are outcomes, not failures.
/// - [`read_index`] owns the ARTIFACT's semantics. It is infallible and
///   memory-bounded, and knows nothing about the job.
///
/// The phase budget wraps the compile alone. A budget around the whole pass
/// would time out with a decode still running on a blocking thread, which is
/// precisely what "the build below gets the rest" must not mean.
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
    let Some(compiled) = compile_index(p, descriptor, inv, ctx, job_id, w_id, conn).await? else {
        return Ok(None);
    };
    let coverage = Coverage::of(&compiled);

    let index = read_index(&index_dir, kept).await;
    // What only the pass knows. The COUNTS are logged where the index is folded
    // into the graph, since the graph decides how much of it is kept.
    match (&index, coverage.caveat()) {
        (Some(_), None) => {}
        (Some(_), Some(note)) => {
            append_logs(
                job_id,
                w_id,
                format!("\n{note}\n{}", diagnostics(&compiled.stderr)),
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
                format!("\n{note}\n{}", diagnostics(&compiled.stderr)),
                conn,
            )
            .await;
        }
    }
    Ok(index)
}

/// How completely the analysis compile covered the project.
///
/// Every way the COMPILE can disappoint is a value here rather than an error. An
/// `Err` from `compile_index` is the JOB's — a cancellation or its deadline —
/// and must fail it; the pass giving up on its own terms is `Ok(None)` and has
/// already been logged.
enum Coverage {
    /// Every model analyzed.
    Whole,
    /// `--static-analysis strict` rejected part of the project. Whatever it did
    /// analyze is still in the index.
    Partial,
    /// The output ceiling killed the compile mid-run. Distinct from `Partial`:
    /// nothing rejected the project, but the index is however far it had got, so
    /// it is not `Whole` either.
    Truncated,
}

impl Coverage {
    fn of(c: &crate::dbt_executor::Captured) -> Self {
        match (c.truncated, c.success) {
            (true, _) => Coverage::Truncated,
            (false, true) => Coverage::Whole,
            (false, false) => Coverage::Partial,
        }
    }

    /// What to tell the reader when an index WAS produced. `None` for a run that
    /// covered everything, which needs no caveat.
    fn caveat(&self) -> Option<&'static str> {
        match self {
            Coverage::Whole => None,
            Coverage::Partial => Some(
                "Column lineage: `--static-analysis strict` rejected part of the project, so \
                 the lineage covers only the models it could analyze.",
            ),
            Coverage::Truncated => Some(
                "Column lineage: the analysis pass printed more than this runtime reads and was \
                 stopped, so the lineage covers only the models it had reached.",
            ),
        }
    }
}

/// Run `dbt compile --static-analysis strict --write-index`, under this phase's
/// share of the job's clock.
///
/// `Ok(None)` is "the pass gave up and said so"; `Err` is the job's own
/// cancellation or deadline and must propagate. Nothing outlives this function:
/// the budget is a race around the child, and dropping that future kills it
/// through `run_captured`'s `kill_on_drop`.
async fn compile_index(
    p: &PreparedProject,
    descriptor: &DbtDescriptor,
    inv: &Invocation,
    ctx: &mut JobCtx<'_>,
    job_id: &Uuid,
    w_id: &str,
    conn: &Connection,
) -> error::Result<Option<crate::dbt_executor::Captured>> {
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
    // Read before the future below borrows `ctx` mutably.
    let budget = phase_budget(ctx);
    let run = crate::dbt_executor::run_captured(
        cmd,
        "dbt compile (column lineage)",
        ctx,
        job_id,
        w_id,
        conn,
        CLL_MAX_OUTPUT_BYTES,
        // The ceiling is this pass's, not the job's: a compile that prints more
        // than it than we care to read has still analyzed the project, and the
        // index it wrote is on disk either way.
        crate::dbt_executor::Overflow::Truncate,
    );
    let Some(budget) = budget else {
        return Ok(Some(run.await?));
    };
    match tokio::time::timeout(budget, run).await {
        Ok(r) => Ok(Some(r?)),
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
/// Spent as a race around the COMPILE rather than as a shortened deadline handed
/// to the runner: the runner reports its expiry as an `Err`, indistinguishable
/// from a cancellation or the job's own deadline, and those two MUST fail the
/// job. Expiring here is this budget and nothing else. The child dies with the
/// dropped future through `run_captured`'s `kill_on_drop`, and the decode is
/// outside the race so nothing survives it.
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
    // ONE pass, with the two kinds bucketed as they arrive. Direct edges are
    // what a trace draws, so they get the whole budget; `scan` — the bulk of a
    // wide project's index and the kind nothing renders — fills only what is
    // left over at the end. Reading the file twice to get that ordering would
    // double the decode of exactly the large index this bound exists for.
    let mut scan: Vec<IngestedColumnEdge> = Vec::new();
    for_each_row(lineage, |row| {
        let lineage_kind = string(row, "lineage_kind");
        let parent_unique_id = string(row, "from_node_unique_id");
        let child_unique_id = string(row, "to_node_unique_id");
        let parent_column = string(row, "from_column_name");
        let child_column = string(row, "to_column_name");
        // A column of a node the analysis could not name is not an endpoint the
        // graph can draw, and neither is one outside this graph's nodes.
        if parent_column.is_empty()
            || child_column.is_empty()
            || !kept.contains(&parent_unique_id)
            || !kept.contains(&child_unique_id)
        {
            return;
        }
        let edge = IngestedColumnEdge {
            parent_unique_id,
            parent_column,
            child_unique_id,
            child_column,
            lineage_kind,
        };
        // The bound covers BOTH buckets, so the pass never holds more than one
        // budget's worth however the kinds are distributed.
        let held = out.edges.len() + scan.len();
        if is_direct(&edge.lineage_kind) {
            if out.edges.len() >= MAX_COLUMN_EDGES {
                return;
            }
            // A direct edge displaces a `scan` one: the budget exists to be
            // spent on what a trace draws.
            if held >= MAX_COLUMN_EDGES {
                scan.pop();
            }
            out.edges.push(edge);
            return;
        }
        if held < MAX_COLUMN_EDGES {
            scan.push(edge);
        }
    })?;
    out.edges.append(&mut scan);
    // Absent is normal — an engine can write the lineage table and not this one —
    // and unreadable is not worth losing the lineage over.
    let mut held = 0usize;
    let _ = for_each_row(columns, |row| {
        let unique_id = string(row, "unique_id");
        let name = string(row, "column_name");
        if name.is_empty() || !kept.contains(&unique_id) || held >= MAX_INDEXED_COLUMNS {
            return;
        }
        held += 1;
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
    });
    Ok(out)
}

/// The most rows of `dbt.node_columns.parquet` one pass keeps. One per column of
/// the project, so the same bound as the edges is far more than any project
/// reaches; it exists for the same reason.
const MAX_INDEXED_COLUMNS: usize = MAX_COLUMN_EDGES;

/// The most rows of an index one pass DECODES, whatever it keeps of them.
///
/// A bound on work rather than on memory, and the two are separate because the
/// input this defends against is the one that cannot be collected: `scan`
/// lineage is emitted from every predicate and join column to every output
/// column, so a project shaped that way writes an index whose row count is
/// quadratic in its widest model. This pass runs outside the phase budget, on a
/// blocking thread, and its whole contract is that it cannot fail a deploy or a
/// run — so the file it walks needs an end even when almost nothing in it is
/// retained.
const MAX_INDEX_ROWS: usize = 4_000_000;

/// Decode a parquet a row at a time, handing each to `f` and never holding two.
///
/// Collecting first would put a `Vec<Row>` — each row carrying its own copy of
/// every column NAME — in front of the caller's own bound, which is what would
/// take the worker process down on the index described above.
fn for_each_row(path: &Path, mut f: impl FnMut(&Row)) -> error::Result<()> {
    let fail = |e: parquet::errors::ParquetError| {
        error::Error::internal_err(format!("reading {}: {e}", path.display()))
    };
    let file = std::fs::File::open(path)
        .map_err(|e| error::Error::internal_err(format!("opening {}: {e}", path.display())))?;
    let reader = SerializedFileReader::new(file).map_err(fail)?;
    for (n, row) in reader.get_row_iter(None).map_err(fail)?.enumerate() {
        if n >= MAX_INDEX_ROWS {
            tracing::warn!(
                "dbt column index: {} holds more than {MAX_INDEX_ROWS} rows; the rest is dropped",
                path.display()
            );
            break;
        }
        f(&row.map_err(fail)?);
    }
    Ok(())
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
