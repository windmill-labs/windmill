//! Wrap-mode classifier + materialization SQL codegen for `// materialize`.
//!
//! `// materialize wrap` promises the script is "setup statements, then one
//! trailing SELECT" — Windmill generates the write DDL around that SELECT. This
//! module is the single source of truth for *which block is that SELECT* and
//! *what DDL gets generated*, so save-time validation (deploy path) and
//! run-time codegen (DuckDB executor) can never disagree about it.
//!
//! Everything here is pure and string-level: no SQL is executed, no type
//! inference is done. The classifier is leading-keyword based and deliberately
//! conservative — anything it can't positively recognize as a read-only output
//! or a known-safe setup statement is rejected, so a script is only accepted
//! for wrapping when its shape is unambiguous.

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
    /// wrap (the user wants literal mode).
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

/// Why a script is not eligible for `// materialize wrap`. Carries enough to
/// render the targeted save-time messages from the spec.
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
        let base = "`// materialize wrap` requires the script to end in a single SELECT";
        match self {
            WrapError::Empty => format!("{base}: the script is empty."),
            WrapError::NoOutput => {
                format!("{base}: found no SELECT — remove `wrap` to write the DDL yourself.")
            }
            WrapError::MultipleOutputs { count } => format!(
                "{base}: found {count} SELECT statements; combine them with a CTE, or use no-wrap."
            ),
            WrapError::OutputNotLast => format!(
                "{base}: found statements after the SELECT — move them above it, or use no-wrap."
            ),
            WrapError::DisallowedBlock { snippet } => format!(
                "{base}: `{snippet}` writes or is unrecognized — remove `wrap` to write the DDL yourself."
            ),
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
    let bytes = sql.as_bytes();
    let mut i = 0;
    let n = bytes.len();
    while i < n {
        let c = bytes[i] as char;
        // line comment
        if c == '-' && i + 1 < n && bytes[i + 1] == b'-' {
            while i < n && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        // block comment
        if c == '/' && i + 1 < n && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < n && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
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
                cur.push(bytes[i] as char);
                if bytes[i] == b'\'' {
                    // doubled '' is an escaped quote, stay in string
                    if i + 1 < n && bytes[i + 1] == b'\'' {
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
                cur.push(bytes[i] as char);
                if bytes[i] == b'"' {
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

/// Validate a script for `// materialize wrap` and, on success, return the
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
/// Derived at deploy from `unique_key`/`append`: `append` → `Append`, else
/// `unique_key` → `Merge`, else `Replace`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaterializeStrategy {
    /// DELETE the current partition, then INSERT — partition becomes exactly
    /// what the SELECT returned. Full-refresh of the slice.
    Replace,
    /// MERGE on `unique_key` — upsert within the slice; rows absent from the
    /// SELECT are left in place.
    Merge { unique_key: String },
    /// INSERT only — immutable event-log semantics.
    Append,
}

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
}

impl<'a> MaterializeCodegen<'a> {
    /// The ordered statements that perform the materialization, to be run after
    /// the setup blocks and inside the caller's execution. The first-run
    /// bootstrap is idempotent (`IF NOT EXISTS`), so this is safe to run every
    /// time. The DELETE/INSERT/MERGE body is wrapped in one transaction so a
    /// partial failure leaves the prior snapshot intact.
    ///
    /// Note: the exact MERGE / `INSERT *` dialect targets DuckDB+DuckLake and
    /// must be validated against the deployed DuckDB version — this function
    /// fixes the *shape*, not the dialect minutiae.
    pub fn statements(&self) -> Vec<String> {
        let t = self.target_qualified;
        let sel = self.select_sql;
        let pcol = self.partition_col;
        let pval = self.partition_value_sql;
        let mut out = Vec::new();

        // First-run bootstrap: create the managed table with the SELECT's
        // schema (+ partition col) but no rows.
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
        // The rows to write, with the partition column appended when partitioned.
        let source = if self.partitioned {
            format!("SELECT *, {pval} AS {pcol} FROM ({sel})")
        } else {
            format!("SELECT * FROM ({sel})")
        };
        match &self.strategy {
            MaterializeStrategy::Replace => {
                if self.partitioned {
                    out.push(format!("DELETE FROM {t} WHERE {pcol} = {pval};"));
                } else {
                    out.push(format!("DELETE FROM {t};"));
                }
                out.push(format!("INSERT INTO {t} {source};"));
            }
            MaterializeStrategy::Append => {
                out.push(format!("INSERT INTO {t} {source};"));
            }
            MaterializeStrategy::Merge { unique_key } => {
                // Scope the match to the current partition when partitioned, so
                // the merge is slice-local: a row with the same unique_key in
                // another partition must not match (and so must not have its
                // partition column rewritten by `UPDATE SET *`). Without this
                // the dedup key would have to be globally unique.
                let on = if self.partitioned {
                    format!("t.{unique_key} = s.{unique_key} AND t.{pcol} = s.{pcol}")
                } else {
                    format!("t.{unique_key} = s.{unique_key}")
                };
                out.push(format!(
                    "MERGE INTO {t} AS t USING ({source}) AS s ON {on} \
                     WHEN MATCHED THEN UPDATE SET * \
                     WHEN NOT MATCHED THEN INSERT *;"
                ));
            }
        }
        out.push("COMMIT;".to_string());
        out
    }
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

/// Assemble the full ordered statement list the DuckDB executor runs for a
/// `// materialize wrap` script. This is the single entry point the worker
/// calls; it composes the already-tested pieces (classifier split → target
/// ATTACH → strategy codegen → snapshot capture) so their ordering lives in one
/// tested place rather than inline in the executor.
///
/// `target_attach` is the real `ATTACH 'ducklake:…' AS _wm_target (…);` string
/// the worker built from config (it depends on resolved credentials, so it
/// can't be generated here). `target_table` is the table within that catalog
/// (e.g. `orders_daily`), referenced as `_wm_target.<table>`. The trailing
/// statement is the snapshot-capture read whose scalar the worker records.
pub fn build_wrap_blocks(
    plan: &WrapPlan,
    target_attach: &str,
    target_table: &str,
    partition_col: &str,
    partition_value_sql: &str,
    partitioned: bool,
    strategy: MaterializeStrategy,
) -> Vec<String> {
    let target_qualified = format!("{TARGET_ALIAS}.{target_table}");
    let cg = MaterializeCodegen {
        target_qualified: &target_qualified,
        select_sql: &plan.output,
        partition_col,
        partition_value_sql,
        partitioned,
        strategy,
    };
    let mut blocks: Vec<String> = Vec::new();
    blocks.extend(plan.setup.iter().cloned());
    blocks.push(target_attach.to_string());
    blocks.extend(cg.statements());
    blocks.push(snapshot_capture_sql(TARGET_ALIAS));
    blocks
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
    fn codegen_merge_uses_unique_key() {
        let cg = MaterializeCodegen {
            target_qualified: "_wm_target.orders_daily",
            select_sql: "SELECT order_id, amount FROM dl.orders",
            partition_col: "_wm_partition",
            partition_value_sql: "'2026-06-19'",
            partitioned: true,
            strategy: MaterializeStrategy::Merge { unique_key: "order_id".to_string() },
        };
        let st = cg.statements();
        let merge = st
            .iter()
            .find(|s| s.starts_with("MERGE INTO"))
            .expect("merge stmt");
        assert!(merge.contains("ON t.order_id = s.order_id"));
        // partitioned merge must be slice-local (see codegen comment)
        assert!(merge.contains("AND t._wm_partition = s._wm_partition"));
        assert!(merge.contains("WHEN MATCHED THEN UPDATE SET *"));
        assert!(!st.iter().any(|s| s.starts_with("DELETE")));
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
        };
        let st = cg.statements();
        assert!(st
            .iter()
            .any(|s| s.starts_with("INSERT INTO _wm_target.events")));
        assert!(!st.iter().any(|s| s.starts_with("DELETE")));
        assert!(!st.iter().any(|s| s.starts_with("MERGE")));
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
        let blocks = build_wrap_blocks(
            &plan,
            "ATTACH 'ducklake:postgres:…' AS _wm_target (DATA_PATH 's3://b/p');",
            "orders_daily",
            "_wm_partition",
            "'2026-06-19'",
            true,
            MaterializeStrategy::Replace,
        );
        // setup block first, then the target ATTACH, then codegen, then capture.
        assert!(blocks[0].starts_with("ATTACH 'ducklake://main' AS dl"));
        assert_eq!(
            blocks[1],
            "ATTACH 'ducklake:postgres:…' AS _wm_target (DATA_PATH 's3://b/p');"
        );
        assert!(blocks.iter().any(|b| b.contains("_wm_target.orders_daily")));
        assert!(blocks.iter().any(|b| b.starts_with(
            "DELETE FROM _wm_target.orders_daily WHERE _wm_partition = '2026-06-19'"
        )));
        assert!(blocks
            .last()
            .unwrap()
            .contains("ducklake_snapshots('_wm_target')"));
    }
}
