use axum::{
    extract::{Path, Query},
    routing::{get, post},
    Extension, Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use windmill_common::{
    assets::{AssetKind, AssetUsageKind},
    db::UserDB,
    error::JsonResult,
};

use crate::db::ApiAuthed;

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list", get(list_assets))
        .route("/list_by_usages", post(list_assets_by_usages))
        .route("/list_jobs", get(list_asset_jobs))
}

async fn list_assets(
    authed: ApiAuthed,
    Path(w_id): Path<String>,
    Extension(user_db): Extension<UserDB>,
) -> JsonResult<Vec<Value>> {
    let assets = sqlx::query_scalar!(
        r#"SELECT
            jsonb_strip_nulls(jsonb_build_object(
                'path', asset.path,
                'kind', asset.kind,
                'usages', ARRAY_AGG(DISTINCT jsonb_build_object(
                    'path', asset.usage_path,
                    'kind', asset.usage_kind,
                    'access_type', asset.usage_access_type,
                    'detection_kinds', (
                      SELECT ARRAY_AGG(DISTINCT a2.asset_detection_kind)
                      FROM asset a2
                      WHERE a2.workspace_id = asset.workspace_id
                        AND a2.path = asset.path
                        AND a2.kind = asset.kind
                        AND a2.usage_path = asset.usage_path
                        AND a2.usage_kind = asset.usage_kind
                    )
                )),
                'metadata', (CASE
                  WHEN asset.kind = 'resource' THEN
                    jsonb_build_object('resource_type', resource.resource_type)
                  ELSE
                    NULL
                  END
                )
            )) as "list!: _"
        FROM asset
        LEFT JOIN resource ON asset.kind = 'resource'
          AND array_to_string((string_to_array(asset.path, '/'))[1:3], '/') = resource.path -- With specific table, asset path can be e.g u/diego/pg_db/table_name
          AND resource.workspace_id = $1
        WHERE asset.workspace_id = $1
          AND (asset.kind <> 'resource' OR resource.path IS NOT NULL)
          AND (asset.usage_kind <> 'flow' OR asset.usage_path = ANY(SELECT path FROM flow WHERE workspace_id = $1))
          AND (asset.usage_kind <> 'script' OR asset.usage_path = ANY(SELECT path FROM script WHERE workspace_id = $1))
          GROUP BY asset.path, asset.kind, resource.resource_type
        ORDER BY asset.path, asset.kind"#,
        w_id,
    )
    .fetch_all(&mut *user_db.begin(&authed).await?)
    .await?;

    Ok(Json(assets))
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
              AND asset_detection_kind = 'static'
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

#[derive(Deserialize)]
struct ListAssetJobsQuery {
    asset_path: String,
    asset_kind: String,
    page: Option<i64>,
    per_page: Option<i64>,
}

#[derive(Serialize)]
struct AssetJobListResponse {
    jobs: Vec<AssetJobInfo>,
    total: i64,
    page: i64,
    per_page: i64,
}

#[derive(Serialize)]
struct AssetJobInfo {
    id: uuid::Uuid,
    created_at: chrono::DateTime<chrono::Utc>,
    created_by: String,
    runnable_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<String>,
}

async fn list_asset_jobs(
    authed: ApiAuthed,
    Path(w_id): Path<String>,
    Query(query): Query<ListAssetJobsQuery>,
    Extension(user_db): Extension<UserDB>,
) -> JsonResult<AssetJobListResponse> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).clamp(1, 100);
    let asset_path = query.asset_path;
    let asset_kind = query.asset_kind;
    let offset = (page - 1) * per_page;

    // Parse asset_kind
    let asset_kind: AssetKind = serde_json::from_value(serde_json::Value::String(asset_kind))
        .map_err(|e| {
            windmill_common::error::Error::BadRequest(format!("Invalid asset kind: {}", e))
        })?;

    let mut tx = user_db.begin(&authed).await?;

    // Get total count of jobs that used this asset
    let total = sqlx::query_scalar!(
        r#"SELECT COUNT(DISTINCT asset.job_id)::bigint as "count!"
        FROM asset
        WHERE asset.workspace_id = $1
          AND asset.path = $2
          AND asset.kind = $3
          AND asset.asset_detection_kind = 'runtime'
          AND asset.job_id IS NOT NULL"#,
        w_id,
        asset_path,
        asset_kind as AssetKind
    )
    .fetch_one(&mut *tx)
    .await?;

    // Get paginated jobs with their info
    let jobs = sqlx::query_as!(
        AssetJobInfo,
        r#"SELECT DISTINCT
            v2_job.id,
            v2_job.created_at,
            v2_job.created_by,
            v2_job.runnable_path,
            CASE
              WHEN v2_job_completed.id IS NOT NULL THEN v2_job_completed.status::text
              ELSE NULL
            END as status
        FROM asset
        INNER JOIN v2_job ON asset.job_id = v2_job.id
        LEFT JOIN v2_job_completed ON v2_job.id = v2_job_completed.id
        WHERE asset.workspace_id = $1
          AND asset.path = $2
          AND asset.kind = $3
          AND asset.asset_detection_kind = 'runtime'
          AND asset.job_id IS NOT NULL
        ORDER BY v2_job.created_at DESC
        LIMIT $4 OFFSET $5"#,
        w_id,
        asset_path,
        asset_kind as AssetKind,
        per_page,
        offset
    )
    .fetch_all(&mut *tx)
    .await?;

    Ok(Json(AssetJobListResponse { jobs, total, page, per_page }))
}
