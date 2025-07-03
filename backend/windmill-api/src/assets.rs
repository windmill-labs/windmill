use axum::{
    extract::Path,
    routing::{get, post},
    Extension, Json, Router,
};
use serde::Deserialize;
use serde_json::Value;
use windmill_common::{assets::AssetUsageKind, db::UserDB, error::JsonResult};

use crate::db::ApiAuthed;

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list", get(list_assets))
        .route("/list_by_usages", post(list_assets_by_usages))
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
                    'path', usage_path,
                    'kind', usage_kind
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
            WHERE workspace_id = $1 AND usage_path = $2 AND usage_kind = $3"#,
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
