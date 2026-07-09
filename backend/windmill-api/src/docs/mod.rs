//! Self-hosted documentation search.
//!
//! The backend embeds a vendored docs snapshot (see [`corpus`]) and exposes two
//! read-only endpoints over it, so docs search works with no runtime egress:
//!   - `GET /api/docs/search?query=...`  — full-text + index search
//!   - `GET /api/docs/page?url=...&section=...` — read one page (or a section)
//!
//! These back the AI chat `search_docs`/`read_docs_page` tools, the MCP
//! `searchDocs`/`readDocsPage` tools, and the `wmill docs` CLI. The routes are
//! nested behind the global authed service in `lib.rs`, so a valid token is
//! required but no workspace.

mod corpus;
mod search;

use axum::{extract::Query, routing::get, Json, Router};
use serde::{Deserialize, Serialize};
use windmill_common::error::JsonResult;

use search::DocsSearchResult;

pub fn global_service() -> Router {
    Router::new()
        .route("/search", get(search_docs))
        .route("/page", get(read_docs_page))
}

#[derive(Deserialize)]
struct SearchQuery {
    query: String,
}

#[derive(Serialize)]
struct SearchResponse {
    /// Model-ready rendering of the results (the exact string the AI/MCP tool
    /// returns). Built once here so every consumer is identical.
    text: String,
    /// Structured results for non-AI consumers (e.g. the CLI's pretty/`--json`).
    results: Vec<DocsSearchResult>,
}

#[derive(Deserialize)]
struct PageQuery {
    /// A page's `Source` URL (as returned by the docs search tool); a bare `/docs/...`
    /// path is also accepted and canonicalized before lookup.
    url: String,
    section: Option<String>,
}

#[derive(Serialize)]
struct PageResponse {
    text: String,
    source_url: String,
}

async fn search_docs(Query(q): Query<SearchQuery>) -> JsonResult<SearchResponse> {
    let query = q.query.trim().to_string();
    if query.is_empty() {
        return Ok(Json(SearchResponse {
            text: "No search query was provided. Provide a `query` of one or more keywords."
                .to_string(),
            results: Vec::new(),
        }));
    }

    // Lazy corpus init (gzip decompress + parse) and the per-query full-corpus scan
    // are CPU-bound; keep them off the async runtime.
    let (text, results) = tokio::task::spawn_blocking(move || {
        let corpus = corpus::corpus();
        // Body grep first (concrete content hits), then index titles/descriptions to
        // surface named features body grep misses; merge dedupes by canonical URL.
        let body = search::search_docs_pages(&corpus.pages, &query, 5);
        let index = search::search_docs_index(&corpus.index, &query, 4);
        let results = search::merge_docs_search_results(body, index, search::SEARCH_MAX_PAGES);
        let text = search::format_docs_search_results(&query, &results);
        (text, results)
    })
    .await
    .map_err(|e| windmill_common::error::Error::InternalErr(format!("docs search task: {e}")))?;

    Ok(Json(SearchResponse { text, results }))
}

async fn read_docs_page(Query(q): Query<PageQuery>) -> JsonResult<PageResponse> {
    let url = q.url.trim();
    if url.is_empty() {
        return Ok(Json(PageResponse {
            text: "No documentation page URL was provided. Provide a `url` — e.g. a `Source` URL returned by the docs search tool.".to_string(),
            source_url: String::new(),
        }));
    }

    let url = url.to_string();
    let section = q.section.filter(|s| !s.trim().is_empty());

    // Corpus init + page sanitize/render are CPU-bound; keep them off the runtime.
    let resp = tokio::task::spawn_blocking(move || {
        let corpus = corpus::corpus();
        match corpus.find_page(&url) {
            Some(page) => {
                // Rewrite docusaurus source-file links to canonical published URLs
                // so the model never echoes a broken `.mdx` path.
                let sanitized = search::sanitize_docs_markdown_links(&page.body, &page.url);
                let rendered = search::render_docs_page_result(&sanitized, section.as_deref());
                let text = format!(
                    "Source page — cite this URL when referencing this page: {}\n\n{}",
                    page.url, rendered
                );
                PageResponse { text, source_url: page.url.clone() }
            }
            None => PageResponse {
                text: format!(
                    "No documentation page found for \"{}\". Use the docs search tool to find the correct Source URL first.",
                    url
                ),
                source_url: search::canonical_docs_page_url(&url),
            },
        }
    })
    .await
    .map_err(|e| windmill_common::error::Error::InternalErr(format!("docs page task: {e}")))?;

    Ok(Json(resp))
}
