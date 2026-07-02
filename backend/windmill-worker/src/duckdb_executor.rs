use std::cell::RefCell;
use std::env;
use std::ffi::{c_char, c_uint, CStr, CString};
use std::ptr::NonNull;
use std::sync::{Arc, Mutex};

use libloading::{Library, Symbol};
use serde::Serialize;
use serde_json::value::RawValue;
use serde_json::{json, Value};
use uuid::Uuid;
use windmill_common::error::{to_anyhow, Error, Result};
use windmill_common::utils::sanitize_string_from_password;
use windmill_common::worker::{get_memory, Connection, SqlResultCollectionStrategy};
use windmill_common::workspaces::{
    get_datatable_resource_from_db_unchecked, get_ducklake_from_db_unchecked,
    DucklakeCatalogResourceType,
};
use windmill_common::PgDatabase;
use windmill_object_store::S3_PROXY_LAST_ERRORS_CACHE;
use windmill_parser_sql::{parse_duckdb_sig, parse_sql_blocks};
use windmill_queue::{CanceledBy, MiniPulledJob};
use windmill_types::s3::S3Object;

use crate::agent_workers::{get_datatable_resource_from_agent_http, get_ducklake_from_agent_http};
use crate::common::{build_args_values, get_reserved_variables, OccupancyMetrics};
use crate::handle_child::run_future_with_polling_update_job_poller;
#[cfg(feature = "mysql")]
use crate::mysql_executor::MysqlDatabase;
use crate::sanitized_sql_params::sanitize_and_interpolate_unsafe_sql_args;
use crate::sql_utils::remove_comments;
use windmill_common::client::AuthedClient;
use windmill_object_store::DEFAULT_STORAGE;

// What a `// materialize` run records into `materialized_partition` once it
// finishes. `asset_path` is the full `<name>/<table>` (the asset identity);
// `partition` is "" for an unpartitioned (whole-table) materialization.
struct MaterializeExec {
    asset_kind: windmill_common::assets::AssetKind,
    asset_path: String,
    partition: String,
    // Number of `// data_test` checks the codegen embedded. Enforcement recovers
    // the per-test outcomes from the summary row; if it recovers fewer than this
    // (e.g. an FFI serialization change drops the column), we fail loud rather
    // than silently pass declared-but-unverified tests.
    n_data_tests: usize,
}

// Fetch and validate a custom data-test script's body. v1 custom tests are
// DuckDB scripts holding a single SELECT/CTE that returns the violating rows
// (dbt's singular-test convention); the worker embeds that query as a subquery
// check in the materialize connection (the single-statement constraint is
// enforced in sql_materialize.rs). Server workers only — agent (Http) workers
// have no script cache to read deployed content from.
async fn fetch_custom_test_body(conn: &Connection, w_id: &str, path: &str) -> Result<String> {
    let Connection::Sql(db) = conn else {
        return Err(Error::ExecutionErr(format!(
            "data_test custom `{path}`: custom tests require a server worker (not supported on \
             agent workers in v1)"
        )));
    };
    let hash = windmill_common::get_latest_script_hash(db, path, w_id)
        .await?
        .ok_or_else(|| {
            Error::ExecutionErr(format!(
                "data_test custom `{path}`: no deployed script found at this path"
            ))
        })?;
    let content =
        crate::get_script_content_by_hash(&windmill_common::scripts::ScriptHash(hash), w_id, conn)
            .await?;
    if !matches!(
        content.language,
        Some(windmill_common::scripts::ScriptLang::DuckDb)
    ) {
        return Err(Error::ExecutionErr(format!(
            "data_test custom `{path}`: must be a DuckDB script returning the violating rows \
             (got language {:?})",
            content.language
        )));
    }
    Ok(content.content)
}

// If `query` declares `// materialize <ducklake>`, return what to record plus,
// for the default managed mode, the rewritten managed-write SQL (in `manual`
// mode the script writes its own DDL, so the rewrite is `None`). The rewritten
// SQL contains a synthetic `ATTACH 'ducklake://<name>' AS _wm_target` that the
// normal ATTACH-transform pass resolves to real credentials — the same path as
// the user's own ATTACH. `// data_test` lines append verifier probes that run
// against the freshly-materialized target and raise (failing the run) on
// violation. Returns `None` when there is no materialize annotation or the
// target isn't a ducklake (only ducklake is materialized in v1).
fn build_materialized_query(
    query: &str,
    partition_value: Option<&str>,
    // Custom (`// data_test <path>`) test bodies, pre-fetched by the caller
    // (`fetch_custom_test_bodies`) so this stays pure/sync and unit-testable —
    // the DB read is the only thing that needs a connection. Keyed by script path.
    custom_test_bodies: &std::collections::HashMap<String, String>,
) -> Result<Option<(Option<String>, MaterializeExec)>> {
    use windmill_parser::asset_parser::{
        parse_pipeline_annotations, AssetKind as PAssetKind, DataTest,
    };
    use windmill_parser::sql_materialize::{
        build_wrap_blocks, DataTestResolved, MaterializeStrategy, TARGET_ALIAS,
    };

    let ann = parse_pipeline_annotations(query);
    let has_tests = !ann.data_tests.is_empty();
    let Some(m) = ann.materialize else {
        // Data tests run *against the materialized asset*; without a
        // `// materialize` target there is nothing to test. Fail loudly rather
        // than silently skip the declared checks.
        if has_tests {
            return Err(Error::ExecutionErr(
                "data_test: requires a `// materialize` target — data tests run against the \
                 materialized asset"
                    .to_string(),
            ));
        }
        return Ok(None);
    };
    if m.target_kind != PAssetKind::Ducklake {
        if has_tests {
            return Err(Error::ExecutionErr(
                "data_test: only `ducklake://` materialization targets support data tests in v1"
                    .to_string(),
            ));
        }
        return Ok(None);
    }
    let partitioned = ann.partition.is_some();
    let partition = partition_value.unwrap_or("").to_string();
    // Partition *resolution* is enterprise; in its absence a partitioned
    // materialize only runs with an explicit `partition` arg. Fail loudly rather
    // than silently materialize the wrong (empty) slice.
    if partitioned && partition.is_empty() {
        return Err(Error::ExecutionErr(
            "materialize: a `// partitioned` script ran with no resolved partition — pass an \
             explicit `partition` arg, or enable enterprise partition resolution"
                .to_string(),
        ));
    }
    // Convention: `ducklake://<name>/<table>` — <name> is the configured
    // ducklake (resolved like a user ATTACH), <table> is the rest.
    let (ducklake_name, table) = m
        .target_path
        .split_once('/')
        .unwrap_or((m.target_path.as_str(), ""));
    let meta = MaterializeExec {
        asset_kind: windmill_common::assets::AssetKind::Ducklake,
        asset_path: m.target_path.clone(),
        partition: partition.clone(),
        n_data_tests: ann.data_tests.len(),
    };

    // `{partition}` → escaped SQL literal substitution, applied to the managed
    // SELECT, its setup, and any custom-test body so a partitioned test can
    // filter by the active slice. Always a complete `'…'` literal (with `'`
    // doubled) whether or not the author quoted it, so a run caller can't break
    // out and alter statement boundaries. The pre-quoted `'{partition}'` form is
    // matched first so it doesn't become `''…''`. No-op when unpartitioned.
    let lit = format!("'{}'", partition.replace('\'', "''"));
    let substitute = |s: &str| -> String {
        if !partitioned {
            return s.to_string();
        }
        let tok = windmill_common::assets::PARTITION_TOKEN;
        let quoted_tok = format!("'{tok}'");
        s.replace(&quoted_tok, &lit).replace(tok, &lit)
    };

    if m.manual {
        // Escape hatch: the script owns its DDL. We can't reliably attach the
        // managed target or know the partition column it wrote, so data tests
        // are not generated for manual mode in v1.
        if has_tests {
            return Err(Error::ExecutionErr(
                "data_test: not supported with `// materialize manual` in v1 — use managed \
                 `// materialize`"
                    .to_string(),
            ));
        }
        return Ok(Some((None, meta)));
    }
    if table.is_empty() {
        return Err(Error::ExecutionErr(format!(
            "materialize: target `ducklake://{}` has no table (use ducklake://<name>/<table>)",
            m.target_path
        )));
    }
    let mut plan = classify_wrap_or_err(query)?;
    plan.output = substitute(&plan.output);
    for s in plan.setup.iter_mut() {
        *s = substitute(s);
    }
    let strategy = if m.scd2 {
        // SCD2 needs a natural key to identify an entity across versions, and its
        // diff/close/open shape has no partition-scoped form in v1.
        let key = m.unique_key.clone().ok_or_else(|| {
            Error::ExecutionErr(
                "materialize scd2: requires a natural key — add `key=<col>`".to_string(),
            )
        })?;
        if partitioned {
            return Err(Error::ExecutionErr(
                "materialize scd2: `// partitioned` is not supported with scd2 in v1".to_string(),
            ));
        }
        MaterializeStrategy::Scd2 { key, track: m.track.clone(), close_deleted: m.close_deleted }
    } else if m.append {
        MaterializeStrategy::Append
    } else if let Some(uk) = m.unique_key {
        MaterializeStrategy::Merge { unique_key: uk }
    } else {
        MaterializeStrategy::Replace
    };
    // Inline the partition as an escaped SQL literal (DuckLake has no bind for
    // the partition column in our generated DDL).
    let pval = lit.clone();
    let synthetic_attach = format!("ATTACH 'ducklake://{ducklake_name}' AS {TARGET_ALIAS};");

    // Resolve data tests (fetch + partition-substitute custom bodies) so codegen
    // can embed every check's violating-row count in the materialize summary.
    // The summary then carries the full per-test breakdown back to the worker,
    // which runs them all and decides pass/fail (no abort-on-first). Empty when
    // there are no `// data_test` lines.
    let mut resolved = Vec::with_capacity(ann.data_tests.len());
    for test in &ann.data_tests {
        match test {
            DataTest::Custom { path } => {
                let raw = custom_test_bodies.get(path).ok_or_else(|| {
                    Error::ExecutionErr(format!(
                        "data_test custom `{path}`: body not fetched before codegen (internal)"
                    ))
                })?;
                resolved
                    .push(DataTestResolved::Custom { path: path.clone(), body: substitute(raw) });
            }
            other => resolved.push(DataTestResolved::BuiltIn(other.clone())),
        }
    }

    let blocks = build_wrap_blocks(
        &plan,
        &synthetic_attach,
        table,
        &m.target_path,
        "_wm_partition",
        &pval,
        partitioned,
        strategy,
        &resolved,
    )
    .map_err(Error::ExecutionErr)?;

    Ok(Some((Some(blocks.join("\n")), meta)))
}

// Fetch the deployed body of every `// data_test <path>` custom test declared in
// `query`, keyed by path, so the sync `build_materialized_query` can splice them
// in. The DB read is the only part of materialize codegen that needs a
// connection; isolating it here keeps the codegen pure and unit-testable.
// Server workers only (`fetch_custom_test_body` errors on agent workers). Empty
// when there are no custom tests.
async fn fetch_custom_test_bodies(
    query: &str,
    conn: &Connection,
    w_id: &str,
) -> Result<std::collections::HashMap<String, String>> {
    use windmill_parser::asset_parser::{parse_pipeline_annotations, DataTest};
    let ann = parse_pipeline_annotations(query);
    let mut bodies = std::collections::HashMap::new();
    for test in &ann.data_tests {
        if let DataTest::Custom { path } = test {
            if !bodies.contains_key(path) {
                let body = fetch_custom_test_body(conn, w_id, path).await?;
                bodies.insert(path.clone(), body);
            }
        }
    }
    Ok(bodies)
}

// classify_wrap with the spec's actionable message turned into an executor error.
fn classify_wrap_or_err(query: &str) -> Result<windmill_parser::sql_materialize::WrapPlan> {
    windmill_parser::sql_materialize::classify_wrap(query)
        .map_err(|e| Error::ExecutionErr(e.message()))
}

// One workspace-macro registry row (`macro_definition`, written at deploy of a
// `// macros` library script).
struct MacroRow {
    name: String,
    params: String,
    body: String,
    is_table_macro: bool,
    provider_path: String,
}

// Selection pass: seed = macros the (comment-stripped, transformed) blocks
// call, plus every macro of each `// use` library; then the transitive
// closure over macro bodies. Local definitions win — their names are removed
// from the injectable set entirely, so a later workspace-library deploy can
// never silently replace a script's own macro. Pure and separate from
// `plan_macro_injection` so the shell knows which provider libraries to
// fetch (their setup statements are injected too) before planning.
fn select_workspace_macros(
    blocks: &[String],
    registry: &[MacroRow],
    use_libs: &[String],
) -> Result<std::collections::HashSet<String>> {
    use std::collections::{BTreeMap, HashSet};
    use windmill_parser::duckdb_macros::{detect_macro_calls, locally_defined_macro_names};

    let local = locally_defined_macro_names(blocks);
    let all_names: HashSet<String> = registry
        .iter()
        .map(|r| r.name.clone())
        .filter(|n| !local.contains(n))
        .collect();
    let by_name: BTreeMap<&str, &MacroRow> =
        registry.iter().map(|r| (r.name.as_str(), r)).collect();

    let mut selected: HashSet<String> = HashSet::new();
    for path in use_libs {
        let lib_names: Vec<&str> = registry
            .iter()
            .filter(|r| &r.provider_path == path)
            .map(|r| r.name.as_str())
            .collect();
        if lib_names.is_empty() {
            return Err(Error::ExecutionErr(format!(
                "`// use {path}`: no deployed macro library at this path"
            )));
        }
        selected.extend(
            lib_names
                .into_iter()
                .filter(|n| all_names.contains(*n))
                .map(String::from),
        );
    }
    selected.extend(detect_macro_calls(&blocks.join("\n"), &all_names));

    // Transitive closure over macro bodies (a selected macro may call others).
    let mut frontier: Vec<String> = selected.iter().cloned().collect();
    while let Some(name) = frontier.pop() {
        if let Some(row) = by_name.get(name.as_str()) {
            for dep in detect_macro_calls(&row.body, &all_names) {
                if selected.insert(dep.clone()) {
                    frontier.push(dep);
                }
            }
        }
    }
    Ok(selected)
}

// Fixpoint over library-level `// use`: a library may declare `// use` for
// dynamic calls its macro bodies make (string-hidden from lexical detection,
// e.g. inside `query('…')`). Those declarations are honored transitively —
// consuming a macro from lib B pulls in whatever B `// use`s, so the dynamic
// dependency stays encapsulated in the library instead of leaking to every
// consumer. `lib_uses` maps provider path → its parsed `// use` list; the
// shell grows it lazily and re-resolves until no new library appears.
// Returns the selected macro names plus the effective `// use` list
// (consumer's own first, in annotation order, then discovered libs).
fn resolve_macro_selection(
    blocks: &[String],
    registry: &[MacroRow],
    consumer_use_libs: &[String],
    lib_uses: &std::collections::BTreeMap<String, Vec<String>>,
) -> Result<(std::collections::HashSet<String>, Vec<String>)> {
    let mut effective: Vec<String> = Vec::new();
    for p in consumer_use_libs {
        if !effective.contains(p) {
            effective.push(p.clone());
        }
    }
    loop {
        let selected = select_workspace_macros(blocks, registry, &effective)?;
        let mut relevant: Vec<String> = effective.clone();
        for name in &selected {
            if let Some(r) = registry.iter().find(|r| &r.name == name) {
                if !relevant.contains(&r.provider_path) {
                    relevant.push(r.provider_path.clone());
                }
            }
        }
        let mut grew = false;
        for lib in &relevant {
            for u in lib_uses.get(lib).map(Vec::as_slice).unwrap_or(&[]) {
                if !effective.contains(u) {
                    effective.push(u.clone());
                    grew = true;
                }
            }
        }
        if !grew {
            return Ok((selected, effective));
        }
    }
}

// Emit the blocks to inject: first every relevant library's setup statements
// (a macro body may reference its own lib's ATTACH, and DuckDB bind-checks
// at CREATE — so setup is injected for the provider of every selected macro,
// not just `// use` libs), then the selected definitions in dependency order.
// `lib_bodies` carries the parsed deployed content per provider path; setup
// order = `// use` libs in annotation order, then remaining providers sorted.
// Exact-duplicate setup statements across libraries are deduped (two libs
// attaching the same catalog the same way must not double-ATTACH). Pure —
// fetches happen in `inject_workspace_macros` — so this is unit-testable.
fn plan_macro_injection(
    selected: &std::collections::HashSet<String>,
    registry: &[MacroRow],
    use_libs: &[String],
    lib_bodies: &std::collections::BTreeMap<
        String,
        Vec<windmill_parser::duckdb_macros::LibStatement>,
    >,
) -> Result<Vec<String>> {
    use std::collections::{BTreeMap, BTreeSet, HashSet};
    use windmill_parser::duckdb_macros::{macro_create_statement, topo_order_macros, LibStatement};

    let by_name: BTreeMap<&str, &MacroRow> =
        registry.iter().map(|r| (r.name.as_str(), r)).collect();

    // Providers whose setup must run: every `// use` lib (annotation order —
    // even with zero selected macros, the explicit `use` carries its ATTACH
    // side effects), then the provider of each selected macro (sorted).
    let mut providers: Vec<&str> = use_libs.iter().map(String::as_str).collect();
    let selected_providers: BTreeSet<&str> = selected
        .iter()
        .filter_map(|n| by_name.get(n.as_str()).map(|r| r.provider_path.as_str()))
        .collect();
    for p in selected_providers {
        if !providers.contains(&p) {
            providers.push(p);
        }
    }

    let mut injected: Vec<String> = Vec::new();
    let mut seen_setup: HashSet<String> = HashSet::new();
    for provider in providers {
        let Some(statements) = lib_bodies.get(provider) else {
            return Err(Error::ExecutionErr(format!(
                "macro library `{provider}` has registry entries but its deployed script could \
                 not be loaded"
            )));
        };
        for s in statements {
            if let LibStatement::Setup(stmt) = s {
                let stmt = format!("{};", stmt.trim_end_matches(';').trim_end());
                if seen_setup.insert(stmt.clone()) {
                    injected.push(stmt);
                }
            }
        }
    }

    if selected.is_empty() {
        return Ok(injected);
    }
    let defs: BTreeMap<String, String> = selected
        .iter()
        .filter_map(|n| by_name.get(n.as_str()).map(|r| (n.clone(), r.body.clone())))
        .collect();
    let order = topo_order_macros(selected, &defs).map_err(Error::ExecutionErr)?;
    for name in order {
        let r = by_name.get(name.as_str()).ok_or_else(|| {
            Error::ExecutionErr(format!("workspace macro `{name}` has no registry row"))
        })?;
        injected.push(macro_create_statement(
            &r.name,
            &r.params,
            r.is_table_macro,
            &r.body,
        ));
    }
    Ok(injected)
}

// Weave the injected blocks into the user's statement list. DuckDB
// bind-checks macro bodies at CREATE, in both directions:
//   - an injected definition that a LOCAL macro calls must land *before*
//     that local definition (pulling its own injected dependencies with it);
//   - an injected definition that references a local macro must stay *after*
//     it — a conflict between the two requirements is an error, not a
//     silent mis-ordering.
// The default slot (no local interplay) is after the leading prefix of setup
// statements and local definitions, as before. Injected setup statements
// (library ATTACHes) go at the earliest slot any injected definition landed
// on: injected bodies are self-contained w.r.t. their own library's setup
// and never depend on the consumer's setup.
fn weave_macro_blocks(blocks: Vec<String>, injected: Vec<String>) -> Result<Vec<String>> {
    if injected.is_empty() {
        return Ok(blocks);
    }
    use std::collections::HashSet;
    use windmill_parser::duckdb_macros::{
        detect_macro_calls, is_macro_definition, parse_macro_definition,
    };
    use windmill_parser::sql_materialize::{classify_block, BlockClass};

    // Default slot: insert before the first block that is neither setup nor
    // a local macro definition (i.e. after the whole leading prefix).
    let default_slot = blocks
        .iter()
        .position(|b| !matches!(classify_block(b), BlockClass::Setup) && !is_macro_definition(b))
        .unwrap_or(blocks.len());

    // The user's own macro definitions, as placement anchors.
    let locals: Vec<(usize, windmill_parser::duckdb_macros::ParsedMacro)> = blocks
        .iter()
        .enumerate()
        .filter_map(|(i, b)| parse_macro_definition(b).map(|m| (i, m)))
        .collect();
    let local_names: HashSet<String> = locals.iter().map(|(_, m)| m.name.clone()).collect();

    // Plan emits [lib setup…, definitions in topo order…].
    let (setup, defs): (Vec<String>, Vec<String>) =
        injected.into_iter().partition(|s| !is_macro_definition(s));
    let def_metas: Vec<windmill_parser::duckdb_macros::ParsedMacro> = defs
        .iter()
        .map(|s| {
            parse_macro_definition(s).ok_or_else(|| {
                Error::ExecutionErr(format!(
                    "internal: generated macro statement failed to re-parse: {}",
                    s.chars().take(80).collect::<String>()
                ))
            })
        })
        .collect::<Result<_>>()?;
    let def_names: HashSet<String> = def_metas.iter().map(|m| m.name.clone()).collect();

    // Per-injected-definition slot bounds from the local anchors. `slot = i`
    // means "insert before block i".
    let mut max_slot: Vec<usize> = vec![default_slot; defs.len()];
    let mut min_slot: Vec<usize> = vec![0; defs.len()];
    for (li, lm) in &locals {
        let local_calls = detect_macro_calls(&lm.body, &def_names);
        let referenced_locals_of: Vec<usize> = def_metas
            .iter()
            .enumerate()
            .filter(|(_, dm)| detect_macro_calls(&dm.body, &local_names).contains(&lm.name))
            .map(|(di, _)| di)
            .collect();
        for (di, dm) in def_metas.iter().enumerate() {
            if local_calls.contains(&dm.name) {
                max_slot[di] = max_slot[di].min(*li);
            }
        }
        for di in referenced_locals_of {
            min_slot[di] = min_slot[di].max(*li + 1);
        }
    }
    // An injected definition must not land after any injected definition
    // that depends on it: propagate upper bounds backwards through the topo
    // order (dependents come later in `defs`).
    let mut eff_slot = max_slot.clone();
    for i in (0..defs.len()).rev() {
        for j in (i + 1)..defs.len() {
            if detect_macro_calls(&def_metas[j].body, &def_names).contains(&def_metas[i].name) {
                eff_slot[i] = eff_slot[i].min(eff_slot[j]);
            }
        }
        if min_slot[i] > eff_slot[i] {
            return Err(Error::ExecutionErr(format!(
                "workspace macro `{}` and this script's own macro definitions have conflicting \
                 order requirements (a local macro calls it while it references a local macro \
                 defined later); reorder the local definitions",
                def_metas[i].name
            )));
        }
    }
    // Library setup precedes the earliest definition.
    let setup_slot = eff_slot.iter().copied().min().unwrap_or(default_slot);

    let mut out: Vec<String> = Vec::with_capacity(blocks.len() + defs.len() + setup.len());
    for slot in 0..=blocks.len() {
        if slot == setup_slot {
            out.extend(setup.iter().cloned());
        }
        for (di, d) in defs.iter().enumerate() {
            if eff_slot[di] == slot {
                out.push(d.clone());
            }
        }
        if let Some(b) = blocks.get(slot) {
            out.push(b.clone());
        }
    }
    Ok(out)
}

// Workspace-macro injection (`// macros` libraries): fetch the registry, plan
// the needed `CREATE OR REPLACE TEMP MACRO` blocks and splice them into the
// job's statement list. Late-bound: every run reads the current registry, so a
// lib redeploy applies to the next run. On agent (Http) workers — no DB — the
// implicit path silently degrades (a called macro then fails with DuckDB's
// clear Catalog Error) but an explicit `// use` hard-errors like custom tests.
async fn inject_workspace_macros(
    conn: &Connection,
    w_id: &str,
    is_macro_lib: bool,
    use_libs: &[String],
    blocks: Vec<String>,
) -> Result<Vec<String>> {
    if is_macro_lib {
        // A library run executes its own definitions; nothing to inject.
        return Ok(blocks);
    }
    let db = match conn {
        Connection::Sql(db) => db,
        Connection::Http(_) => {
            if !use_libs.is_empty() {
                return Err(Error::ExecutionErr(
                    "`// use` requires a server worker (not supported on agent workers in v1)"
                        .to_string(),
                ));
            }
            return Ok(blocks);
        }
    };
    let registry = sqlx::query_as!(
        MacroRow,
        "SELECT name, params, body, is_table_macro, provider_path FROM macro_definition WHERE workspace_id = $1",
        w_id
    )
    .fetch_all(db)
    .await?;
    if registry.is_empty() && use_libs.is_empty() {
        return Ok(blocks);
    }
    if select_workspace_macros(&blocks, &registry, use_libs)?.is_empty() && use_libs.is_empty() {
        return Ok(blocks);
    }
    // Every relevant provider's deployed body is fetched: its setup
    // statements are injected ahead of the definitions (a macro body may
    // reference its own lib's ATTACH, which must run before the injected
    // CREATE binds), and its own `// use` declarations are honored
    // transitively — so the loop fetches lazily and re-resolves until no new
    // library appears. Content fetches are cached by hash.
    let mut lib_bodies: std::collections::BTreeMap<
        String,
        Vec<windmill_parser::duckdb_macros::LibStatement>,
    > = Default::default();
    let mut lib_uses: std::collections::BTreeMap<String, Vec<String>> = Default::default();
    let (selected, effective_use) = loop {
        let (selected, effective) =
            resolve_macro_selection(&blocks, &registry, use_libs, &lib_uses)?;
        let mut relevant: Vec<String> = effective.clone();
        for name in &selected {
            if let Some(r) = registry.iter().find(|r| &r.name == name) {
                if !relevant.contains(&r.provider_path) {
                    relevant.push(r.provider_path.clone());
                }
            }
        }
        let missing: Vec<String> = relevant
            .into_iter()
            .filter(|l| !lib_bodies.contains_key(l))
            .collect();
        if missing.is_empty() {
            break (selected, effective);
        }
        for path in missing {
            let content = fetch_macro_lib_body(conn, w_id, &path).await?;
            let statements = windmill_parser::duckdb_macros::parse_macro_library(&content)
                .map_err(|e| {
                    Error::ExecutionErr(format!("macro library `{path}`: invalid content: {e}"))
                })?;
            lib_uses.insert(
                path.clone(),
                windmill_parser::asset_parser::parse_pipeline_annotations(&content).use_libs,
            );
            lib_bodies.insert(path, statements);
        }
    };
    let injected = plan_macro_injection(&selected, &registry, &effective_use, &lib_bodies)?;
    weave_macro_blocks(blocks, injected)
}

// Fetch a macro library's deployed body — for its setup statements; the
// macro definitions themselves come from the registry. Used both for `// use`
// libs and for the provider of every selected macro. Same server-worker
// fetch path as custom data tests.
async fn fetch_macro_lib_body(conn: &Connection, w_id: &str, path: &str) -> Result<String> {
    let Connection::Sql(db) = conn else {
        return Err(Error::ExecutionErr(
            "workspace macros require a server worker (not supported on agent workers in v1)"
                .to_string(),
        ));
    };
    let hash = windmill_common::get_latest_script_hash(db, path, w_id)
        .await?
        .ok_or_else(|| {
            Error::ExecutionErr(format!(
                "macro library `{path}`: no deployed script found at this path"
            ))
        })?;
    let content =
        crate::get_script_content_by_hash(&windmill_common::scripts::ScriptHash(hash), w_id, conn)
            .await?;
    if !matches!(
        content.language,
        Some(windmill_common::scripts::ScriptLang::DuckDb)
    ) {
        return Err(Error::ExecutionErr(format!(
            "macro library `{path}`: must be a DuckDB `// macros` library (got language {:?})",
            content.language
        )));
    }
    Ok(content.content)
}

// Pull a named i64 field (`snapshot_id` / `rows`) out of the trailing summary
// read — which in wrap mode is the job result. Shape-tolerant (object / array /
// nested), returns None if absent (literal mode, or capture failed).
fn extract_i64(result: &RawValue, field: &str) -> Option<i64> {
    fn find(v: &Value, field: &str) -> Option<i64> {
        match v {
            Value::Number(n) => n.as_i64(),
            Value::Object(m) => m.get(field).and_then(|x| find(x, field)),
            Value::Array(a) => a.iter().find_map(|x| find(x, field)),
            _ => None,
        }
    }
    find(&serde_json::from_str::<Value>(result.get()).ok()?, field)
}

// One data test's outcome as carried by the materialize summary's `data_tests`
// column: its display name and how many rows violated it (0 = pass).
struct DataTestOutcome {
    name: String,
    violating: i64,
}

// Pull the per-test breakdown out of the materialize summary result. The
// `data_tests` column is a DuckLake list-of-struct `[{test, violating}, …]`;
// the FFI may surface it as a nested JSON array or as a JSON string, so accept
// both. Returns empty when there are no tests (the column is absent).
fn extract_data_tests(result: &RawValue) -> Vec<DataTestOutcome> {
    fn collect(v: &Value, out: &mut Vec<DataTestOutcome>) {
        if let Value::Array(arr) = v {
            for item in arr {
                if let Value::Object(o) = item {
                    if let Some(Value::String(name)) = o.get("test") {
                        let violating = o
                            .get("violating")
                            .and_then(|x| x.as_i64().or_else(|| x.as_f64().map(|f| f as i64)))
                            .unwrap_or(0);
                        out.push(DataTestOutcome { name: name.clone(), violating });
                    }
                }
            }
        }
    }
    fn find_field(v: &Value) -> Option<&Value> {
        match v {
            Value::Object(o) => o.get("data_tests"),
            Value::Array(a) => a.iter().find_map(find_field),
            _ => None,
        }
    }
    let mut out = Vec::new();
    let Ok(root) = serde_json::from_str::<Value>(result.get()) else {
        return out;
    };
    match find_field(&root) {
        Some(arr @ Value::Array(_)) => collect(arr, &mut out),
        // FFI serialized the list-of-struct as a JSON string — parse it.
        Some(Value::String(s)) => {
            if let Ok(parsed) = serde_json::from_str::<Value>(s) {
                collect(&parsed, &mut out);
            }
        }
        _ => {}
    }
    out
}

// Pull the captured output schema out of the materialize summary's
// `output_schema` column (gap #2a): a list-of-struct `[{name, type}, …]` the
// codegen built from a `DESCRIBE`. Like `data_tests`, the FFI may surface it as
// a nested JSON array or a JSON string — accept both. Returns `None` when the
// column is absent (literal mode, manual mode, or capture failed) so the worker
// records the run without a schema rather than an empty one.
fn extract_schema(
    result: &RawValue,
) -> Option<Vec<windmill_common::materialization::SchemaColumn>> {
    use windmill_common::materialization::SchemaColumn;
    fn collect(v: &Value) -> Option<Vec<SchemaColumn>> {
        let Value::Array(arr) = v else { return None };
        let mut out = Vec::with_capacity(arr.len());
        for item in arr {
            let o = item.as_object()?;
            let name = o.get("name")?.as_str()?.to_string();
            let data_type = o.get("type")?.as_str()?.to_string();
            out.push(SchemaColumn { name, data_type });
        }
        Some(out)
    }
    fn find_field(v: &Value) -> Option<&Value> {
        match v {
            Value::Object(o) => o.get("output_schema"),
            Value::Array(a) => a.iter().find_map(find_field),
            _ => None,
        }
    }
    let root = serde_json::from_str::<Value>(result.get()).ok()?;
    match find_field(&root)? {
        arr @ Value::Array(_) => collect(arr),
        // FFI serialized the list-of-struct as a JSON string — parse it.
        Value::String(s) => collect(&serde_json::from_str::<Value>(s).ok()?),
        _ => None,
    }
}

// Render the full pass/fail breakdown for a failed data-test run — every test,
// not just the first failure, so the user sees the whole picture in one place.
fn format_data_test_breakdown(asset_path: &str, tests: &[DataTestOutcome]) -> String {
    let failed = tests.iter().filter(|t| t.violating > 0).count();
    let mut lines = vec![format!(
        "data tests failed on {asset_path} ({failed}/{} failed):",
        tests.len()
    )];
    for t in tests {
        if t.violating > 0 {
            lines.push(format!("  ✗ {} — {} violating row(s)", t.name, t.violating));
        } else {
            lines.push(format!("  ✓ {}", t.name));
        }
    }
    lines.join("\n")
}

// Best-effort record of a materialization outcome. On a Sql connection it writes
// the row directly; on an agent worker (Http, no direct DB) it posts to the API
// so state lands the same way. Never fails the job — a lost row degrades the
// grid, not the run.
async fn record_mat(
    conn: &Connection,
    w_id: &str,
    job_id: Uuid,
    meta: &MaterializeExec,
    status: windmill_common::materialization::MaterializationStatus,
    snapshot_id: Option<i64>,
    row_count: Option<i64>,
    // Captured output schema (gap #2a). Only set on a successful materialize;
    // when present, also upserts a `materialized_asset_schema` version.
    schema: Option<Vec<windmill_common::materialization::SchemaColumn>>,
    error: Option<&str>,
) {
    let req = windmill_common::materialization::RecordMaterializationRequest {
        asset_kind: meta.asset_kind,
        asset_path: meta.asset_path.clone(),
        partition: meta.partition.clone(),
        status,
        snapshot_id,
        row_count,
        job_id: Some(job_id),
        error: error.map(|e| e.to_string()),
        schema: schema.clone(),
    };
    let res: anyhow::Result<()> = match conn {
        Connection::Sql(db) => {
            let partition_res = windmill_common::materialization::record_materialization(
                db,
                w_id,
                req.asset_kind,
                &req.asset_path,
                &req.partition,
                req.status,
                req.snapshot_id,
                req.row_count,
                req.job_id,
                req.error.as_deref(),
            )
            .await
            .map_err(|e| anyhow::anyhow!("{e:#}"));
            // Schema capture is a separate, independently best-effort write (its
            // own transaction for the per-asset advisory lock); a failure here
            // must not lose the partition row above.
            if let Some(cols) = schema.as_ref() {
                if let Err(e) = record_asset_schema_best_effort(
                    db,
                    w_id,
                    meta.asset_kind,
                    &meta.asset_path,
                    cols,
                    snapshot_id,
                    job_id,
                )
                .await
                {
                    tracing::warn!("failed to record captured asset schema: {e:#}");
                }
            }
            partition_res
        }
        Connection::Http(client) => {
            crate::agent_workers::record_materialization_from_agent_http(client, w_id, &req).await
        }
    };
    if let Err(e) = res {
        tracing::warn!("failed to record materialization state: {e:#}");
    }
}

// Open a short transaction (needed for the per-asset advisory lock) and upsert
// the captured schema version. Isolated so its tx lifetime doesn't entangle the
// partition write.
async fn record_asset_schema_best_effort(
    db: &windmill_common::DB,
    w_id: &str,
    asset_kind: windmill_common::assets::AssetKind,
    asset_path: &str,
    columns: &[windmill_common::materialization::SchemaColumn],
    snapshot_id: Option<i64>,
    job_id: Uuid,
) -> anyhow::Result<()> {
    let mut tx = db.begin().await?;
    windmill_common::materialization::record_asset_schema(
        &mut tx,
        w_id,
        asset_kind,
        asset_path,
        columns,
        snapshot_id,
        Some(job_id),
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn do_duckdb(
    job: &MiniPulledJob,
    client: &AuthedClient,
    query: &str,
    conn: &Connection,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    worker_name: &str,
    // TODO
    #[allow(unused_variables)] column_order_ref: &mut Option<Vec<String>>,
    occupancy_metrics: &mut OccupancyMetrics,
    parent_runnable_path: Option<String>,
    job_dir: &str,
    run_inline: bool,
) -> Result<Box<RawValue>> {
    let annotations = windmill_common::worker::SqlAnnotations::parse(query);
    let collection_strategy =
        if annotations.result_collection == SqlResultCollectionStrategy::Legacy {
            // Before result_collection was introduced, duckdb ignored all statements results except the last one
            SqlResultCollectionStrategy::LastStatementAllRows
        } else {
            annotations.result_collection
        };
    if annotations.return_last_result {
        return Err(Error::ExecutionErr(
            "return_last_result annotation is deprecated, use result_collection=last_statement_all_rows instead".to_string(),
        ));
    }

    let token = client.token.clone();
    let hidden_passwords = Arc::new(Mutex::new(Vec::<String>::new()));

    let result_f = async {
        let mut hidden_passwords = hidden_passwords.clone();
        let mut bigquery_credentials = None;

        // Materialization (`// materialize`): rewrite a wrap script into managed
        // DDL (its synthetic target ATTACH is resolved by the transform pass
        // below, like the user's own ATTACH); a literal script is left as-is.
        // `materialize` also carries what to record once the run finishes.
        let partition_value: Option<String> = job
            .args
            .as_ref()
            .and_then(|a| a.0.get(windmill_common::partition::PARTITION_ARG))
            .and_then(|rv| serde_json::from_str::<String>(rv.get()).ok())
            .filter(|s| !s.is_empty());
        let materialize = if query.contains("materialize") || query.contains("data_test") {
            // Custom-test bodies need a DB read; fetch them first so the codegen
            // itself stays pure/sync.
            let custom_test_bodies =
                fetch_custom_test_bodies(query, conn, &job.workspace_id).await?;
            build_materialized_query(query, partition_value.as_deref(), &custom_test_bodies)?
        } else {
            None
        };
        // Parse the signature from the ORIGINAL script: managed materialize wraps
        // the trailing SELECT and strips line comments, which drops the
        // `-- $name (type)` arg declarations while their `$name` references
        // survive in the embedded SELECT. Parsing args here (pre-wrap) keeps them
        // declared so they are still bound — and s3object args translated to
        // `s3://` URIs — at run time.
        let sig = parse_duckdb_sig(query)?.args;

        // `// macros` / `// use` also come off the ORIGINAL script text (the
        // materialize rewrite strips the annotation comments). Drives the
        // workspace-macro injection after the ATTACH-transform pass below.
        let macro_ann = windmill_parser::asset_parser::parse_pipeline_annotations(query);

        let materialized_query;
        let query: &str = match &materialize {
            Some((Some(rewritten), _)) => {
                materialized_query = rewritten.clone();
                &materialized_query
            }
            _ => query,
        };

        // Managed materialize generates its own trailing summary row (asset /
        // rows / snapshot_id / data_tests), and data-test enforcement below reads
        // the `data_tests` column off that row. The row shape is ours, not the
        // user's — so force the full-last-row strategy regardless of any
        // `// result_collection` annotation, which would otherwise reshape it
        // (e.g. a scalar mode drops every column but the first) and silently
        // bypass test enforcement.
        let collection_strategy = if matches!(&materialize, Some((Some(_), _))) {
            SqlResultCollectionStrategy::LastStatementAllRows
        } else {
            collection_strategy
        };
        let mut job_args = build_args_values(job, client, conn).await?;

        let reserved_variables =
            get_reserved_variables(job, &client.token, conn, parent_runnable_path).await?;

        let (query, _) =
            &sanitize_and_interpolate_unsafe_sql_args(query, &sig, &job_args, &reserved_variables)?;
        let query = transform_s3_uris(query).await?;

        let job_args = {
            let mut m = Vec::new();
            for sig_arg in sig.into_iter() {
                let json_value = job_args
                    .remove(&sig_arg.name)
                    .or_else(|| sig_arg.default)
                    .unwrap_or_else(|| json!(null));

                if matches!(&sig_arg.otyp.as_ref().map(String::as_str), Some("s3object")) {
                    let s3_obj = serde_json::from_value::<S3Object>(json_value).map_err(|e| {
                        Error::ExecutionErr(format!("Failed to deserialize S3Object: {}", e))
                    })?;
                    let uri = format!(
                        "s3://{}/{}",
                        s3_obj.storage.as_deref().unwrap_or(DEFAULT_STORAGE),
                        s3_obj.s3
                    );
                    m.push(Arg {
                        json_value: serde_json::Value::String(uri),
                        name: sig_arg.name,
                        arg_type: "string".to_string(),
                    });
                } else {
                    m.push(Arg {
                        json_value,
                        name: sig_arg.name,
                        arg_type: sig_arg.otyp.unwrap_or_else(|| "text".to_string()),
                    });
                }
            }
            m
        };

        let query_block_list = parse_sql_blocks(&query, true);

        // Replace custom ATTACH statements with the real instructions
        let query_block_list = {
            let mut v = vec![];
            for query_block in query_block_list.iter() {
                let query_block = remove_comments(&query_block);
                if let Some(parsed) = parse_attach_db_resource(query_block) {
                    v.extend(
                        transform_attach_db_resource_query(
                            &parsed,
                            &job.id,
                            client,
                            &mut hidden_passwords,
                        )
                        .await?,
                    );
                    if parsed.db_type == "bigquery" {
                        bigquery_credentials = Some(UseBigQueryCredentialsFile::new(
                            job.id,
                            parsed.resource_path,
                        )?);
                    }
                } else if let Some(ducklake_query) = transform_attach_ducklake(
                    &query_block,
                    conn,
                    &mut hidden_passwords,
                    &job.workspace_id,
                )
                .await?
                {
                    v.extend(ducklake_query);
                } else if let Some(datatable_query) = transform_attach_datatable(
                    &query_block,
                    conn,
                    &mut hidden_passwords,
                    &job.workspace_id,
                )
                .await?
                {
                    v.extend(datatable_query);
                } else {
                    v.push(query_block.to_string());
                }
            }
            v
        };

        // Workspace macros: splice `CREATE OR REPLACE TEMP MACRO` blocks for
        // registry macros this script calls (plus whole `// use` libraries)
        // after the setup/ATTACH prefix — post-transform, so macro bodies can
        // reference the attached catalogs when DuckDB bind-checks the CREATE.
        let query_block_list = inject_workspace_macros(
            conn,
            &job.workspace_id,
            macro_ann.macros,
            &macro_ann.use_libs,
            query_block_list,
        )
        .await?;

        let base_internal_url = client.base_internal_url.clone();
        let w_id = job.workspace_id.clone();
        let job_dir = job_dir.to_string();

        if annotations.prepare {
            let result = tokio::task::spawn_blocking(move || {
                prepare_duckdb_ffi_safe(
                    query_block_list.iter().map(String::as_str),
                    &token,
                    &base_internal_url,
                    &w_id,
                    &job_dir,
                )
            })
            .await
            .map_err(|e| Error::from(to_anyhow(e)))
            .and_then(|r| r)?;

            return Ok(result);
        }

        let result = tokio::task::spawn_blocking(move || {
            run_duckdb_ffi_safe(
                query_block_list.iter().map(String::as_str),
                query_block_list.len(),
                job_args,
                &token,
                &base_internal_url,
                &w_id,
                &job_dir,
                collection_strategy,
            )
        })
        .await
        .map_err(|e| Error::from(to_anyhow(e)))
        .and_then(|r| r);

        let (result, column_order) = match result {
            Ok(r) => r,
            Err(e) => {
                if let Some((_, meta)) = &materialize {
                    record_mat(
                        conn,
                        &job.workspace_id,
                        job.id,
                        meta,
                        windmill_common::materialization::MaterializationStatus::Failed,
                        None,
                        None,
                        None,
                        Some(&e.to_string()),
                    )
                    .await;
                }
                if let Some(s3_proxy_err) = S3_PROXY_LAST_ERRORS_CACHE.get(&client.token) {
                    return Err(Error::ExecutionErr(format!(
                        "{}\n\nS3 Related Error: {}",
                        e.to_string(),
                        s3_proxy_err,
                    )));
                }
                return Err(e);
            }
        };

        if let Some((_, meta)) = &materialize {
            // In wrap mode the job result is the summary read (snapshot_id +
            // rows + the per-test breakdown); in literal mode there is none.
            let snapshot_id = extract_i64(&result, "snapshot_id");
            let row_count = extract_i64(&result, "rows");
            // Data tests all ran (every check counted in one query); decide
            // pass/fail here. Any violation fails the run — the write is already
            // committed (like dbt), so the slice is recorded `Failed` and the
            // cascade stops. The error lists *every* test so the user sees the
            // whole picture, not just the first failure.
            let tests = extract_data_tests(&result);
            // Captured output schema (gap #2a) — recorded only on the successful
            // path below, not on the failure paths (a failed run shouldn't
            // advance the asset's recorded schema version). Managed mode ONLY:
            // in `// materialize manual` the result is the user's own query
            // output (we generate no summary), so an `output_schema` field there
            // is caller-shaped and must not be trusted — `materialize` is
            // `Some((Some(_), _))` for managed, `Some((None, _))` for manual.
            let is_managed = matches!(&materialize, Some((Some(_), _)));
            let schema = if is_managed {
                extract_schema(&result)
            } else {
                None
            };
            // Defense-in-depth: codegen embedded `n_data_tests` checks, so the
            // summary row must carry that many outcomes. Recovering fewer means
            // the `data_tests` column was dropped/reshaped before we read it —
            // fail loud rather than silently pass unverified tests.
            if tests.len() < meta.n_data_tests {
                let msg = format!(
                    "data tests on {}: expected {} test outcome(s) but recovered {} from the \
                     result — aborting to avoid a silent pass",
                    meta.asset_path,
                    meta.n_data_tests,
                    tests.len()
                );
                record_mat(
                    conn,
                    &job.workspace_id,
                    job.id,
                    meta,
                    windmill_common::materialization::MaterializationStatus::Failed,
                    snapshot_id,
                    row_count,
                    None,
                    Some(&msg),
                )
                .await;
                return Err(Error::ExecutionErr(msg));
            }
            if tests.iter().any(|t| t.violating > 0) {
                let breakdown = format_data_test_breakdown(&meta.asset_path, &tests);
                record_mat(
                    conn,
                    &job.workspace_id,
                    job.id,
                    meta,
                    windmill_common::materialization::MaterializationStatus::Failed,
                    snapshot_id,
                    row_count,
                    None,
                    Some(&breakdown),
                )
                .await;
                return Err(Error::ExecutionErr(breakdown));
            }
            record_mat(
                conn,
                &job.workspace_id,
                job.id,
                meta,
                windmill_common::materialization::MaterializationStatus::Materialized,
                snapshot_id,
                row_count,
                schema,
                None,
            )
            .await;
        }

        drop(bigquery_credentials);

        *column_order_ref = column_order;
        Ok(result)
    };

    let result = if run_inline {
        result_f.await
    } else {
        run_future_with_polling_update_job_poller(
            job.id,
            job.timeout,
            conn,
            mem_peak,
            canceled_by,
            result_f,
            worker_name,
            &job.workspace_id,
            &mut Some(occupancy_metrics),
            Box::pin(futures::stream::once(async { 0 })),
        )
        .await
    };

    match result {
        Ok(result) => Ok(result),
        Err(e) => {
            // Passwords might appear in the error message
            let mut err_str = e.to_string();
            for pwd in hidden_passwords.lock().unwrap().iter() {
                if let Some(sanitized) = sanitize_string_from_password(&err_str, &pwd.clone()) {
                    err_str = sanitized;
                }
            }
            Err(Error::ExecutionErr(err_str))
        }
    }
}

thread_local! {
    static DUCKDB_FFI_LIB_SINGLETON: RefCell<*const DuckDbFfiLib> = RefCell::new(std::ptr::null());
}

struct DuckDbFfiLib {
    run_duckdb_ffi: Symbol<
        'static,
        unsafe extern "C" fn(
            query_block_list: *const *const c_char,
            query_block_list_count: usize,
            job_args: *const c_char,
            token: *const c_char,
            base_internal_url: *const c_char,
            w_id: *const c_char,
            memory_limit: *const c_char,
            temp_directory: *const c_char,
            column_order_ptr: *mut *mut c_char,
            collect_last_only: bool,
            collect_first_row_only: bool,
        ) -> *mut c_char,
    >,
    prepare_duckdb_ffi: Option<
        Symbol<
            'static,
            unsafe extern "C" fn(
                query_block_list: *const *const c_char,
                query_block_list_count: usize,
                token: *const c_char,
                base_internal_url: *const c_char,
                w_id: *const c_char,
                memory_limit: *const c_char,
                temp_directory: *const c_char,
            ) -> *mut c_char,
        >,
    >,
    free_cstr: Symbol<'static, unsafe extern "C" fn(string: *mut c_char) -> ()>,
}

impl DuckDbFfiLib {
    fn get_singleton() -> Result<&'static DuckDbFfiLib> {
        DUCKDB_FFI_LIB_SINGLETON.with(|cell| unsafe {
            let mut singleton = cell.borrow_mut();
            if singleton.is_null() {
                let lib = DuckDbFfiLib::init()?;
                let boxed_lib = Box::new(lib);
                let lib_ptr = Box::leak(boxed_lib);
                *singleton = lib_ptr as *const _;
                Ok(NonNull::new_unchecked(*singleton as *mut DuckDbFfiLib).as_ref())
            } else {
                Ok(&**singleton)
            }
        })
    }
    fn init() -> Result<Self> {
        let lib = unsafe {
            Library::new(if cfg!(target_os = "macos") {
                "libwindmill_duckdb_ffi_internal.dylib"
            } else if cfg!(target_os = "windows") {
                "windmill_duckdb_ffi_internal.dll"
            } else {
                "libwindmill_duckdb_ffi_internal.so"
            })
            .map_err(|e| {
                Error::InternalErr(format!(
                    "Could not init duckdb. Make sure you have the latest windmill_duckdb_ffi_lib.{} alongside your binary : https://github.com/windmill-labs/windmill/releases \n{}",
                    if cfg!(target_os = "macos") { "dylib" }
                        else if cfg!(target_os = "windows") { "dll" }
                        else { "so" },
                    e.to_string()
                ))
            })?
        };

        let lib = Box::leak(Box::new(lib));

        // Version mismatch should only be possible on Windows agent workers
        // We check for it because FFI interface mismatch will cause undefined behavior / crashes
        unsafe {
            let expected_version: c_uint = 2;
            let get_version: Symbol<'static, unsafe extern "C" fn() -> c_uint> = 
            lib.get(b"get_version")
                .map_err(|e| return Error::ExecutionErr(format!("Could not find get_version in the duckdb ffi library. If you are not using docker, consider manually upgrading windmill_duckdb_ffi_lib. {}", e.to_string())))?;
            let actual_version = get_version();
            if actual_version < expected_version {
                return Err(Error::InternalErr(
                    format!("Incompatible duckdb ffi library version. Expected: {expected_version}, actual: {actual_version}. Please update to the latest windmill_duckdb_ffi_lib."),
                ));
            } else if actual_version > expected_version {
                return Err(Error::InternalErr(
                    format!("Incompatible duckdb ffi library version. Expected: {expected_version}, actual: {actual_version}. Please upgrade your worker to the latest windmill version."),
                ));
            }
        }

        let prepare_duckdb_ffi = unsafe { lib.get(b"prepare_duckdb_ffi").ok() };

        Ok(DuckDbFfiLib {
            run_duckdb_ffi: unsafe { lib.get(b"run_duckdb_ffi").map_err(to_anyhow)? },
            prepare_duckdb_ffi,
            free_cstr: unsafe { lib.get(b"free_cstr").map_err(to_anyhow)? },
        })
    }
}

// 20% headroom for Rust runtime + DuckDB's untracked allocations. Mirrors
// DuckDB's own default ratio, but applied to the worker's cgroup budget
// instead of host RAM.
const DUCKDB_MEMORY_FRACTION: f64 = 0.8;
// Treat cgroup values above 1 PiB as "unlimited" (kernels report page-aligned
// huge numbers when uncapped). get_memory() falls back to host RAM in that
// case, which is exactly what we want to leave to DuckDB's own default.
const CGROUP_UNLIMITED_THRESHOLD: i64 = 1024 * 1024 * 1024 * 1024 * 1024;

// `DUCKDB_MEMORY_LIMIT` env override, else fraction of the worker's cgroup
// memory (as reported by windmill-common), else None (keep DuckDB's default).
fn resolve_duckdb_memory_limit() -> Option<String> {
    if let Ok(v) = env::var("DUCKDB_MEMORY_LIMIT") {
        let v = v.trim();
        if !v.is_empty() {
            return Some(v.to_string());
        }
    }
    cgroup_bytes_to_duckdb_memory_limit(get_memory()?)
}

fn cgroup_bytes_to_duckdb_memory_limit(bytes: i64) -> Option<String> {
    if bytes <= 0 || bytes >= CGROUP_UNLIMITED_THRESHOLD {
        return None;
    }
    let mib = ((bytes as f64 * DUCKDB_MEMORY_FRACTION) as i64) / (1024 * 1024);
    Some(format!("{}MiB", mib.max(64)))
}

// Read backend/windmill-duckdb-ffi-internal/README_DEV.md for details about why we use FFI
fn run_duckdb_ffi_safe<'a>(
    query_block_list: impl Iterator<Item = &'a str>,
    query_block_list_count: usize,
    job_args: Vec<Arg>,
    token: &str,
    base_internal_url: &str,
    w_id: &str,
    job_dir: &str,
    collection_strategy: SqlResultCollectionStrategy,
) -> Result<(Box<RawValue>, Option<Vec<String>>)> {
    let query_block_list = query_block_list
        .map(|s| {
            CString::new(s).map_err(|e| {
                Error::ExecutionErr(format!("Failed CString conversion: {}", e.to_string()))
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let query_block_list = query_block_list
        .iter()
        .map(|s| s.as_ptr())
        .collect::<Vec<_>>();
    let job_args = serde_json::to_string(&job_args).map_err(to_anyhow)?;

    let job_args = CString::new(job_args).map_err(to_anyhow)?;
    let token = CString::new(token).map_err(to_anyhow)?;
    let base_internal_url = CString::new(base_internal_url).map_err(to_anyhow)?;
    let w_id = CString::new(w_id).map_err(to_anyhow)?;
    let memory_limit =
        CString::new(resolve_duckdb_memory_limit().unwrap_or_default()).map_err(to_anyhow)?;
    let temp_directory = CString::new(job_dir).map_err(to_anyhow)?;

    let run_duckdb_ffi = &DuckDbFfiLib::get_singleton()?.run_duckdb_ffi;
    let free_cstr = &DuckDbFfiLib::get_singleton()?.free_cstr;
    let mut column_order: *mut c_char = std::ptr::null_mut();
    let result_str = unsafe {
        let ptr = run_duckdb_ffi(
            query_block_list.as_ptr(),
            query_block_list_count,
            job_args.as_ptr(),
            token.as_ptr(),
            base_internal_url.as_ptr(),
            w_id.as_ptr(),
            memory_limit.as_ptr(),
            temp_directory.as_ptr(),
            &mut column_order,
            collection_strategy.collect_last_statement_only(query_block_list_count),
            collection_strategy.collect_first_row_only(),
        );
        let str = CStr::from_ptr(ptr).to_string_lossy().to_string();
        free_cstr(ptr);
        str
    };

    let column_order = if column_order.is_null()
        || !collection_strategy.collect_last_statement_only(query_block_list_count)
        || collection_strategy.collect_scalar()
    {
        None
    } else {
        let str = unsafe { CStr::from_ptr(column_order).to_string_lossy().to_string() };
        unsafe { free_cstr(column_order) };
        Some(serde_json::from_str::<Vec<String>>(&str)?)
    };

    if result_str.starts_with("ERROR") {
        Err(Error::ExecutionErr(result_str[6..].to_string()))
    } else {
        let result = if collection_strategy == SqlResultCollectionStrategy::AllStatementsAllRows {
            // Avoid parsing JSON
            serde_json::value::RawValue::from_string(result_str).map_err(to_anyhow)?
        } else {
            let result =
                serde_json::from_str::<Vec<Vec<Box<RawValue>>>>(&result_str).map_err(to_anyhow)?;
            collection_strategy.collect(result)?
        };
        Ok((result, column_order))
    }
}

fn prepare_duckdb_ffi_safe<'a>(
    query_block_list: impl Iterator<Item = &'a str>,
    token: &str,
    base_internal_url: &str,
    w_id: &str,
    job_dir: &str,
) -> Result<Box<RawValue>> {
    let query_block_list = query_block_list
        .map(|s| {
            CString::new(s).map_err(|e| {
                Error::ExecutionErr(format!("Failed CString conversion: {}", e.to_string()))
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let query_block_list = query_block_list
        .iter()
        .map(|s| s.as_ptr())
        .collect::<Vec<_>>();

    let token = CString::new(token).map_err(to_anyhow)?;
    let base_internal_url = CString::new(base_internal_url).map_err(to_anyhow)?;
    let w_id = CString::new(w_id).map_err(to_anyhow)?;
    let memory_limit =
        CString::new(resolve_duckdb_memory_limit().unwrap_or_default()).map_err(to_anyhow)?;
    let temp_directory = CString::new(job_dir).map_err(to_anyhow)?;

    let lib = DuckDbFfiLib::get_singleton()?;
    let prepare_fn = lib.prepare_duckdb_ffi.as_ref().ok_or_else(|| {
        Error::InternalErr(
            "prepare_duckdb_ffi not available in duckdb ffi library. Please update to the latest windmill_duckdb_ffi_lib.".to_string(),
        )
    })?;
    let free_cstr = &lib.free_cstr;

    let result_str = unsafe {
        let ptr = prepare_fn(
            query_block_list.as_ptr(),
            query_block_list.len(),
            token.as_ptr(),
            base_internal_url.as_ptr(),
            w_id.as_ptr(),
            memory_limit.as_ptr(),
            temp_directory.as_ptr(),
        );
        let str = CStr::from_ptr(ptr).to_string_lossy().to_string();
        free_cstr(ptr);
        str
    };

    if result_str.starts_with("ERROR") {
        Err(Error::ExecutionErr(result_str[6..].to_string()))
    } else {
        Ok(serde_json::value::RawValue::from_string(result_str).map_err(to_anyhow)?)
    }
}

struct ParsedAttachDbResource<'a> {
    resource_path: &'a str,
    name: &'a str,
    db_type: &'a str,
    extra_args: Option<&'a str>,
}
fn parse_attach_db_resource<'a>(query: &'a str) -> Option<ParsedAttachDbResource<'a>> {
    lazy_static::lazy_static! {
        static ref RE: regex::Regex = regex::Regex::new(r"(?i)ATTACH '(\$res:|res://)([^']+)' AS (\S+) \(TYPE (\w+)(.*)\)").unwrap();
    }

    for cap in RE.captures_iter(query) {
        if let (Some(resource_path), Some(name), Some(db_type)) =
            (cap.get(2), cap.get(3), cap.get(4))
        {
            let extra_args = cap.get(5).map(|m| query[m.start()..m.end()].trim());
            return Some(ParsedAttachDbResource {
                resource_path: query[resource_path.start()..resource_path.end()].trim(),
                name: query[name.start()..name.end()].trim(),
                db_type: query[db_type.start()..db_type.end()].trim(),
                extra_args,
            });
        }
    }
    None
}

fn format_attach_db_conn_str(db_resource: Value, db_type: &str) -> Result<String> {
    let s = match db_type.to_lowercase().as_str() {
        "postgres" | "postgresql" => {
            let res: PgDatabase = serde_json::from_value(db_resource)?;
            res.to_uri()
        }
        #[cfg(feature = "mysql")]
        "mysql" => {
            let resource: MysqlDatabase = serde_json::from_value(db_resource)?;
            format!(
                "database={} host={} ssl_mode={} {} {} {}",
                resource.database,
                resource.host,
                resource
                    .ssl
                    .map(|ssl| if ssl { "required" } else { "disabled" })
                    .unwrap_or("preferred"),
                resource
                    .password
                    .map(|p| format!("password={}", p))
                    .unwrap_or_default(),
                resource
                    .port
                    .map(|p| format!("port={}", p))
                    .unwrap_or_default(),
                resource
                    .user
                    .map(|u| format!("user={}", u))
                    .unwrap_or_default(),
            )
        }
        "bigquery" => {
            let project_id: String = serde_json::from_value(
                db_resource
                    .get("project_id")
                    .ok_or_else(|| {
                        Error::ExecutionErr("BigQuery resource must contain project_id".to_string())
                    })?
                    .to_owned(),
            )
            .map_err(|_e| Error::ExecutionErr("failed project_id deserialize".to_string()))?;
            format!("project={}", project_id,)
        }
        _ => {
            return Err(Error::ExecutionErr(format!(
                "Unsupported db type in DuckDB ATTACH: {db_type}",
            )))
        }
    };
    Ok(s)
}

fn get_attach_db_install_str(db_type: &str) -> Result<&str> {
    match db_type.to_lowercase().as_str() {
        "postgres" => Ok("INSTALL postgres;"),
        "mysql" => {
            #[cfg(not(feature = "mysql"))]
            return Err(Error::ExecutionErr(
                "MySQL feature is not enabled".to_string(),
            ));
            #[cfg(feature = "mysql")]
            Ok("INSTALL mysql;")
        }
        "bigquery" => Ok("INSTALL bigquery FROM community;"),
        _ => Err(Error::ExecutionErr(format!(
            "Unsupported db type in DuckDB ATTACH: {}",
            db_type
        ))),
    }
}

async fn transform_attach_db_resource_query(
    parsed: &ParsedAttachDbResource<'_>,
    job_id: &Uuid,
    client: &AuthedClient,
    hidden_passwords: &mut Arc<Mutex<Vec<String>>>,
) -> Result<Vec<String>> {
    let db_resource: Value = client
        .get_resource_value_interpolated(parsed.resource_path, Some(job_id.to_string()))
        .await?;
    if let Some(pwd) = db_resource.get("password").and_then(|p| p.as_str()) {
        hidden_passwords.lock().unwrap().push(pwd.to_string());
    }
    db_resource_to_attach_statements(db_resource, parsed.name, parsed.db_type, parsed.extra_args)
        .await
}

async fn db_resource_to_attach_statements(
    db_resource: Value,
    ident_name: &str,
    db_type: &str,
    extra_args: Option<&str>,
) -> Result<Vec<String>> {
    // Escape single quotes: the connection string is built from resource fields
    // (host/db/user/password) and embedded in a single-quoted DuckDB literal, so an
    // unescaped quote in any field would otherwise break out of the ATTACH statement.
    let conn_str = format_attach_db_conn_str(db_resource, db_type)?.replace('\'', "''");
    let attach_str = format!(
        "ATTACH '{}' as {} (TYPE {}{});",
        conn_str,
        ident_name,
        db_type,
        extra_args.unwrap_or("")
    )
    .to_string();

    Ok(vec![
        get_attach_db_install_str(db_type)?.to_string(),
        format!("LOAD {};", db_type),
        attach_str,
    ])
}

async fn transform_attach_ducklake(
    query: &str,
    conn: &Connection,
    hidden_passwords: &mut Arc<Mutex<Vec<String>>>,
    w_id: &str,
) -> Result<Option<Vec<String>>> {
    lazy_static::lazy_static! {
        static ref RE: regex::Regex = regex::Regex::new(r"(?i)ATTACH\s*'ducklake(://[^':]+)?'\s*AS\s+([^ ;]+)\s*(\([^)]*\))?").unwrap();
    }
    let Some(cap) = RE.captures(query) else {
        return Ok(None);
    };
    let name = cap.get(1).map(|m| &m.as_str()[3..]).unwrap_or("main");
    let alias_name = cap.get(2).map(|m| m.as_str()).unwrap_or("");
    let extra_args = cap
        .get(3)
        .map(|m| format!(", {}", &m.as_str()[1..m.as_str().len() - 1]))
        .unwrap_or("".to_string());

    let ducklake = match conn {
        Connection::Http(client) => get_ducklake_from_agent_http(client, name, w_id).await?,
        Connection::Sql(db) => get_ducklake_from_db_unchecked(name, w_id, db).await?,
    };
    let db_type = match ducklake.catalog.resource_type {
        DucklakeCatalogResourceType::Instance => "postgres",
        _ => ducklake.catalog.resource_type.as_ref(),
    };

    if let Some(pwd) = ducklake
        .catalog_resource
        .get("password")
        .and_then(|p| p.as_str())
    {
        hidden_passwords.lock().unwrap().push(pwd.to_string());
    }

    // Escape single quotes: db_conn_str, storage and data_path are embedded in
    // single-quoted DuckDB literals below, so an unescaped quote in a resource
    // field would break out of the ATTACH statement.
    let db_conn_str =
        format_attach_db_conn_str(ducklake.catalog_resource, db_type)?.replace('\'', "''");
    let storage = ducklake
        .storage
        .storage
        .as_deref()
        .unwrap_or(DEFAULT_STORAGE)
        .replace('\'', "''");
    let data_path = ducklake.storage.path.replace('\'', "''");

    let extra_args = if let Some(default_extra_args) = ducklake.extra_args {
        format!("{},{}", extra_args, default_extra_args)
    } else {
        extra_args
    };
    // Ducklake 0.3 only requires DATA_PATH at creation and then stores it internally in the catalog
    // But it will fail if DATA_PATH changes afterwards which is annoying for us
    // So we always enable override
    let extra_args = if extra_args.contains("OVERRIDE_DATA_PATH") {
        extra_args
    } else {
        format!(", OVERRIDE_DATA_PATH TRUE{extra_args}")
    };
    // Automatically migrate ducklake
    let extra_args = if extra_args.contains("AUTOMATIC_MIGRATION") {
        extra_args
    } else {
        format!(", AUTOMATIC_MIGRATION TRUE{extra_args}")
    };

    let attach_str = format!(
        "ATTACH 'ducklake:{db_type}:{db_conn_str}' AS {alias_name} (DATA_PATH 's3://{storage}/{data_path}'{extra_args});",
    );

    let install_db_ext_str = get_attach_db_install_str(db_type)?;
    Ok(Some(vec![
        "INSTALL ducklake;".to_string(),
        install_db_ext_str.to_string(),
        attach_str,
    ]))
}

async fn transform_attach_datatable(
    query: &str,
    conn: &Connection,
    hidden_passwords: &mut Arc<Mutex<Vec<String>>>,
    w_id: &str,
) -> Result<Option<Vec<String>>> {
    lazy_static::lazy_static! {
        static ref RE: regex::Regex = regex::Regex::new(r"(?i)ATTACH\s*'datatable(://[^':]+)?'\s*AS\s+([^ ;]+)").unwrap();
    }
    let Some(cap) = RE.captures(query) else {
        return Ok(None);
    };
    let name = cap.get(1).map(|m| &m.as_str()[3..]).unwrap_or("main");
    let alias_name = cap.get(2).map(|m| m.as_str()).unwrap_or("");

    let db_resource = match conn {
        Connection::Http(client) => {
            get_datatable_resource_from_agent_http(client, name, w_id).await?
        }
        Connection::Sql(db) => get_datatable_resource_from_db_unchecked(db, w_id, name).await?,
    };
    let db_type = "postgres";

    if let Some(pwd) = db_resource.get("password").and_then(|p| p.as_str()) {
        hidden_passwords.lock().unwrap().push(pwd.to_string());
    }

    Ok(Some(
        db_resource_to_attach_statements(db_resource, alias_name, db_type, None).await?,
    ))
}

async fn transform_s3_uris(query: &str) -> Result<String> {
    let mut transformed_query = None;
    lazy_static::lazy_static! {
        static ref RE: regex::Regex = regex::Regex::new(r"'s3://([^'/]*)/([^']*)'").unwrap();
    }
    for cap in RE.captures_iter(query) {
        if let (storage, Some(s3_path)) = (cap.get(1), cap.get(2)) {
            let s3_path = s3_path.as_str();
            let mut storage = storage.map(|m| m.as_str()).unwrap_or("");
            if !storage.is_empty() {
                continue;
            }
            let original_str_lit: String = format!("'s3://{}/{}'", storage, s3_path);
            storage = DEFAULT_STORAGE;

            let new_s3_lit = format!("'s3://{}/{}'", storage, s3_path);
            transformed_query = Some(
                transformed_query
                    .unwrap_or_else(|| query.to_string())
                    .replace(&original_str_lit, &new_s3_lit),
            );
        }
    }
    Ok(transformed_query.unwrap_or(query.to_string()))
}

// BigQuery extension requires a json file as credentials
// The file path is set as an env var by do_duckdb
// It is created by transform_attach_db_resource_query (when bigquery is detected)
// and deleted by do_duckdb after the query is executed.
//
// This relies on the fact that DuckDB does not run in native worker, so
// a worker will only run a single job at a time.
pub struct UseBigQueryCredentialsFile {
    path: String,
}
impl UseBigQueryCredentialsFile {
    fn new(job_id: Uuid, bigquery_resource: &str) -> Result<Self> {
        let path = format!("/tmp/service-account-credentials-{}.json", job_id);
        unsafe {
            env::set_var("GOOGLE_APPLICATION_CREDENTIALS", &path);
        }
        std::fs::write(&path, bigquery_resource)
            .map_err(|e| Error::ExecutionErr(format!("Failed to write BigQuery creds: {e}")))?;
        Ok(Self { path })
    }
}
impl Drop for UseBigQueryCredentialsFile {
    fn drop(&mut self) {
        unsafe {
            env::remove_var("GOOGLE_APPLICATION_CREDENTIALS");
        }
        if matches!(std::fs::exists(&self.path), Ok(true)) {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

// Shared with ffi module
#[derive(Serialize, Clone, Debug, PartialEq, Default)]
pub struct Arg {
    pub name: String,
    pub arg_type: String,
    pub json_value: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mrow(name: &str, params: &str, body: &str, is_table: bool, provider: &str) -> MacroRow {
        MacroRow {
            name: name.to_string(),
            params: params.to_string(),
            body: body.to_string(),
            is_table_macro: is_table,
            provider_path: provider.to_string(),
        }
    }

    fn blocks(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    // Mimics the shell: resolve (incl. transitive library `// use`), parse the
    // given lib sources, plan. `libs` maps provider path → deployed source
    // (must cover every relevant provider).
    fn select_and_plan(
        b: &[String],
        registry: &[MacroRow],
        use_libs: &[&str],
        libs: &[(&str, &str)],
    ) -> Result<Vec<String>> {
        use windmill_parser::asset_parser::parse_pipeline_annotations;
        use windmill_parser::duckdb_macros::parse_macro_library;
        let use_libs: Vec<String> = use_libs.iter().map(|s| s.to_string()).collect();
        let lib_uses = libs
            .iter()
            .map(|(p, src)| (p.to_string(), parse_pipeline_annotations(src).use_libs))
            .collect();
        let (selected, effective) = resolve_macro_selection(b, registry, &use_libs, &lib_uses)?;
        let lib_bodies = libs
            .iter()
            .map(|(p, src)| (p.to_string(), parse_macro_library(src).unwrap()))
            .collect();
        plan_macro_injection(&selected, registry, &effective, &lib_bodies)
    }

    #[test]
    fn macro_injection_detects_and_orders_transitively() {
        // consumer calls `outer`; `outer` calls `inner` — both injected, inner first.
        let registry = vec![
            mrow("outer", "a", "inner(a) + 1", false, "f/lib/m"),
            mrow("inner", "a", "a * 2", false, "f/lib/m"),
            mrow("unused", "a", "a", false, "f/lib/m"),
        ];
        let b = blocks(&["ATTACH 'x.duckdb' AS ext;", "SELECT outer(1);"]);
        let injected = select_and_plan(&b, &registry, &[], &[("f/lib/m", "")]).unwrap();
        assert_eq!(
            injected,
            vec![
                "CREATE OR REPLACE TEMP MACRO inner(a) AS a * 2;".to_string(),
                "CREATE OR REPLACE TEMP MACRO outer(a) AS inner(a) + 1;".to_string(),
            ]
        );
    }

    #[test]
    fn implicit_call_injects_provider_setup() {
        // A macro whose body references its own lib's ATTACH must carry that
        // setup even on the implicit (detection) path — DuckDB bind-checks the
        // body at CREATE, so `ext` must be attached first.
        let registry = vec![mrow(
            "lookup",
            "k",
            "(SELECT v FROM ext.kv WHERE key = k)",
            false,
            "f/lib/m",
        )];
        let b = blocks(&["SELECT lookup('a');"]);
        let injected = select_and_plan(
            &b,
            &registry,
            &[],
            &[(
                "f/lib/m",
                "ATTACH 'ext.duckdb' AS ext;\nCREATE MACRO lookup(k) AS (SELECT v FROM ext.kv WHERE key = k);",
            )],
        )
        .unwrap();
        assert_eq!(injected[0], "ATTACH 'ext.duckdb' AS ext;");
        assert!(injected[1].contains("TEMP MACRO lookup(k)"));
    }

    #[test]
    fn provider_lib_use_is_honored_transitively() {
        // Lib B's macro calls `base_macro` inside a string (invisible to
        // lexical detection), so B declares `// use f/lib/base`. A consumer
        // that merely calls B's macro must still get base's whole library —
        // the dynamic dependency is encapsulated in B, not leaked to every
        // consumer.
        let registry = vec![
            mrow(
                "str_macro",
                "",
                "(SELECT v FROM query('SELECT base_macro() AS v'))",
                false,
                "f/lib/b",
            ),
            mrow("base_macro", "", "42", false, "f/lib/base"),
        ];
        let b = blocks(&["SELECT str_macro();"]);
        let injected = select_and_plan(
            &b,
            &registry,
            &[],
            &[
                (
                    "f/lib/b",
                    "-- macros\n-- use f/lib/base\nCREATE MACRO str_macro() AS (SELECT v FROM query('SELECT base_macro() AS v'));",
                ),
                ("f/lib/base", "-- macros\nCREATE MACRO base_macro() AS 42;"),
            ],
        )
        .unwrap();
        assert!(
            injected
                .iter()
                .any(|s| s.contains("TEMP MACRO base_macro()")),
            "{injected:?}"
        );
        assert!(injected
            .iter()
            .any(|s| s.contains("TEMP MACRO str_macro()")));
        // Both defs are injected; string-hidden deps carry no topo edge, so
        // their relative order falls back to name-sorted ties (deterministic
        // for this input — the assertion pins the current behavior).
        let base_idx = injected
            .iter()
            .position(|s| s.contains("TEMP MACRO base_macro()"))
            .unwrap();
        let str_idx = injected
            .iter()
            .position(|s| s.contains("TEMP MACRO str_macro()"))
            .unwrap();
        assert!(base_idx < str_idx, "{injected:?}");
    }

    #[test]
    fn duplicate_setup_across_libs_is_deduped() {
        // Two libs attaching the same catalog identically must not double-ATTACH.
        let registry = vec![
            mrow("m1", "a", "a", false, "f/lib/one"),
            mrow("m2", "a", "a", false, "f/lib/two"),
        ];
        let b = blocks(&["SELECT m1(1), m2(2);"]);
        let src = "ATTACH 'ext.duckdb' AS ext;\nCREATE MACRO m1(a) AS a;";
        let src2 = "ATTACH 'ext.duckdb' AS ext;\nCREATE MACRO m2(a) AS a;";
        let injected = select_and_plan(
            &b,
            &registry,
            &[],
            &[("f/lib/one", src), ("f/lib/two", src2)],
        )
        .unwrap();
        assert_eq!(
            injected
                .iter()
                .filter(|s| s.starts_with("ATTACH 'ext.duckdb'"))
                .count(),
            1
        );
    }

    #[test]
    fn macro_injection_empty_when_nothing_called() {
        let registry = vec![mrow("m", "a", "a", false, "f/lib/m")];
        let b = blocks(&["SELECT 1;"]);
        assert!(select_workspace_macros(&b, &registry, &[])
            .unwrap()
            .is_empty());
    }

    #[test]
    fn local_definition_wins_over_registry() {
        // A consumer defining its own `dbl` must never get the registry's
        // version injected — a library deploy can't change this job.
        let registry = vec![mrow("dbl", "a", "a * 10", false, "f/lib/m")];
        let b = blocks(&["CREATE TEMP MACRO dbl(a) AS a * 2;", "SELECT dbl(4);"]);
        assert!(select_workspace_macros(&b, &registry, &[])
            .unwrap()
            .is_empty());
    }

    #[test]
    fn use_lib_setup_survives_when_no_macros_selected() {
        // Every lib macro shadowed locally — the explicit `// use` must still
        // carry the lib's setup statements (its ATTACH side effects).
        let registry = vec![mrow("dbl", "a", "a * 10", false, "f/lib/m")];
        let b = blocks(&["CREATE TEMP MACRO dbl(a) AS a * 2;", "SELECT dbl(4);"]);
        let injected = select_and_plan(
            &b,
            &registry,
            &["f/lib/m"],
            &[(
                "f/lib/m",
                "ATTACH 'ext.duckdb' AS ext;\nCREATE MACRO dbl(a) AS a * 10;",
            )],
        )
        .unwrap();
        assert_eq!(injected, vec!["ATTACH 'ext.duckdb' AS ext;".to_string()]);
    }

    #[test]
    fn weave_lands_after_local_definition_it_references() {
        // Injected bodies may call a local macro (local-wins excludes it from
        // the registry set), so the leading local CREATE must run first.
        let b = blocks(&[
            "ATTACH 'x' AS a;",
            "CREATE MACRO local_dbl(a) AS a * 2;",
            "SELECT registry_m(1);",
        ]);
        let out = weave_macro_blocks(
            b,
            vec!["CREATE OR REPLACE TEMP MACRO registry_m(a) AS local_dbl(a) + 1;".into()],
        )
        .unwrap();
        assert_eq!(
            out[2],
            "CREATE OR REPLACE TEMP MACRO registry_m(a) AS local_dbl(a) + 1;"
        );
    }

    #[test]
    fn weave_lands_before_local_definition_that_calls_it() {
        // The inverse direction: a LOCAL macro whose body calls a registry
        // macro bind-checks at its own CREATE, so the injected definition
        // (and its library setup) must come first.
        let b = blocks(&[
            "CREATE MACRO outer(x) AS shared_inner(x) + 1;",
            "SELECT outer(1);",
        ]);
        let out = weave_macro_blocks(
            b,
            vec![
                "ATTACH 'ext.duckdb' AS ext;".into(),
                "CREATE OR REPLACE TEMP MACRO shared_inner(x) AS x * 2;".into(),
            ],
        )
        .unwrap();
        assert_eq!(
            out,
            blocks(&[
                "ATTACH 'ext.duckdb' AS ext;",
                "CREATE OR REPLACE TEMP MACRO shared_inner(x) AS x * 2;",
                "CREATE MACRO outer(x) AS shared_inner(x) + 1;",
                "SELECT outer(1);",
            ])
        );
    }

    #[test]
    fn weave_pulls_injected_dependencies_before_the_calling_local() {
        // outer(local) calls injected `mid`, whose body calls injected `base`:
        // both must precede the local definition, base before mid.
        let b = blocks(&["CREATE MACRO outer(x) AS mid(x) + 1;", "SELECT outer(1);"]);
        let out = weave_macro_blocks(
            b,
            vec![
                "CREATE OR REPLACE TEMP MACRO base(x) AS x * 2;".into(),
                "CREATE OR REPLACE TEMP MACRO mid(x) AS base(x) + 1;".into(),
            ],
        )
        .unwrap();
        let pos = |needle: &str| out.iter().position(|s| s.contains(needle)).unwrap();
        assert!(pos("MACRO base(x)") < pos("MACRO mid(x)"));
        assert!(pos("MACRO mid(x)") < pos("MACRO outer(x)"));
    }

    #[test]
    fn weave_conflicting_local_order_errors() {
        // local_a calls injected X; X references local_b, defined after
        // local_a — unsatisfiable, must error rather than silently mis-order.
        let b = blocks(&[
            "CREATE MACRO local_a(x) AS conflicted(x);",
            "CREATE MACRO local_b(x) AS x * 3;",
            "SELECT local_a(1);",
        ]);
        let err = weave_macro_blocks(
            b,
            vec!["CREATE OR REPLACE TEMP MACRO conflicted(x) AS local_b(x) + 1;".into()],
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("conflicting"), "{err}");
    }

    #[test]
    fn use_lib_injects_all_macros_and_setup() {
        let registry = vec![
            mrow("m1", "a", "a", false, "f/lib/m"),
            mrow("m2", "", "SELECT 1", true, "f/lib/m"),
        ];
        let b = blocks(&["SELECT 'no calls here';"]);
        let injected = select_and_plan(
            &b,
            &registry,
            &["f/lib/m"],
            &[(
                "f/lib/m",
                "ATTACH 'ext.duckdb' AS ext;\nCREATE MACRO m1(a) AS a;\nCREATE MACRO m2() AS TABLE SELECT 1;",
            )],
        )
        .unwrap();
        assert_eq!(injected[0], "ATTACH 'ext.duckdb' AS ext;");
        assert!(injected[1..].iter().any(|s| s.contains("TEMP MACRO m1(a)")));
        assert!(injected[1..]
            .iter()
            .any(|s| s.contains("TEMP MACRO m2() AS TABLE")));
    }

    #[test]
    fn use_lib_without_registry_rows_errors() {
        let b = blocks(&["SELECT 1;"]);
        let err = select_workspace_macros(&b, &[], &["f/lib/gone".to_string()])
            .unwrap_err()
            .to_string();
        assert!(err.contains("no deployed macro library"), "{err}");
    }

    #[test]
    fn weave_lands_after_setup_prefix() {
        let b = blocks(&[
            "INSTALL ducklake;",
            "ATTACH 'ducklake:postgres:...' AS lake (DATA_PATH 's3://x');",
            "CREATE TABLE IF NOT EXISTS lake.t AS SELECT 1;",
            "SELECT dbl(1);",
        ]);
        let out = weave_macro_blocks(
            b,
            vec!["CREATE OR REPLACE TEMP MACRO dbl(a) AS a * 2;".into()],
        )
        .unwrap();
        assert_eq!(out[2], "CREATE OR REPLACE TEMP MACRO dbl(a) AS a * 2;");
        assert_eq!(out.len(), 5);
    }

    #[test]
    fn weave_appends_when_all_setup_or_empty() {
        let out = weave_macro_blocks(blocks(&["ATTACH 'x' AS a;"]), blocks(&["m;"])).unwrap();
        assert_eq!(out, blocks(&["ATTACH 'x' AS a;", "m;"]));
        let out = weave_macro_blocks(vec![], blocks(&["m;"])).unwrap();
        assert_eq!(out, blocks(&["m;"]));
    }

    #[test]
    fn cgroup_bytes_unlimited_or_invalid_returns_none() {
        assert_eq!(cgroup_bytes_to_duckdb_memory_limit(0), None);
        assert_eq!(cgroup_bytes_to_duckdb_memory_limit(-1), None);
        // 1 PiB sentinel: cgroup v1 reports ~i64::MAX when uncapped.
        assert_eq!(
            cgroup_bytes_to_duckdb_memory_limit(CGROUP_UNLIMITED_THRESHOLD),
            None
        );
    }

    #[test]
    fn cgroup_bytes_real_values_take_80_percent() {
        // 1 GiB -> 80% -> 819 MiB (floored to MiB)
        assert_eq!(
            cgroup_bytes_to_duckdb_memory_limit(1024 * 1024 * 1024),
            Some("819MiB".to_string())
        );
        // 4 GiB -> 3276 MiB
        assert_eq!(
            cgroup_bytes_to_duckdb_memory_limit(4 * 1024 * 1024 * 1024),
            Some("3276MiB".to_string())
        );
    }

    #[test]
    fn cgroup_bytes_tiny_values_floored_to_64mib() {
        // Tiny cgroup must not produce a 0/unusable limit.
        assert_eq!(
            cgroup_bytes_to_duckdb_memory_limit(1024 * 1024),
            Some("64MiB".to_string())
        );
        assert_eq!(
            cgroup_bytes_to_duckdb_memory_limit(1),
            Some("64MiB".to_string())
        );
    }

    // Managed `// materialize` may take SQL args (e.g. an s3object uploaded on
    // the run form). The wrap strips line comments — including the
    // `-- $name (type)` declarations — so the executor parses the signature from
    // the original script (done above, before the rewrite) while the `$name`
    // references survive inside the wrapped SELECT. This pins both halves of that
    // contract so a regression that drops either is caught.
    #[test]
    fn materialize_preserves_sql_args() {
        let script = "-- materialize ducklake://main/rows\n\
                      -- $file (s3object)\n\
                      SELECT * FROM read_json_auto($file)";

        // The signature is recoverable from the original (un-wrapped) script.
        let sig = parse_duckdb_sig(script).expect("sig parses").args;
        let file_arg = sig
            .iter()
            .find(|a| a.name == "file")
            .expect("`$file` declared");
        assert_eq!(file_arg.otyp.as_deref(), Some("s3object"));

        // The wrapped query still references `$file`, so the parsed sig binds it.
        // No custom data tests here, so no fetched bodies are needed.
        let (rewritten, _) =
            build_materialized_query(script, None, &std::collections::HashMap::new())
                .expect("materialize builds")
                .expect("materialize present");
        let rewritten = rewritten.expect("managed mode rewrites the query");
        assert!(
            rewritten.contains("$file"),
            "wrapped query must keep the `$file` reference, got:\n{rewritten}"
        );
        // The declaration comment is gone (wrap strips line comments) — which is
        // exactly why the sig must come from the original, not the rewrite.
        assert!(!rewritten.contains("-- $file"));
    }

    // SCD2 managed mode wraps the SELECT into the diff → close-old → open-new
    // shape (unit-covered in the parser's codegen tests); here we pin the
    // executor-level wiring: the natural key flows through and the wrap is
    // generated (not the manual track-only path).
    #[test]
    fn materialize_scd2_wraps_with_history() {
        // Primary spelling: `key=<col> history` on a merge.
        let script = "-- materialize ducklake://main/dim key=id history track=name\n\
                      SELECT id, name FROM dl.src";
        let (rewritten, _) =
            build_materialized_query(script, None, &std::collections::HashMap::new())
                .expect("materialize builds")
                .expect("materialize present");
        let rewritten = rewritten.expect("scd2 is managed — must rewrite");
        assert!(
            rewritten.contains("valid_from"),
            "adds SCD2 columns:\n{rewritten}"
        );
        assert!(rewritten.contains("is_current"));
        assert!(
            rewritten.contains("_wm_scd2_changed"),
            "captures changed keys"
        );
        assert!(
            rewritten.contains("UPDATE _wm_target.dim SET valid_to"),
            "closes prior version"
        );
        assert!(
            rewritten.contains("CREATE VIEW IF NOT EXISTS _wm_target.dim_current"),
            "emits the consumer-convenience current view"
        );
        // default is soft-delete — no deleted-key set without `deletes=close`
        assert!(!rewritten.contains("_wm_scd2_deleted"));
        assert!(!rewritten.contains("MERGE INTO"));
    }

    #[test]
    fn materialize_scd2_deletes_close_wires_through() {
        let script = "-- materialize ducklake://main/dim key=id history deletes=close\n\
                      SELECT id, name FROM dl.src";
        let (rewritten, _) =
            build_materialized_query(script, None, &std::collections::HashMap::new())
                .expect("materialize builds")
                .expect("materialize present");
        let rewritten = rewritten.expect("scd2 is managed — must rewrite");
        assert!(
            rewritten.contains("_wm_scd2_deleted"),
            "deletes=close adds the vanished-key set + close:\n{rewritten}"
        );
    }

    #[test]
    fn materialize_scd2_requires_key() {
        let script = "-- materialize scd2 ducklake://main/dim\nSELECT id, name FROM dl.src";
        let err = match build_materialized_query(script, None, &std::collections::HashMap::new()) {
            Err(e) => e,
            Ok(_) => panic!("scd2 without key must error"),
        };
        assert!(
            format!("{err}").contains("requires a natural key"),
            "got: {err}"
        );
    }

    #[test]
    fn materialize_scd2_rejects_partitioned() {
        let script = "-- pipeline\n-- partitioned daily\n\
                      -- materialize scd2 ducklake://main/dim key=id\nSELECT id, name FROM dl.src";
        let err = match build_materialized_query(
            script,
            Some("2026-07-01"),
            &std::collections::HashMap::new(),
        ) {
            Err(e) => e,
            Ok(_) => panic!("partitioned + scd2 must error"),
        };
        assert!(
            format!("{err}").contains("not supported with scd2"),
            "got: {err}"
        );
    }

    // Tests for parse_attach_db_resource function
    #[test]
    fn test_parse_attach_db_resource_postgres_res_prefix() {
        let query = "ATTACH '$res:u/user/my_postgres' AS mydb (TYPE POSTGRES)";
        let result = parse_attach_db_resource(query);
        assert!(result.is_some());
        let parsed = result.unwrap();
        assert_eq!(parsed.resource_path, "u/user/my_postgres");
        assert_eq!(parsed.name, "mydb");
        assert_eq!(parsed.db_type, "POSTGRES");
        assert!(parsed.extra_args.is_none() || parsed.extra_args.unwrap().is_empty());
    }

    #[test]
    fn test_parse_attach_db_resource_res_protocol() {
        let query = "ATTACH 'res://f/folder/database' AS db (TYPE postgresql)";
        let result = parse_attach_db_resource(query);
        assert!(result.is_some());
        let parsed = result.unwrap();
        assert_eq!(parsed.resource_path, "f/folder/database");
        assert_eq!(parsed.name, "db");
        assert_eq!(parsed.db_type, "postgresql");
    }

    #[test]
    fn test_parse_attach_db_resource_mysql() {
        let query = "ATTACH '$res:u/admin/mysql_prod' AS mysql_db (TYPE mysql)";
        let result = parse_attach_db_resource(query);
        assert!(result.is_some());
        let parsed = result.unwrap();
        assert_eq!(parsed.resource_path, "u/admin/mysql_prod");
        assert_eq!(parsed.name, "mysql_db");
        assert_eq!(parsed.db_type, "mysql");
    }

    #[test]
    fn test_parse_attach_db_resource_bigquery() {
        let query = "ATTACH '$res:u/user/bq_resource' AS bq (TYPE bigquery)";
        let result = parse_attach_db_resource(query);
        assert!(result.is_some());
        let parsed = result.unwrap();
        assert_eq!(parsed.resource_path, "u/user/bq_resource");
        assert_eq!(parsed.name, "bq");
        assert_eq!(parsed.db_type, "bigquery");
    }

    #[test]
    fn test_parse_attach_db_resource_with_extra_args() {
        let query = "ATTACH '$res:u/user/db' AS mydb (TYPE POSTGRES, READ_ONLY)";
        let result = parse_attach_db_resource(query);
        assert!(result.is_some());
        let parsed = result.unwrap();
        assert_eq!(parsed.resource_path, "u/user/db");
        assert_eq!(parsed.name, "mydb");
        assert_eq!(parsed.db_type, "POSTGRES");
        assert_eq!(parsed.extra_args.unwrap(), ", READ_ONLY");
    }

    #[test]
    fn test_parse_attach_db_resource_case_insensitive() {
        let query = "attach '$res:u/user/db' as mydb (type postgres)";
        let result = parse_attach_db_resource(query);
        assert!(result.is_some());
        let parsed = result.unwrap();
        assert_eq!(parsed.resource_path, "u/user/db");
        assert_eq!(parsed.name, "mydb");
        assert_eq!(parsed.db_type, "postgres");
    }

    #[test]
    fn test_parse_attach_db_resource_no_match() {
        let query = "SELECT * FROM table";
        let result = parse_attach_db_resource(query);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_attach_db_resource_regular_attach() {
        // Regular ATTACH without $res: or res:// should not match
        let query = "ATTACH 'mydb.duckdb' AS mydb (TYPE duckdb)";
        let result = parse_attach_db_resource(query);
        assert!(result.is_none());
    }

    // Tests for format_attach_db_conn_str function
    #[test]
    fn test_format_attach_db_conn_str_postgres_full() {
        let db_resource = json!({
            "host": "localhost",
            "port": 5432,
            "user": "admin",
            "password": "secret123",
            "dbname": "mydb",
            "sslmode": "require"
        });
        let result = format_attach_db_conn_str(db_resource, "postgres").unwrap();
        // Should be in URI format: postgres://user:password@host:port/dbname?sslmode=require
        assert!(result.starts_with("postgres://"));
        assert!(result.contains("admin:secret123@localhost:5432/mydb"));
        assert!(result.contains("sslmode=require"));
    }

    #[test]
    fn test_format_attach_db_conn_str_postgres_minimal() {
        let db_resource = json!({
            "host": "db.example.com",
            "dbname": "production"
        });
        let result = format_attach_db_conn_str(db_resource, "postgres").unwrap();
        // Should be in URI format with defaults: postgres://postgres:@host:5432/dbname?sslmode=prefer
        assert!(result.starts_with("postgres://"));
        assert!(result.contains("@db.example.com:5432/production"));
        assert!(result.contains("sslmode=prefer"));
    }

    #[test]
    fn test_format_attach_db_conn_str_postgresql_alias() {
        let db_resource = json!({
            "host": "localhost",
            "dbname": "test"
        });
        let result = format_attach_db_conn_str(db_resource, "postgresql").unwrap();
        // Should be in URI format (postgresql is treated the same as postgres)
        assert!(result.starts_with("postgres://"));
        assert!(result.contains("@localhost:5432/test"));
        assert!(result.contains("sslmode=prefer"));
    }

    #[test]
    fn test_format_attach_db_conn_str_bigquery() {
        let db_resource = json!({
            "project_id": "my-gcp-project"
        });
        let result = format_attach_db_conn_str(db_resource, "bigquery").unwrap();
        assert_eq!(result, "project=my-gcp-project");
    }

    #[test]
    fn test_format_attach_db_conn_str_bigquery_missing_project_id() {
        let db_resource = json!({
            "other_field": "value"
        });
        let result = format_attach_db_conn_str(db_resource, "bigquery");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("project_id"));
    }

    #[test]
    fn test_format_attach_db_conn_str_unsupported_type() {
        let db_resource = json!({});
        let result = format_attach_db_conn_str(db_resource, "oracle");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unsupported db type"));
    }

    #[test]
    fn test_format_attach_db_conn_str_case_insensitive() {
        let db_resource = json!({
            "host": "localhost",
            "dbname": "test"
        });
        let result = format_attach_db_conn_str(db_resource, "POSTGRES").unwrap();
        // Should be in URI format
        assert!(result.starts_with("postgres://"));
        assert!(result.contains("@localhost:5432/test"));
    }

    #[cfg(feature = "mysql")]
    #[test]
    fn test_format_attach_db_conn_str_mysql_full() {
        let db_resource = json!({
            "host": "mysql.example.com",
            "port": 3306,
            "user": "root",
            "password": "mysecret",
            "database": "app_db",
            "ssl": true
        });
        let result = format_attach_db_conn_str(db_resource, "mysql").unwrap();
        assert!(result.contains("database=app_db"));
        assert!(result.contains("host=mysql.example.com"));
        assert!(result.contains("ssl_mode=required"));
        assert!(result.contains("password=mysecret"));
        assert!(result.contains("port=3306"));
        assert!(result.contains("user=root"));
    }

    #[cfg(feature = "mysql")]
    #[test]
    fn test_format_attach_db_conn_str_mysql_ssl_disabled() {
        let db_resource = json!({
            "host": "localhost",
            "database": "test",
            "ssl": false
        });
        let result = format_attach_db_conn_str(db_resource, "mysql").unwrap();
        assert!(result.contains("ssl_mode=disabled"));
    }

    // Tests for get_attach_db_install_str function
    #[test]
    fn test_get_attach_db_install_str_postgres() {
        let result = get_attach_db_install_str("postgres").unwrap();
        assert_eq!(result, "INSTALL postgres;");
    }

    #[test]
    fn test_get_attach_db_install_str_bigquery() {
        let result = get_attach_db_install_str("bigquery").unwrap();
        assert_eq!(result, "INSTALL bigquery FROM community;");
    }

    #[test]
    fn test_get_attach_db_install_str_unsupported() {
        let result = get_attach_db_install_str("sqlite");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unsupported db type"));
    }

    #[test]
    fn test_get_attach_db_install_str_case_insensitive() {
        let result = get_attach_db_install_str("POSTGRES").unwrap();
        assert_eq!(result, "INSTALL postgres;");
    }

    #[cfg(feature = "mysql")]
    #[test]
    fn test_get_attach_db_install_str_mysql() {
        let result = get_attach_db_install_str("mysql").unwrap();
        assert_eq!(result, "INSTALL mysql;");
    }

    #[cfg(not(feature = "mysql"))]
    #[test]
    fn test_get_attach_db_install_str_mysql_disabled() {
        let result = get_attach_db_install_str("mysql");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("MySQL feature is not enabled"));
    }

    // Tests for transform_s3_uris function
    #[tokio::test]
    async fn test_transform_s3_uris_empty_storage() {
        let query = "SELECT * FROM read_parquet('s3:///path/to/file.parquet')";
        let result = transform_s3_uris(query).await.unwrap();
        assert_eq!(
            result,
            "SELECT * FROM read_parquet('s3://_default_/path/to/file.parquet')"
        );
    }

    #[tokio::test]
    async fn test_transform_s3_uris_with_storage() {
        // URIs with explicit storage should not be transformed
        let query = "SELECT * FROM read_parquet('s3://mybucket/path/to/file.parquet')";
        let result = transform_s3_uris(query).await.unwrap();
        assert_eq!(result, query);
    }

    #[tokio::test]
    async fn test_transform_s3_uris_multiple_empty() {
        let query = "SELECT * FROM read_parquet('s3:///file1.parquet') UNION SELECT * FROM read_parquet('s3:///file2.parquet')";
        let result = transform_s3_uris(query).await.unwrap();
        assert!(result.contains("s3://_default_/file1.parquet"));
        assert!(result.contains("s3://_default_/file2.parquet"));
    }

    #[tokio::test]
    async fn test_transform_s3_uris_no_s3() {
        let query = "SELECT * FROM my_table";
        let result = transform_s3_uris(query).await.unwrap();
        assert_eq!(result, query);
    }

    #[tokio::test]
    async fn test_transform_s3_uris_mixed() {
        let query = "SELECT * FROM read_parquet('s3:///default.parquet'), read_csv('s3://explicit/file.csv')";
        let result = transform_s3_uris(query).await.unwrap();
        assert!(result.contains("s3://_default_/default.parquet"));
        assert!(result.contains("s3://explicit/file.csv"));
    }

    #[tokio::test]
    async fn test_transform_s3_uris_nested_path() {
        let query = "SELECT * FROM read_parquet('s3:///deep/nested/path/file.parquet')";
        let result = transform_s3_uris(query).await.unwrap();
        assert_eq!(
            result,
            "SELECT * FROM read_parquet('s3://_default_/deep/nested/path/file.parquet')"
        );
    }

    // Tests for Arg struct
    #[test]
    fn test_arg_serialization() {
        let arg = Arg {
            name: "test_arg".to_string(),
            arg_type: "string".to_string(),
            json_value: json!("hello"),
        };
        let serialized = serde_json::to_string(&arg).unwrap();
        assert!(serialized.contains("\"name\":\"test_arg\""));
        assert!(serialized.contains("\"arg_type\":\"string\""));
        assert!(serialized.contains("\"json_value\":\"hello\""));
    }

    #[test]
    fn test_arg_serialization_number() {
        let arg = Arg {
            name: "count".to_string(),
            arg_type: "integer".to_string(),
            json_value: json!(42),
        };
        let serialized = serde_json::to_string(&arg).unwrap();
        assert!(serialized.contains("\"json_value\":42"));
    }

    #[test]
    fn test_arg_serialization_null() {
        let arg = Arg {
            name: "optional".to_string(),
            arg_type: "text".to_string(),
            json_value: json!(null),
        };
        let serialized = serde_json::to_string(&arg).unwrap();
        assert!(serialized.contains("\"json_value\":null"));
    }

    #[test]
    fn test_arg_serialization_array() {
        let arg = Arg {
            name: "items".to_string(),
            arg_type: "array".to_string(),
            json_value: json!([1, 2, 3]),
        };
        let serialized = serde_json::to_string(&arg).unwrap();
        assert!(serialized.contains("\"json_value\":[1,2,3]"));
    }

    #[test]
    fn test_arg_serialization_object() {
        let arg = Arg {
            name: "config".to_string(),
            arg_type: "object".to_string(),
            json_value: json!({"key": "value"}),
        };
        let serialized = serde_json::to_string(&arg).unwrap();
        assert!(serialized.contains("\"json_value\":{\"key\":\"value\"}"));
    }

    fn raw(s: &str) -> Box<RawValue> {
        serde_json::from_str(s).unwrap()
    }

    #[test]
    fn extract_data_tests_parses_nested_array() {
        // The real result shape: an array of one summary row carrying a nested
        // `data_tests` array (how the FFI serialises the list-of-struct).
        let r = raw(
            r#"[{"rows":3,"snapshot_id":17,"materialized":"ducklake://a/b",
                "data_tests":[{"test":"unique(order_id)","violating":0},
                              {"test":"accepted_values(status)","violating":2}]}]"#,
        );
        let out = extract_data_tests(&r);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].name, "unique(order_id)");
        assert_eq!(out[0].violating, 0);
        assert_eq!(out[1].name, "accepted_values(status)");
        assert_eq!(out[1].violating, 2);
    }

    #[test]
    fn extract_data_tests_handles_string_encoded_and_absent() {
        // Fallback: some serialisations surface the list-of-struct as a JSON string.
        let s = raw(r#"{"data_tests":"[{\"test\":\"not_null(x)\",\"violating\":1}]"}"#);
        let out = extract_data_tests(&s);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name, "not_null(x)");
        assert_eq!(out[0].violating, 1);
        // Absent column (no tests) -> empty, no panic.
        assert!(extract_data_tests(&raw(r#"[{"rows":3}]"#)).is_empty());
    }

    #[test]
    fn extract_schema_parses_nested_and_string_encoded() {
        // Real shape: the summary row carries a nested `output_schema`
        // list-of-struct from the DESCRIBE fold.
        let r = raw(
            r#"[{"materialized":"ducklake://a/b","rows":3,"snapshot_id":17,
                "output_schema":[{"name":"order_id","type":"BIGINT"},
                                 {"name":"status","type":"VARCHAR"}]}]"#,
        );
        let cols = extract_schema(&r).expect("schema present");
        assert_eq!(cols.len(), 2);
        assert_eq!(cols[0].name, "order_id");
        assert_eq!(cols[0].data_type, "BIGINT");
        assert_eq!(cols[1].name, "status");
        assert_eq!(cols[1].data_type, "VARCHAR");
        // Fallback: FFI serialised the list-of-struct as a JSON string.
        let s = raw(r#"{"output_schema":"[{\"name\":\"x\",\"type\":\"INTEGER\"}]"}"#);
        let cols = extract_schema(&s).expect("schema present");
        assert_eq!(cols.len(), 1);
        assert_eq!(cols[0].name, "x");
        assert_eq!(cols[0].data_type, "INTEGER");
        // Absent column (literal/manual mode) -> None, no panic.
        assert!(extract_schema(&raw(r#"[{"rows":3}]"#)).is_none());
    }

    #[test]
    fn format_data_test_breakdown_lists_all_with_marks() {
        let tests = vec![
            DataTestOutcome { name: "unique(order_id)".into(), violating: 1 },
            DataTestOutcome { name: "not_null(user_id)".into(), violating: 0 },
            DataTestOutcome { name: "accepted_values(status)".into(), violating: 2 },
        ];
        let msg = format_data_test_breakdown("analytics/orders", &tests);
        assert_eq!(
            msg,
            "data tests failed on analytics/orders (2/3 failed):\n  \
             ✗ unique(order_id) — 1 violating row(s)\n  \
             ✓ not_null(user_id)\n  \
             ✗ accepted_values(status) — 2 violating row(s)"
        );
    }
}
