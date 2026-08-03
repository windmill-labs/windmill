use axum::{
    extract::{Path, Query},
    routing::{get, post},
    Extension, Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Row;
use windmill_common::{
    assets::{parse_asset_trigger_ref, AssetKind, AssetUsageKind},
    db::UserDB,
    error::JsonResult,
    utils::escape_ilike_pattern,
};

use windmill_api_auth::{build_scope_path_predicate, ApiAuthed};

// Partition-range backfill preview. The logic (producer resolution, range
// enumeration, status join) is enterprise: the `private` build compiles the
// EE module, the public build a stub that errors.
#[cfg(feature = "private")]
mod backfill_ee;
#[cfg(feature = "private")]
use backfill_ee as backfill;
#[cfg(not(feature = "private"))]
mod backfill_oss;
#[cfg(not(feature = "private"))]
use backfill_oss as backfill;

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list", get(list_assets))
        .route("/list_by_usages", post(list_assets_by_usages))
        .route("/list_favorites", get(list_favorites))
        .route("/graph", get(asset_graph))
        .route("/pipelines", get(list_pipeline_folders))
        .route("/partitions", get(list_partitions))
        .route("/partitions_in_range", get(list_partitions_in_range))
        .route("/asset_schemas", get(list_asset_schemas))
        .route("/record_materialization", post(record_materialization))
        .route("/macros", get(list_macros))
}

// One registry macro, with its full definition — drives the macro-explorer
// drawer (body preview) and the DuckDB editor autocomplete (signatures).
#[derive(Serialize)]
struct MacroListItem {
    name: String,
    params: String,
    body: String,
    is_table: bool,
    provider_path: String,
}

// Every workspace macro (`// macros` libraries), grouped client-side by
// provider. Small by construction — one row per macro definition. The rows
// copy script body text, so visibility must match reading the provider
// script itself: the EXISTS join runs under the user_db transaction (script
// RLS filters libraries the caller can't read) and the scope predicate
// covers path-scoped tokens, mirroring `list_scripts`.
async fn list_macros(
    authed: ApiAuthed,
    Path(w_id): Path<String>,
    Extension(user_db): Extension<UserDB>,
) -> JsonResult<Vec<MacroListItem>> {
    let scope_allowed = build_scope_path_predicate(&authed, "scripts", "read");
    let mut tx = user_db.begin(&authed).await?;
    let rows = sqlx::query!(
        r#"SELECT m.name AS "name!", m.params AS "params!", m.body AS "body!",
                  m.is_table_macro AS "is_table_macro!", m.provider_path AS "provider_path!"
           FROM macro_definition m
           WHERE m.workspace_id = $1
             AND EXISTS (
                SELECT 1 FROM script s
                WHERE s.workspace_id = m.workspace_id
                  AND s.path = m.provider_path
                  AND s.archived = false
                  AND s.deleted = false
             )
           ORDER BY m.provider_path, m.name"#,
        &w_id,
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(
        rows.into_iter()
            .filter(|r| scope_allowed(&r.provider_path))
            .map(|r| MacroListItem {
                name: r.name,
                params: r.params,
                body: r.body,
                is_table: r.is_table_macro,
                provider_path: r.provider_path,
            })
            .collect(),
    ))
}

#[derive(Deserialize)]
struct PartitionsQuery {
    // The materialized asset path (`<ducklake>/<table>`).
    path: String,
}

// Per-partition materialization status for a ducklake asset — drives the
// partition-status grid and the backfill worklist. Materialization targets are
// ducklake-only in v1, so the kind is fixed.
async fn list_partitions(
    authed: ApiAuthed,
    Path(w_id): Path<String>,
    Extension(user_db): Extension<UserDB>,
    Query(q): Query<PartitionsQuery>,
) -> JsonResult<Vec<windmill_common::materialization::MaterializedPartition>> {
    let mut tx = user_db.begin(&authed).await?;
    let rows = windmill_common::materialization::list_materialized_partitions(
        &mut *tx,
        &w_id,
        AssetKind::Ducklake,
        &q.path,
    )
    .await?;
    tx.commit().await?;
    Ok(Json(rows))
}

// Only the EE `backfill` module reads the fields; the OSS stub errors without
// touching them.
#[cfg_attr(not(feature = "private"), allow(dead_code))]
#[derive(Deserialize)]
struct PartitionsInRangeQuery {
    // The materialized ducklake asset path (`<ducklake>/<table>`).
    path: String,
    // Inclusive calendar-day range (YYYY-MM-DD), local to the producer's
    // partition tz.
    from: chrono::NaiveDate,
    to: chrono::NaiveDate,
}

#[derive(Serialize)]
struct PartitionInRange {
    partition: String,
    // `missing` | `running` | `materialized` | `failed` — `missing` means no
    // materialization was ever recorded for the slice.
    status: &'static str,
}

#[derive(Serialize)]
struct PartitionsInRangeResponse {
    // The pipeline script that materializes the asset (managed `// materialize`
    // target, or a partitioned writer using the SDK helpers) — the runnable a
    // backfill launches (with an explicit `partition` arg per slice).
    producer_path: String,
    partition_kind: String,
    partitions: Vec<PartitionInRange>,
}

// Backfill range preview: every partition the producer's `// partitioned` spec
// expects in `[from, to]`, joined with what `materialized_partition` records —
// the missing/failed subset is the backfill worklist. The logic is in the
// `backfill` module pair: EE resolves and enumerates, the OSS stub errors
// (single-partition runs stay available everywhere; fanning out over a range
// is enterprise).
async fn list_partitions_in_range(
    authed: ApiAuthed,
    Path(w_id): Path<String>,
    Extension(user_db): Extension<UserDB>,
    Query(q): Query<PartitionsInRangeQuery>,
) -> JsonResult<PartitionsInRangeResponse> {
    let mut tx = user_db.begin(&authed).await?;
    let res = backfill::partitions_in_range(&mut tx, &w_id, &q).await?;
    tx.commit().await?;
    Ok(Json(res))
}

// Per-asset captured output schema versions for a ducklake asset (gap #2a) —
// the schema-evolution history persisted after each managed `// materialize`.
// Newest version first; materialization targets are ducklake-only in v1, so the
// kind is fixed.
async fn list_asset_schemas(
    authed: ApiAuthed,
    Path(w_id): Path<String>,
    Extension(user_db): Extension<UserDB>,
    Query(q): Query<PartitionsQuery>,
) -> JsonResult<Vec<windmill_common::materialization::AssetSchemaVersion>> {
    let mut tx = user_db.begin(&authed).await?;
    let rows = windmill_common::materialization::list_asset_schemas(
        &mut *tx,
        &w_id,
        AssetKind::Ducklake,
        &q.path,
    )
    .await?;
    tx.commit().await?;
    Ok(Json(rows))
}

// Record a materialization outcome from a polyglot (Python/TS) `wmill.ducklake`
// helper running as a pipeline step. The DuckDB `// materialize` engine records
// this itself; the SDK helpers post here instead so SDK-materialized slices show
// up in the grid identically. When the helper also captured the output schema,
// that schema version is upserted too. RLS-scoped to the caller's workspace.
async fn record_materialization(
    authed: ApiAuthed,
    Path(w_id): Path<String>,
    Extension(user_db): Extension<UserDB>,
    Json(req): Json<windmill_common::materialization::RecordMaterializationRequest>,
) -> JsonResult<()> {
    let mut tx = user_db.clone().begin(&authed).await?;
    windmill_common::materialization::record_materialization(
        &mut *tx,
        &w_id,
        req.asset_kind,
        &req.asset_path,
        &req.partition,
        req.status,
        req.snapshot_id,
        req.row_count,
        req.job_id,
        req.error.as_deref(),
    )
    .await?;
    tx.commit().await?;
    // Schema capture is independently best-effort (its own transaction for the
    // per-asset advisory lock) and must never roll back the partition record
    // above — mirroring the worker's `record_mat`. A lost schema version
    // degrades the history, not the run. Only a successful (`Materialized`) write
    // advances the recorded schema — a failed/running write must not (and a
    // client shouldn't be able to bump the history by attaching a schema to one).
    let is_materialized = matches!(
        req.status,
        windmill_common::materialization::MaterializationStatus::Materialized
    );
    if let (true, Some(columns)) = (is_materialized, req.schema.as_ref()) {
        let res: windmill_common::error::Result<()> = async {
            let mut tx = user_db.clone().begin(&authed).await?;
            windmill_common::materialization::record_asset_schema(
                &mut tx,
                &w_id,
                req.asset_kind,
                &req.asset_path,
                columns,
                req.snapshot_id,
                req.job_id,
            )
            .await?;
            tx.commit().await?;
            Ok(())
        }
        .await;
        if let Err(e) = res {
            tracing::warn!("failed to record captured asset schema: {e:#}");
        }
    }
    Ok(Json(()))
}

#[derive(Deserialize)]
struct ListAssetsQuery {
    #[serde(default = "default_per_page")]
    per_page: i64,
    cursor_created_at: Option<chrono::DateTime<chrono::Utc>>,
    cursor_id: Option<i64>,
    pub asset_path: Option<String>,
    pub usage_path: Option<String>,
    pub asset_kinds: Option<String>,
    // Exact path match filter
    pub path: Option<String>,
    // Filter by matching a subset of the columns using base64 encoded json subset
    pub columns: Option<String>,
    pub broad_filter: Option<String>,
}

fn default_per_page() -> i64 {
    50
}

#[derive(Serialize)]
struct ListAssetsResponse {
    assets: Vec<Value>,
    next_cursor: Option<AssetCursor>,
}

#[derive(Serialize)]
struct AssetCursor {
    created_at: chrono::DateTime<chrono::Utc>,
    id: i64,
}

#[derive(Debug)]
struct AssetRow {
    result: Value,
    max_created_at: chrono::DateTime<chrono::Utc>,
    max_id: i64,
}

async fn list_assets(
    authed: ApiAuthed,
    Path(w_id): Path<String>,
    Extension(user_db): Extension<UserDB>,
    Query(query): Query<ListAssetsQuery>,
) -> JsonResult<ListAssetsResponse> {
    let per_page = query.per_page.min(1000).max(1);
    let limit = per_page + 1;

    let mut tx = user_db.begin(&authed).await?;

    // Build dynamic filter SQL
    let mut asset_summary_filters = vec![
        "asset.workspace_id = $1".to_string(),
        "(asset.usage_kind <> 'flow' OR asset.usage_path = ANY(SELECT path FROM flow WHERE workspace_id = $1))".to_string(),
        "(asset.usage_kind <> 'script' OR asset.usage_path = ANY(SELECT path FROM script WHERE workspace_id = $1))".to_string(),
    ];

    let mut param_count = 2; // $1 = workspace_id, $2 = limit

    // Asset path filter (ILIKE pattern match)
    if query.asset_path.is_some() {
        param_count += 1;
        asset_summary_filters.push(format!("asset.path ILIKE ${}", param_count));
    }

    // Exact path filter
    if query.path.is_some() {
        param_count += 1;
        asset_summary_filters.push(format!("asset.path = ${}", param_count));
    }

    // Columns filter (check if JSONB has all specified keys)
    if query.columns.is_some() {
        param_count += 1;
        asset_summary_filters.push(format!("asset.columns ?& ${}", param_count));
    }

    // Usage path filter - for jobs, also check runnable_path
    let needs_job_join_in_cte = query.usage_path.is_some();
    if query.usage_path.is_some() {
        param_count += 1;
        asset_summary_filters.push(format!(
            "(asset.usage_path ILIKE ${} OR (asset.usage_kind = 'job' AND job_cte.runnable_path ILIKE ${}))",
            param_count, param_count
        ));
    }

    // Asset kinds filter
    let asset_kinds = query
        .asset_kinds
        .map(|kinds_str| {
            kinds_str
                .split(',')
                .map(|kind_str| {
                    serde_json::from_str::<AssetKind>(&format!("\"{}\"", kind_str.trim()))
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()
        .map_err(|_| {
            windmill_common::error::Error::BadRequest("Invalid asset_kinds parameter".to_string())
        })?;
    let has_asset_kinds = asset_kinds.as_ref().map(|v| !v.is_empty()).unwrap_or(false);
    if has_asset_kinds {
        param_count += 1;
        asset_summary_filters.push(format!("asset.kind = ANY(${})", param_count));
    }

    if query.broad_filter.is_some() {
        param_count += 1;
        asset_summary_filters.push(format!(
            "(asset.path ILIKE ${p} OR asset.kind::text ILIKE ${p})",
            p = param_count
        ));
    }

    let asset_summary_where = asset_summary_filters.join(" AND ");

    // Build cursor condition
    let cursor_having = if query.cursor_created_at.is_some() && query.cursor_id.is_some() {
        param_count += 2;
        format!("HAVING MAX(asset.created_at) < ${} OR (MAX(asset.created_at) = ${} AND MAX(asset.id) < ${})",
                param_count - 1, param_count - 1, param_count)
    } else {
        String::new()
    };

    // Build FROM clause for CTE with optional job join
    let cte_from = if needs_job_join_in_cte {
        format!(
            r#"FROM asset
            LEFT JOIN v2_job job_cte ON asset.usage_kind = 'job'
              AND job_cte.id = CASE WHEN asset.usage_kind = 'job' THEN asset.usage_path::uuid END
              AND job_cte.workspace_id = $1"#
        )
    } else {
        "FROM asset".to_string()
    };

    let sql = format!(
        r#"
        WITH asset_summary AS (
            SELECT
                asset.path,
                asset.kind,
                MAX(asset.created_at) as max_created_at,
                MAX(asset.id) as max_id
            {}
            WHERE {}
            GROUP BY asset.path, asset.kind
            {}
            ORDER BY max_created_at DESC, max_id DESC
            LIMIT $2
        )
        SELECT
            jsonb_strip_nulls(jsonb_build_object(
                'path', asset.path,
                'kind', asset.kind,
                'usages', ARRAY_AGG(
                    jsonb_strip_nulls(jsonb_build_object(
                        'path', asset.usage_path,
                        'kind', asset.usage_kind,
                        'access_type', asset.usage_access_type,
                        'columns', asset.columns,
                        'created_at', asset.created_at,
                        'metadata', (CASE
                            WHEN asset.usage_kind = 'job' THEN
                                jsonb_build_object('runnable_path', job.runnable_path, 'job_kind', job.kind)
                            ELSE
                                NULL
                            END
                        )
                    ))
                    ORDER BY asset.created_at DESC
                ),
                'metadata', (CASE
                  WHEN asset.kind = 'resource' THEN
                    jsonb_build_object('resource_type', resource.resource_type)
                  ELSE
                    NULL
                  END
                )
            )) as result,
            asset_summary.max_created_at,
            asset_summary.max_id
        FROM asset
        INNER JOIN asset_summary ON asset.path = asset_summary.path AND asset.kind = asset_summary.kind
        LEFT JOIN resource ON asset.kind = 'resource'
          AND (
            -- Extract base path before '?' for ?table= syntax
            CASE
              WHEN asset.path LIKE '%?%' THEN split_part(asset.path, '?', 1)
              ELSE asset.path
            END
          ) = resource.path
          AND resource.workspace_id = $1
        LEFT JOIN v2_job job ON asset.usage_kind = 'job'
          AND job.id = CASE WHEN asset.usage_kind = 'job' THEN asset.usage_path::uuid END
          AND job.workspace_id = $1
        WHERE asset.workspace_id = $1
          AND (asset.kind <> 'resource' OR resource.path IS NOT NULL)
          AND (asset.usage_kind <> 'job' OR job.id IS NOT NULL)
        GROUP BY asset.path, asset.kind, resource.resource_type, asset_summary.max_created_at, asset_summary.max_id
        ORDER BY asset_summary.max_created_at DESC, asset_summary.max_id DESC
        "#,
        cte_from, asset_summary_where, cursor_having
    );

    // Build query with dynamic parameters
    let mut query_builder = sqlx::query(&sql).bind(&w_id).bind(limit);

    if let Some(ref asset_path) = query.asset_path {
        query_builder = query_builder.bind(format!("%{}%", escape_ilike_pattern(asset_path)));
    }

    if let Some(ref path) = query.path {
        query_builder = query_builder.bind(path);
    }

    if let Some(ref columns) = query.columns {
        // Columns is a comma-separated string, split into array for ?& operator
        let columns_array: Vec<String> = columns
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        query_builder = query_builder.bind(columns_array);
    }

    if let Some(ref usage_path) = query.usage_path {
        query_builder = query_builder.bind(format!("%{}%", escape_ilike_pattern(usage_path)));
    }

    if let Some(ref asset_kinds) = asset_kinds {
        if !asset_kinds.is_empty() {
            query_builder = query_builder.bind(asset_kinds);
        }
    }

    if let Some(ref broad_filter) = query.broad_filter {
        query_builder = query_builder.bind(format!("%{}%", escape_ilike_pattern(broad_filter)));
    }

    if let (Some(cursor_created_at), Some(cursor_id)) = (query.cursor_created_at, query.cursor_id) {
        query_builder = query_builder.bind(cursor_created_at).bind(cursor_id);
    }

    let db_rows = query_builder.fetch_all(&mut *tx).await?;

    let rows: Vec<AssetRow> = db_rows
        .iter()
        .map(|row| AssetRow {
            result: row.try_get("result").unwrap_or(Value::Null),
            max_created_at: row.try_get("max_created_at").unwrap(),
            max_id: row.try_get("max_id").unwrap(),
        })
        .collect();

    let assets: Vec<Value> = rows
        .iter()
        .take(per_page as usize)
        .map(|r| r.result.clone())
        .collect();

    let next_cursor = if rows.len() as i64 > per_page {
        let last = &rows[per_page as usize - 1];
        Some(AssetCursor { created_at: last.max_created_at, id: last.max_id })
    } else {
        None
    };

    Ok(Json(ListAssetsResponse { assets, next_cursor }))
}

#[derive(Deserialize)]
pub struct ListAssetsByUsagesBodyInner {
    kind: AssetUsageKind,
    path: String,
}

#[derive(Deserialize)]
struct ListAssetsByUsagesBody {
    usages: Vec<ListAssetsByUsagesBodyInner>,
}

async fn list_assets_by_usages(
    authed: ApiAuthed,
    Path(w_id): Path<String>,
    Extension(user_db): Extension<UserDB>,
    Json(body): Json<ListAssetsByUsagesBody>,
) -> JsonResult<Vec<Vec<Value>>> {
    let mut tx = user_db.begin(&authed).await?;
    let mut assets_vec = vec![];
    for usage in body.usages {
        let assets = sqlx::query_scalar!(
            r#"SELECT
                jsonb_strip_nulls(jsonb_build_object(
                    'path', path,
                    'kind', kind,
                    'access_type', usage_access_type,
                    'columns', columns
                )) as "list!: _"
            FROM asset
            WHERE workspace_id = $1 AND usage_path = $2 AND usage_kind = $3
            ORDER BY path, kind"#,
            w_id,
            usage.path,
            usage.kind as AssetUsageKind
        )
        .fetch_all(&mut *tx)
        .await?;
        assets_vec.push(assets);
    }
    Ok(Json(assets_vec))
}

async fn list_favorites(
    authed: ApiAuthed,
    Path(w_id): Path<String>,
    Extension(user_db): Extension<UserDB>,
) -> JsonResult<Vec<Value>> {
    let mut tx = user_db.begin(&authed).await?;

    let favorites = sqlx::query_scalar!(
        r#"SELECT
            jsonb_strip_nulls(jsonb_build_object(
                'path', favorite.path
            )) as "favorite_asset!: _"
        FROM favorite
        WHERE favorite.workspace_id = $1
          AND favorite.usr = $2
          AND favorite_kind = 'asset'
        "#,
        &w_id,
        &authed.username
    )
    .fetch_all(&mut *tx)
    .await?;

    Ok(Json(favorites))
}

// ------------------------------------------------------------------
// GET /w/:workspace/assets/graph
// ------------------------------------------------------------------
// Workspace-wide asset ↔ runnable graph. One row per unique
// (asset_kind, asset_path, usage_kind, usage_path, access_type) — the
// frontend aggregates into nodes and edges.

#[derive(Deserialize)]
pub struct GraphQuery {
    pub asset_kinds: Option<String>,
    pub folder: Option<String>,
    /// Render a dbt project as a given deployed version rather than as it is
    /// now. A run page passes the version its job ran, so an old run shows the
    /// models, SQL and `ref()` lineage of that deploy instead of today's.
    /// Absent — the usual case — means the newest live version per path.
    /// Hex, like every other script-hash parameter — `ScriptHash` deserializes
    /// it, so the run page can pass `job.script_hash` verbatim.
    pub dbt_script_hash: Option<windmill_common::scripts::ScriptHash>,
}

#[derive(Serialize, Debug)]
struct GraphAssetNode {
    kind: AssetKind,
    path: String,
    // Fork workspaces only: 'fork' when the fork has materialized this ducklake asset itself,
    // 'deferred' when reads fall back to the parent workspace's current table (defer view).
    // Absent outside forks, for non-ducklake assets, and for assets never materialized
    // anywhere. Lockstep with TS `AssetGraphAssetNode.fork_materialization`.
    #[serde(skip_serializing_if = "Option::is_none")]
    fork_materialization: Option<String>,
    // The base dimension this asset is the SCD2 `<dim>_current` companion view
    // of — set only on a `ducklake://…/<dim>_current` node whose producer
    // declares `// materialize … history` on `<dim>`. The producer edge already
    // links it to that script (both writes are registered at deploy); this lets
    // the canvas render it as a derived "current view" of the base rather than an
    // unrelated table. Absent for every other asset. Lockstep with TS
    // `AssetGraphAssetNode.derived_from`.
    #[serde(skip_serializing_if = "Option::is_none")]
    derived_from: Option<String>,
    // Set on a `dbt://` asset produced (or, for a source, consumed) by a dbt
    // script: which dbt node it is and what dbt says about it. A dbt project is
    // one runnable node with many model assets, so per-model metadata belongs
    // here rather than on the script (docs/dbt-runtime.md, decision 15).
    // Lockstep with TS `AssetGraphAssetNode.dbt`.
    #[serde(skip_serializing_if = "Option::is_none")]
    dbt: Option<DbtAssetProvenance>,
}

#[derive(Serialize, Debug, Clone)]
struct DbtAssetProvenance {
    unique_id: String,
    // `model` | `snapshot` | `seed` | `source` — a source is read, not written,
    // and the canvas distinguishes them.
    resource_type: String,
    // dbt's own word (`table`, `view`, `incremental`, `snapshot`), kept because
    // `view` and `ephemeral` have no Windmill write-strategy analogue.
    #[serde(skip_serializing_if = "Option::is_none")]
    materialized: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    materialize_strategy: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    data_tests: Vec<DbtDataTest>,
    /// Declared column metadata (name -> description). NOT column lineage —
    /// `manifest.json` carries none (docs/dbt-runtime.md, decision 14).
    #[serde(skip_serializing_if = "Option::is_none")]
    columns: Option<serde_json::Value>,
    /// A source's declared freshness policy, for the staleness chip.
    #[serde(skip_serializing_if = "Option::is_none")]
    freshness: Option<serde_json::Value>,
    /// The model's SQL as written. Read-only in Windmill — the file lives in
    /// the producing script's bundle, and this is the copy captured when the
    /// graph being read was parsed: at deploy, or by a refresh from the
    /// editor's buffer.
    #[serde(skip_serializing_if = "Option::is_none")]
    raw_code: Option<String>,
    /// Its path inside the repo, e.g. `models/staging/stg_orders.sql`.
    #[serde(skip_serializing_if = "Option::is_none")]
    original_file_path: Option<String>,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
struct DbtDataTest {
    // `unique` | `not_null` | `accepted_values` | `relationships` — the four
    // generic tests, one-for-one with the `// data_test` kinds — or a package
    // test's namespaced name (`dbt_utils.accepted_range`).
    kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    column: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    args: Option<serde_json::Value>,
    // Lowercased. dbt's own severity decides whether a failure fails the run,
    // so the canvas shows it rather than assuming every test is blocking.
    #[serde(skip_serializing_if = "Option::is_none")]
    severity: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
struct DbtRunnableProvenance {
    model_count: usize,
}

#[derive(Serialize, Debug)]
struct GraphRunnableNode {
    path: String,
    usage_kind: AssetUsageKind,
    // True iff the script was deployed with `// pipeline` — drives the
    // pipeline-member visual state on the frontend.
    #[serde(skip_serializing_if = "std::ops::Not::not", default)]
    in_pipeline: bool,
    // Annotation badges parsed from the deployed script body, so the canvas
    // shows partition/freshness/tag/retry/data-test chips on *deployed* nodes
    // (not only on live-edited drafts, which the frontend parses itself). Kept
    // in lockstep with the TS `AssetGraphRunnableNode` fields the node renders.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    partition_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    freshness: Option<String>,
    // Completion time of the most recently started successful run of this
    // pipeline member. The canvas checks it against the `// freshness` window
    // to color the badge fresh/stale. The badge itself is passive; on EE the
    // freshness watchdog (windmill-queue) separately re-runs stale
    // unpartitioned producers. Absent when no successful run is visible to
    // the caller (job RLS applies).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    last_success_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    tag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    retry: Option<windmill_common::assets::RetrySpec>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    data_tests: Vec<windmill_common::assets::DataTest>,
    // `// column <out> <- <src>.<col>` declared column-level lineage, surfaced
    // so the canvas can draw the column-lineage view on deployed nodes (not
    // only live drafts). Lockstep with TS `AssetGraphRunnableNode.column_lineage`.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    column_lineage: Vec<windmill_common::assets::ColumnLineage>,
    // `// materialize <asset>` target — the asset this script's `column_lineage`
    // describes. Lets the column-graph anchor lineage to the exact output asset
    // instead of guessing a ducklake write-edge (a multi-output script writes
    // several). Absent for scripts with no `// materialize` annotation.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    materialize_target: Option<MaterializeTargetNode>,
    // Managed `// materialize` write strategy (`replace` | `append` | `merge`),
    // absent for non-materializing or `manual` scripts. Surfaced so the asset
    // panel can tell whether the captured schema can evolve: only whole-table
    // `replace` (CREATE OR REPLACE) can change columns run-to-run; `append` /
    // `merge` / any partitioned write INSERTs into a fixed-schema table.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    materialize_strategy: Option<String>,
    // `on_schema_change=ignore` on the managed materialize — the producer's
    // opt-out from downstream schema-contract warnings. Threaded to the editor
    // so its client-side contract mirror suppresses the same warnings the
    // server check does. Only serialized when set to `ignore` (default `warn`
    // is absent). Lockstep with TS `AssetGraphRunnableNode.materialize_on_schema_change`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    materialize_on_schema_change: Option<String>,
    // Macros this script provides to the workspace registry (deployed
    // `// macros` library). Drives the library node state + details-pane
    // signature list. Lockstep with TS `AssetGraphRunnableNode.macros`.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    macros: Vec<MacroInfo>,
    // Set on a `ScriptLang::Dbt` script: it owns a whole dbt project, so the
    // node says how many models it materializes rather than pretending to be a
    // single-output script. Lockstep with TS `AssetGraphRunnableNode.dbt`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    dbt: Option<DbtRunnableProvenance>,
}

// One macro of a `// macros` library, as surfaced on its graph node.
#[derive(Serialize, Debug, Clone)]
struct MacroInfo {
    name: String,
    // Verbatim parameter list, for the `name(params)` signature display.
    params: String,
    is_table: bool,
}

// A macro-library → consumer edge: the consumer calls `macro_names` of
// `lib_path`'s macros (deploy-recorded detection), or pulls in the whole
// library via `// use` (`via_use`, in which case `macro_names` lists all of
// the library's macros).
#[derive(Serialize, Debug)]
struct MacroEdge {
    lib_path: String,
    consumer_path: String,
    macro_names: Vec<String>,
    via_use: bool,
}

// The output asset a producer's column lineage belongs to (the `// materialize`
// target). Kept minimal — the column graph only needs (kind, path) to anchor.
#[derive(Serialize, Debug)]
struct MaterializeTargetNode {
    kind: windmill_common::assets::AssetKind,
    path: String,
}

// The partition's kind word for the node badge (the full PartitionSpec carries
// tz/format/start, which the badge doesn't need).
fn partition_kind_word(kind: &windmill_common::assets::PartitionKind) -> &'static str {
    use windmill_common::assets::PartitionKind::*;
    match kind {
        Daily => "daily",
        Hourly => "hourly",
        Weekly => "weekly",
        Monthly => "monthly",
        Dynamic { .. } => "dynamic",
    }
}

// Lineage edge from parsed r/w usages. One per (runnable, asset, access_type)
// tuple. Informational — not the DAG execution edges.
#[derive(Serialize, Debug)]
struct GraphEdge {
    runnable_path: String,
    runnable_kind: AssetUsageKind,
    asset_kind: AssetKind,
    asset_path: String,
    access_type: Option<String>,
}

// Declared `// on <trigger>` trigger edge — the actual execution DAG.
// Asset edges come from `script_trigger`; the eight native variants
// (Schedule/Email/Kafka/…/Gcp) come from the per-kind trigger tables joined
// on `script_path`. Each native variant carries just the trigger row's path;
// the config (cron, broker, topic, auth, …) lives in its own UI.
//
// `webhook` is parsed as an annotation marker but has no dedicated trigger
// table — every script gets an implicit webhook endpoint — so no variant
// here. The frontend renders the marker from the source annotations alone.
#[derive(Serialize, Debug)]
#[serde(tag = "trigger_kind", rename_all = "lowercase")]
enum TriggerEdge {
    Asset {
        asset_kind: AssetKind,
        asset_path: String,
        runnable_kind: AssetUsageKind,
        runnable_path: String,
    },
    Schedule {
        path: String,
        runnable_kind: AssetUsageKind,
        runnable_path: String,
    },
    Email {
        path: String,
        runnable_kind: AssetUsageKind,
        runnable_path: String,
    },
    Kafka {
        path: String,
        runnable_kind: AssetUsageKind,
        runnable_path: String,
    },
    Mqtt {
        path: String,
        runnable_kind: AssetUsageKind,
        runnable_path: String,
    },
    Amqp {
        path: String,
        runnable_kind: AssetUsageKind,
        runnable_path: String,
    },
    Nats {
        path: String,
        runnable_kind: AssetUsageKind,
        runnable_path: String,
    },
    Postgres {
        path: String,
        runnable_kind: AssetUsageKind,
        runnable_path: String,
    },
    Sqs {
        path: String,
        runnable_kind: AssetUsageKind,
        runnable_path: String,
    },
    Gcp {
        path: String,
        runnable_kind: AssetUsageKind,
        runnable_path: String,
    },
}

// Ordering-only "must-run-after" edge: `runnable_path`'s data test reads
// `asset` (a `// data_test relationships` ref, or a custom test whose body
// reads a known pipeline asset), so the asset's in-pipeline producer must
// materialize before `runnable_path` runs. NOT a data-consumption edge — the
// tested script doesn't ingest the asset's rows, it only needs the table to
// exist at test time. Rendered dashed (like macro edges) and fed into the
// cascade topo-sort so a cold cascade orders the referenced dimension first.
// Only emitted when the referenced asset has a producer in the graph; an
// external table (no producer) adds no edge — the runtime error stands.
#[derive(Serialize, Debug)]
struct TestEdge {
    producer_kind: AssetUsageKind,
    producer_path: String,
    runnable_kind: AssetUsageKind,
    runnable_path: String,
    asset_kind: AssetKind,
    asset_path: String,
}

#[derive(Serialize, Debug)]
pub struct AssetGraphResponse {
    assets: Vec<GraphAssetNode>,
    runnables: Vec<GraphRunnableNode>,
    edges: Vec<GraphEdge>,
    triggers: Vec<TriggerEdge>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    macro_edges: Vec<MacroEdge>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    test_edges: Vec<TestEdge>,
    /// `ref()` lineage BETWEEN two dbt models. Without it every model hangs off
    /// the one dbt runnable, which reads as a flat source-to-model fan-out
    /// instead of the project's actual shape.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    dbt_edges: Vec<DbtLineageEdge>,
    /// The job whose snapshot the dbt half was resolved from, when one was
    /// asked for and found. A run page polls the graph while its job runs
    /// because a dynamic descriptor's snapshot is written mid-run, and this is
    /// what tells it to stop: without it the page cannot distinguish "the
    /// snapshot has not been written yet" from "this run has none", and either
    /// polls forever or gives up before the ingest.
    #[serde(skip_serializing_if = "Option::is_none")]
    dbt_snapshot_job: Option<uuid::Uuid>,
    /// When the dbt half was parsed, for a graph pinned to a job. What the
    /// editor labels its provenance with: a buffer refresh and the deployed
    /// version's graph are drawn identically, so without saying which one is on
    /// screen the ambiguity the explicit refresh removes just moves into the
    /// editor.
    #[serde(skip_serializing_if = "Option::is_none")]
    dbt_graph_ingested_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// One `ref()`/`source()` edge, in the terms the canvas draws: the two
/// relations, not dbt's node ids.
#[derive(Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct DbtLineageEdge {
    from_asset_path: String,
    to_asset_path: String,
}

async fn asset_graph(
    authed: ApiAuthed,
    Path(w_id): Path<String>,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<windmill_common::DB>,
    Query(q): Query<GraphQuery>,
) -> JsonResult<AssetGraphResponse> {
    // `None`: pinning the graph to one run is job-scoped and this endpoint is
    // authorized as `assets:read`. See `asset_graph_for`.
    asset_graph_for(&authed, &w_id, user_db, db, q, None).await
}

/// A run the caller is already authorized to read, and the deployed version it
/// ran. Path and hash come from the job row rather than the query, so a graph
/// cannot be pointed at one project's version while claiming another's run.
pub struct PinnedRun {
    pub job_id: uuid::Uuid,
    pub script_path: String,
    /// `None` for a job that names no deployed version: a `parse` of the
    /// EDITOR's buffer, whose graph belongs to that job alone. Such a job pins
    /// only because it stored one — every other preview answers with the
    /// workspace graph, as before.
    pub script_hash: Option<i64>,
}

/// The asset graph, optionally as one run saw it.
///
/// AUTHORIZES NOTHING BY ITSELF. Every caller owes it two checks, because the
/// answer is workspace asset data reached through a route whose own URL segment
/// decides the scope domain: `assets:read` always, and `require_job_read_access`
/// for the job when passing `Some(pinned)` — which is then taken as already
/// done, since the pinned path and hash come from that job's row.
pub async fn asset_graph_for(
    authed: &ApiAuthed,
    w_id: &str,
    user_db: UserDB,
    db: windmill_common::DB,
    q: GraphQuery,
    pinned: Option<PinnedRun>,
) -> JsonResult<AssetGraphResponse> {
    let dbt_job_id = pinned.as_ref().map(|p| p.job_id);
    // The version is the job's own, not the caller's `dbt_script_hash` — and
    // that holds when the job names NONE, so the parameter cannot supply a
    // version for a graph that has no business claiming one.
    let dbt_script_hash = match pinned.as_ref() {
        Some(p) => p.script_hash,
        None => q.dbt_script_hash.map(|h| h.0),
    };
    // Set only for a pinned run, where it lets `live` resolve without reading
    // `script`: a share-link viewer is entitled to the run but usually has no
    // grant on the script, and RLS there would empty the graph. `raw_code` has
    // its own `script` check, so the body stays hidden either way.
    let pinned_path = pinned.as_ref().map(|p| p.script_path.as_str());
    let w_id = w_id.to_string();
    let mut tx = user_db.begin(authed).await?;
    // Built once: a scoped token's `scripts:read` paths decide whether a dbt
    // node's SQL body may be returned, independently of the `assets:read` scope
    // that authorizes this endpoint.
    let dbt_source_scope = build_scope_path_predicate(authed, "scripts", "read");

    // REFUSED, not dropped: a caller that asks for a kind this server does not
    // know has asked for something, and answering with the other kinds returns a
    // graph missing exactly what they came for — silently. That is how a rename
    // of one kind emptied three callers' graphs with nothing logged.
    let kind_filter: Option<Vec<AssetKind>> = q
        .asset_kinds
        .as_ref()
        .map(|s| {
            s.split(',')
                .map(|k| {
                    serde_json::from_value::<AssetKind>(Value::String(k.trim().into())).map_err(
                        |_| {
                            windmill_common::error::Error::BadRequest(format!(
                                "`{}` is not an asset kind",
                                k.trim()
                            ))
                        },
                    )
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?;
    let kind_filter_ref = kind_filter.as_deref();

    let folder_filter = q.folder.as_deref().map(|f| format!("f/{}/%", f));

    // One row per (asset_kind, asset_path, usage_kind, usage_path, access_type).
    // The `usage_kind IN ('script','flow')` clause excludes `job`-kind usage rows
    // (runtime-detected, ephemeral) so the graph stays stable.
    let rows = sqlx::query!(
        r#"
        SELECT
            asset.kind        AS "asset_kind!: AssetKind",
            asset.path        AS "asset_path!",
            asset.usage_kind  AS "usage_kind!: AssetUsageKind",
            asset.usage_path  AS "usage_path!",
            asset.usage_access_type::text AS "access_type"
        FROM asset
        WHERE asset.workspace_id = $1
          AND asset.usage_kind IN ('script', 'flow')
          AND ($2::asset_kind[] IS NULL OR asset.kind = ANY($2))
          AND ($3::text IS NULL OR asset.usage_path LIKE $3)
        GROUP BY asset.kind, asset.path, asset.usage_kind, asset.usage_path, asset.usage_access_type
        "#,
        &w_id,
        kind_filter_ref as Option<&[AssetKind]>,
        folder_filter.as_deref(),
    )
    .fetch_all(&mut *tx)
    .await?;

    // Pipeline asset trigger edges, fetched separately so we can widen the
    // runnable_set for trigger-only endpoints (e.g. an asset trigger whose
    // asset has no usage in the pipeline yet). Native trigger kinds
    // (schedule, kafka, mqtt, …) are *not* in `script_trigger` — they're
    // discovered below by querying each native trigger table directly.
    let trigger_rows = sqlx::query!(
        r#"
        SELECT
            runnable_kind AS "runnable_kind!: AssetUsageKind",
            runnable_path AS "runnable_path!",
            trigger_kind::text AS "trigger_kind!",
            trigger_ref   AS "trigger_ref!"
        FROM script_trigger
        WHERE workspace_id = $1
          AND trigger_kind = 'asset'
          AND ($2::text IS NULL OR runnable_path LIKE $2)
        "#,
        &w_id,
        folder_filter.as_deref(),
    )
    .fetch_all(&mut *tx)
    .await?;

    // Native triggers in scope. Each native trigger table stores its
    // single-destination `script_path` directly, so we resolve attachment by
    // joining on that field rather than via `script_trigger`. UNION ALL keeps
    // it a single round trip; the `kind` column drives the TriggerEdge ctor
    // below. `schedule` lives in the `schedule` table, which has its own
    // shape (no workspace_id-only filter — it shares `is_flow` like the
    // others), but the columns we need line up.
    let native_trigger_rows = sqlx::query!(
        r#"
        SELECT kind, path, script_path, is_flow FROM (
            SELECT 'schedule' AS kind, path, script_path, is_flow FROM schedule
                WHERE workspace_id = $1
                  AND script_path IS NOT NULL
            UNION ALL
            SELECT 'email', path, script_path, is_flow FROM email_trigger
                WHERE workspace_id = $1
            UNION ALL
            SELECT 'kafka', path, script_path, is_flow FROM kafka_trigger
                WHERE workspace_id = $1
            UNION ALL
            SELECT 'mqtt', path, script_path, is_flow FROM mqtt_trigger
                WHERE workspace_id = $1
            UNION ALL
            SELECT 'amqp', path, script_path, is_flow FROM amqp_trigger
                WHERE workspace_id = $1
            UNION ALL
            SELECT 'nats', path, script_path, is_flow FROM nats_trigger
                WHERE workspace_id = $1
            UNION ALL
            SELECT 'postgres', path, script_path, is_flow FROM postgres_trigger
                WHERE workspace_id = $1
            UNION ALL
            SELECT 'sqs', path, script_path, is_flow FROM sqs_trigger
                WHERE workspace_id = $1
            UNION ALL
            SELECT 'gcp', path, script_path, is_flow FROM gcp_trigger
                WHERE workspace_id = $1
        ) t
        WHERE ($2::text IS NULL OR script_path LIKE $2)
        "#,
        &w_id,
        folder_filter.as_deref(),
    )
    .fetch_all(&mut *tx)
    .await?;

    // Which scripts in scope are pipeline members (have `// pipeline`).
    // Pipeline members + their latest deployed body, so the graph can surface
    // annotation badges (partition/freshness/tag/retry/data_test) on deployed
    // nodes. `DISTINCT ON (path) … ORDER BY created_at DESC` picks the newest
    // non-archived version per path (a redeploy archives the prior one, but be
    // defensive against transient overlaps).
    let pipeline_member_paths = sqlx::query!(
        r#"
        SELECT DISTINCT ON (path) path AS "path!", content AS "content!",
               language AS "language!: windmill_common::scripts::ScriptLang"
        FROM script
        WHERE workspace_id = $1
          AND auto_kind = 'pipeline'
          AND archived = false
          AND deleted = false
          AND ($2::text IS NULL OR path LIKE $2)
        ORDER BY path, created_at DESC
        "#,
        &w_id,
        folder_filter.as_deref(),
    )
    .fetch_all(&mut *tx)
    .await?;

    // Newest successful completed run per pipeline member, for the passive
    // freshness status on the canvas. Correlated per-path lookup walks
    // ix_job_root_job_index_by_path_2 newest-first until the first success,
    // so cost is bounded by the member count, not run history. "Newest" is
    // by created_at (the index order), not completed_at: with overlapping
    // runs of one path this can pick an earlier completion, erring toward
    // stale — never toward false-fresh. Inside the user tx so job-visibility
    // RLS applies — a caller who can't see the runs gets no timestamp rather
    // than leaked completion times.
    let member_paths: Vec<String> = pipeline_member_paths
        .iter()
        .map(|r| r.path.clone())
        .collect();
    let last_success_rows = sqlx::query!(
        r#"
        SELECT p.path AS "path!",
               (SELECT c.completed_at
                  FROM v2_job j
                  JOIN v2_job_completed c ON c.id = j.id
                 WHERE j.workspace_id = $1
                   AND j.runnable_path = p.path
                   AND j.parent_job IS NULL
                   -- No 'singlestepflow': flows may share a script's path, and
                   -- a same-path flow run must not read as the script being
                   -- fresh (false-fresh). Script retries land as native
                   -- 'script' jobs; only the rare flow-wrapper fallback is
                   -- missed, which errs stale. Kept in lockstep with the
                   -- freshness watchdog's queries (freshness_watchdog_ee).
                   AND j.kind IN ('script', 'preview')
                   AND c.status = 'success'
                 ORDER BY j.created_at DESC
                 LIMIT 1) AS last_success_at
          FROM unnest($2::text[]) AS p(path)
        "#,
        &w_id,
        &member_paths,
    )
    .fetch_all(&mut *tx)
    .await?;

    // Existing scripts / flows in the workspace. Used to filter out
    // orphan trigger rows whose `script_path` no longer resolves — those
    // would otherwise be added to `runnable_set` below and surface as
    // phantom "deployed" runnables on the canvas (matching what the user
    // can deploy a new trigger against: nothing).
    let existing_script_paths = sqlx::query_scalar!(
        r#"SELECT path AS "path!" FROM script
           WHERE workspace_id = $1
             AND archived = false
             AND deleted = false"#,
        &w_id,
    )
    .fetch_all(&mut *tx)
    .await?;
    let existing_flow_paths = sqlx::query_scalar!(
        r#"SELECT path AS "path!" FROM flow WHERE workspace_id = $1 AND archived = false"#,
        &w_id,
    )
    .fetch_all(&mut *tx)
    .await?;

    // Workspace macro registry + deploy-recorded call edges. Definitions are
    // fetched unfiltered so an out-of-folder library still appears as the
    // provider endpoint of in-scope consumers' edges; consumers honor the
    // folder filter like every other runnable query.
    let macro_def_rows = sqlx::query!(
        r#"SELECT name AS "name!", provider_path AS "provider_path!",
                  params AS "params!", is_table_macro AS "is_table_macro!"
           FROM macro_definition
           WHERE workspace_id = $1
           ORDER BY provider_path, name"#,
        &w_id,
    )
    .fetch_all(&mut *tx)
    .await?;
    let macro_usage_rows = sqlx::query!(
        r#"SELECT consumer_path AS "consumer_path!", macro_name AS "macro_name!"
           FROM macro_usage
           WHERE workspace_id = $1
             AND ($2::text IS NULL OR consumer_path LIKE $2)"#,
        &w_id,
        folder_filter.as_deref(),
    )
    .fetch_all(&mut *tx)
    .await?;

    // dbt provenance. A dbt script is one runnable node whose models are many
    // `dbt://` asset nodes (decision 15), so the per-model metadata has to
    // hang off the assets, not off the script. It describes the RELATION, not a
    // producer: several dbt scripts (different selections of one project) can
    // materialize the same model, and the producer edges already name them.
    //
    // Scoped to the relations this graph actually renders, so a workspace with
    // many dbt projects does not pay for all of them on every request. Not
    // scoped by the script's folder: an out-of-folder dbt script still has to
    // explain a model an in-scope consumer reads, same as macro definitions.
    // Tests come along via `attached_node` — they carry no `asset_path` of
    // their own.
    let dbt_rows = sqlx::query!(
        r#"WITH live AS (
             -- The graph is stored per deployed VERSION, so this endpoint — which
             -- describes the project as it is now — takes the newest live one per
             -- path. Resolved once here rather than per row: a correlated lookup
             -- on every node is what makes these queries fall over.
             SELECT * FROM (
               SELECT DISTINCT ON (s.path) s.path, s.hash
                 FROM script s
                WHERE $5::text IS NULL AND s.workspace_id = $1 AND s.language = 'dbt'
                  AND ($3::bigint IS NULL OR s.hash = $3)
                  -- A pinned version may be archived by now; that is precisely
                  -- the case a historical run needs, so the liveness filter
                  -- applies only when picking the current one.
                  AND ($3::bigint IS NOT NULL OR (s.deleted = false AND s.archived = false))
                ORDER BY s.path, s.created_at DESC
             ) cur
             UNION ALL
             -- A pinned run names its own version, so `script` is not consulted:
             -- under RLS it would answer for the CALLER's grants on the project,
             -- emptying the graph for a share-link viewer who is entitled to the
             -- run but not the script. A NULL hash here is a job that names no
             -- version at all — an editor buffer parse — and matches only the
             -- version-less rows that parse stored.
             SELECT $5::text, $3::bigint WHERE $5::text IS NOT NULL
           ),
           -- The run's own snapshot when it left one, the version's graph
           -- otherwise. A static descriptor never snapshots, so all of its runs
           -- fall through to the same rows. Existence comes from the marker, not
           -- from a node row: a dynamic run that disabled every model has a
           -- snapshot whose graph is legitimately empty.
           chosen AS (
             -- No visibility check on the job here: reaching this with a job at
             -- all means the caller passed `require_job_read_access` for it, and
             -- re-deciding it under plain RLS can only DISAGREE with that answer
             -- — silently, by falling back to the deployed graph rather than
             -- erroring. A share-link viewer is entitled to the run and would be
             -- shown a different run's model set. See `asset_graph_for`.
             SELECT CASE WHEN $4::uuid IS NOT NULL AND EXISTS (
                      SELECT 1 FROM dbt_graph_snapshot g
                       WHERE g.workspace_id = $1 AND g.job_id = $4)
                    THEN $4::uuid
                    ELSE '00000000-0000-0000-0000-000000000000'::uuid END AS job_id
           ),
           scoped AS (
             SELECT n.script_path, n.unique_id FROM dbt_node n
              -- `=` still, with the NULL-to-NULL case spelled out and gated on
              -- the pin: a version-less row's hash is NULL on both sides, which
              -- `=` never matches, but `IS NOT DISTINCT FROM` would cost the
              -- equality its index bound on the UNPINNED workspace graph — the
              -- hot path. Unpinned, `$5` is NULL and the second arm folds away.
              JOIN live l ON l.path = n.script_path
                         AND (n.script_hash = l.hash
                              OR ($5::text IS NOT NULL AND l.hash IS NULL
                                  AND n.script_hash IS NULL))
              JOIN chosen ch ON ch.job_id = n.job_id
              WHERE n.workspace_id = $1 AND n.asset_path IS NOT NULL
                -- Unpinned, the scope is the relations in view: `asset` says
                -- which of them this folder touches. Pinned, that table is the
                -- WRONG scope — it holds one row set per path, describing the
                -- current deploy, so a model this version had and the current one
                -- dropped would be filtered out of its own run's graph. The
                -- pinned graph's nodes are the scope. Keyed on the pin rather
                -- than on the hash: an editor parse pins without naming one, and
                -- its models are precisely the ones `asset` does not know yet.
                AND ($3::bigint IS NOT NULL OR $5::text IS NOT NULL
                     OR n.asset_path IN (
                  SELECT path FROM asset
                   WHERE workspace_id = $1 AND kind = 'dbt'
                     AND ($2::text IS NULL OR usage_path LIKE $2)))
           )
           SELECT n.script_path AS "script_path!", n.unique_id AS "unique_id!",
                  n.resource_type AS "resource_type!", n.name AS "name!", n.asset_path,
                  n.materialized, n.materialize_strategy, n.tags AS "tags!", n.description,
                  n.test_kind, n.test_column, n.test_args, n.severity, n.attached_node,
                  n.columns, n.freshness,
                  n.raw_code, n.original_file_path,
                  -- Whether the caller may read the project this row describes.
                  -- The query deliberately reaches outside the requested folder
                  -- so an in-scope consumer can explain the relation it reads,
                  -- and `dbt_node` carries no RLS of its own; the relation's
                  -- SHAPE is fine to answer that way, everything the project's
                  -- author WROTE is not. Applied in Rust, over one predicate, so
                  -- the fields it covers are named in one place. This runs in the
                  -- authed transaction, so `script`'s RLS answers it. Matched on
                  -- the HASH as well: `extra_perms` is per row, so a path
                  -- recreated with narrower ones leaves the archived version
                  -- readable, and a path-only probe would answer for THAT grant
                  -- while returning this version's source.
                  --
                  -- A version-less row has no `script` row to ask, and needs
                  -- none: it exists only because this caller's own parse job
                  -- created it from a buffer they wrote, and the unpinned `live`
                  -- branch — fed from `script` — can never join to one.
                  (n.script_hash IS NULL OR EXISTS (
                      SELECT 1 FROM script sc
                       WHERE sc.workspace_id = n.workspace_id AND sc.path = n.script_path
                         AND sc.hash = n.script_hash
                  )) AS "script_visible!"
             FROM dbt_node n
             JOIN live l ON l.path = n.script_path
                        AND (n.script_hash = l.hash
                             OR ($5::text IS NOT NULL AND l.hash IS NULL
                                 AND n.script_hash IS NULL))
             -- Every join onto `dbt_node` needs this, not just the scoping CTE:
             -- `job_id` is part of the key, so without it each model comes back
             -- once per retained snapshot plus once for the version's graph.
             JOIN chosen ch ON ch.job_id = n.job_id
            WHERE n.workspace_id = $1
              -- Joined on BOTH columns: a dbt `unique_id` is project-local, so
              -- two projects with the same model name would otherwise pull each
              -- other's rows.
              AND (EXISTS (SELECT 1 FROM scoped s
                            WHERE s.script_path = n.script_path
                              AND s.unique_id = n.unique_id)
                   OR EXISTS (SELECT 1 FROM scoped s
                               WHERE s.script_path = n.script_path
                                 AND s.unique_id = n.attached_node))
            ORDER BY n.script_path, n.unique_id"#,
        &w_id,
        folder_filter.as_deref(),
        dbt_script_hash,
        dbt_job_id,
        pinned_path,
    )
    .fetch_all(&mut *tx)
    .await?;

    // `ref()` lineage between two models, resolved to the relations they
    // produce. Joined to `dbt_node` on both key columns because a dbt
    // `unique_id` is only unique within its project.
    let dbt_edge_rows = sqlx::query!(
        r#"WITH live AS (
             SELECT * FROM (
               SELECT DISTINCT ON (s.path) s.path, s.hash
                 FROM script s
                WHERE $5::text IS NULL AND s.workspace_id = $1 AND s.language = 'dbt'
                  AND ($3::bigint IS NULL OR s.hash = $3)
                  AND ($3::bigint IS NOT NULL OR (s.deleted = false AND s.archived = false))
                ORDER BY s.path, s.created_at DESC
             ) cur
             UNION ALL
             SELECT $5::text, $3::bigint WHERE $5::text IS NOT NULL
           ),
           -- The run's own snapshot when it left one, the version's graph
           -- otherwise. A static descriptor never snapshots, so all of its runs
           -- fall through to the same rows. Existence comes from the marker, not
           -- from a node row: a dynamic run that disabled every model has a
           -- snapshot whose graph is legitimately empty.
           chosen AS (
             -- No visibility check on the job here: reaching this with a job at
             -- all means the caller passed `require_job_read_access` for it, and
             -- re-deciding it under plain RLS can only DISAGREE with that answer
             -- — silently, by falling back to the deployed graph rather than
             -- erroring. A share-link viewer is entitled to the run and would be
             -- shown a different run's model set. See `asset_graph_for`.
             SELECT CASE WHEN $4::uuid IS NOT NULL AND EXISTS (
                      SELECT 1 FROM dbt_graph_snapshot g
                       WHERE g.workspace_id = $1 AND g.job_id = $4)
                    THEN $4::uuid
                    ELSE '00000000-0000-0000-0000-000000000000'::uuid END AS job_id
           )
           SELECT p.asset_path AS "from_path!", c.asset_path AS "to_path!"
             FROM dbt_edge e
             JOIN live l ON l.path = e.script_path
                        AND (e.script_hash = l.hash
                             OR ($5::text IS NOT NULL AND l.hash IS NULL
                                 AND e.script_hash IS NULL))
             JOIN chosen ch ON ch.job_id = e.job_id
             -- Gated the same way as the `live` joins above, and for the same
             -- reason twice over: unpinned this folds to a plain equality the
             -- versioned key can bound, and pinned it is the only way an editor
             -- graph's NULL-to-NULL hashes meet at all.
             JOIN dbt_node p ON p.workspace_id = e.workspace_id
                            AND p.script_path = e.script_path
                            AND (p.script_hash = e.script_hash
                                 OR ($5::text IS NOT NULL AND e.script_hash IS NULL
                                     AND p.script_hash IS NULL))
                            AND p.job_id = ch.job_id
                            AND p.unique_id = e.parent_unique_id
             JOIN dbt_node c ON c.workspace_id = e.workspace_id
                            AND c.script_path = e.script_path
                            AND (c.script_hash = e.script_hash
                                 OR ($5::text IS NOT NULL AND e.script_hash IS NULL
                                     AND c.script_hash IS NULL))
                            AND c.job_id = ch.job_id
                            AND c.unique_id = e.child_unique_id
            WHERE e.workspace_id = $1
              AND p.asset_path IS NOT NULL AND c.asset_path IS NOT NULL
              -- Tests attach to their model as a badge, not as a lineage edge.
              AND c.resource_type <> 'test'
              -- Scoped by the RELATIONS, like the node provenance above, not by
              -- the producing script's folder: two tables consumed in this
              -- folder but produced by a dbt project outside it would otherwise
              -- both render with their `ref()` edge missing.
              -- Same as the node scope: pinned, `asset` describes the CURRENT
              -- deploy, so gating on it drops the edges of models this version
              -- had and a later one removed. The pinned graph's own edges are
              -- the answer.
              AND ($3::bigint IS NOT NULL OR $5::text IS NOT NULL OR EXISTS (
                SELECT 1 FROM asset a
                 WHERE a.workspace_id = $1 AND a.kind = 'dbt'
                   AND a.path = c.asset_path
                   AND ($2::text IS NULL OR a.usage_path LIKE $2)))"#,
        &w_id,
        folder_filter.as_deref(),
        dbt_script_hash,
        dbt_job_id,
        pinned_path,
    )
    .fetch_all(&mut *tx)
    .await?;

    // The marker `chosen` resolved to, answered once for the caller: its
    // EXISTENCE is what lets a run page stop polling, and its timestamp is what
    // an editor labels the graph's provenance with — "parsed from the editor at
    // 14:32" against "as of last deploy". No `v2_job` recheck, for the reason
    // `chosen` gives — disagreeing with the route's decision here would leave a
    // share-link viewer with the right graph and a null marker, polling it 40
    // times over.
    let dbt_marker = match (dbt_job_id, pinned_path) {
        (Some(job), Some(path)) => {
            sqlx::query!(
                "SELECT g.job_id, g.ingested_at FROM dbt_graph_snapshot g
                  WHERE g.workspace_id = $1 AND g.script_path = $2
                    AND g.script_hash IS NOT DISTINCT FROM $3
                    AND g.job_id = CASE WHEN EXISTS (
                          SELECT 1 FROM dbt_graph_snapshot s
                           WHERE s.workspace_id = $1 AND s.job_id = $4)
                        THEN $4::uuid
                        ELSE '00000000-0000-0000-0000-000000000000'::uuid END
                  LIMIT 1",
                &w_id,
                path,
                dbt_script_hash,
                job,
            )
            .fetch_optional(&mut *tx)
            .await?
        }
        _ => None,
    };
    // Only a graph of the RUN's own, never the version's fallback: the run page
    // compares this against its job id to decide whether to keep polling.
    let dbt_snapshot_job = dbt_marker
        .as_ref()
        .map(|m| m.job_id)
        .filter(|j| !j.is_nil());
    let dbt_graph_ingested_at = dbt_marker.as_ref().map(|m| m.ingested_at);
    tx.commit().await?;

    // Parse each pipeline member's body once into its badge annotations, keyed
    // by path, for the runnable-node construction below.
    let annotations_by_path: std::collections::HashMap<
        String,
        windmill_common::assets::PipelineAnnotations,
    > = pipeline_member_paths
        .iter()
        .map(|r| {
            (
                r.path.clone(),
                windmill_common::assets::parse_pipeline_annotations(&r.content),
            )
        })
        .collect();
    // Column-level lineage per member. The annotation-only lineage (already
    // parsed above) is the baseline. For DuckDB scripts we additionally run the
    // full SQL asset parser to infer output→input column edges from the AST; it
    // merges them with the `// column` annotations (annotation wins). If the SQL
    // can't be parsed (DuckDB accepts grammar `sqlparser` rejects), we fall back
    // to the annotation-only baseline rather than dropping explicit annotations.
    let column_lineage_by_path: std::collections::HashMap<
        String,
        Vec<windmill_common::assets::ColumnLineage>,
    > = pipeline_member_paths
        .iter()
        .map(|r| {
            let annotated = || {
                annotations_by_path
                    .get(&r.path)
                    .map(|a| a.column_lineage.clone())
                    .unwrap_or_default()
            };
            let lineage = if r.language == windmill_common::scripts::ScriptLang::DuckDb {
                windmill_parser_sql_asset::parse_assets(&r.content)
                    .map(|o| o.column_lineage)
                    .unwrap_or_else(|_| annotated())
            } else {
                annotated()
            };
            (r.path.clone(), lineage)
        })
        .collect();
    // Model → its provenance, and script → how many models it owns. Tests are
    // folded onto the model they are attached to, which is what makes dbt's
    // four generic tests render through the existing data-test node.
    let mut dbt_by_asset_path: std::collections::HashMap<String, DbtAssetProvenance> =
        Default::default();
    let mut dbt_model_count: std::collections::HashMap<String, usize> = Default::default();
    // Which relations each dbt script actually BUILDS. The sidecar also holds a
    // row for a parent kept only to anchor a cross-selection edge, which the
    // script reads rather than materializes — counting those would make a script
    // selecting one mart claim the staging models upstream of it, and the badge
    // says "materializes N models".
    let dbt_writes: std::collections::HashSet<(&str, &str)> = rows
        .iter()
        .filter(|r| {
            r.asset_kind == AssetKind::Dbt
                && matches!(r.access_type.as_deref(), Some("w") | Some("rw"))
        })
        .map(|r| (r.usage_path.as_str(), r.asset_path.as_str()))
        .collect();
    // Keyed by (script_path, unique_id) — the sidecar's own primary key — since
    // a dbt `unique_id` is only unique within its project.
    let dbt_asset_path_by_unique_id: std::collections::HashMap<(&str, &str), &str> = dbt_rows
        .iter()
        .filter_map(|r| {
            Some((
                (r.script_path.as_str(), r.unique_id.as_str()),
                r.asset_path.as_deref()?,
            ))
        })
        .collect();
    for r in &dbt_rows {
        let Some(asset_path) = r.asset_path.as_deref() else {
            continue;
        };
        if r.resource_type != "source" && dbt_writes.contains(&(r.script_path.as_str(), asset_path))
        {
            *dbt_model_count.entry(r.script_path.clone()).or_default() += 1;
        }
        // What the project's author WROTE — the model's SQL, its path in the
        // repo, the prose and labels around it — as opposed to the shape of the
        // relation it produces. RLS says whether the caller may see the script,
        // but not whether a scoped token may: this endpoint is authorized as
        // `assets:read`, so a token deliberately narrowed to it would otherwise
        // read a project outside its `scripts:read` paths. Both answers gate the
        // same set of fields, because a share-link viewer entitled to the RUN is
        // not thereby entitled to the documentation of a project they cannot
        // open.
        let source_allowed = r.script_visible && dbt_source_scope(&r.script_path);
        let candidate = DbtAssetProvenance {
            raw_code: r.raw_code.clone().filter(|_| source_allowed),
            original_file_path: r.original_file_path.clone().filter(|_| source_allowed),
            unique_id: r.unique_id.clone(),
            resource_type: r.resource_type.clone(),
            materialized: r.materialized.clone(),
            materialize_strategy: r.materialize_strategy.clone(),
            tags: if source_allowed {
                r.tags.clone()
            } else {
                vec![]
            },
            description: r.description.clone().filter(|_| source_allowed),
            data_tests: vec![],
            columns: r.columns.clone().filter(|_| source_allowed),
            freshness: r.freshness.clone().filter(|_| source_allowed),
        };
        // One relation can carry rows from several projects — typically a model
        // in one and a source declaring it in another. The producer describes
        // the relation and the source only names it, so which one wins must not
        // depend on script-path ordering. The source's freshness policy is
        // still worth keeping, so it fills in rather than overwrites.
        let existing = dbt_by_asset_path
            .entry(asset_path.to_string())
            .or_insert_with(|| candidate.clone());
        if existing.unique_id == candidate.unique_id {
            continue;
        }
        let wins = match (
            existing.resource_type.as_str(),
            candidate.resource_type.as_str(),
        ) {
            ("source", t) if t != "source" => true,
            (t, "source") if t != "source" => false,
            // Two rows of the same nature: pick by id so the graph does not
            // change shape between requests.
            _ => candidate.unique_id < existing.unique_id,
        };
        let freshness = existing
            .freshness
            .clone()
            .or_else(|| candidate.freshness.clone());
        if wins {
            *existing = candidate;
        }
        existing.freshness = freshness;
    }
    // Tests fold onto the model they assert, which is what makes dbt's four
    // generic tests render through the existing data-test node. A separate pass
    // because rows arrive in `unique_id` order, so a test can precede its model.
    for r in &dbt_rows {
        if r.resource_type != "test" {
            continue;
        }
        let Some(target) = r
            .attached_node
            .as_deref()
            .and_then(|n| dbt_asset_path_by_unique_id.get(&(r.script_path.as_str(), n)))
        else {
            continue;
        };
        let Some(entry) = dbt_by_asset_path.get_mut(*target) else {
            continue;
        };
        let test = DbtDataTest {
            kind: r.test_kind.clone().unwrap_or_else(|| r.name.clone()),
            column: r.test_column.clone(),
            // The test's kind and column are the badge's shape, already spelled
            // out by its `unique_id`. Its arguments are authored data — an
            // `accepted_values` list is a column's domain — so they follow the
            // same gate as the model's own source.
            args: r
                .test_args
                .clone()
                .filter(|_| r.script_visible && dbt_source_scope(&r.script_path)),
            // dbt-core 1.x echoes the author's casing, 2.x uppercases; fold so
            // the badge reads the same whichever engine deployed the script.
            severity: r.severity.as_deref().map(|s| s.to_ascii_lowercase()),
        };
        if !entry.data_tests.contains(&test) {
            entry.data_tests.push(test);
        }
    }

    let last_success_by_path: std::collections::HashMap<String, chrono::DateTime<chrono::Utc>> =
        last_success_rows
            .into_iter()
            .filter_map(|r| r.last_success_at.map(|t| (r.path, t)))
            .collect();
    let pipeline_member_script_paths: std::collections::HashSet<String> =
        pipeline_member_paths.into_iter().map(|r| r.path).collect();
    let existing_script_paths: std::collections::HashSet<String> =
        existing_script_paths.into_iter().collect();
    let existing_flow_paths: std::collections::HashSet<String> =
        existing_flow_paths.into_iter().collect();
    let runnable_exists = |kind: AssetUsageKind, path: &str| match kind {
        AssetUsageKind::Script => existing_script_paths.contains(path),
        AssetUsageKind::Flow => existing_flow_paths.contains(path),
        // `Job` is a runtime-detected ephemeral runnable (asset usage rows
        // only), never a target of a stored trigger row. Treat as existing
        // so we don't accidentally drop ephemeral lineage edges.
        AssetUsageKind::Job => true,
    };

    let mut edges = Vec::with_capacity(rows.len());
    let mut asset_set: std::collections::HashSet<(AssetKind, String)> = Default::default();
    // Pinned, the relations come from that graph's own nodes. The `asset` rows
    // above are path-keyed — one set per script, always the current deploy — so
    // a model this version had and a later one dropped would be missing from its
    // own run's graph, and an editor buffer's new models would be missing
    // outright, `asset` having never heard of them.
    if dbt_script_hash.is_some() || pinned_path.is_some() {
        for r in &dbt_rows {
            if let Some(p) = r.asset_path.as_deref() {
                asset_set.insert((AssetKind::Dbt, p.to_string()));
            }
        }
    }
    let mut runnable_set: std::collections::HashSet<(AssetUsageKind, String)> = Default::default();

    // Every pipeline member in scope goes into the graph, even when the parser
    // didn't detect any asset r/w and the script has no triggers yet. Without
    // this, a freshly-saved pipeline script whose template body hasn't been
    // filled in would vanish from the pipeline view on graph refetch.
    for path in &pipeline_member_script_paths {
        runnable_set.insert((AssetUsageKind::Script, path.clone()));
    }

    for r in rows {
        // Drop asset usage rows whose runnable target was archived/deleted
        // but whose row in `asset` is still around — those would otherwise
        // surface as a phantom "deployed" runnable on the canvas with no
        // way to interact with it, since the underlying script/flow no
        // longer exists.
        if !runnable_exists(r.usage_kind, &r.usage_path) {
            continue;
        }
        asset_set.insert((r.asset_kind, r.asset_path.clone()));
        runnable_set.insert((r.usage_kind, r.usage_path.clone()));
        edges.push(GraphEdge {
            runnable_path: r.usage_path,
            runnable_kind: r.usage_kind,
            asset_kind: r.asset_kind,
            asset_path: r.asset_path,
            access_type: r.access_type,
        });
    }

    let mut triggers: Vec<TriggerEdge> =
        Vec::with_capacity(trigger_rows.len() + native_trigger_rows.len());
    for t in trigger_rows {
        // Drop orphan asset-trigger rows — their target runnable no longer
        // exists (script/flow archived or deleted, or was never deployed).
        // Without this, an orphan row would surface as a phantom "deployed"
        // runnable on the canvas (no `unsaved` flag, can't actually be
        // run / re-targeted by a new trigger).
        if !runnable_exists(t.runnable_kind, &t.runnable_path) {
            continue;
        }
        runnable_set.insert((t.runnable_kind, t.runnable_path.clone()));
        if t.trigger_kind.as_str() == "asset" {
            // trigger_ref is `<prefix><path>` — parse back out so both
            // endpoints match what the frontend uses for node ids.
            if let Some((asset_kind, asset_path)) = parse_asset_trigger_ref(&t.trigger_ref) {
                // Make sure the source asset has a node even if nothing
                // reads/writes it in this folder.
                asset_set.insert((asset_kind, asset_path.clone()));
                triggers.push(TriggerEdge::Asset {
                    asset_kind,
                    asset_path,
                    runnable_kind: t.runnable_kind,
                    runnable_path: t.runnable_path,
                });
            }
        }
        // Native kinds (schedule, kafka, mqtt, …) come from per-kind trigger
        // tables below.
    }

    // Native trigger attachments — one TriggerEdge per row, the kind chosen
    // from the discriminator. Add the runnable to the set so a script with
    // no asset edges but a kafka/schedule attachment still renders on the
    // canvas.
    for t in native_trigger_rows {
        let kind = t.kind.unwrap_or_default();
        let path = t.path.unwrap_or_default();
        let script_path = t.script_path.unwrap_or_default();
        let runnable_kind = if t.is_flow.unwrap_or(false) {
            AssetUsageKind::Flow
        } else {
            AssetUsageKind::Script
        };
        // Same orphan filter as the asset-trigger loop above — drop trigger
        // rows whose target script/flow no longer exists so the graph
        // doesn't synthesize a phantom deployed runnable.
        if !runnable_exists(runnable_kind, &script_path) {
            continue;
        }
        runnable_set.insert((runnable_kind, script_path.clone()));
        let edge = match kind.as_str() {
            "schedule" => TriggerEdge::Schedule { path, runnable_kind, runnable_path: script_path },
            "email" => TriggerEdge::Email { path, runnable_kind, runnable_path: script_path },
            "kafka" => TriggerEdge::Kafka { path, runnable_kind, runnable_path: script_path },
            "mqtt" => TriggerEdge::Mqtt { path, runnable_kind, runnable_path: script_path },
            "amqp" => TriggerEdge::Amqp { path, runnable_kind, runnable_path: script_path },
            "nats" => TriggerEdge::Nats { path, runnable_kind, runnable_path: script_path },
            "postgres" => TriggerEdge::Postgres { path, runnable_kind, runnable_path: script_path },
            "sqs" => TriggerEdge::Sqs { path, runnable_kind, runnable_path: script_path },
            "gcp" => TriggerEdge::Gcp { path, runnable_kind, runnable_path: script_path },
            _ => continue,
        };
        triggers.push(edge);
    }

    // Macro libraries + lib→consumer edges. Group per-provider macro lists,
    // resolve each usage row's name to its provider (names are
    // workspace-unique), and merge `// use` whole-lib edges from the parsed
    // member annotations. Both endpoints are forced into the runnable set so
    // an out-of-folder library still renders as the edge's provider node.
    let mut macros_by_provider: std::collections::HashMap<String, Vec<MacroInfo>> =
        Default::default();
    let mut provider_by_name: std::collections::HashMap<String, String> = Default::default();
    for r in macro_def_rows {
        provider_by_name.insert(r.name.clone(), r.provider_path.clone());
        macros_by_provider
            .entry(r.provider_path)
            .or_default()
            .push(MacroInfo { name: r.name, params: r.params, is_table: r.is_table_macro });
    }
    let mut macro_edge_map: std::collections::BTreeMap<
        (String, String),
        (std::collections::BTreeSet<String>, bool),
    > = Default::default();
    for u in macro_usage_rows {
        // Same orphan filter as the other edge loops.
        if !runnable_exists(AssetUsageKind::Script, &u.consumer_path) {
            continue;
        }
        let Some(lib) = provider_by_name.get(&u.macro_name) else {
            continue;
        };
        let e = macro_edge_map
            .entry((lib.clone(), u.consumer_path))
            .or_default();
        e.0.insert(u.macro_name);
    }
    for (path, ann) in &annotations_by_path {
        for lib in &ann.use_libs {
            // An undeployed `// use` target has no registry rows — the live
            // draft overlay is the only surface that can render it.
            let Some(lib_macros) = macros_by_provider.get(lib) else {
                continue;
            };
            let e = macro_edge_map
                .entry((lib.clone(), path.clone()))
                .or_default();
            e.1 = true;
            e.0.extend(lib_macros.iter().map(|m| m.name.clone()));
        }
    }
    let macro_edges: Vec<MacroEdge> = macro_edge_map
        .into_iter()
        // Same orphan filter as the other edge families: a registry row whose
        // provider script no longer exists must not synthesize a phantom
        // library node (consumers were filtered above, but the `// use` pass
        // re-adds them, so re-check both endpoints).
        .filter(|((lib_path, consumer_path), _)| {
            runnable_exists(AssetUsageKind::Script, lib_path)
                && runnable_exists(AssetUsageKind::Script, consumer_path)
        })
        .map(|((lib_path, consumer_path), (names, via_use))| MacroEdge {
            lib_path,
            consumer_path,
            macro_names: names.into_iter().collect(),
            via_use,
        })
        .collect();
    for e in &macro_edges {
        runnable_set.insert((AssetUsageKind::Script, e.lib_path.clone()));
        runnable_set.insert((AssetUsageKind::Script, e.consumer_path.clone()));
    }

    // Data-test ordering edges. A `// data_test relationships <col> -> <asset>`
    // (and, best-effort, a custom `// data_test <script>` whose body reads a
    // pipeline asset) needs the referenced asset materialized before the tested
    // script runs — but that dependency is otherwise absent from the execution
    // DAG, so a cold cascade can run the tested script first and hard-fail
    // ("Catalog Error: Table … does not exist"). Resolve the referenced asset's
    // producer(s) from the write edges built above and add an ordering-only
    // edge producer → testing_script. No producer (external table) ⇒ no edge.
    let mut producers_by_asset: std::collections::HashMap<
        (AssetKind, String),
        Vec<(AssetUsageKind, String)>,
    > = Default::default();
    for e in &edges {
        if matches!(e.access_type.as_deref(), Some("w") | Some("rw")) {
            producers_by_asset
                .entry((e.asset_kind, e.asset_path.clone()))
                .or_default()
                .push((e.runnable_kind, e.runnable_path.clone()));
        }
    }
    // Assets a runnable reads, keyed by (usage_kind, path) — used to order a
    // custom-test *script*'s producers before whichever member declares
    // `// data_test <path>`. Keyed on the kind too because a flow can share a
    // script's path; `// data_test <path>` resolves a deployed script body, so
    // the lookup below pins `Script` and never pulls a same-path flow's reads.
    let mut reads_by_runnable: std::collections::HashMap<
        (AssetUsageKind, String),
        Vec<(AssetKind, String)>,
    > = Default::default();
    for e in &edges {
        if matches!(e.access_type.as_deref(), None | Some("r") | Some("rw")) {
            reads_by_runnable
                .entry((e.runnable_kind, e.runnable_path.clone()))
                .or_default()
                .push((e.asset_kind, e.asset_path.clone()));
        }
    }
    let mut test_edges: Vec<TestEdge> = Vec::new();
    // Dedup on (producer, tested_script, asset) so several tests referencing the
    // same asset, or a repeated producer, yield one edge.
    let mut seen_test_edges: std::collections::HashSet<(String, String, AssetKind, String)> =
        Default::default();
    for (member_path, ann) in &annotations_by_path {
        for dt in &ann.data_tests {
            use windmill_common::assets::DataTest;
            let referenced: Vec<(AssetKind, String)> = match dt {
                DataTest::Relationships { to_kind, to_path, .. } => {
                    vec![(
                        windmill_common::assets::asset_kind_from_parser(*to_kind),
                        to_path.clone(),
                    )]
                }
                // Best-effort: the custom test *script*'s parsed reads. Reading
                // the member's OWN output resolves to the member as producer and
                // is dropped by the self-edge guard below.
                DataTest::Custom { path } => reads_by_runnable
                    .get(&(AssetUsageKind::Script, path.clone()))
                    .cloned()
                    .unwrap_or_default(),
                _ => vec![],
            };
            for (asset_kind, asset_path) in referenced {
                let Some(producers) = producers_by_asset.get(&(asset_kind, asset_path.clone()))
                else {
                    continue;
                };
                for (producer_kind, producer_path) in producers {
                    // Skip a self-edge: the tested script produces the asset it
                    // tests (a materialize + relationships/custom on its own output).
                    if *producer_kind == AssetUsageKind::Script && producer_path == member_path {
                        continue;
                    }
                    if seen_test_edges.insert((
                        producer_path.clone(),
                        member_path.clone(),
                        asset_kind,
                        asset_path.clone(),
                    )) {
                        test_edges.push(TestEdge {
                            producer_kind: *producer_kind,
                            producer_path: producer_path.clone(),
                            runnable_kind: AssetUsageKind::Script,
                            runnable_path: member_path.clone(),
                            asset_kind,
                            asset_path: asset_path.clone(),
                        });
                    }
                }
            }
        }
    }
    for e in &test_edges {
        runnable_set.insert((e.producer_kind, e.producer_path.clone()));
        runnable_set.insert((e.runnable_kind, e.runnable_path.clone()));
    }

    // Fork data-environment state per ducklake asset. The parent's rows are read on the plain
    // pool: fork membership does not imply parent membership, and defer already exposes the
    // parent's data to fork jobs — surfacing its materialization status is strictly less.
    let fork_materialization_by_path: std::collections::HashMap<String, &'static str> = {
        // The WHOLE ancestor chain, matching defer discovery: a grandchild fork whose direct
        // parent only defers a table still reads it (from the grandparent), so it must show
        // as deferred, not unmarked.
        let ancestors = windmill_common::workspaces::fork_ancestor_chain(&db, &w_id).await?;
        match ancestors {
            // An orphaned `wm-fork-*` (parent deleted, chain empty) is still isolated — its
            // own materializations must keep their 'fork' chips (nothing can be 'deferred').
            a if a.is_empty() && !w_id.starts_with(windmill_common::workspaces::WM_FORK_PREFIX) => {
                std::collections::HashMap::new()
            }
            ancestors => {
                // Lakes the fork chose to SHARE at creation have no fork namespace — their
                // assets live in the parent's tables for real, so no chip applies.
                let shared_lakes: std::collections::HashSet<String> = sqlx::query_scalar!(
                    "SELECT ducklake->'ducklakes' FROM workspace_settings WHERE workspace_id = $1",
                    &w_id,
                )
                .fetch_optional(&db)
                .await?
                .flatten()
                .and_then(|v| v.as_object().cloned())
                .map(|lakes| {
                    lakes
                        .into_iter()
                        .filter(|(_, cfg)| {
                            cfg.get("fork_behavior").and_then(|b| b.as_str()) == Some("shared")
                        })
                        .map(|(name, _)| name)
                        .collect()
                })
                .unwrap_or_default();
                sqlx::query!(
                    r#"
                    -- Fork rows also count when a snapshot ever committed (a failed run
                    -- preserves it): the physical table exists, so reads hit the FORK's
                    -- data — showing 'deferred' would misstate what a query returns.
                    -- Ancestor rows still require a clean materialization.
                    SELECT DISTINCT asset_path AS "asset_path!", workspace_id AS "workspace_id!"
                    FROM materialized_partition
                    WHERE (workspace_id = $1 OR workspace_id = ANY($2))
                      AND asset_kind = 'ducklake'
                      AND (status = 'materialized'
                           OR (workspace_id = $1 AND snapshot_id IS NOT NULL))
                    "#,
                    &w_id,
                    &ancestors,
                )
                .fetch_all(&db)
                .await?
                .into_iter()
                .filter(|r| {
                    !shared_lakes.contains(r.asset_path.split('/').next().unwrap_or_default())
                })
                .fold(std::collections::HashMap::new(), |mut m, r| {
                    // A fork row wins over an inherited 'deferred' from any ancestor's row.
                    if r.workspace_id == w_id {
                        m.insert(r.asset_path, "fork");
                    } else {
                        m.entry(r.asset_path).or_insert("deferred");
                    }
                    m
                })
            }
        }
    };

    // (kind, `<dim>_current` path) → base `<dim>` path, for every managed scd2
    // producer in the graph. Reads of the companion view resolve to a node
    // whose producer edge already exists (the deploy path registers both writes)
    // — this map lets that node advertise which base dimension it derives from.
    let scd2_current_base: std::collections::HashMap<(AssetKind, String), String> =
        annotations_by_path
            .values()
            .filter_map(|a| a.materialize.as_ref())
            .filter_map(|m| {
                m.scd2_current_target().map(|(k, current)| {
                    (
                        (windmill_common::assets::asset_kind_from_parser(k), current),
                        m.target_path.clone(),
                    )
                })
            })
            .collect();

    let mut assets: Vec<GraphAssetNode> = asset_set
        .into_iter()
        .map(|(kind, path)| GraphAssetNode {
            fork_materialization: (kind == AssetKind::Ducklake)
                .then(|| {
                    fork_materialization_by_path
                        .get(&path)
                        .map(|s| s.to_string())
                })
                .flatten(),
            derived_from: scd2_current_base.get(&(kind, path.clone())).cloned(),
            dbt: (kind == AssetKind::Dbt)
                .then(|| dbt_by_asset_path.get(&path).cloned())
                .flatten(),
            kind,
            path,
        })
        .collect();
    assets.sort_by(|a, b| a.path.cmp(&b.path));

    let mut runnables: Vec<GraphRunnableNode> = runnable_set
        .into_iter()
        .map(|(usage_kind, path)| {
            let in_pipeline = usage_kind == AssetUsageKind::Script
                && pipeline_member_script_paths.contains(&path);
            // Annotation badges, only for pipeline-member scripts (the only
            // bodies we parsed). Gate on the runnable kind too: a flow sharing a
            // path with a pipeline script must not inherit its badges.
            let ann = (usage_kind == AssetUsageKind::Script)
                .then(|| annotations_by_path.get(&path))
                .flatten();
            GraphRunnableNode {
                in_pipeline,
                partition_kind: ann
                    .and_then(|a| a.partition.as_ref())
                    .map(|p| partition_kind_word(&p.kind).to_string()),
                freshness: ann
                    .and_then(|a| a.freshness.as_ref())
                    .map(|f| f.duration.clone()),
                last_success_at: (usage_kind == AssetUsageKind::Script)
                    .then(|| last_success_by_path.get(&path))
                    .flatten()
                    .copied(),
                tag: ann.and_then(|a| a.tag.clone()),
                retry: ann.and_then(|a| a.retry.clone()),
                data_tests: ann.map(|a| a.data_tests.clone()).unwrap_or_default(),
                // Inferred (DuckDB AST) + annotation column lineage, gated to
                // scripts like the badges above.
                column_lineage: (usage_kind == AssetUsageKind::Script)
                    .then(|| column_lineage_by_path.get(&path))
                    .flatten()
                    .cloned()
                    .unwrap_or_default(),
                materialize_target: ann.and_then(|a| a.materialize.as_ref()).map(|m| {
                    MaterializeTargetNode {
                        kind: windmill_common::assets::asset_kind_from_parser(m.target_kind),
                        path: m.target_path.clone(),
                    }
                }),
                materialize_strategy: ann.and_then(|a| a.materialize.as_ref()).and_then(|m| {
                    // Precedence mirrors the runtime strategy derivation:
                    // scd2 (`history`) > append > merge (`key=`) > replace.
                    if m.manual {
                        None
                    } else if m.scd2 {
                        Some("scd2".to_string())
                    } else if m.append {
                        Some("append".to_string())
                    } else if m.unique_key.is_some() {
                        Some("merge".to_string())
                    } else {
                        Some("replace".to_string())
                    }
                }),
                materialize_on_schema_change: ann
                    .and_then(|a| a.materialize.as_ref())
                    .filter(|m| {
                        m.on_schema_change == windmill_common::assets::OnSchemaChange::Ignore
                    })
                    .map(|_| "ignore".to_string()),
                macros: (usage_kind == AssetUsageKind::Script)
                    .then(|| macros_by_provider.get(&path))
                    .flatten()
                    .cloned()
                    .unwrap_or_default(),
                dbt: (usage_kind == AssetUsageKind::Script)
                    .then(|| dbt_model_count.get(&path))
                    .flatten()
                    .map(|n| DbtRunnableProvenance { model_count: *n }),
                path,
                usage_kind,
            }
        })
        .collect();
    runnables.sort_by(|a, b| a.path.cmp(&b.path));

    // Only between relations this graph actually renders — an edge to a node
    // that is not here has nothing to draw.
    let rendered: std::collections::HashSet<&str> =
        assets.iter().map(|a| a.path.as_str()).collect();
    let mut dbt_edges: Vec<DbtLineageEdge> = dbt_edge_rows
        .into_iter()
        .filter(|r| {
            r.from_path != r.to_path
                && rendered.contains(r.from_path.as_str())
                && rendered.contains(r.to_path.as_str())
        })
        .map(|r| DbtLineageEdge { from_asset_path: r.from_path, to_asset_path: r.to_path })
        .collect();
    dbt_edges.sort();
    dbt_edges.dedup();

    Ok(Json(AssetGraphResponse {
        assets,
        runnables,
        edges,
        triggers,
        macro_edges,
        test_edges,
        dbt_edges,
        dbt_snapshot_job,
        dbt_graph_ingested_at,
    }))
}

// ------------------------------------------------------------------
// GET /w/:workspace/assets/pipelines
// ------------------------------------------------------------------
// Distinct folder names that contain at least one pipeline-member script
// (auto_kind='pipeline'). Used by the pipeline-editor folder picker and
// the "Pipeline" entry in folder views. Keyed by the partial index on
// `script (workspace_id, path) WHERE auto_kind='pipeline' ...` so this
// is effectively O(matches).

#[derive(Serialize, Debug)]
struct PipelineFolder {
    folder: String,
    script_count: i64,
}

async fn list_pipeline_folders(
    authed: ApiAuthed,
    Path(w_id): Path<String>,
    Extension(user_db): Extension<UserDB>,
) -> JsonResult<Vec<PipelineFolder>> {
    let mut tx = user_db.begin(&authed).await?;
    let rows = sqlx::query!(
        r#"
        SELECT
            substring(path from '^f/([^/]+)/') AS "folder!",
            COUNT(*) AS "script_count!"
        FROM script
        WHERE workspace_id = $1
          AND auto_kind = 'pipeline'
          AND archived = false
          AND deleted = false
          AND path LIKE 'f/%'
        GROUP BY substring(path from '^f/([^/]+)/')
        ORDER BY substring(path from '^f/([^/]+)/')
        "#,
        &w_id,
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(Json(
        rows.into_iter()
            .map(|r| PipelineFolder { folder: r.folder, script_count: r.script_count })
            .collect(),
    ))
}
