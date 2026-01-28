use axum::{
    extract::{Path, Query},
    routing::{get, post},
    Extension, Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use windmill_common::{assets::AssetUsageKind, db::UserDB, error::JsonResult};

use crate::db::ApiAuthed;

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list", get(list_assets))
        .route("/list_by_usages", post(list_assets_by_usages))
}

#[derive(Deserialize)]
struct ListAssetsQuery {
    #[serde(default = "default_per_page")]
    per_page: i64,
    cursor_created_at: Option<chrono::DateTime<chrono::Utc>>,
    cursor_id: Option<i64>,
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

    // Build the query with cursor pagination
    let rows = if let (Some(cursor_created_at), Some(cursor_id)) =
        (query.cursor_created_at, query.cursor_id)
    {
        sqlx::query_as!(
            AssetRow,
            r#"
            WITH asset_summary AS (
                SELECT
                    asset.path,
                    asset.kind,
                    MAX(asset.created_at) as max_created_at,
                    MAX(asset.id) as max_id
                FROM asset
                WHERE asset.workspace_id = $1
                  AND (asset.usage_kind <> 'flow' OR asset.usage_path = ANY(SELECT path FROM flow WHERE workspace_id = $1))
                  AND (asset.usage_kind <> 'script' OR asset.usage_path = ANY(SELECT path FROM script WHERE workspace_id = $1))
                GROUP BY asset.path, asset.kind
                HAVING MAX(asset.created_at) < $3 OR (MAX(asset.created_at) = $3 AND MAX(asset.id) < $4)
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
                )) as "result!",
                asset_summary.max_created_at as "max_created_at!",
                asset_summary.max_id as "max_id!"
            FROM asset
            INNER JOIN asset_summary ON asset.path = asset_summary.path AND asset.kind = asset_summary.kind
            LEFT JOIN resource ON asset.kind = 'resource'
              AND array_to_string((string_to_array(asset.path, '/'))[1:3], '/') = resource.path
              AND resource.workspace_id = $1
            LEFT JOIN v2_job job ON asset.usage_kind = 'job'
              AND asset.usage_path = job.id::text
              AND job.workspace_id = $1
            WHERE asset.workspace_id = $1
              AND (asset.kind <> 'resource' OR resource.path IS NOT NULL)
              AND (asset.usage_kind <> 'job' OR job.id IS NOT NULL)
            GROUP BY asset.path, asset.kind, resource.resource_type, asset_summary.max_created_at, asset_summary.max_id
            ORDER BY asset_summary.max_created_at DESC, asset_summary.max_id DESC
            "#,
            &w_id,
            limit,
            cursor_created_at,
            cursor_id
        )
        .fetch_all(&mut *tx)
        .await?
    } else {
        sqlx::query_as!(
            AssetRow,
            r#"
            WITH asset_summary AS (
                SELECT
                    asset.path,
                    asset.kind,
                    MAX(asset.created_at) as max_created_at,
                    MAX(asset.id) as max_id
                FROM asset
                WHERE asset.workspace_id = $1
                  AND (asset.usage_kind <> 'flow' OR asset.usage_path = ANY(SELECT path FROM flow WHERE workspace_id = $1))
                  AND (asset.usage_kind <> 'script' OR asset.usage_path = ANY(SELECT path FROM script WHERE workspace_id = $1))
                GROUP BY asset.path, asset.kind
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
                )) as "result!",
                asset_summary.max_created_at as "max_created_at!",
                asset_summary.max_id as "max_id!"
            FROM asset
            INNER JOIN asset_summary ON asset.path = asset_summary.path AND asset.kind = asset_summary.kind
            LEFT JOIN resource ON asset.kind = 'resource'
              AND array_to_string((string_to_array(asset.path, '/'))[1:3], '/') = resource.path
              AND resource.workspace_id = $1
            LEFT JOIN v2_job job ON asset.usage_kind = 'job'
              AND asset.usage_path = job.id::text
              AND job.workspace_id = $1
            WHERE asset.workspace_id = $1
              AND (asset.kind <> 'resource' OR resource.path IS NOT NULL)
              AND (asset.usage_kind <> 'job' OR job.id IS NOT NULL)
            GROUP BY asset.path, asset.kind, resource.resource_type, asset_summary.max_created_at, asset_summary.max_id
            ORDER BY asset_summary.max_created_at DESC, asset_summary.max_id DESC
            "#,
            &w_id,
            limit
        )
        .fetch_all(&mut *tx)
        .await?
    };

    let assets: Vec<Value> = rows
        .iter()
        .take(per_page as usize)
        .map(|r| r.result.clone())
        .collect();

    let next_cursor = if rows.len() as i64 > per_page {
        let last = &rows[per_page as usize - 1];
        Some(AssetCursor {
            created_at: last.max_created_at,
            id: last.max_id,
        })
    } else {
        None
    };

    Ok(Json(ListAssetsResponse {
        assets,
        next_cursor,
    }))
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
                jsonb_build_object(
                    'path', path,
                    'kind', kind,
                    'access_type', usage_access_type
                ) as "list!: _"
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
