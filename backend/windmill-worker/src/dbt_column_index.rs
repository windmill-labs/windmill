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

use std::path::Path;

use parquet::file::reader::{FileReader, SerializedFileReader};
use parquet::record::{Field, Row};
use uuid::Uuid;
use windmill_common::dbt_manifest::{ColumnIndex, IndexedColumn, IngestedColumnEdge};
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
/// Best-effort throughout: every failure returns `None` with a line in the job
/// log saying which one, because the graph without column lineage is exactly the
/// graph this project had before it asked for any.
pub(crate) async fn collect(
    p: &PreparedProject,
    descriptor: &DbtDescriptor,
    inv: &Invocation,
    ctx: &mut JobCtx<'_>,
    job_id: &Uuid,
    w_id: &str,
    conn: &Connection,
) -> Option<ColumnIndex> {
    if !descriptor.column_lineage {
        return None;
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
        return None;
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
    if let Err(e) = crate::dbt_executor::add_vars(&mut cmd, descriptor, inv) {
        tracing::warn!("dbt column lineage: {e:#}");
        return None;
    }
    // Captured rather than streamed: a strict-analysis failure is a wall of
    // diagnostics about SQL the build itself accepts, and this pass decides
    // nothing about whether that build runs.
    let outcome = crate::dbt_executor::run_capturing(
        cmd,
        "dbt compile (column lineage)",
        ctx,
        job_id,
        w_id,
        conn,
        CLL_MAX_OUTPUT_BYTES,
    )
    .await;

    let index = read_index(&index_dir).await;
    // What the caller cannot say for itself. The COUNTS are logged where the
    // index is folded into the graph, since the graph is what decides how much
    // of it is kept; this is the part only the pass knows.
    match (&index, &outcome) {
        // The ordinary partial outcome: some models did not analyze, the rest
        // did, and their lineage is real.
        (Some(_), Err(e)) => {
            let note = "Column lineage: `--static-analysis strict` rejected part of the project, \
                        so the lineage covers only the models it could analyze.";
            append_logs(job_id, w_id, format!("\n{note}\n{}", diagnostics(&e.to_string())), conn)
                .await;
        }
        (Some(_), Ok(_)) => {}
        (None, outcome) => {
            let note = format!(
                "No column lineage: the analysis pass wrote no `{COLUMN_LINEAGE_PARQUET}`. Only \
                 an engine that computes it does, and only for the warehouses it analyzes \
                 natively — the flag alone is not the capability."
            );
            // The engine's own diagnostics. They are how a reader learns that
            // this adapter turned static analysis off, which it reports as a
            // warning on a SUCCESSFUL compile that nothing else would show.
            let detail = match outcome {
                Ok(c) => diagnostics(&c.stderr),
                Err(e) => diagnostics(&e.to_string()),
            };
            append_logs(job_id, w_id, format!("\n{note}\n{detail}"), conn).await;
        }
    }
    index
}

/// stdout the pass may produce. It is a compile, so this is diagnostics rather
/// than data.
const CLL_MAX_OUTPUT_BYTES: usize = 1 << 20;

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
async fn read_index(index_dir: &Path) -> Option<ColumnIndex> {
    let lineage = index_dir.join(COLUMN_LINEAGE_PARQUET);
    if !tokio::fs::try_exists(&lineage).await.unwrap_or(false) {
        return None;
    }
    let columns = index_dir.join(NODE_COLUMNS_PARQUET);
    // Decompressing and decoding a parquet is CPU work on a file the engine just
    // wrote, so it does not belong on the runtime's poll thread.
    tokio::task::spawn_blocking(move || read_index_blocking(&lineage, &columns))
        .await
        .map_err(|e| tracing::warn!("reading the dbt column index: {e:#}"))
        .ok()?
        .map_err(|e| tracing::warn!("reading the dbt column index: {e:#}"))
        .ok()
}

fn read_index_blocking(lineage: &Path, columns: &Path) -> error::Result<ColumnIndex> {
    let mut out = ColumnIndex::default();
    for row in rows(lineage)? {
        let parent_unique_id = string(&row, "from_node_unique_id");
        let child_unique_id = string(&row, "to_node_unique_id");
        let parent_column = string(&row, "from_column_name");
        let child_column = string(&row, "to_column_name");
        // A column of a node the analysis could not name is not an endpoint the
        // graph can draw.
        if parent_unique_id.is_empty()
            || child_unique_id.is_empty()
            || parent_column.is_empty()
            || child_column.is_empty()
        {
            continue;
        }
        out.edges.push(IngestedColumnEdge {
            parent_unique_id,
            parent_column,
            child_unique_id,
            child_column,
            lineage_kind: string(&row, "lineage_kind"),
        });
    }
    // Absent is normal — an engine can write the lineage table and not this one —
    // and unreadable is not worth losing the lineage over.
    if let Ok(rows) = rows(columns) {
        for row in rows {
            let unique_id = string(&row, "unique_id");
            let name = string(&row, "column_name");
            if unique_id.is_empty() || name.is_empty() {
                continue;
            }
            // The author's `data_type` where `schema.yml` gives one, since that
            // is what the project calls the column; the analysis's own inference
            // otherwise.
            let column_type = match string(&row, "declared_type") {
                t if !t.is_empty() => t,
                _ => string(&row, "inferred_type"),
            };
            out.columns
                .entry(unique_id)
                .or_default()
                .push(IndexedColumn {
                    name,
                    column_type,
                    index: int(&row, "column_index").unwrap_or(i64::MAX),
                });
        }
    }
    Ok(out)
}

fn rows(path: &Path) -> error::Result<Vec<Row>> {
    let file = std::fs::File::open(path)
        .map_err(|e| error::Error::internal_err(format!("opening {}: {e}", path.display())))?;
    let reader = SerializedFileReader::new(file)
        .map_err(|e| error::Error::internal_err(format!("reading {}: {e}", path.display())))?;
    reader
        .get_row_iter(None)
        .map_err(|e| error::Error::internal_err(format!("reading {}: {e}", path.display())))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| error::Error::internal_err(format!("reading {}: {e}", path.display())))
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
