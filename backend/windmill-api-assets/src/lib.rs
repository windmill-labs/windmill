use axum::{
    extract::{Path, Query},
    routing::{get, post},
    Extension, Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Row;
use windmill_common::{
    assets::{AssetKind, AssetUsageKind},
    db::UserDB,
    error::JsonResult,
    utils::escape_ilike_pattern,
};

use windmill_api_auth::ApiAuthed;

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list", get(list_assets))
        .route("/list_by_usages", post(list_assets_by_usages))
        .route("/list_favorites", get(list_favorites))
        .route("/graph", get(asset_graph))
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
struct GraphQuery {
    pub asset_kinds: Option<String>,
    pub folder: Option<String>,
}

#[derive(Serialize, Debug)]
struct GraphAssetNode {
    kind: AssetKind,
    path: String,
}

#[derive(Serialize, Debug)]
struct GraphRunnableNode {
    path: String,
    usage_kind: AssetUsageKind,
}

#[derive(Serialize, Debug)]
struct GraphEdge {
    runnable_path: String,
    runnable_kind: AssetUsageKind,
    asset_kind: AssetKind,
    asset_path: String,
    access_type: Option<String>,
}

#[derive(Serialize, Debug)]
struct AssetGraphResponse {
    assets: Vec<GraphAssetNode>,
    runnables: Vec<GraphRunnableNode>,
    edges: Vec<GraphEdge>,
}

async fn asset_graph(
    authed: ApiAuthed,
    Path(w_id): Path<String>,
    Extension(user_db): Extension<UserDB>,
    Query(q): Query<GraphQuery>,
) -> JsonResult<AssetGraphResponse> {
    let mut tx = user_db.begin(&authed).await?;

    let kind_filter: Option<Vec<AssetKind>> = q.asset_kinds.as_ref().map(|s| {
        s.split(',')
            .filter_map(|k| {
                serde_json::from_value::<AssetKind>(Value::String(k.trim().into())).ok()
            })
            .collect()
    });
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

    tx.commit().await?;

    let mut edges = Vec::with_capacity(rows.len());
    let mut asset_set: std::collections::HashSet<(AssetKind, String)> = Default::default();
    let mut runnable_set: std::collections::HashSet<(AssetUsageKind, String)> = Default::default();

    for r in rows {
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

    let mut assets: Vec<GraphAssetNode> = asset_set
        .into_iter()
        .map(|(kind, path)| GraphAssetNode { kind, path })
        .collect();
    assets.sort_by(|a, b| a.path.cmp(&b.path));

    let mut runnables: Vec<GraphRunnableNode> = runnable_set
        .into_iter()
        .map(|(usage_kind, path)| GraphRunnableNode { path, usage_kind })
        .collect();
    runnables.sort_by(|a, b| a.path.cmp(&b.path));

    Ok(Json(AssetGraphResponse { assets, runnables, edges }))
}
