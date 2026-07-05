//! Eligibility classifier + materialization SQL codegen for managed `// materialize`.
//!
//! Managed `// materialize` (the default) promises the script is "setup
//! statements, then one trailing SELECT" — Windmill generates the write DDL
//! around that SELECT (the `// materialize manual` escape hatch opts out and
//! writes its own DDL). This module is the single source of truth for *which
//! block is that SELECT* and *what DDL gets generated*, so save-time validation
//! (deploy path) and run-time codegen (DuckDB executor) can never disagree.
//!
//! Everything here is pure and string-level: no SQL is executed, no type
//! inference is done. The classifier is leading-keyword based and deliberately
//! conservative — anything it can't positively recognize as a read-only output
//! or a known-safe setup statement is rejected, so a script is only accepted
//! for managed mode when its shape is unambiguous.

/// One top-level statement's role in a wrap-mode script.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockClass {
    /// Read-only relation the wrap writes from: `SELECT` / `WITH …SELECT` /
    /// `FROM` (DuckDB from-first) / `VALUES` / `TABLE x` / `(UN)PIVOT`.
    Output,
    /// Known-safe preamble: `ATTACH` / `INSTALL` / `LOAD` / `SET` / `PRAGMA` /
    /// `USE` / `CREATE TEMP …`. Runs verbatim before the generated write.
    Setup,
    /// Anything that writes or whose effect we can't vouch for: non-temp
    /// `CREATE` / `INSERT` / `UPDATE` / `DELETE` / `MERGE` / `DROP` / `COPY` /
    /// `ALTER` / `TRUNCATE`, or an unrecognized leading keyword. Disqualifies
    /// managed mode (the user should use `// materialize manual`).
    Disallowed,
}

/// A script accepted for wrapping: zero+ setup blocks then one terminal SELECT.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrapPlan {
    /// Setup statements in source order, verbatim, **without** trailing `;`.
    pub setup: Vec<String>,
    /// The single terminal output statement, verbatim, **without** trailing `;`.
    pub output: String,
}

/// Why a script is not eligible for managed `// materialize`. Carries enough to
/// render the targeted save-time messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WrapError {
    /// No statements at all (empty / comments only).
    Empty,
    /// No terminal SELECT — nothing to wrap.
    NoOutput,
    /// More than one top-level SELECT. `count` is how many were found.
    MultipleOutputs { count: usize },
    /// A SELECT exists but isn't the last statement (something runs after it).
    OutputNotLast,
    /// A write/unknown statement appears among the setup blocks. `snippet` is a
    /// short prefix of the offending statement for the error message.
    DisallowedBlock { snippet: String },
}

impl WrapError {
    /// Human-facing, actionable message (matches the spec's rejection text).
    pub fn message(&self) -> String {
        let base =
            "managed `// materialize` requires the script to be setup statements then a single trailing SELECT";
        let manual = "use `// materialize manual` to write the DDL yourself";
        match self {
            WrapError::Empty => format!("{base}: the script is empty."),
            WrapError::NoOutput => format!("{base}: found no SELECT — {manual}."),
            WrapError::MultipleOutputs { count } => format!(
                "{base}: found {count} SELECT statements; combine them with a CTE, or {manual}."
            ),
            WrapError::OutputNotLast => format!(
                "{base}: found statements after the SELECT — move them above it, or {manual}."
            ),
            WrapError::DisallowedBlock { snippet } => {
                format!("{base}: `{snippet}` writes or is unrecognized — {manual}.")
            }
        }
    }
}

/// Split SQL into top-level, `;`-separated statements, skipping line comments
/// (`-- …`), block comments (`/* … */`), single-quoted strings (`'…'` with
/// `''` escape) and double-quoted identifiers (`"…"`). Semicolons inside any of
/// those are not separators. Returns each statement trimmed, comments stripped,
/// empties dropped. Self-contained so the parser crate stays dependency-free;
/// it must stay behaviourally aligned with the executor's block splitter (both
/// route wrap through `classify_wrap`, so the split they see is this one).
pub fn split_statements(sql: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    // Char-wise, not byte-wise: the emitted strings are re-executed
    // (materialize codegen) and persisted (macro registry), so a Latin-1
    // `bytes[i] as char` decode would corrupt any multi-byte text inside
    // statements. All delimiters are ASCII — split positions are unaffected.
    let chars: Vec<char> = sql.chars().collect();
    let mut i = 0;
    let n = chars.len();
    while i < n {
        let c = chars[i];
        // line comment — `--` (SQL) or `//`. The `//` form is not SQL, but it
        // is how Windmill pipeline annotations (`// materialize`, `// pipeline`,
        // …) are written, and they sit above the SQL in the same script; strip
        // them so they don't pollute the first statement block's classification
        // or the generated setup SQL.
        if (c == '-' && i + 1 < n && chars[i + 1] == '-')
            || (c == '/' && i + 1 < n && chars[i + 1] == '/')
        {
            while i < n && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        // block comment
        if c == '/' && i + 1 < n && chars[i + 1] == '*' {
            i += 2;
            while i + 1 < n && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i += 2;
            continue;
        }
        // single-quoted string
        if c == '\'' {
            cur.push(c);
            i += 1;
            while i < n {
                cur.push(chars[i]);
                if chars[i] == '\'' {
                    // doubled '' is an escaped quote, stay in string
                    if i + 1 < n && chars[i + 1] == '\'' {
                        cur.push('\'');
                        i += 2;
                        continue;
                    }
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }
        // double-quoted identifier
        if c == '"' {
            cur.push(c);
            i += 1;
            while i < n {
                cur.push(chars[i]);
                if chars[i] == '"' {
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }
        if c == ';' {
            let t = cur.trim();
            if !t.is_empty() {
                out.push(t.to_string());
            }
            cur.clear();
            i += 1;
            continue;
        }
        cur.push(c);
        i += 1;
    }
    let t = cur.trim();
    if !t.is_empty() {
        out.push(t.to_string());
    }
    out
}

/// Lowercased top-level keyword tokens of a single statement (parens collapsed
/// away: tokens *inside* balanced `(...)` are skipped, so a CTE body's verbs
/// don't leak up). Strings/identifiers are already gone from the split, but we
/// re-guard quotes defensively. Used to disambiguate `WITH …` and `CREATE …`.
fn top_level_keywords(stmt: &str) -> Vec<String> {
    let mut toks = Vec::new();
    let mut cur = String::new();
    let mut depth: i32 = 0;
    let bytes = stmt.as_bytes();
    let mut i = 0;
    let n = bytes.len();
    let flush = |cur: &mut String, toks: &mut Vec<String>| {
        if !cur.is_empty() {
            toks.push(cur.to_lowercase());
            cur.clear();
        }
    };
    while i < n {
        let c = bytes[i] as char;
        if c == '\'' || c == '"' {
            let q = bytes[i];
            i += 1;
            while i < n && bytes[i] != q {
                i += 1;
            }
            i += 1;
            continue;
        }
        if c == '(' {
            flush(&mut cur, &mut toks);
            depth += 1;
            i += 1;
            continue;
        }
        if c == ')' {
            if depth > 0 {
                depth -= 1;
            }
            i += 1;
            continue;
        }
        if depth > 0 {
            i += 1;
            continue;
        }
        if c.is_alphanumeric() || c == '_' {
            cur.push(c);
        } else {
            flush(&mut cur, &mut toks);
        }
        i += 1;
    }
    flush(&mut cur, &mut toks);
    toks
}

const OUTPUT_KW: &[&str] = &["select", "from", "values", "table", "pivot", "unpivot"];
const SETUP_KW: &[&str] = &["attach", "install", "load", "set", "pragma", "use"];
const WRITE_VERBS: &[&str] = &["insert", "update", "delete", "merge"];

/// Classify a single statement by its leading keyword (with `WITH`/`CREATE`
/// disambiguation). See [`BlockClass`].
pub fn classify_block(stmt: &str) -> BlockClass {
    let kws = top_level_keywords(stmt);
    let Some(first) = kws.first().map(String::as_str) else {
        return BlockClass::Disallowed;
    };

    // CREATE TEMP … is setup (staging); any other CREATE is a write.
    if first == "create" {
        let temp = kws
            .iter()
            .skip(1)
            .take(3)
            .any(|k| k == "temp" || k == "temporary");
        return if temp {
            BlockClass::Setup
        } else {
            BlockClass::Disallowed
        };
    }

    // WITH … : the main statement's verb decides. CTE bodies are parenthesized,
    // so their verbs are not in `kws`; the first top-level write verb or SELECT
    // after the CTE list is the real one.
    if first == "with" {
        for k in kws.iter().skip(1) {
            if k == "select" {
                return BlockClass::Output;
            }
            if WRITE_VERBS.contains(&k.as_str()) {
                return BlockClass::Disallowed;
            }
        }
        // `WITH x AS (...) SELECT` where SELECT got collapsed is impossible
        // (SELECT here is top-level), so a WITH with no top-level verb is a
        // malformed/unknown statement — reject conservatively.
        return BlockClass::Disallowed;
    }

    if OUTPUT_KW.contains(&first) {
        return BlockClass::Output;
    }
    if SETUP_KW.contains(&first) {
        return BlockClass::Setup;
    }
    BlockClass::Disallowed
}

/// Validate a script for managed `// materialize` and, on success, return the
/// setup/output split. Enforces the four conditions from the spec:
/// 1. exactly one Output block, 2. it is last, 3. all preceding blocks are
/// Setup, 4. nothing after it.
pub fn classify_wrap(sql: &str) -> Result<WrapPlan, WrapError> {
    let stmts = split_statements(sql);
    if stmts.is_empty() {
        return Err(WrapError::Empty);
    }
    let classes: Vec<BlockClass> = stmts.iter().map(|s| classify_block(s)).collect();

    let output_idxs: Vec<usize> = classes
        .iter()
        .enumerate()
        .filter(|(_, c)| **c == BlockClass::Output)
        .map(|(i, _)| i)
        .collect();

    match output_idxs.len() {
        0 => return Err(WrapError::NoOutput),
        1 => {}
        count => return Err(WrapError::MultipleOutputs { count }),
    }
    let out_idx = output_idxs[0];
    if out_idx != stmts.len() - 1 {
        return Err(WrapError::OutputNotLast);
    }
    // Everything before the output must be Setup (no Disallowed preamble).
    for (i, c) in classes.iter().enumerate().take(out_idx) {
        if *c != BlockClass::Setup {
            return Err(WrapError::DisallowedBlock { snippet: snippet(&stmts[i]) });
        }
    }
    Ok(WrapPlan { setup: stmts[..out_idx].to_vec(), output: stmts[out_idx].clone() })
}

fn snippet(stmt: &str) -> String {
    let one_line: String = stmt.split_whitespace().collect::<Vec<_>>().join(" ");
    if one_line.chars().count() > 40 {
        let truncated: String = one_line.chars().take(40).collect();
        format!("{truncated}…")
    } else {
        one_line
    }
}

// ---------------------------------------------------------------------------
// Codegen
// ---------------------------------------------------------------------------

/// How a (partition of a) materialized table is reconciled on each run.
/// Derived at deploy from the annotation: `key=<col> history` (or the `scd2`
/// alias) → `Scd2`, else `append` → `Append`, else `unique_key` → `Merge`, else
/// `Replace`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaterializeStrategy {
    /// DELETE the current partition, then INSERT — partition becomes exactly
    /// what the SELECT returned. Full-refresh of the slice.
    Replace,
    /// Upsert within the slice on `unique_key` (delete-by-key + insert); rows
    /// absent from the SELECT are left in place. This is SCD type 1: a changed
    /// row overwrites the prior value, keeping no history.
    Merge { unique_key: String },
    /// INSERT only — immutable event-log semantics.
    Append,
    /// Slowly Changing Dimension type 2: the SELECT is the *current* snapshot
    /// (one row per `key`); a change to any tracked column closes the prior
    /// version (`valid_to`/`is_current=false`) and opens a new one, so the full
    /// history is preserved. `track` empty ⇒ every non-key column is tracked.
    /// `close_deleted` (opt-in `deletes=close`) also closes the current version
    /// of a key that disappears from the snapshot (dbt's `hard_deletes=close`);
    /// default (false) leaves absent keys current (soft delete).
    /// Unpartitioned only (the worker rejects `// partitioned` + scd2).
    Scd2 { key: String, track: Vec<String>, close_deleted: bool },
}

/// SCD2 metadata columns appended to the managed history table. Fixed names so
/// the generated diff/close/open SQL and any `// data_test` on them agree.
const SCD2_VALID_FROM: &str = "valid_from";
const SCD2_VALID_TO: &str = "valid_to";
const SCD2_IS_CURRENT: &str = "is_current";
/// Connection-local temp table holding the keys whose version must be rotated
/// this run (changed + new). Captured before the write so the close and the
/// open see the same set. `_wm_` prefix so it never collides with user tables.
const SCD2_CHANGED_KEYS: &str = "_wm_scd2_changed";
/// Connection-local temp table holding the keys that disappeared from the
/// snapshot this run (present-and-current in the table, absent from the SELECT).
/// Only used when `close_deleted` (`deletes=close`) is set.
const SCD2_DELETED_KEYS: &str = "_wm_scd2_deleted";

/// Inputs to materialization codegen, all resolved at run time by the worker.
/// Pure: produces SQL text; executes nothing.
#[derive(Debug, Clone)]
pub struct MaterializeCodegen<'a> {
    /// Fully-qualified target, e.g. `_wm_target.orders_daily`. Always qualified
    /// so a user `USE …;` in setup can't redirect the write.
    pub target_qualified: &'a str,
    /// The user's output SELECT (verbatim, no trailing `;`) — embedded as a
    /// subquery so its own shape is irrelevant to the generated wrapper.
    pub select_sql: &'a str,
    /// Physical partition column added to the managed table.
    pub partition_col: &'a str,
    /// SQL expression for the current partition value — a literal like
    /// `'2026-06-19'` or a bind placeholder. The caller is responsible for
    /// safe quoting/binding.
    pub partition_value_sql: &'a str,
    /// Whether `// partitioned` applies. When false the table is unpartitioned
    /// and the partition column / `SET PARTITIONED BY` are omitted.
    pub partitioned: bool,
    pub strategy: MaterializeStrategy,
    /// Write-time guardrail for a drifted SELECT vs the fixed table schema.
    /// Only the persist-and-mutate strategies (partitioned replace, merge,
    /// append) act on it: `Fail` emits an in-txn guard that raises on drift,
    /// `Sync` writes BY NAME and expects the executor to inject `ALTER TABLE`
    /// DDL at the [`SYNC_ALTER_SENTINEL`] slot, `Warn`/`Ignore` write
    /// positionally (drift surfaced by the summary in `Warn`, silent in
    /// `Ignore`). See [`MaterializeCodegen::is_persist_and_mutate`].
    pub on_schema_change: OnSchemaChange,
}

/// The exact statement the `sync` codegen emits right after `BEGIN
/// TRANSACTION;` as the injection slot for `ALTER TABLE … ADD/DROP COLUMN`
/// DDL. The executor computes the drift with a pre-pass probe and replaces this
/// literal in the assembled query text (with the DDL, or removes it when there
/// is no drift). Classified `Write` so the EE write-audit-publish reassembly
/// keeps it inside the transaction with the mutations; a plain no-op SELECT so
/// that if it is somehow left un-replaced the run still succeeds unchanged.
pub const SYNC_ALTER_SENTINEL: &str = "SELECT '__wm_sync_alter_sentinel__' AS _wm_sync;";

impl<'a> MaterializeCodegen<'a> {
    /// Whether this (strategy, partitioned) uses the positional persist-and-
    /// mutate write whose table schema is fixed at first CREATE — the only case
    /// the `on_schema_change` write-time guardrail applies to. Whole-table
    /// replace (`CREATE OR REPLACE`) and scd2 self-heal / are name-mapped, so
    /// they are excluded.
    pub fn is_persist_and_mutate(&self) -> bool {
        match self.strategy {
            MaterializeStrategy::Scd2 { .. } => false,
            MaterializeStrategy::Replace => self.partitioned,
            MaterializeStrategy::Append | MaterializeStrategy::Merge { .. } => true,
        }
    }

    /// The ordered statements that perform the materialization, to be run after
    /// the setup blocks and inside the caller's execution. The first-run
    /// bootstrap is idempotent (`IF NOT EXISTS`), so this is safe to run every
    /// time. The DELETE/INSERT body is wrapped in one transaction so a partial
    /// failure leaves the prior snapshot intact. Every strategy reduces to
    /// DELETE+INSERT (no `MERGE INTO`) — see the `Merge` arm for why.
    pub fn statements(&self) -> Vec<String> {
        let t = self.target_qualified;
        let sel = self.select_sql;
        let pcol = self.partition_col;
        let pval = self.partition_value_sql;
        let mut out = Vec::new();

        // SCD2 has a shape unlike the DELETE/INSERT strategies (diff → close old
        // → open new) and does not support partitioning (rejected at the worker),
        // so it is generated up front by its own helper.
        if let MaterializeStrategy::Scd2 { key, track, close_deleted } = &self.strategy {
            return self.scd2_statements(key, track, *close_deleted);
        }

        // Whole-table replace: rebuild the table to match the SELECT's *current*
        // schema each run with one atomic `CREATE OR REPLACE` (which DuckLake
        // still snapshots). This is the only path that survives a changed SELECT
        // or a pre-existing table with a different schema — the persist-and-
        // mutate paths below fix the schema at first create.
        if !self.partitioned && matches!(self.strategy, MaterializeStrategy::Replace) {
            out.push(format!(
                "CREATE OR REPLACE TABLE {t} AS SELECT * FROM ({sel});"
            ));
            return out;
        }

        // Persist-and-mutate (partitioned, or merge/append): bootstrap the table
        // if absent, then write into it. The schema is fixed at first create —
        // a later SELECT-schema change needs a manual rebuild (schema evolution
        // is a follow-up).
        if self.partitioned {
            out.push(format!(
                "CREATE TABLE IF NOT EXISTS {t} AS \
                 SELECT *, CAST(NULL AS VARCHAR) AS {pcol} FROM ({sel}) WHERE false;"
            ));
            out.push(format!("ALTER TABLE {t} SET PARTITIONED BY ({pcol});"));
        } else {
            out.push(format!(
                "CREATE TABLE IF NOT EXISTS {t} AS SELECT * FROM ({sel}) WHERE false;"
            ));
        }
        out.push("BEGIN TRANSACTION;".to_string());
        // Write-time schema guardrail, emitted right after BEGIN so it runs
        // before any mutation (a failing guard aborts before touching data; the
        // sync ALTERs run before the INSERT so the sets match). Reached only on
        // the persist-and-mutate path here (whole-table replace and scd2 return
        // above), so no extra strategy gate is needed.
        match self.on_schema_change {
            OnSchemaChange::Fail => out.push(schema_drift_guard_sql(sel, t, pcol)),
            OnSchemaChange::Sync => out.push(SYNC_ALTER_SENTINEL.to_string()),
            OnSchemaChange::Warn | OnSchemaChange::Ignore => {}
        }
        // The rows to write, with the partition column appended when partitioned.
        let source = if self.partitioned {
            format!("SELECT *, {pval} AS {pcol} FROM ({sel})")
        } else {
            format!("SELECT * FROM ({sel})")
        };
        // `sync` maps columns by name (positional would cross-wire: an ALTERed
        // ADD COLUMN appends at the end, so a positional INSERT of the SELECT
        // would fill it from the wrong source column).
        let by_name = if self.on_schema_change == OnSchemaChange::Sync {
            " BY NAME"
        } else {
            ""
        };
        match &self.strategy {
            MaterializeStrategy::Replace => {
                // Only reached when partitioned (whole-table replace returned above).
                out.push(format!("DELETE FROM {t} WHERE {pcol} = {pval};"));
                out.push(format!("INSERT INTO {t}{by_name} {source};"));
            }
            MaterializeStrategy::Append => {
                out.push(format!("INSERT INTO {t}{by_name} {source};"));
            }
            MaterializeStrategy::Merge { unique_key } => {
                // Upsert within the slice via delete-by-key + insert (dbt's
                // `delete+insert`): rows whose key is in the incoming SELECT are
                // replaced, others are left in place. This deliberately avoids
                // `MERGE INTO` — DuckLake's MERGE fails writing the first rows of
                // a fresh partition (HTTP 404 on the new parquet), and a failed
                // write leaves the table needing a DROP. DELETE+INSERT is the
                // same write shape as `replace`, which is reliable. The DELETE is
                // scoped to the current partition when partitioned so it stays
                // slice-local (a key present in another partition is untouched).
                let scope = if self.partitioned {
                    format!("{pcol} = {pval} AND ")
                } else {
                    String::new()
                };
                out.push(format!(
                    "DELETE FROM {t} WHERE {scope}{unique_key} IN (SELECT {unique_key} FROM ({sel}));"
                ));
                out.push(format!("INSERT INTO {t}{by_name} {source};"));
            }
            // Handled by the early return above (scd2 has no partitioned form).
            MaterializeStrategy::Scd2 { .. } => unreachable!("scd2 handled before this match"),
        }
        out.push("COMMIT;".to_string());
        out
    }

    /// SCD2 codegen: the incoming SELECT is the *current desired snapshot* (one
    /// row per `key`); we diff it against the live current rows, close the prior
    /// version of every changed/new key, and open a fresh one — so history is
    /// kept. `track` empty ⇒ every non-key column is tracked for change
    /// detection.
    ///
    /// Shape (all one transaction for the mutation, mirroring the other
    /// strategies so a partial failure leaves the prior snapshot intact):
    ///  1. bootstrap the table (business columns + `valid_from/valid_to/
    ///     is_current`), idempotent;
    ///  2. capture changed+new keys into a connection-local temp table *before*
    ///     the write — the close below flips `is_current`, so recomputing the
    ///     diff after it would see a different set;
    ///  3. close the prior open version of those keys (`UPDATE` — not `MERGE`:
    ///     DuckLake's MERGE is the unreliable path, plain UPDATE works);
    ///  3b. when `close_deleted`, also capture the keys that vanished from the
    ///     snapshot and close their current version (no reopen) — dbt's
    ///     `hard_deletes=close`;
    ///  4. open a new current version from the snapshot;
    ///  5. create the `<dim>_current` convenience view once (`IF NOT EXISTS`),
    ///     inside the same transaction so it doesn't advance the DuckLake snapshot
    ///     past the data write the summary records (and so an unchanged rerun,
    ///     whose UPDATE/INSERT touch no rows, stays a true no-op).
    ///
    /// Close/open match keys with `IS NOT DISTINCT FROM` (via a correlated
    /// `EXISTS`), not `key IN (…)`: SQL `IN` never matches `NULL`, so a `NULL`
    /// natural key would be flagged as changed yet silently skipped by both the
    /// close and the open, dropping the row. Null-safe matching materializes it
    /// instead (a `NULL` key is still ill-formed for a dimension — guard it with
    /// `// data_test not_null <key>` — but it must not vanish).
    ///
    /// Without `close_deleted`, keys present in the table but absent from the
    /// SELECT are left current (soft delete — dbt's `hard_deletes=ignore` default;
    /// with `close_deleted` they are closed instead — see step 3b). The effective
    /// timestamp is `now()`, which DuckDB fixes to
    /// the transaction start, so `valid_from`/`valid_to` are consistent within a
    /// run without a nondeterministic per-statement clock.
    ///
    /// Reserved columns: `valid_from`/`valid_to`/`is_current` are appended to the
    /// user's SELECT with these fixed names (kept clean so consumers write
    /// `WHERE is_current` / `ASOF JOIN … >= valid_from`). A SELECT that already
    /// projects one of them is a v1 constraint violation — the bootstrap then
    /// produces a duplicate output column and the run fails at execution
    /// (documented; not statically checkable here since the SELECT's columns
    /// aren't known at codegen time).
    fn scd2_statements(&self, key: &str, track: &[String], close_deleted: bool) -> Vec<String> {
        let t = self.target_qualified;
        let sel = self.select_sql;
        let k = quote_ident(key);
        let vf = SCD2_VALID_FROM;
        let vt = SCD2_VALID_TO;
        let ic = SCD2_IS_CURRENT;
        let changed = SCD2_CHANGED_KEYS;
        let deleted = SCD2_DELETED_KEYS;
        // Transaction-stable effective timestamp (see doc above). Cast to plain
        // TIMESTAMP so it matches the bootstrapped column type (now() is TZ-aware).
        let ts = "CAST(now() AS TIMESTAMP)";

        // Projection compared to detect change. Empty `track` ⇒ all business
        // columns via `* EXCLUDE (<scd cols>)` on the table side (which carries
        // the extra metadata columns) and `*` on the snapshot side. An explicit
        // `track` ⇒ key + those columns on both sides. `EXCEPT` treats NULLs as
        // equal, so an unchanged NULL is not read as a change.
        let (src_proj, tgt_proj) = if track.is_empty() {
            (
                format!("SELECT * FROM ({sel})"),
                format!("SELECT * EXCLUDE ({vf}, {vt}, {ic}) FROM {t} WHERE {ic}"),
            )
        } else {
            let cols = std::iter::once(key)
                .chain(track.iter().map(String::as_str))
                .map(quote_ident)
                .collect::<Vec<_>>()
                .join(", ");
            (
                format!("SELECT {cols} FROM ({sel})"),
                format!("SELECT {cols} FROM {t} WHERE {ic}"),
            )
        };

        let mut out = vec![
            format!(
                "CREATE TABLE IF NOT EXISTS {t} AS SELECT *, \
                 CAST(NULL AS TIMESTAMP) AS {vf}, \
                 CAST(NULL AS TIMESTAMP) AS {vt}, \
                 CAST(NULL AS BOOLEAN) AS {ic} FROM ({sel}) WHERE false;"
            ),
            format!(
                "CREATE OR REPLACE TEMP TABLE {changed} AS \
                 SELECT {k} FROM ({src_proj} EXCEPT {tgt_proj});"
            ),
        ];
        // Hard-delete-close (`deletes=close`): the keys that vanished from the
        // snapshot — present-and-current in the table, absent from the SELECT.
        // Captured before the close (like `changed`) and disjoint from it (a
        // key is either in the snapshot or not), so the two closes never overlap.
        if close_deleted {
            out.push(format!(
                "CREATE OR REPLACE TEMP TABLE {deleted} AS \
                 SELECT {k} FROM (SELECT {k} FROM {t} WHERE {ic} EXCEPT SELECT {k} FROM ({sel}));"
            ));
        }
        out.push("BEGIN TRANSACTION;".to_string());
        out.push(format!(
            "UPDATE {t} SET {vt} = {ts}, {ic} = false \
             WHERE {ic} AND EXISTS (SELECT 1 FROM {changed} \
             WHERE {changed}.{k} IS NOT DISTINCT FROM {t}.{k});"
        ));
        // Close vanished keys — no matching INSERT below, so they close without
        // reopening. A key that later reappears isn't in `WHERE is_current`, so the
        // `changed` diff treats it as new and opens a fresh version (a validity gap
        // between the delete and the reactivation — correct SCD2).
        if close_deleted {
            out.push(format!(
                "UPDATE {t} SET {vt} = {ts}, {ic} = false \
                 WHERE {ic} AND EXISTS (SELECT 1 FROM {deleted} \
                 WHERE {deleted}.{k} IS NOT DISTINCT FROM {t}.{k});"
            ));
        }
        out.push(format!(
            "INSERT INTO {t} SELECT s.*, {ts} AS {vf}, CAST(NULL AS TIMESTAMP) AS {vt}, \
             true AS {ic} FROM ({sel}) s WHERE EXISTS (SELECT 1 FROM {changed} c \
             WHERE c.{k} IS NOT DISTINCT FROM s.{k});"
        ));
        out.push(
            // Consumer convenience: a `<dim>_current` view (the live slice) so the
            // common "just the latest version" read needs no `WHERE is_current`,
            // and downstream scripts can `// on` / read it directly. For the
            // effective-dated payoff, consumers `ASOF JOIN <dim> ON fact.key =
            // dim.<key> AND fact.ts >= dim.valid_from`.
            //
            // `CREATE VIEW IF NOT EXISTS` (not `OR REPLACE`), created inside the
            // write transaction, on purpose: the view definition never changes
            // (`SELECT * WHERE is_current` always reflects live data), and a
            // catalog write advances the DuckLake snapshot — so `OR REPLACE` on
            // every run would (a) advance the snapshot on an otherwise no-op
            // unchanged run and (b) make the summary's `max(snapshot_id)` record
            // the view DDL instead of the data write. `IF NOT EXISTS` creates it
            // once (folded into the first data-write snapshot) and is a true no-op
            // afterwards. The `<dim>_current` name is reserved: if a real table by
            // that name already exists, `IF NOT EXISTS` skips silently (no view,
            // no error) — documented as a reserved suffix.
            format!("CREATE VIEW IF NOT EXISTS {t}_current AS SELECT * FROM {t} WHERE {ic};"),
        );
        out.push("COMMIT;".to_string());
        out
    }
}

// ---------------------------------------------------------------------------
// on_schema_change drift detection (write-time guardrail)
// ---------------------------------------------------------------------------
//
// Drift is computed entirely in SQL against the live DuckDB session: the
// SELECT's output columns come from `DESCRIBE`, the table's from `DESCRIBE` of
// the target (the managed `_wm_partition` column is excluded so it is compared
// as the producer's logical output, matching schema capture). `added` = SELECT
// columns absent from the table, `removed` = table columns absent from the
// SELECT. When the table was just created this run (first materialize) the two
// DESCRIBEs agree, so both lists are empty and no guard fires.

/// A `DESCRIBE`-derived set of column names of `rel_sql` (any SELECT-able
/// relation, already parenthesized/qualified by the caller), optionally
/// dropping the managed partition column.
fn describe_col_names(rel_sql: &str, exclude_col: Option<&str>) -> String {
    let filter = match exclude_col {
        Some(c) => format!(" WHERE column_name <> {}", quote_lit(c)),
        None => String::new(),
    };
    format!("SELECT column_name FROM (DESCRIBE SELECT * FROM {rel_sql}){filter}")
}

/// Scalar subqueries `(added, removed)` — the list of column names in the
/// SELECT but not the table, and vice versa. Each is a `list(...)` over an
/// `EXCEPT`; empty ⇒ `list()` yields an empty list (`len` 0). The partition
/// column is excluded on the table side only.
///
/// Set difference, not ordered: this catches the add/remove/rename that a
/// positional INSERT misaligns on, but by design NOT a pure reorder of
/// same-named columns (identical sets ⇒ empty added/removed). See the
/// `OnSchemaChange` doc in asset_parser.rs — reorder-safety is `sync`'s job
/// (INSERT BY NAME); the set difference is deliberately kept over an ordered
/// comparison so the `fail` guard cannot false-positive on a correctly-aligned
/// write.
fn drift_lists(sel_sql: &str, target_qualified: &str, partition_col: &str) -> (String, String) {
    let sel = describe_col_names(&format!("({sel_sql})"), None);
    let tbl = describe_col_names(target_qualified, Some(partition_col));
    let added = format!("(SELECT list(column_name) FROM (({sel}) EXCEPT ({tbl})))");
    let removed = format!("(SELECT list(column_name) FROM (({tbl}) EXCEPT ({sel})))");
    (added, removed)
}

/// The `fail`-mode guard: a single statement that raises via DuckDB `error(...)`
/// when the SELECT's columns diverge from the table's, naming the added/removed
/// columns and the target. Both `CASE` branches are cast to VARCHAR so the
/// planner cannot constant-fold the `error(...)` away, and the condition depends
/// on the runtime drift subqueries so it is never folded to a constant.
fn schema_drift_guard_sql(sel_sql: &str, target_qualified: &str, partition_col: &str) -> String {
    let (added, removed) = drift_lists(sel_sql, target_qualified, partition_col);
    // `target_qualified` is safe in table-reference position (quoted identifiers)
    // but here it lands inside a SQL string literal, so single-quotes must be
    // doubled (same as `quote_lit`).
    let tq = target_qualified.replace('\'', "''");
    format!(
        "SELECT CASE WHEN coalesce(len(_wm_added), 0) + coalesce(len(_wm_removed), 0) > 0 \
         THEN CAST(error('managed materialize: on_schema_change=fail blocked a schema-drifted \
         write to {tq} — added column(s): [' || coalesce(array_to_string(_wm_added, ', '), '') || \
         '], removed column(s): [' || coalesce(array_to_string(_wm_removed, ', '), '') || \
         ']. The table schema is fixed at first create; set on_schema_change=sync to auto-migrate, \
         or align the SELECT with the table.') AS VARCHAR) ELSE 'ok' END \
         FROM (SELECT {added} AS _wm_added, {removed} AS _wm_removed);",
        tq = tq,
    )
}

/// The `warn`-mode summary column: a `schema_drift` struct `{added, removed}`
/// when the SELECT drifted from the table, else NULL. Appended to the
/// materialize summary row so the executor can log it and fold it into the job
/// result without an extra round-trip.
fn schema_drift_summary_field(
    sel_sql: &str,
    target_qualified: &str,
    partition_col: &str,
) -> String {
    let (added, removed) = drift_lists(sel_sql, target_qualified, partition_col);
    format!(
        "(SELECT CASE WHEN coalesce(len(_wm_added), 0) + coalesce(len(_wm_removed), 0) > 0 \
         THEN {{'added': _wm_added, 'removed': _wm_removed}} END \
         FROM (SELECT {added} AS _wm_added, {removed} AS _wm_removed)) AS schema_drift"
    )
}

/// The read that captures the DuckLake snapshot id produced by the write, for
/// the given attach alias (e.g. `_wm_target`). The worker runs this last and
/// records the result into `materialized_partition`.
pub fn snapshot_capture_sql(alias: &str) -> String {
    format!("SELECT max(snapshot_id) AS snapshot_id FROM ducklake_snapshots('{alias}');")
}

/// Reserved attach alias for the materialization target, fully-qualified in all
/// generated SQL so a user `USE …;` in the setup blocks can't redirect the
/// write. The worker resolves the real `ATTACH 'ducklake:…' AS _wm_target (…)`
/// from the target ducklake's config and passes it in as `target_attach`.
pub const TARGET_ALIAS: &str = "_wm_target";

/// Structural role of one statement in a [`MaterializePlan`]. The public build
/// executes the plan verbatim, so the kinds are pure metadata there; they exist
/// so a downstream assembler (`pipeline_advanced::finalize_materialize_query`)
/// can reason about the plan without parsing SQL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterializeStmtKind {
    /// Pre-write statement: user setup, the target ATTACH, referenced-asset
    /// ATTACHes.
    Setup,
    /// Write work against the target: bootstrap DDL, SCD2 temp-table captures,
    /// the mutation itself, the `_current` view.
    Write,
    /// The `BEGIN TRANSACTION;` marker emitted by the strategy codegen.
    TxnBegin,
    /// The `COMMIT;` marker emitted by the strategy codegen.
    TxnCommit,
    /// The trailing one-row summary read (asset / rows / snapshot_id /
    /// data_tests breakdown).
    Summary,
}

/// One planned statement: its structural role and the SQL text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterializeStmt {
    pub kind: MaterializeStmtKind,
    pub sql: String,
}

/// The full ordered materialization plan [`build_wrap_blocks`] produces:
/// statements in execution order plus the compiled data-test checks (also
/// embedded in the summary statement's breakdown). Assembled into the final
/// statement list by `pipeline_advanced::finalize_materialize_query`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterializePlan {
    pub stmts: Vec<MaterializeStmt>,
    pub checks: Vec<DataTestCheck>,
}

/// Assemble the full ordered statement list the DuckDB executor runs for a
/// managed `// materialize` script. This is the single entry point the worker
/// calls; it composes the already-tested pieces (classifier split → target
/// ATTACH → strategy codegen → snapshot capture) so their ordering lives in one
/// tested place rather than inline in the executor.
///
/// `target_attach` is the real `ATTACH 'ducklake:…' AS _wm_target (…);` string
/// the worker built from config (it depends on resolved credentials, so it
/// can't be generated here). `target_table` is the table within that catalog
/// (e.g. `orders_daily`), referenced as `_wm_target.<table>`. `asset_path` is
/// the full `<name>/<table>` for the result summary. The trailing statement is
/// a one-row summary read (asset / rows / snapshot_id) that is both the job's
/// result (a useful preview) and what the worker records.
///
/// Returns a [`MaterializePlan`] — the statements plus their structural role
/// and the compiled data-test checks — rather than raw SQL: the executor hands
/// the plan to `windmill_common::pipeline_advanced::finalize_materialize_query`
/// (which this crate cannot depend on), whose public-build implementation
/// assembles the statements verbatim. Everything this function produces runs
/// as-is on the public build; the plan's structure is metadata about it, not a
/// second mode.
pub fn build_wrap_blocks(
    plan: &WrapPlan,
    target_attach: &str,
    target_table: &str,
    asset_path: &str,
    partition_col: &str,
    partition_value_sql: &str,
    partitioned: bool,
    strategy: MaterializeStrategy,
    on_schema_change: OnSchemaChange,
    tests: &[DataTestResolved],
) -> Result<MaterializePlan, String> {
    let target_qualified = format!("{TARGET_ALIAS}.{target_table}");
    let scd2 = matches!(strategy, MaterializeStrategy::Scd2 { .. });
    let ctx = DataTestCtx {
        target_qualified: &target_qualified,
        asset_path,
        partition_col,
        partition_value_sql,
        partitioned,
        scd2,
    };
    let test_sql = build_data_test_checks(tests, &ctx)?;
    let cg = MaterializeCodegen {
        target_qualified: &target_qualified,
        select_sql: &plan.output,
        partition_col,
        partition_value_sql,
        partitioned,
        strategy,
        on_schema_change,
    };
    // `warn` folds the post-write drift into the summary row (executor logs it +
    // returns it) — only for the positional persist-and-mutate path, and only in
    // `warn`: `fail`/`sync` guard the write itself, `ignore` is silent.
    let drift_summary_select =
        if on_schema_change == OnSchemaChange::Warn && cg.is_persist_and_mutate() {
            Some(plan.output.as_str())
        } else {
            None
        };
    let mut stmts: Vec<MaterializeStmt> = Vec::new();
    let setup = |sql: String| MaterializeStmt { kind: MaterializeStmtKind::Setup, sql };
    // Setup blocks come from the splitter with their `;` stripped — re-terminate
    // each so that when the executor re-joins and re-splits the assembled query,
    // adjacent statements (e.g. the user ATTACH and the synthetic target ATTACH)
    // don't merge into one malformed statement.
    stmts.extend(plan.setup.iter().map(|s| setup(terminate(s))));
    stmts.push(setup(target_attach.to_string()));
    // Referenced-asset ATTACHes (relationships tests) — read-only, before the
    // write and the summary that probes them.
    stmts.extend(test_sql.attaches.into_iter().map(setup));
    // Classify the codegen statements by matching the exact transaction-marker
    // literals this module emits (`BEGIN TRANSACTION;` / `COMMIT;`); everything
    // else the codegen produces is write work.
    stmts.extend(cg.statements().into_iter().map(|sql| {
        let kind = match sql.as_str() {
            "BEGIN TRANSACTION;" => MaterializeStmtKind::TxnBegin,
            "COMMIT;" => MaterializeStmtKind::TxnCommit,
            _ => MaterializeStmtKind::Write,
        };
        MaterializeStmt { kind, sql }
    }));
    // The summary read carries the per-test breakdown (when any tests apply).
    stmts.push(MaterializeStmt {
        kind: MaterializeStmtKind::Summary,
        sql: materialize_result_sql(
            &target_qualified,
            asset_path,
            partition_col,
            partition_value_sql,
            partitioned,
            &test_sql.checks,
            drift_summary_select,
        ),
    });
    Ok(MaterializePlan { stmts, checks: test_sql.checks })
}

/// The trailing one-row summary the materialize run returns: the asset it
/// produced, the row count of the materialized slice (the partition when
/// partitioned, else the whole table), and the DuckLake snapshot it created.
/// This is both a useful preview result and the row the worker records.
pub fn materialize_result_sql(
    target_qualified: &str,
    asset_path: &str,
    partition_col: &str,
    partition_value_sql: &str,
    partitioned: bool,
    checks: &[DataTestCheck],
    // `on_schema_change=warn` on a persist-and-mutate strategy: the SELECT to
    // diff against the (post-write) table for the `schema_drift` summary column.
    // `None` ⇒ no drift column (every other mode / strategy).
    drift_summary_select: Option<&str>,
) -> String {
    let (count_expr, partition_sel) = if partitioned {
        // Row count is the slice this run wrote (the partition); `partition`
        // lets the UI label the count and scope the preview to it.
        (
            format!(
                "(SELECT count(*) FROM {target_qualified} WHERE {partition_col} = {partition_value_sql})"
            ),
            format!("{partition_value_sql} AS partition, "),
        )
    } else {
        (
            format!("(SELECT count(*) FROM {target_qualified})"),
            String::new(),
        )
    };
    // Capture the materialized output schema (gap #2a) in the same summary row —
    // no extra round-trip. `DESCRIBE SELECT * FROM <target>` yields one row per
    // column (`column_name`, `column_type`); fold them into a list-of-struct the
    // worker reads back and persists as asset metadata. The write just
    // committed, so the latest snapshot (no `AT (VERSION)` needed) is exactly the
    // slice recorded in `snapshot_id`.
    //
    // Two correctness details:
    // - `_wm_ord` (a `row_number()` over the DESCRIBE) is captured so the
    //   list-of-struct is ordered *explicitly* (`list(... ORDER BY _wm_ord)`).
    //   DESCRIBE returns columns in physical order; without the explicit ORDER
    //   the `list()` aggregate could reorder them and spuriously bump the schema
    //   version on a re-materialize.
    // - For a `// partitioned` asset the physical table carries the synthetic
    //   `_wm_partition` column; it must be filtered out so the recorded schema is
    //   the producer's logical output, not Windmill's storage detail (this is the
    //   grain #2b contract enforcement reads back).
    let partition_filter = if partitioned {
        format!(
            " WHERE column_name <> '{}'",
            partition_col.replace('\'', "''")
        )
    } else {
        String::new()
    };
    let schema_capture = format!(
        "(SELECT list({{'name': column_name, 'type': column_type}} ORDER BY _wm_ord) \
         FROM (SELECT column_name, column_type, row_number() OVER () AS _wm_ord \
               FROM (DESCRIBE SELECT * FROM {target_qualified}){partition_filter})) AS output_schema"
    );
    // `on_schema_change=warn`: fold the drift `{added, removed}` (or NULL) into
    // the same row so the executor logs it + returns it with no extra probe.
    let drift_col = match drift_summary_select {
        Some(sel) => format!(
            ", {}",
            schema_drift_summary_field(sel, target_qualified, partition_col)
        ),
        None => String::new(),
    };
    let base_cols = format!(
        "'ducklake://{asset_path}' AS materialized, \
         {partition_sel}{count_expr} AS rows, \
         (SELECT max(snapshot_id) FROM ducklake_snapshots('{TARGET_ALIAS}')) AS snapshot_id, \
         {schema_capture}{drift_col}"
    );
    if checks.is_empty() {
        return format!("SELECT {base_cols};");
    }
    // Per-test breakdown. Each check's one-row probe `(v, s)` becomes a CTE
    // (`_wm_t0`, `_wm_t1`, …); `_wm_tr` cross-joins them (all one-row, so the
    // join stays one row) and the `data_tests` list-of-struct references the
    // flattened columns — DuckDB rejects scalar subqueries *inside* a
    // struct/list literal, hence the CTE lift. Names are single-quote-escaped.
    // The result row carries the whole breakdown so the worker runs every
    // test (no abort-on-first) and decides pass/fail itself.
    let probe_ctes = checks
        .iter()
        .enumerate()
        .map(|(i, c)| format!("_wm_t{i} AS ({})", c.probe))
        .collect::<Vec<_>>()
        .join(", ");
    let tr_cols = checks
        .iter()
        .enumerate()
        .map(|(i, _)| format!("_wm_t{i}.v AS c{i}, _wm_t{i}.s AS s{i}"))
        .collect::<Vec<_>>()
        .join(", ");
    let tr_from = checks
        .iter()
        .enumerate()
        .map(|(i, _)| format!("_wm_t{i}"))
        .collect::<Vec<_>>()
        .join(", ");
    let list_items = checks
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let name = c.name.replace('\'', "''");
            format!("{{'test': '{name}', 'violating': c{i}, 'sample': s{i}}}")
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "WITH {probe_ctes}, _wm_tr AS (SELECT {tr_cols} FROM {tr_from}) \
         SELECT {base_cols}, [{list_items}] AS data_tests FROM _wm_tr;"
    )
}

// Ensure a statement ends with a single `;`.
fn terminate(stmt: &str) -> String {
    let t = stmt.trim_end();
    if t.ends_with(';') {
        t.to_string()
    } else {
        format!("{t};")
    }
}

// ---------------------------------------------------------------------------
// Data tests (`// data_test`)
// ---------------------------------------------------------------------------
//
// A data test is the FIRST extensible annotation: the parser yields a
// `DataTest` from a known vocabulary, and this module turns each into a
// *check* — a `(name, probe)` pair whose probe counts and samples the
// violating rows — that runs against the freshly-materialized target after
// the write commits. The materialize summary query embeds every check's
// outcome in one `data_tests` column, so all tests run in a single pass (no
// abort-on-first) and the worker, not the SQL, decides pass/fail and reports
// the full per-test breakdown.
//
// The pattern is deliberately open: a verifier is just `(name, violating-rows
// query)` handed to `push_check`. Built-ins differ only in their rows query;
// the `Custom` escape hatch supplies its own (a user SELECT returning the
// violating rows). A sibling annotation family (column-lineage) can emit its
// own checks through the same `push_check` shape rather than bolting on a
// parallel mechanism. See `docs/ducklake-materialization.md`.

use crate::asset_parser::{AssetKind, DataTest, OnSchemaChange};

/// Target context a data-test probe runs against — the materialized table and
/// the partition slice (when partitioned, tests are scoped to the slice just
/// written, so a rerun/backfill is independent of other partitions' data).
#[derive(Debug, Clone)]
pub struct DataTestCtx<'a> {
    /// Fully-qualified materialized target, e.g. `_wm_target.orders`.
    pub target_qualified: &'a str,
    /// `<name>/<table>` of the target, for human-readable probe messages.
    pub asset_path: &'a str,
    /// Physical partition column on the managed table.
    pub partition_col: &'a str,
    /// SQL literal/expression for the current partition value (already escaped).
    pub partition_value_sql: &'a str,
    /// Whether the target is partitioned (scopes probes to the slice).
    pub partitioned: bool,
    /// Whether the target is an SCD2 history table. Built-in probes then assert
    /// the *current snapshot* (`is_current` rows): the history legitimately
    /// repeats the natural key across closed versions, so an unscoped
    /// `unique(<key>)` would fail on the second change of any key.
    pub scd2: bool,
}

/// A data test resolved enough to generate SQL. Built-ins carry only their
/// parsed `DataTest`; `Custom` additionally carries the fetched script body
/// (the parser crate can't fetch it — the worker does and passes it in).
#[derive(Debug, Clone)]
pub enum DataTestResolved {
    BuiltIn(DataTest),
    Custom { path: String, body: String },
}

/// One compiled data-test check: a human-readable `name` and a one-row probe
/// query yielding `(v, s)` — the violating-row count (0 = pass) and a bounded
/// `to_json` sample of the violating rows as a VARCHAR (NULL when there are
/// none, or when the serialized sample exceeds the size cap). The materialize
/// summary query embeds every check's outcome so the worker gets the whole
/// breakdown in one result — all tests run (no abort-on-first) and the
/// worker, not the SQL, decides pass/fail. The sample is decoration only:
/// enforcement reads `v`, never `s`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataTestCheck {
    pub name: String,
    /// One-row subquery `SELECT … AS v, … AS s FROM (…)` counting and
    /// sampling the check's violating rows in a single scan.
    pub probe: String,
}

/// Row cap on a data-test sample. Bounded so the sample stays a debugging aid
/// (the full count is still reported); no ORDER BY on the violating rows, so
/// which rows land in the sample is nondeterministic.
const SAMPLE_MAX_ROWS: usize = 20;
/// Byte cap on one serialized sample. Oversized samples are dropped entirely
/// (NULL), never truncated — truncated JSON would fail parsing downstream
/// after paying the bytes anyway.
const SAMPLE_MAX_BYTES: usize = 51_200;

/// The SQL a set of data tests compiles to: referenced-asset `ATTACH`
/// statements (resolved by the executor's ATTACH-transform pass) and the
/// per-test checks, both in declaration order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DataTestChecks {
    pub attaches: Vec<String>,
    pub checks: Vec<DataTestCheck>,
}

/// Alias prefix for a relationships test's referenced asset, attached
/// read-only alongside the target. `_wm_ref_<n>` so it never collides with the
/// user's aliases or the reserved `_wm_target`.
const REF_ALIAS_PREFIX: &str = "_wm_ref_";

// Double-quote a SQL identifier, escaping embedded quotes — so an arbitrary
// column name from an annotation can't break out of the identifier.
fn quote_ident(id: &str) -> String {
    format!("\"{}\"", id.replace('"', "\"\""))
}

// Quote a possibly schema-qualified table reference (`schema.table`) by quoting
// each dotted segment independently: `main.dim_products` → `"main"."dim_products"`.
// Quoting the whole thing would make DuckDB read it as one table name containing
// a literal dot, querying the wrong table.
fn quote_qualified(name: &str) -> String {
    name.split('.')
        .map(quote_ident)
        .collect::<Vec<_>>()
        .join(".")
}

// Single-quote a SQL string literal, escaping embedded quotes.
fn quote_lit(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

// The `WHERE`/`AND` fragment scoping a probe to the current partition and/or
// the SCD2 current snapshot, or empty when neither applies. `prefix` is
// `WHERE ` or `AND ` per call site; further conditions chain with `AND`.
// (`partitioned` and `scd2` are mutually exclusive today — the combo is
// rejected at codegen — but the chaining keeps this correct if that changes.)
fn partition_scope(ctx: &DataTestCtx, prefix: &str, table_alias: Option<&str>) -> String {
    let qualify = |col: String| match table_alias {
        Some(a) => format!("{a}.{col}"),
        None => col,
    };
    let mut conds: Vec<String> = Vec::new();
    if ctx.partitioned {
        conds.push(format!(
            "{} = {}",
            qualify(quote_ident(ctx.partition_col)),
            ctx.partition_value_sql
        ));
    }
    if ctx.scd2 {
        conds.push(qualify(SCD2_IS_CURRENT.to_string()));
    }
    if conds.is_empty() {
        return String::new();
    }
    format!("{prefix}{}", conds.join(" AND "))
}

// Record one check: its display `name` plus `rows_query`, the SELECT of its
// violating rows. The probe counts and samples those rows in one scan:
// `_wm_v` (the subquery alias) referenced as a column is the whole row as a
// struct; `to_json(...)::VARCHAR` keeps the sample a JSON *string* through
// the FFI — expanded rows would be visible to the executor's key-recursive
// `extract_i64(result, "rows"/"snapshot_id")` scans, which a user column of
// the same name could corrupt. `list()` over zero rows and an over-cap
// sample both degrade to NULL (`s` is optional by contract).
fn push_check(out: &mut DataTestChecks, name: String, rows_query: String) {
    // `strlen` counts bytes (unlike `length`, characters) — the cap bounds
    // payload size on the wire, so bytes are the right unit.
    let probe = format!(
        "SELECT v, CASE WHEN strlen(s_raw) <= {SAMPLE_MAX_BYTES} THEN s_raw END AS s \
         FROM (SELECT count(*) AS v, \
         to_json(list(_wm_v ORDER BY _wm_rn) FILTER (WHERE _wm_rn <= {SAMPLE_MAX_ROWS}))::VARCHAR AS s_raw \
         FROM (SELECT _wm_v, row_number() OVER () AS _wm_rn FROM ({rows_query}) _wm_v))"
    );
    out.checks.push(DataTestCheck { name, probe });
}

// `SELECT *` for a sample rows-query, excluding the synthetic physical
// partition column on partitioned targets — it's Windmill's storage detail,
// not part of the producer's logical output (same rule as schema capture).
// `qualifier` scopes the star when the query aliases the target (`_wm_src`).
fn sample_star(ctx: &DataTestCtx, qualifier: Option<&str>) -> String {
    let star = match qualifier {
        Some(q) => format!("{q}.*"),
        None => "*".to_string(),
    };
    if ctx.partitioned {
        format!("{star} EXCLUDE ({})", quote_ident(ctx.partition_col))
    } else {
        star
    }
}

// Self-teaching tail appended to every malformed-custom-test error. It states
// the two rules that aren't documented or scaffolded anywhere else — the body
// is a single SELECT, and it reads the freshly-materialized target through the
// internal `_wm_target.<table>` alias — and doubles that alias into a copyable
// one-line example. `target_qualified` is already `_wm_target.<table>`.
fn custom_test_hint(target_qualified: &str) -> String {
    format!(
        "Write a single SELECT against `{target_qualified}` returning the offending rows, e.g. \
         `SELECT * FROM {target_qualified} WHERE <condition>` — an empty result means the test \
         passes."
    )
}

// Whether a custom-test statement reads the materialized target through the
// reserved `_wm_target` alias (the only handle the runtime attaches it under).
// SQL identifiers are case-insensitive, so match case-insensitively;
// `split_statements` has already stripped comments, so a match here is a real
// reference, not one buried in a comment. `TARGET_ALIAS` is lowercase.
fn references_target(stmt: &str) -> bool {
    stmt.to_lowercase().contains(TARGET_ALIAS)
}

/// Compile resolved data tests into ATTACH statements + per-test checks for
/// `ctx`'s target. Pure: returns SQL text, executes nothing. Errors carry an
/// actionable message (e.g. a relationships target that isn't an attachable
/// table).
pub fn build_data_test_checks(
    tests: &[DataTestResolved],
    ctx: &DataTestCtx,
) -> Result<DataTestChecks, String> {
    let t = ctx.target_qualified;
    let mut out = DataTestChecks::default();
    // Dedup ref attaches by (kind, name): a database can't be attached twice,
    // so multiple relationships into the same db share one alias.
    let mut ref_aliases: Vec<(AssetKind, String, String)> = Vec::new();

    for resolved in tests {
        match resolved {
            DataTestResolved::BuiltIn(DataTest::Unique { column }) => {
                let c = quote_ident(column);
                let scope = partition_scope(ctx, " AND ", None);
                // The rows are the GROUP BY result — one `{value, count}` per
                // duplicated key — so the count (number of duplicated values,
                // not of rows) and the sample share one grain and can't
                // contradict each other in the UI.
                let q = format!(
                    "SELECT {c} AS \"value\", count(*) AS \"count\" FROM {t} \
                     WHERE {c} IS NOT NULL{scope} GROUP BY {c} HAVING count(*) > 1"
                );
                push_check(&mut out, format!("unique({column})"), q);
            }
            DataTestResolved::BuiltIn(DataTest::NotNull { column }) => {
                let c = quote_ident(column);
                let scope = partition_scope(ctx, " AND ", None);
                let star = sample_star(ctx, None);
                let q = format!("SELECT {star} FROM {t} WHERE {c} IS NULL{scope}");
                push_check(&mut out, format!("not_null({column})"), q);
            }
            DataTestResolved::BuiltIn(DataTest::AcceptedValues { column, values }) => {
                let c = quote_ident(column);
                let scope = partition_scope(ctx, " AND ", None);
                let star = sample_star(ctx, None);
                let list = values
                    .iter()
                    .map(|v| quote_lit(v))
                    .collect::<Vec<_>>()
                    .join(", ");
                let q = format!(
                    "SELECT {star} FROM {t} WHERE {c} IS NOT NULL AND {c} NOT IN ({list}){scope}"
                );
                push_check(&mut out, format!("accepted_values({column})"), q);
            }
            DataTestResolved::BuiltIn(DataTest::Relationships {
                column,
                to_kind,
                to_path,
                to_column,
            }) => {
                let (ref_name, ref_table) = to_path.split_once('/').ok_or_else(|| {
                    format!("data_test relationships: target `{to_path}` must be `<name>/<table>`")
                })?;
                if ref_table.is_empty() {
                    return Err(format!(
                        "data_test relationships: target `{to_path}` has no table"
                    ));
                }
                let scheme = match to_kind {
                    AssetKind::Ducklake => "ducklake",
                    AssetKind::DataTable => "datatable",
                    other => {
                        return Err(format!(
                            "data_test relationships: target kind {other:?} is not an attachable \
                             table (use ducklake:// or datatable://)"
                        ))
                    }
                };
                // The materialize target's ducklake is already attached as
                // `_wm_target`; a reference into that same lake must reuse it
                // rather than ATTACH the same database again under a fresh alias
                // (DuckDB forbids attaching one database twice). `asset_path` is
                // the target's `<lake>/<table>`, so its lake is the part before
                // the first `/`.
                let target_lake = ctx.asset_path.split('/').next().unwrap_or("");
                let alias = if *to_kind == AssetKind::Ducklake && ref_name == target_lake {
                    TARGET_ALIAS.to_string()
                } else {
                    // Reuse an existing alias for the same (kind, name), else mint one.
                    match ref_aliases
                        .iter()
                        .find(|(k, n, _)| k == to_kind && n == ref_name)
                    {
                        Some((_, _, a)) => a.clone(),
                        None => {
                            let a = format!("{REF_ALIAS_PREFIX}{}", ref_aliases.len());
                            // Escape the name — it is interpolated into a
                            // single-quoted DuckDB literal (defense-in-depth: the
                            // name is deploy-time annotation content, but the parser
                            // places no character restriction on asset paths).
                            let esc_name = ref_name.replace('\'', "''");
                            out.attaches
                                .push(format!("ATTACH '{scheme}://{esc_name}' AS {a};"));
                            ref_aliases.push((*to_kind, ref_name.to_string(), a.clone()));
                            a
                        }
                    }
                };
                let c = quote_ident(column);
                let rc = quote_ident(to_column);
                // `ref_table` may be schema-qualified (`schema.table`); quote each
                // segment so the dot stays a schema separator, not a literal.
                let rt = quote_qualified(ref_table);
                let scope = partition_scope(ctx, " AND ", Some("_wm_src"));
                let star = sample_star(ctx, Some("_wm_src"));
                let q = format!(
                    "SELECT {star} FROM {t} _wm_src \
                     WHERE _wm_src.{c} IS NOT NULL{scope} \
                     AND NOT EXISTS (SELECT 1 FROM {alias}.{rt} _wm_ref \
                     WHERE _wm_ref.{rc} = _wm_src.{c})"
                );
                push_check(
                    &mut out,
                    format!("relationships({column} -> {to_path}.{to_column})"),
                    q,
                );
            }
            // A parsed Custom must be resolved (body fetched) before codegen.
            DataTestResolved::BuiltIn(DataTest::Custom { path }) => {
                return Err(format!(
                    "data_test custom `{path}`: body not resolved before codegen (internal)"
                ));
            }
            DataTestResolved::Custom { path, body } => {
                // dbt singular-test convention: the body is a *single* SELECT
                // (or CTE) returning the violating rows, reading the
                // freshly-materialized target through the internal `_wm_target`
                // schema. It is embedded as a subquery (`FROM (<body>)`), so a
                // multi-statement or non-SELECT body would produce invalid SQL.
                // Neither rule is documented or scaffolded elsewhere, so the
                // errors are self-teaching: they name the exact violation and
                // append a correct one-line example. It runs in the target's
                // connection (can read `_wm_target` + the user's attaches);
                // partition substitution is already applied by the worker.
                let hint = custom_test_hint(t);
                let stmts = split_statements(body);
                if stmts.is_empty() {
                    return Err(format!(
                        "data_test custom `{path}`: empty test body. {hint}"
                    ));
                }
                if stmts.len() > 1 {
                    return Err(format!(
                        "data_test custom `{path}`: a custom data test must be a single SELECT, \
                         but found {} statements. {hint}",
                        stmts.len()
                    ));
                }
                let stmt = &stmts[0];
                if classify_block(stmt) != BlockClass::Output {
                    return Err(format!(
                        "data_test custom `{path}`: a custom data test must be a single SELECT, \
                         not a write or DDL statement. {hint}"
                    ));
                }
                if !references_target(stmt) {
                    return Err(format!(
                        "data_test custom `{path}`: the test never reads the freshly-materialized \
                         target — reference it through the internal `{TARGET_ALIAS}` schema (as \
                         `{t}`), not the table name on its own. {hint}"
                    ));
                }
                push_check(&mut out, format!("custom({path})"), stmt.to_string());
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(sql: &str) -> WrapPlan {
        classify_wrap(sql).expect("expected wrap-eligible")
    }
    fn err(sql: &str) -> WrapError {
        classify_wrap(sql).expect_err("expected wrap-ineligible")
    }

    #[test]
    fn split_respects_strings_comments_idents() {
        let sql = "SET x=1; -- a; comment\nSELECT ';' AS a, \"weird;col\" /* ; */ FROM t;";
        let s = split_statements(sql);
        assert_eq!(s.len(), 2);
        assert_eq!(s[0], "SET x=1");
        assert!(s[1].starts_with("SELECT"));
        assert!(s[1].contains("\"weird;col\""));
    }

    #[test]
    fn split_handles_escaped_quote() {
        let s = split_statements("SELECT 'it''s; fine' AS a;");
        assert_eq!(s.len(), 1);
        assert!(s[0].contains("it''s; fine"));
    }

    #[test]
    fn pipeline_annotations_are_stripped() {
        // The real shape: `//` annotation lines above the SQL must not pollute
        // the first block's classification (regression — they were being read
        // as a leading `pipeline` keyword and rejected).
        let p = ok("// pipeline\n// materialize ducklake://main/t\n// partitioned daily\nATTACH 'ducklake://main' AS dl;\nSELECT 1 AS id");
        assert_eq!(p.setup.len(), 1);
        // The annotation lines are gone — the setup block starts at the real
        // SQL (the `//` inside `ducklake://main` is legitimately retained).
        assert!(p.setup[0].starts_with("ATTACH"));
        assert!(p.output.starts_with("SELECT"));
    }

    #[test]
    fn bare_select_is_eligible() {
        let p = ok("SELECT a, b FROM t WHERE c = '{partition}'");
        assert!(p.setup.is_empty());
        assert!(p.output.starts_with("SELECT"));
    }

    #[test]
    fn setup_then_select_is_eligible() {
        let p = ok(
            "ATTACH 'ducklake://main' AS dl;\n SET memory_limit='4GB';\n SELECT * FROM dl.orders",
        );
        assert_eq!(p.setup.len(), 2);
        assert!(p.output.starts_with("SELECT"));
    }

    #[test]
    fn create_temp_staging_is_setup() {
        let p = ok("CREATE TEMP TABLE s AS SELECT 1; SELECT * FROM s");
        assert_eq!(p.setup.len(), 1);
        assert_eq!(
            classify_block("CREATE TEMP TABLE s AS SELECT 1"),
            BlockClass::Setup
        );
        assert_eq!(
            classify_block("CREATE OR REPLACE TEMPORARY VIEW v AS SELECT 1"),
            BlockClass::Setup
        );
    }

    #[test]
    fn with_cte_select_is_output_write_is_disallowed() {
        assert_eq!(
            classify_block("WITH x AS (SELECT 1) SELECT * FROM x"),
            BlockClass::Output
        );
        // CTE whose main statement inserts is a write, even though it starts WITH.
        assert_eq!(
            classify_block("WITH x AS (SELECT 1) INSERT INTO t SELECT * FROM x"),
            BlockClass::Disallowed
        );
    }

    #[test]
    fn from_first_and_values_are_output() {
        assert_eq!(classify_block("FROM t SELECT a"), BlockClass::Output);
        assert_eq!(classify_block("VALUES (1),(2)"), BlockClass::Output);
        assert_eq!(classify_block("TABLE t"), BlockClass::Output);
    }

    #[test]
    fn trailing_write_rejected() {
        assert_eq!(
            err("SELECT * FROM t; INSERT INTO u VALUES (1)"),
            WrapError::OutputNotLast
        );
    }

    #[test]
    fn write_in_preamble_rejected() {
        match err("INSERT INTO t VALUES (1); SELECT * FROM t") {
            WrapError::DisallowedBlock { snippet } => assert!(snippet.starts_with("INSERT")),
            e => panic!("wrong error: {e:?}"),
        }
    }

    #[test]
    fn multiple_selects_rejected() {
        assert_eq!(
            err("SELECT 1; SELECT 2"),
            WrapError::MultipleOutputs { count: 2 }
        );
    }

    #[test]
    fn no_select_and_empty_rejected() {
        assert_eq!(err("CREATE TABLE t (a INT)"), WrapError::NoOutput);
        assert_eq!(err("   -- just a comment\n"), WrapError::Empty);
    }

    #[test]
    fn use_cannot_redirect_is_classified_setup() {
        // `USE` is allowed setup; generated SQL is fully qualified regardless.
        assert_eq!(classify_block("USE dl"), BlockClass::Setup);
    }

    #[test]
    fn codegen_replace_partitioned() {
        let cg = MaterializeCodegen {
            target_qualified: "_wm_target.orders_daily",
            select_sql: "SELECT a FROM dl.orders",
            partition_col: "_wm_partition",
            partition_value_sql: "'2026-06-19'",
            partitioned: true,
            strategy: MaterializeStrategy::Replace,
            on_schema_change: OnSchemaChange::Warn,
        };
        let st = cg.statements();
        assert!(st[0].contains("CREATE TABLE IF NOT EXISTS _wm_target.orders_daily"));
        assert!(st[0].contains("CAST(NULL AS VARCHAR) AS _wm_partition"));
        assert!(st.iter().any(
            |s| s == "ALTER TABLE _wm_target.orders_daily SET PARTITIONED BY (_wm_partition);"
        ));
        assert!(st.iter().any(|s| s.starts_with(
            "DELETE FROM _wm_target.orders_daily WHERE _wm_partition = '2026-06-19'"
        )));
        assert!(st.iter().any(|s| s.contains(
            "INSERT INTO _wm_target.orders_daily SELECT *, '2026-06-19' AS _wm_partition"
        )));
        assert_eq!(st.first().map(|_| &st[st.len() - 1]).unwrap(), "COMMIT;");
    }

    #[test]
    fn codegen_merge_is_delete_by_key_plus_insert() {
        let cg = MaterializeCodegen {
            target_qualified: "_wm_target.orders_daily",
            select_sql: "SELECT order_id, amount FROM dl.orders",
            partition_col: "_wm_partition",
            partition_value_sql: "'2026-06-19'",
            partitioned: true,
            strategy: MaterializeStrategy::Merge { unique_key: "order_id".to_string() },
            on_schema_change: OnSchemaChange::Warn,
        };
        let st = cg.statements();
        // upsert = delete-by-key (partition-scoped) + insert — NO `MERGE INTO`
        // (DuckLake's MERGE fails on fresh partitions).
        assert!(!st.iter().any(|s| s.contains("MERGE INTO")));
        let del = st
            .iter()
            .find(|s| s.starts_with("DELETE FROM"))
            .expect("delete stmt");
        assert!(del.contains(
            "WHERE _wm_partition = '2026-06-19' AND order_id IN (SELECT order_id FROM (SELECT order_id, amount FROM dl.orders))"
        ));
        assert!(st
            .iter()
            .any(|s| s.starts_with("INSERT INTO _wm_target.orders_daily SELECT *, '2026-06-19'")));
    }

    #[test]
    fn codegen_append_inserts_only() {
        let cg = MaterializeCodegen {
            target_qualified: "_wm_target.events",
            select_sql: "SELECT * FROM dl.raw",
            partition_col: "_wm_partition",
            partition_value_sql: "'2026-06-19'",
            partitioned: true,
            strategy: MaterializeStrategy::Append,
            on_schema_change: OnSchemaChange::Warn,
        };
        let st = cg.statements();
        assert!(st
            .iter()
            .any(|s| s.starts_with("INSERT INTO _wm_target.events")));
        assert!(!st.iter().any(|s| s.starts_with("DELETE")));
        assert!(!st.iter().any(|s| s.starts_with("MERGE")));
    }

    #[test]
    fn codegen_whole_table_replace_is_create_or_replace() {
        // Unpartitioned replace must use CREATE OR REPLACE so a changed SELECT
        // schema (or a pre-existing table with a different schema) doesn't break
        // — and nothing else (no bootstrap / DELETE / INSERT / txn).
        let cg = MaterializeCodegen {
            target_qualified: "_wm_target.customer_dim",
            select_sql: "SELECT a, b, c FROM dl.src",
            partition_col: "_wm_partition",
            partition_value_sql: "''",
            partitioned: false,
            strategy: MaterializeStrategy::Replace,
            on_schema_change: OnSchemaChange::Warn,
        };
        let st = cg.statements();
        assert_eq!(
            st,
            vec![
                "CREATE OR REPLACE TABLE _wm_target.customer_dim AS SELECT * FROM (SELECT a, b, c FROM dl.src);"
                    .to_string()
            ]
        );
    }

    #[test]
    fn codegen_scd2_default_track_closes_old_opens_new() {
        // Empty `track` ⇒ diff on all business columns via `* EXCLUDE (scd cols)`.
        let cg = MaterializeCodegen {
            target_qualified: "_wm_target.dim_scd2",
            select_sql: "SELECT id, name FROM dl.src",
            partition_col: "_wm_partition",
            partition_value_sql: "''",
            partitioned: false,
            strategy: MaterializeStrategy::Scd2 {
                key: "id".to_string(),
                track: vec![],
                close_deleted: false,
            },
            on_schema_change: OnSchemaChange::Warn,
        };
        let st = cg.statements();
        // bootstrap adds the three SCD metadata columns
        assert!(st[0].starts_with("CREATE TABLE IF NOT EXISTS _wm_target.dim_scd2 AS SELECT *,"));
        assert!(st[0].contains("AS valid_from"));
        assert!(st[0].contains("AS valid_to"));
        assert!(st[0].contains("AS is_current"));
        // changed-key set captured before the transaction, all cols compared
        assert!(
            st[1].contains("CREATE OR REPLACE TEMP TABLE _wm_scd2_changed AS SELECT \"id\" FROM")
        );
        assert!(st[1].contains("SELECT * FROM (SELECT id, name FROM dl.src) EXCEPT"));
        assert!(st[1].contains("SELECT * EXCLUDE (valid_from, valid_to, is_current) FROM _wm_target.dim_scd2 WHERE is_current"));
        assert_eq!(st[2], "BEGIN TRANSACTION;");
        // close: UPDATE (not MERGE) the prior open version of changed keys, with
        // null-safe key matching (IS NOT DISTINCT FROM, not IN — IN drops NULLs)
        assert!(st[3].starts_with("UPDATE _wm_target.dim_scd2 SET valid_to = CAST(now() AS TIMESTAMP), is_current = false"));
        assert!(st[3].contains(
            "WHERE is_current AND EXISTS (SELECT 1 FROM _wm_scd2_changed \
             WHERE _wm_scd2_changed.\"id\" IS NOT DISTINCT FROM _wm_target.dim_scd2.\"id\");"
        ));
        // open: INSERT the new current version, null-safe key matching
        assert!(st[4].starts_with(
            "INSERT INTO _wm_target.dim_scd2 SELECT s.*, CAST(now() AS TIMESTAMP) AS valid_from"
        ));
        assert!(st[4].contains(
            "true AS is_current FROM (SELECT id, name FROM dl.src) s WHERE EXISTS \
             (SELECT 1 FROM _wm_scd2_changed c WHERE c.\"id\" IS NOT DISTINCT FROM s.\"id\");"
        ));
        // consumer-convenience `<dim>_current` view: `IF NOT EXISTS` (created once,
        // no-op on unchanged reruns) and INSIDE the txn (folded into the write snapshot)
        assert_eq!(
            st[5],
            "CREATE VIEW IF NOT EXISTS _wm_target.dim_scd2_current AS SELECT * FROM _wm_target.dim_scd2 WHERE is_current;"
        );
        assert_eq!(st[6], "COMMIT;");
        // no fragile constructs: no MERGE INTO, and no NULL-dropping `IN (SELECT`
        assert!(!st.iter().any(|s| s.contains("MERGE INTO")));
        assert!(!st.iter().any(|s| s.contains("IN (SELECT")));
        // soft-delete default: no deleted-key set, no second close
        assert!(!st.iter().any(|s| s.contains("_wm_scd2_deleted")));
    }

    #[test]
    fn codegen_scd2_explicit_track_projects_key_and_tracked_cols() {
        let cg = MaterializeCodegen {
            target_qualified: "_wm_target.dim",
            select_sql: "SELECT id, name, addr FROM dl.src",
            partition_col: "_wm_partition",
            partition_value_sql: "''",
            partitioned: false,
            strategy: MaterializeStrategy::Scd2 {
                key: "id".to_string(),
                track: vec!["name".to_string()],
                close_deleted: false,
            },
            on_schema_change: OnSchemaChange::Warn,
        };
        let st = cg.statements();
        // only key + tracked cols are compared (addr changes don't rotate a version)
        assert!(st[1].contains("SELECT \"id\", \"name\" FROM (SELECT id, name, addr FROM dl.src) EXCEPT SELECT \"id\", \"name\" FROM _wm_target.dim WHERE is_current"));
    }

    #[test]
    fn codegen_scd2_close_deleted_adds_deleted_set_and_second_close() {
        let cg = MaterializeCodegen {
            target_qualified: "_wm_target.dim",
            select_sql: "SELECT id, name FROM dl.src",
            partition_col: "_wm_partition",
            partition_value_sql: "''",
            partitioned: false,
            strategy: MaterializeStrategy::Scd2 {
                key: "id".to_string(),
                track: vec![],
                close_deleted: true,
            },
            on_schema_change: OnSchemaChange::Warn,
        };
        let st = cg.statements();
        // the deleted-key set: current keys absent from the snapshot, captured
        // before the transaction (like `changed`)
        assert!(st.iter().any(|s| s.contains(
            "CREATE OR REPLACE TEMP TABLE _wm_scd2_deleted AS SELECT \"id\" FROM \
             (SELECT \"id\" FROM _wm_target.dim WHERE is_current EXCEPT SELECT \"id\" FROM (SELECT id, name FROM dl.src));"
        )));
        // a second close UPDATE against the deleted set (null-safe), and NO INSERT
        // that reopens deleted keys (the only INSERT filters on `_wm_scd2_changed`)
        assert!(st
            .iter()
            .any(|s| s.starts_with("UPDATE _wm_target.dim SET valid_to")
                && s.contains(
                    "EXISTS (SELECT 1 FROM _wm_scd2_deleted \
                WHERE _wm_scd2_deleted.\"id\" IS NOT DISTINCT FROM _wm_target.dim.\"id\");"
                )));
        assert_eq!(
            st.iter().filter(|s| s.starts_with("INSERT INTO")).count(),
            1
        );
        assert!(st
            .iter()
            .find(|s| s.starts_with("INSERT INTO"))
            .unwrap()
            .contains("_wm_scd2_changed"));
        // the deleted close is inside the transaction (between BEGIN and COMMIT)
        let begin = st.iter().position(|s| s == "BEGIN TRANSACTION;").unwrap();
        let commit = st.iter().position(|s| s == "COMMIT;").unwrap();
        let del_close = st
            .iter()
            .position(|s| s.starts_with("UPDATE") && s.contains("_wm_scd2_deleted"))
            .unwrap();
        assert!(begin < del_close && del_close < commit);
    }

    fn persist_cg(strategy: MaterializeStrategy, osc: OnSchemaChange) -> Vec<String> {
        MaterializeCodegen {
            target_qualified: "_wm_target.t",
            select_sql: "SELECT a, c FROM dl.src",
            partition_col: "_wm_partition",
            partition_value_sql: "''",
            partitioned: false,
            strategy,
            on_schema_change: osc,
        }
        .statements()
    }

    #[test]
    fn codegen_fail_emits_drift_guard_inside_txn_before_write() {
        let st = persist_cg(
            MaterializeStrategy::Merge { unique_key: "a".into() },
            OnSchemaChange::Fail,
        );
        let begin = st.iter().position(|s| s == "BEGIN TRANSACTION;").unwrap();
        let guard = st
            .iter()
            .position(|s| s.contains("error(") && s.contains("on_schema_change=fail"))
            .expect("fail emits a guard");
        let del = st
            .iter()
            .position(|s| s.starts_with("DELETE FROM"))
            .unwrap();
        let insert = st
            .iter()
            .position(|s| s.starts_with("INSERT INTO"))
            .unwrap();
        let commit = st.iter().position(|s| s == "COMMIT;").unwrap();
        // guard runs right after BEGIN and before any mutation, inside the txn
        assert!(begin < guard && guard < del && del < insert && insert < commit);
        // both CASE branches are VARCHAR so the planner can't fold error() away
        assert!(st[guard].contains("CAST(error("));
        assert!(st[guard].contains("ELSE 'ok' END"));
        // fail is positional (no BY NAME) and emits no sync sentinel
        assert!(st[insert].starts_with("INSERT INTO _wm_target.t SELECT"));
        assert!(!st.iter().any(|s| s == SYNC_ALTER_SENTINEL));
    }

    #[test]
    fn fail_guard_drift_is_name_set_based_not_ordered() {
        // Documents a deliberate boundary: the fail guard fires on the column
        // SET difference (EXCEPT over column_name), so a pure reorder of
        // same-named columns is NOT caught here — that is `sync`'s job (BY NAME).
        // Keeping this name-set (not an ordered comparison) is what prevents the
        // guard from false-positive-aborting a correctly-aligned write. If this
        // ever moves to an ordered comparison, update the OnSchemaChange doc.
        let guard =
            schema_drift_guard_sql("SELECT b, a FROM dl.src", "_wm_target.t", "_wm_partition");
        assert!(guard.contains("column_name"));
        assert!(guard.contains("EXCEPT"));
        // No positional/ordinal comparison in the guard condition.
        assert!(!guard.to_lowercase().contains("ordinal"));
        assert!(!guard.to_lowercase().contains("row_number"));
    }

    #[test]
    fn codegen_sync_uses_by_name_and_sentinel() {
        let st = persist_cg(MaterializeStrategy::Append, OnSchemaChange::Sync);
        let begin = st.iter().position(|s| s == "BEGIN TRANSACTION;").unwrap();
        let sentinel = st.iter().position(|s| s == SYNC_ALTER_SENTINEL).unwrap();
        let insert = st
            .iter()
            .position(|s| s.starts_with("INSERT INTO"))
            .unwrap();
        // the ALTER-injection slot is right after BEGIN, before the write
        assert!(begin < sentinel && sentinel < insert);
        // name-mapped insert (a positional insert would cross-wire ALTERed cols)
        assert!(st[insert].starts_with("INSERT INTO _wm_target.t BY NAME SELECT"));
        assert!(!st.iter().any(|s| s.contains("error(")));
    }

    #[test]
    fn codegen_ignore_and_warn_write_positionally_no_guard() {
        for osc in [OnSchemaChange::Ignore, OnSchemaChange::Warn] {
            let st = persist_cg(MaterializeStrategy::Append, osc);
            assert!(!st.iter().any(|s| s.contains("error(")), "{osc:?}");
            assert!(!st.iter().any(|s| s == SYNC_ALTER_SENTINEL), "{osc:?}");
            let insert = st.iter().find(|s| s.starts_with("INSERT INTO")).unwrap();
            assert!(
                insert.starts_with("INSERT INTO _wm_target.t SELECT"),
                "{osc:?}"
            );
        }
    }

    #[test]
    fn codegen_whole_table_replace_and_scd2_ignore_guardrail() {
        // Whole-table replace (unpartitioned) and scd2 are not persist-and-mutate:
        // fail/sync must not add a guard/sentinel/BY NAME there.
        let repl = MaterializeCodegen {
            target_qualified: "_wm_target.t",
            select_sql: "SELECT a FROM dl.src",
            partition_col: "_wm_partition",
            partition_value_sql: "''",
            partitioned: false,
            strategy: MaterializeStrategy::Replace,
            on_schema_change: OnSchemaChange::Fail,
        };
        assert!(!repl.is_persist_and_mutate());
        let st = repl.statements();
        assert!(!st
            .iter()
            .any(|s| s.contains("error(") || s == SYNC_ALTER_SENTINEL));

        let scd2 = MaterializeCodegen {
            strategy: MaterializeStrategy::Scd2 {
                key: "a".into(),
                track: vec![],
                close_deleted: false,
            },
            on_schema_change: OnSchemaChange::Sync,
            ..repl
        };
        assert!(!scd2.is_persist_and_mutate());
        let st = scd2.statements();
        assert!(!st.iter().any(|s| s == SYNC_ALTER_SENTINEL));
        assert!(!st.iter().any(|s| s.contains(" BY NAME ")));
    }

    #[test]
    fn summary_schema_drift_field_only_for_warn_persist_and_mutate() {
        // warn + persist-and-mutate ⇒ summary carries the drift column
        let warn = plan_for_osc(MaterializeStrategy::Append, false, OnSchemaChange::Warn);
        assert!(warn.stmts.last().unwrap().sql.contains("AS schema_drift"));
        // ignore / fail / sync ⇒ no summary drift column (they guard the write)
        for osc in [
            OnSchemaChange::Ignore,
            OnSchemaChange::Fail,
            OnSchemaChange::Sync,
        ] {
            let p = plan_for_osc(MaterializeStrategy::Append, false, osc);
            assert!(
                !p.stmts.last().unwrap().sql.contains("schema_drift"),
                "{osc:?} must not emit the summary drift column"
            );
        }
        // whole-table replace + warn ⇒ not persist-and-mutate ⇒ no drift column
        let repl = plan_for_osc(MaterializeStrategy::Replace, false, OnSchemaChange::Warn);
        assert!(!repl.stmts.last().unwrap().sql.contains("schema_drift"));
    }

    #[test]
    fn fail_guard_is_write_kind_between_txn_markers() {
        use MaterializeStmtKind::*;
        let plan = plan_for_osc(
            MaterializeStrategy::Merge { unique_key: "a".into() },
            false,
            OnSchemaChange::Fail,
        );
        let begin = kidx(&plan, |s| s.kind == TxnBegin);
        let commit = kidx(&plan, |s| s.kind == TxnCommit);
        let guard = kidx(&plan, |s| {
            s.sql.contains("error(") && s.sql.contains("on_schema_change=fail")
        });
        assert_eq!(plan.stmts[guard].kind, Write);
        assert!(begin < guard && guard < commit);
    }

    #[test]
    fn snapshot_capture_targets_alias() {
        assert_eq!(
            snapshot_capture_sql("_wm_target"),
            "SELECT max(snapshot_id) AS snapshot_id FROM ducklake_snapshots('_wm_target');"
        );
    }

    #[test]
    fn build_wrap_blocks_orders_setup_attach_codegen_snapshot() {
        let plan = ok("ATTACH 'ducklake://main' AS dl;\n SELECT a FROM dl.orders WHERE d = '{p}'");
        let blocks: Vec<String> = build_wrap_blocks(
            &plan,
            "ATTACH 'ducklake:postgres:…' AS _wm_target (DATA_PATH 's3://b/p');",
            "orders_daily",
            "main/orders_daily",
            "_wm_partition",
            "'2026-06-19'",
            true,
            MaterializeStrategy::Replace,
            OnSchemaChange::Warn,
            &[],
        )
        .unwrap()
        .stmts
        .into_iter()
        .map(|s| s.sql)
        .collect();
        // setup block first, then the target ATTACH, then codegen, then result.
        assert!(blocks[0].starts_with("ATTACH 'ducklake://main' AS dl"));
        // every setup block must be `;`-terminated so re-splitting can't merge it
        // with the synthetic target ATTACH that follows.
        assert!(blocks[0].ends_with(';'));
        assert_eq!(
            blocks[1],
            "ATTACH 'ducklake:postgres:…' AS _wm_target (DATA_PATH 's3://b/p');"
        );
        assert!(blocks.iter().any(|b| b.contains("_wm_target.orders_daily")));
        assert!(blocks.iter().any(|b| b.starts_with(
            "DELETE FROM _wm_target.orders_daily WHERE _wm_partition = '2026-06-19'"
        )));
        // the trailing block is the one-row summary (asset / rows / snapshot_id),
        // partition-scoped for the row count
        let last = blocks.last().unwrap();
        assert!(last.contains("'ducklake://main/orders_daily' AS materialized"));
        assert!(last.contains("'2026-06-19' AS partition"));
        assert!(last.contains("WHERE _wm_partition = '2026-06-19') AS rows"));
        assert!(last.contains("ducklake_snapshots('_wm_target')"));
    }

    // -- materialize plan structure ------------------------------------------

    fn plan_for(strategy: MaterializeStrategy, partitioned: bool) -> MaterializePlan {
        plan_for_osc(strategy, partitioned, OnSchemaChange::Warn)
    }

    fn plan_for_osc(
        strategy: MaterializeStrategy,
        partitioned: bool,
        on_schema_change: OnSchemaChange,
    ) -> MaterializePlan {
        let plan = ok("SELECT a, b FROM src");
        build_wrap_blocks(
            &plan,
            "ATTACH 'ducklake:…' AS _wm_target;",
            "orders",
            "main/orders",
            "_wm_partition",
            "'2026-06-19'",
            partitioned,
            strategy,
            on_schema_change,
            &[
                DataTestResolved::BuiltIn(DataTest::NotNull { column: "a".into() }),
                DataTestResolved::BuiltIn(DataTest::Unique { column: "b".into() }),
            ],
        )
        .unwrap()
    }

    fn kidx(plan: &MaterializePlan, pred: impl Fn(&MaterializeStmt) -> bool) -> usize {
        plan.stmts
            .iter()
            .position(|s| pred(s))
            .expect("stmt present")
    }

    #[test]
    fn plan_tags_structure_and_carries_checks() {
        use MaterializeStmtKind::*;
        let plan = plan_for(MaterializeStrategy::Replace, true);
        // leading statements are Setup, ending with the target ATTACH
        assert!(plan.stmts[0].kind == Setup);
        assert!(plan
            .stmts
            .iter()
            .take_while(|s| s.kind == Setup)
            .any(|s| s.sql.contains("_wm_target")));
        // txn markers are tagged, everything between them is Write
        let begin = kidx(&plan, |s| s.kind == TxnBegin);
        let commit = kidx(&plan, |s| s.kind == TxnCommit);
        assert!(begin < commit);
        assert!(plan.stmts[begin + 1..commit]
            .iter()
            .all(|s| s.kind == Write));
        // bootstrap DDL is Write work (it targets the table, not the session)
        let bootstrap = kidx(&plan, |s| s.sql.starts_with("CREATE TABLE IF NOT EXISTS"));
        assert_eq!(plan.stmts[bootstrap].kind, Write);
        // summary is last and carries the breakdown; checks ride along
        let last = plan.stmts.last().unwrap();
        assert_eq!(last.kind, Summary);
        assert!(last.sql.contains("AS data_tests"));
        assert_eq!(plan.checks.len(), 2);
        assert!(plan.checks[0].name.contains("not_null(a)"));
    }

    #[test]
    fn plan_whole_table_replace_has_no_txn_markers() {
        use MaterializeStmtKind::*;
        let plan = plan_for(MaterializeStrategy::Replace, false);
        assert!(!plan.stmts.iter().any(|s| s.kind == TxnBegin));
        assert!(!plan.stmts.iter().any(|s| s.kind == TxnCommit));
        assert_eq!(
            plan.stmts.iter().filter(|s| s.kind == Write).count(),
            1,
            "single atomic CREATE OR REPLACE"
        );
    }

    #[test]
    fn plan_scd2_captures_are_write_kind() {
        use MaterializeStmtKind::*;
        let plan = plan_for(
            MaterializeStrategy::Scd2 { key: "a".into(), track: vec![], close_deleted: false },
            false,
        );
        let capture = kidx(&plan, |s| s.sql.contains("TEMP TABLE _wm_scd2_changed"));
        assert_eq!(plan.stmts[capture].kind, Write);
        // no test declared ⇒ empty checks
        let plain = ok("SELECT a FROM src");
        let no_tests = build_wrap_blocks(
            &plain,
            "ATTACH 'ducklake:…' AS _wm_target;",
            "orders",
            "main/orders",
            "_wm_partition",
            "''",
            false,
            MaterializeStrategy::Append,
            OnSchemaChange::Warn,
            &[],
        )
        .unwrap();
        assert!(no_tests.checks.is_empty());
    }

    // -- data tests ---------------------------------------------------------

    fn ctx_partitioned() -> DataTestCtx<'static> {
        DataTestCtx {
            target_qualified: "_wm_target.orders",
            asset_path: "analytics/orders",
            partition_col: "_wm_partition",
            partition_value_sql: "'2026-06-19'",
            partitioned: true,
            scd2: false,
        }
    }
    fn ctx_unpartitioned() -> DataTestCtx<'static> {
        DataTestCtx { partitioned: false, ..ctx_partitioned() }
    }
    fn ctx_scd2() -> DataTestCtx<'static> {
        DataTestCtx { partitioned: false, scd2: true, ..ctx_partitioned() }
    }

    #[test]
    fn data_test_unique_and_not_null_partition_scoped() {
        let tests = vec![
            DataTestResolved::BuiltIn(DataTest::Unique { column: "order_id".into() }),
            DataTestResolved::BuiltIn(DataTest::NotNull { column: "user_id".into() }),
        ];
        let sql = build_data_test_checks(&tests, &ctx_partitioned()).unwrap();
        assert!(sql.attaches.is_empty());
        assert_eq!(sql.checks.len(), 2);
        // short, asset-free names (the asset is shown once by the breakdown).
        assert_eq!(sql.checks[0].name, "unique(order_id)");
        assert_eq!(sql.checks[1].name, "not_null(user_id)");
        // each probe counts and samples the violating rows in one scan, with
        // the size guard on the serialized sample.
        for c in &sql.checks {
            assert!(c
                .probe
                .starts_with("SELECT v, CASE WHEN strlen(s_raw) <= 51200 THEN s_raw END AS s"));
            assert!(c.probe.contains("count(*) AS v"));
            assert!(c.probe.contains("FILTER (WHERE _wm_rn <= 20)"));
        }
        // unique: groups non-null keys within the slice, having count>1; the
        // sample is `{value, count}` pairs at the same grain as the count.
        assert!(sql.checks[0]
            .probe
            .contains("GROUP BY \"order_id\" HAVING count(*) > 1"));
        assert!(sql.checks[0]
            .probe
            .contains("SELECT \"order_id\" AS \"value\", count(*) AS \"count\""));
        assert!(sql.checks[0]
            .probe
            .contains("\"order_id\" IS NOT NULL AND \"_wm_partition\" = '2026-06-19'"));
        // not_null: null rows in the slice; the sample excludes the synthetic
        // partition column (storage detail, not producer output).
        assert!(sql.checks[1]
            .probe
            .contains("WHERE \"user_id\" IS NULL AND \"_wm_partition\" = '2026-06-19'"));
        assert!(sql.checks[1]
            .probe
            .contains("SELECT * EXCLUDE (\"_wm_partition\") FROM"));
    }

    #[test]
    fn data_test_unpartitioned_has_no_partition_scope() {
        let tests = vec![DataTestResolved::BuiltIn(DataTest::NotNull {
            column: "id".into(),
        })];
        let sql = build_data_test_checks(&tests, &ctx_unpartitioned()).unwrap();
        assert!(sql.checks[0].probe.contains("WHERE \"id\" IS NULL"));
        assert!(!sql.checks[0].probe.contains("_wm_partition"));
        assert!(!sql.checks[0].probe.contains("EXCLUDE"));
    }

    #[test]
    fn data_test_scd2_scopes_builtins_to_current_rows() {
        // On an SCD2 history table the natural key repeats across closed
        // versions, so built-in probes must assert the current snapshot only.
        let tests = vec![
            DataTestResolved::BuiltIn(DataTest::Unique { column: "customer_id".into() }),
            DataTestResolved::BuiltIn(DataTest::NotNull { column: "tier".into() }),
            DataTestResolved::BuiltIn(DataTest::AcceptedValues {
                column: "region".into(),
                values: vec!["emea".into()],
            }),
        ];
        let sql = build_data_test_checks(&tests, &ctx_scd2()).unwrap();
        assert!(sql.checks[0]
            .probe
            .contains("WHERE \"customer_id\" IS NOT NULL AND is_current"));
        assert!(sql.checks[1]
            .probe
            .contains("WHERE \"tier\" IS NULL AND is_current"));
        assert!(sql.checks[2].probe.contains("AND is_current"));
        // no partition scope leaks in (scd2 is unpartitioned in v1)
        assert!(!sql.checks[0].probe.contains("_wm_partition"));
    }

    #[test]
    fn data_test_accepted_values_escapes_literals() {
        let tests = vec![DataTestResolved::BuiltIn(DataTest::AcceptedValues {
            column: "status".into(),
            values: vec!["paid".into(), "o'brien".into()],
        })];
        let sql = build_data_test_checks(&tests, &ctx_unpartitioned()).unwrap();
        assert!(sql.checks[0].probe.contains("NOT IN ('paid', 'o''brien')"));
        assert!(sql.checks[0].probe.contains("\"status\" IS NOT NULL"));
    }

    #[test]
    fn data_test_relationships_attaches_ref_and_dedups() {
        let tests = vec![
            DataTestResolved::BuiltIn(DataTest::Relationships {
                column: "user_id".into(),
                to_kind: AssetKind::DataTable,
                to_path: "prod/users".into(),
                to_column: "id".into(),
            }),
            // second relationship into the SAME db reuses the alias (no 2nd attach)
            DataTestResolved::BuiltIn(DataTest::Relationships {
                column: "buyer_id".into(),
                to_kind: AssetKind::DataTable,
                to_path: "prod/buyers".into(),
                to_column: "id".into(),
            }),
        ];
        let sql = build_data_test_checks(&tests, &ctx_partitioned()).unwrap();
        assert_eq!(sql.attaches.len(), 1, "same db attached once");
        assert_eq!(sql.attaches[0], "ATTACH 'datatable://prod' AS _wm_ref_0;");
        assert!(sql.checks[0]
            .probe
            .contains("NOT EXISTS (SELECT 1 FROM _wm_ref_0.\"users\""));
        assert!(sql.checks[1]
            .probe
            .contains("NOT EXISTS (SELECT 1 FROM _wm_ref_0.\"buyers\""));
        assert!(sql.checks[0]
            .probe
            .contains("_wm_src.\"_wm_partition\" = '2026-06-19'"));
        // sample rows come from the aliased target and drop the synthetic
        // partition column.
        assert!(sql.checks[0]
            .probe
            .contains("SELECT _wm_src.* EXCLUDE (\"_wm_partition\") FROM"));
        assert_eq!(
            sql.checks[0].name,
            "relationships(user_id -> prod/users.id)"
        );
    }

    #[test]
    fn data_test_relationships_escapes_ref_name_in_attach() {
        let tests = vec![DataTestResolved::BuiltIn(DataTest::Relationships {
            column: "k".into(),
            to_kind: AssetKind::DataTable,
            to_path: "ev'il/users".into(),
            to_column: "id".into(),
        })];
        let sql = build_data_test_checks(&tests, &ctx_unpartitioned()).unwrap();
        // single quote in the name is doubled so it can't break out of the literal
        assert_eq!(sql.attaches[0], "ATTACH 'datatable://ev''il' AS _wm_ref_0;");
    }

    #[test]
    fn data_test_relationships_same_lake_reuses_target() {
        // A relationship into the SAME ducklake as the materialize target
        // (asset_path = "analytics/orders") must NOT re-ATTACH it — _wm_target
        // already holds that catalog; reuse it.
        let tests = vec![DataTestResolved::BuiltIn(DataTest::Relationships {
            column: "user_id".into(),
            to_kind: AssetKind::Ducklake,
            to_path: "analytics/users".into(),
            to_column: "id".into(),
        })];
        let sql = build_data_test_checks(&tests, &ctx_unpartitioned()).unwrap();
        assert!(
            sql.attaches.is_empty(),
            "same-lake ref must not ATTACH again"
        );
        assert!(sql.checks[0]
            .probe
            .contains("NOT EXISTS (SELECT 1 FROM _wm_target.\"users\""));
    }

    #[test]
    fn data_test_relationships_schema_qualified_target() {
        // `<lake>/<schema>.<table>` — the schema-qualified table must quote each
        // segment so the dot stays a separator, not part of one identifier.
        let tests = vec![DataTestResolved::BuiltIn(DataTest::Relationships {
            column: "sku".into(),
            to_kind: AssetKind::Ducklake,
            to_path: "warehouse/main.dim_products".into(),
            to_column: "sku".into(),
        })];
        let sql = build_data_test_checks(&tests, &ctx_unpartitioned()).unwrap();
        assert_eq!(
            sql.attaches[0],
            "ATTACH 'ducklake://warehouse' AS _wm_ref_0;"
        );
        assert!(
            sql.checks[0]
                .probe
                .contains("FROM _wm_ref_0.\"main\".\"dim_products\""),
            "schema-qualified target should be quoted per segment: {}",
            sql.checks[0].probe
        );
    }

    #[test]
    fn data_test_relationships_rejects_non_attachable_kind() {
        let tests = vec![DataTestResolved::BuiltIn(DataTest::Relationships {
            column: "k".into(),
            to_kind: AssetKind::S3Object,
            to_path: "bucket/file".into(),
            to_column: "c".into(),
        })];
        assert!(build_data_test_checks(&tests, &ctx_unpartitioned()).is_err());
    }

    #[test]
    fn data_test_custom_wraps_body() {
        let tests = vec![DataTestResolved::Custom {
            path: "f/tests/amount".into(),
            body: "SELECT * FROM _wm_target.orders WHERE amount < 0;".into(),
        }];
        let sql = build_data_test_checks(&tests, &ctx_unpartitioned()).unwrap();
        // trailing ; stripped, body embedded as the probe's rows query
        assert!(sql.checks[0]
            .probe
            .contains("FROM (SELECT * FROM _wm_target.orders WHERE amount < 0) _wm_v"));
        assert!(sql.checks[0].probe.contains("count(*) AS v"));
        assert_eq!(sql.checks[0].name, "custom(f/tests/amount)");
    }

    #[test]
    fn data_test_custom_rejects_multi_statement_body() {
        // The body is embedded as a subquery, so a setup-then-SELECT body would
        // produce invalid SQL — reject it up front with a self-teaching error
        // that names the violation and shows the correct single-SELECT shape.
        let tests = vec![DataTestResolved::Custom {
            path: "f/tests/amount".into(),
            body: "SET threads = 1; SELECT * FROM _wm_target.orders WHERE amount < 0".into(),
        }];
        let err = build_data_test_checks(&tests, &ctx_unpartitioned()).unwrap_err();
        assert!(err.contains("single SELECT"), "unexpected error: {err}");
        assert!(
            err.contains("found 2 statements"),
            "unexpected error: {err}"
        );
        // the copyable example points at the internal target alias.
        assert!(
            err.contains("SELECT * FROM _wm_target.orders WHERE <condition>"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn data_test_custom_rejects_non_select_body() {
        // A write/DDL body can't be embedded as `FROM (<body>)`; the error must
        // say so and teach the single-SELECT convention.
        let tests = vec![DataTestResolved::Custom {
            path: "f/tests/amount".into(),
            body: "DELETE FROM _wm_target.orders WHERE amount < 0".into(),
        }];
        let err = build_data_test_checks(&tests, &ctx_unpartitioned()).unwrap_err();
        assert!(err.contains("single SELECT"), "unexpected error: {err}");
        assert!(
            err.contains("SELECT * FROM _wm_target.orders WHERE <condition>"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn data_test_custom_rejects_wrong_target_alias() {
        // Referencing the target by its bare table name (not `_wm_target.<table>`)
        // is the most common custom-test mistake — the runtime only attaches the
        // freshly-materialized target under `_wm_target`, so the query would fail
        // at runtime. Catch it at codegen with a self-teaching error.
        let tests = vec![DataTestResolved::Custom {
            path: "f/tests/amount".into(),
            body: "SELECT * FROM orders WHERE amount < 0".into(),
        }];
        let err = build_data_test_checks(&tests, &ctx_unpartitioned()).unwrap_err();
        assert!(err.contains("_wm_target"), "unexpected error: {err}");
        assert!(
            err.contains("SELECT * FROM _wm_target.orders WHERE <condition>"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn data_test_custom_accepts_from_first_and_uppercased_alias() {
        // DuckDB's FROM-first syntax is a valid Output, and the alias match is
        // case-insensitive (SQL identifiers are), so this passes.
        let tests = vec![DataTestResolved::Custom {
            path: "f/tests/amount".into(),
            body: "FROM _WM_TARGET.orders WHERE amount < 0".into(),
        }];
        let sql = build_data_test_checks(&tests, &ctx_unpartitioned()).unwrap();
        assert!(sql.checks[0]
            .probe
            .contains("FROM (FROM _WM_TARGET.orders WHERE amount < 0) _wm_v"));
    }

    #[test]
    fn data_test_custom_empty_body_teaches_shape() {
        let tests = vec![DataTestResolved::Custom {
            path: "f/tests/amount".into(),
            body: "   \n-- just a comment\n".into(),
        }];
        let err = build_data_test_checks(&tests, &ctx_unpartitioned()).unwrap_err();
        assert!(err.contains("empty test body"), "unexpected error: {err}");
        assert!(
            err.contains("SELECT * FROM _wm_target.orders WHERE <condition>"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn data_test_unresolved_custom_is_internal_error() {
        let tests = vec![DataTestResolved::BuiltIn(DataTest::Custom {
            path: "f/x".into(),
        })];
        assert!(build_data_test_checks(&tests, &ctx_unpartitioned()).is_err());
    }

    #[test]
    fn materialize_result_sql_embeds_data_tests_breakdown() {
        let checks = vec![
            DataTestCheck { name: "unique(order_id)".into(), probe: "SELECT v, s FROM q0".into() },
            DataTestCheck { name: "custom(f/t)".into(), probe: "SELECT v, s FROM q1".into() },
        ];
        let sql = materialize_result_sql(
            "_wm_target.orders",
            "analytics/orders",
            "_wm_partition",
            "'2026-06-19'",
            false,
            &checks,
            None,
        );
        // each probe runs once as a one-row CTE; _wm_tr cross-joins them and
        // the list-of-struct references the flattened count/sample columns.
        assert!(sql.starts_with(
            "WITH _wm_t0 AS (SELECT v, s FROM q0), _wm_t1 AS (SELECT v, s FROM q1), \
             _wm_tr AS (SELECT _wm_t0.v AS c0, _wm_t0.s AS s0, _wm_t1.v AS c1, _wm_t1.s AS s1 \
             FROM _wm_t0, _wm_t1)"
        ));
        assert!(sql.contains("[{'test': 'unique(order_id)', 'violating': c0, 'sample': s0}, "));
        assert!(
            sql.contains("{'test': 'custom(f/t)', 'violating': c1, 'sample': s1}] AS data_tests")
        );
        assert!(sql.contains("FROM _wm_tr;"));
        // no tests -> plain summary, no CTE / data_tests column.
        let plain = materialize_result_sql(
            "_wm_target.orders",
            "analytics/orders",
            "_wm_partition",
            "'x'",
            false,
            &[],
            None,
        );
        assert!(plain.starts_with("SELECT 'ducklake://analytics/orders' AS materialized"));
        assert!(!plain.contains("data_tests"));
        // Schema capture (gap #2a) is in every summary, tests or not. Unpartitioned
        // → explicit ordering, no partition-column filter.
        for s in [&sql, &plain] {
            assert!(s.contains(
                "(SELECT list({'name': column_name, 'type': column_type} ORDER BY _wm_ord) \
                 FROM (SELECT column_name, column_type, row_number() OVER () AS _wm_ord \
                 FROM (DESCRIBE SELECT * FROM _wm_target.orders))) AS output_schema"
            ));
            assert!(!s.contains("WHERE column_name <>"));
        }
    }

    #[test]
    fn materialize_result_sql_schema_excludes_partition_column() {
        // Partitioned → the synthetic `_wm_partition` column is filtered out so
        // the captured schema is the producer's logical output only.
        let sql = materialize_result_sql(
            "_wm_target.orders_daily",
            "analytics/orders_daily",
            "_wm_partition",
            "'2026-06-19'",
            true,
            &[],
            None,
        );
        assert!(sql.contains(
            "FROM (DESCRIBE SELECT * FROM _wm_target.orders_daily) \
             WHERE column_name <> '_wm_partition')) AS output_schema"
        ));
    }
}
