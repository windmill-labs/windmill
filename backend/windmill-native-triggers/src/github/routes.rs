use std::sync::Arc;

use axum::{extract::Path, routing::get, Extension, Json, Router};
use http::Method;
use windmill_common::{error::JsonResult, DB};

use crate::{get_workspace_integration, External, ServiceName};

use super::{GitHub, GithubApiRepoResponse, GithubRepoEntry};

const PER_PAGE: usize = 100;
const MAX_PAGES: usize = 10;

async fn list_repos(
    Extension(handler): Extension<Arc<GitHub>>,
    Extension(db): Extension<DB>,
    Path(workspace_id): Path<String>,
) -> JsonResult<Vec<GithubRepoEntry>> {
    get_workspace_integration(&db, &workspace_id, ServiceName::Github).await?;

    let mut all_entries = Vec::new();

    for page in 1..=MAX_PAGES {
        let url = format!(
            "{}/user/repos?sort=updated&per_page={}&type=all&page={}",
            super::GITHUB_API_BASE,
            PER_PAGE,
            page,
        );

        let repos: Vec<GithubApiRepoResponse> = handler
            .http_client_request::<_, ()>(&url, Method::GET, &workspace_id, &db, None, None)
            .await?;

        let count = repos.len();
        all_entries.extend(repos.into_iter().map(|r| GithubRepoEntry {
            full_name: r.full_name,
            name: r.name,
            owner: r.owner.login,
            private: r.private,
        }));

        if count < PER_PAGE {
            break;
        }
    }

    Ok(Json(all_entries))
}

pub fn github_routes(service: GitHub) -> Router {
    let service = Arc::new(service);
    Router::new()
        .route("/repos", get(list_repos))
        .layer(Extension(service))
}
