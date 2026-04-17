use std::sync::Arc;

use axum::{
    extract::{Path, Query},
    routing::get,
    Extension, Json, Router,
};
use http::Method;
use serde::Deserialize;
use windmill_common::{error::JsonResult, DB};

use crate::{get_workspace_integration, External, ServiceName};

use super::{GitHub, GithubApiRepoResponse, GithubApiSearchResponse, GithubRepoEntry};

const PER_PAGE: usize = 30;

#[derive(Deserialize)]
struct ListReposParams {
    q: Option<String>,
}

async fn list_repos(
    Extension(handler): Extension<Arc<GitHub>>,
    Extension(db): Extension<DB>,
    Path(workspace_id): Path<String>,
    Query(params): Query<ListReposParams>,
) -> JsonResult<Vec<GithubRepoEntry>> {
    get_workspace_integration(&db, &workspace_id, ServiceName::Github).await?;

    let query = params.q.as_deref().map(str::trim).filter(|q| !q.is_empty());

    let repos: Vec<GithubApiRepoResponse> = if let Some(q) = query {
        // Authenticated search over repos the user has access to (incl. private).
        // `user:@me` scopes results to the current user's affiliated repos.
        let raw = format!("{} in:name user:@me", q);
        let encoded = urlencoding::encode(&raw);
        let url = format!(
            "{}/search/repositories?q={}&per_page={}&sort=updated",
            super::GITHUB_API_BASE,
            encoded,
            PER_PAGE,
        );
        let search: GithubApiSearchResponse = handler
            .http_client_request::<_, ()>(&url, Method::GET, &workspace_id, &db, None, None)
            .await?;
        search.items
    } else {
        // Default view: most recently updated repos accessible to the user.
        let url = format!(
            "{}/user/repos?sort=updated&per_page={}&type=all",
            super::GITHUB_API_BASE,
            PER_PAGE,
        );
        handler
            .http_client_request::<_, ()>(&url, Method::GET, &workspace_id, &db, None, None)
            .await?
    };

    let entries = repos
        .into_iter()
        .map(|r| GithubRepoEntry {
            full_name: r.full_name,
            name: r.name,
            owner: r.owner.login,
            private: r.private,
        })
        .collect();

    Ok(Json(entries))
}

pub fn github_routes(service: GitHub) -> Router {
    let service = Arc::new(service);
    Router::new()
        .route("/repos", get(list_repos))
        .layer(Extension(service))
}
