use axum::{
    extract::{Path, Query},
    routing::{get, post},
    Extension, Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{Postgres, Transaction};
use windmill_common::{
    db::UserDB,
    error::{JsonResult, Result},
};

use crate::db::ApiAuthed;

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/link", post(link_assets))
        .route("/list", get(list_assets))
        .route("/list_for_usage", get(list_assets_for_usage))
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Copy, Clone, Hash, Eq, sqlx::Type)]
#[sqlx(type_name = "ASSET_KIND", rename_all = "lowercase")]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum AssetKind {
    S3Object,
    Resource,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Copy, Clone, Hash, Eq, sqlx::Type)]
#[sqlx(type_name = "ASSET_USAGE_KIND", rename_all = "lowercase")]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum AssetUsageKind {
    Script,
    Flow,
}

#[derive(Deserialize)]
pub struct Asset {
    pub path: String,
    pub kind: AssetKind,
}

#[derive(Deserialize)]
pub struct LinkAssetsBody {
    pub assets: Vec<Asset>,
    pub usage_path: String,
    pub usage_kind: AssetUsageKind,
}

async fn link_assets(
    authed: ApiAuthed,
    Path(w_id): Path<String>,
    Extension(user_db): Extension<UserDB>,
    Json(body): Json<LinkAssetsBody>,
) -> JsonResult<()> {
    let mut tx = user_db.begin(&authed).await?;
    link_assets_internal(&mut tx, w_id, body).await?;
    tx.commit().await?;
    Ok(Json(()))
}

async fn link_assets_internal(
    tx: &mut Transaction<'_, Postgres>,
    w_id: String,
    body: LinkAssetsBody,
) -> Result<()> {
    sqlx::query!(
        r#"DELETE FROM asset WHERE workspace_id = $1 AND usage_path = $2 AND usage_kind = $3;"#,
        w_id,
        body.usage_path,
        body.usage_kind as AssetUsageKind
    )
    .execute(&mut **tx)
    .await?;

    for asset in body.assets {
        sqlx::query!(
            r#"INSERT INTO asset (workspace_id, path, kind, usage_path, usage_kind) VALUES ($1, $2, $3, $4, $5);"#,
            w_id,
            asset.path,
            asset.kind as AssetKind,
            body.usage_path,
            body.usage_kind as AssetUsageKind
        )
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

async fn list_assets(
    authed: ApiAuthed,
    Path(w_id): Path<String>,
    Extension(user_db): Extension<UserDB>,
) -> JsonResult<Vec<Value>> {
    let assets = sqlx::query_scalar!(
        r#"SELECT
            jsonb_build_object(
                'path', path,
                'kind', kind,
                'usages', ARRAY_AGG(jsonb_build_object(
                    'usage_path', usage_path,
                    'usage_kind', usage_kind
                ))
            ) as "list!: _"
        FROM asset
        WHERE workspace_id = $1
        GROUP BY path, kind"#,
        w_id,
    )
    .fetch_all(&mut *user_db.begin(&authed).await?)
    .await?;

    Ok(Json(assets))
}

#[derive(Deserialize)]
struct ListAssetForUsageQuery {
    usage_kind: AssetUsageKind,
    usage_path: String,
}

async fn list_assets_for_usage(
    authed: ApiAuthed,
    Path(w_id): Path<String>,
    Query(ListAssetForUsageQuery { usage_kind, usage_path }): Query<ListAssetForUsageQuery>,
    Extension(user_db): Extension<UserDB>,
) -> JsonResult<Vec<Value>> {
    let assets = sqlx::query_scalar!(
        r#"SELECT
            jsonb_build_object(
                'path', path,
                'kind', kind
            ) as "list!: _"
        FROM asset
        WHERE workspace_id = $1 AND usage_path = $2 AND usage_kind = $3"#,
        w_id,
        usage_path,
        usage_kind as AssetUsageKind
    )
    .fetch_all(&mut *user_db.begin(&authed).await?)
    .await?;

    Ok(Json(assets))
}
