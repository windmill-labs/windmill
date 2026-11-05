//! Web search and page fetching through a third-party search API, for models whose provider
//! has no built-in search. Every backend reaches the web from its own servers, so a URL a
//! model picks never makes Windmill itself open a connection to it.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use windmill_common::error::{Error, Result};

use crate::utils::AI_HTTP_CLIENT;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WebSearchProvider {
    Exa,
    Tavily,
    Linkup,
}

pub const MAX_SEARCH_RESULTS: usize = 10;
const DEFAULT_SEARCH_RESULTS: usize = 5;
/// Per result: enough for the passage that answers a query, short enough that ten
/// results stay well inside a tool result.
const SEARCH_CONTENT_CHARS: usize = 1_500;
const FETCH_CONTENT_CHARS: usize = 40_000;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(45);

#[derive(Serialize, Debug, PartialEq)]
pub struct WebSearchResult {
    pub title: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published_date: Option<String>,
    pub content: String,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct WebPage {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub content: String,
    pub truncated: bool,
}

impl WebSearchProvider {
    fn base_url(self) -> &'static str {
        match self {
            WebSearchProvider::Exa => "https://api.exa.ai",
            WebSearchProvider::Tavily => "https://api.tavily.com",
            WebSearchProvider::Linkup => "https://api.linkup.so/v1",
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            WebSearchProvider::Exa => "exa",
            WebSearchProvider::Tavily => "tavily",
            WebSearchProvider::Linkup => "linkup",
        }
    }

    fn name(self) -> &'static str {
        match self {
            WebSearchProvider::Exa => "Exa",
            WebSearchProvider::Tavily => "Tavily",
            WebSearchProvider::Linkup => "Linkup",
        }
    }

    async fn post(self, api_key: &str, path: &str, body: Value) -> Result<Value> {
        let request = AI_HTTP_CLIENT
            .post(format!("{}{path}", self.base_url()))
            .timeout(REQUEST_TIMEOUT)
            .json(&body);
        let request = match self {
            WebSearchProvider::Exa => request.header("x-api-key", api_key),
            WebSearchProvider::Tavily | WebSearchProvider::Linkup => request.bearer_auth(api_key),
        };
        let response = request
            .send()
            .await
            .map_err(|e| Error::BadGateway(format!("{} request failed: {e}", self.name())))?;
        let status = response.status();
        let text = response.text().await.map_err(|e| {
            Error::BadGateway(format!("{} response could not be read: {e}", self.name()))
        })?;
        if !status.is_success() {
            return Err(Error::BadGateway(format!(
                "{} returned {status}: {}",
                self.name(),
                truncate_chars(&text, 500).0
            )));
        }
        serde_json::from_str(&text)
            .map_err(|e| Error::BadGateway(format!("{} returned invalid JSON: {e}", self.name())))
    }

    pub async fn search(
        self,
        api_key: &str,
        query: &str,
        max_results: Option<usize>,
    ) -> Result<Vec<WebSearchResult>> {
        let n = max_results
            .unwrap_or(DEFAULT_SEARCH_RESULTS)
            .clamp(1, MAX_SEARCH_RESULTS);
        let (path, body) = match self {
            WebSearchProvider::Exa => (
                "/search",
                json!({
                    "query": query,
                    "numResults": n,
                    "contents": { "text": { "maxCharacters": SEARCH_CONTENT_CHARS } }
                }),
            ),
            WebSearchProvider::Tavily => (
                "/search",
                json!({ "query": query, "max_results": n, "include_published_date": true }),
            ),
            WebSearchProvider::Linkup => (
                "/search",
                json!({
                    "q": query,
                    "depth": "standard",
                    "outputType": "searchResults",
                    "maxResults": n
                }),
            ),
        };
        let response = self.post(api_key, path, body).await?;
        let mut results = parse_search(self, &response);
        results.truncate(n);
        Ok(results)
    }

    pub async fn fetch(self, api_key: &str, url: &str) -> Result<WebPage> {
        let (path, body) = match self {
            WebSearchProvider::Exa => (
                "/contents",
                json!({ "urls": [url], "text": { "maxCharacters": FETCH_CONTENT_CHARS + 1 } }),
            ),
            WebSearchProvider::Tavily => {
                ("/extract", json!({ "urls": [url], "format": "markdown" }))
            }
            WebSearchProvider::Linkup => ("/fetch", json!({ "url": url })),
        };
        let response = self.post(api_key, path, body).await?;
        parse_fetch(self, url, &response)
    }
}

fn str_field(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// The first `max` chars of `text`, and whether anything was cut.
fn truncate_chars(text: &str, max: usize) -> (String, bool) {
    match text.char_indices().nth(max) {
        Some((cut, _)) => (text[..cut].to_string(), true),
        None => (text.to_string(), false),
    }
}

fn parse_search(provider: WebSearchProvider, response: &Value) -> Vec<WebSearchResult> {
    let items = response
        .get("results")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    items
        .iter()
        .filter_map(|item| {
            let (title, content, published_date) = match provider {
                WebSearchProvider::Exa => (
                    str_field(item, "title"),
                    str_field(item, "text"),
                    str_field(item, "publishedDate"),
                ),
                WebSearchProvider::Tavily => (
                    str_field(item, "title"),
                    str_field(item, "content"),
                    str_field(item, "published_date"),
                ),
                WebSearchProvider::Linkup => {
                    if item.get("type").and_then(Value::as_str) == Some("image") {
                        return None;
                    }
                    (str_field(item, "name"), str_field(item, "content"), None)
                }
            };
            let url = str_field(item, "url")?;
            Some(WebSearchResult {
                title: title.unwrap_or_else(|| url.clone()),
                published_date,
                content: truncate_chars(&content.unwrap_or_default(), SEARCH_CONTENT_CHARS).0,
                url,
            })
        })
        .collect()
}

fn parse_fetch(provider: WebSearchProvider, url: &str, response: &Value) -> Result<WebPage> {
    let not_fetched = |reason: String| {
        Error::BadGateway(format!(
            "{} could not fetch {url}: {reason}",
            provider.name()
        ))
    };
    let first_result = || {
        response
            .get("results")
            .and_then(Value::as_array)
            .and_then(|r| r.first())
    };
    let (title, content) = match provider {
        WebSearchProvider::Exa => {
            let failed = response
                .get("statuses")
                .and_then(Value::as_array)
                .and_then(|s| s.first())
                .filter(|s| s.get("status").and_then(Value::as_str) == Some("error"));
            if let Some(status) = failed {
                let tag = status
                    .pointer("/error/tag")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown error");
                return Err(not_fetched(tag.to_string()));
            }
            let result = first_result().ok_or_else(|| not_fetched("no content".into()))?;
            (str_field(result, "title"), str_field(result, "text"))
        }
        WebSearchProvider::Tavily => match first_result() {
            Some(result) => (None, str_field(result, "raw_content")),
            None => {
                let reason = response
                    .pointer("/failed_results/0/error")
                    .and_then(Value::as_str)
                    .unwrap_or("no content");
                return Err(not_fetched(reason.to_string()));
            }
        },
        WebSearchProvider::Linkup => (None, str_field(response, "markdown")),
    };
    let content = content.ok_or_else(|| not_fetched("the page has no readable text".into()))?;
    let (content, truncated) = truncate_chars(&content, FETCH_CONTENT_CHARS);
    Ok(WebPage { url: url.to_string(), title, content, truncated })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_each_provider_search_shape() {
        let exa = json!({ "results": [
            { "title": "Exa", "url": "https://a", "publishedDate": "2026-01-01", "text": "body" },
            { "title": "no url" }
        ]});
        assert_eq!(
            parse_search(WebSearchProvider::Exa, &exa),
            vec![WebSearchResult {
                title: "Exa".into(),
                url: "https://a".into(),
                published_date: Some("2026-01-01".into()),
                content: "body".into(),
            }]
        );

        let linkup = json!({ "results": [
            { "type": "image", "name": "img", "url": "https://img" },
            { "type": "text", "name": "Linkup", "url": "https://b", "content": "text" }
        ]});
        let parsed = parse_search(WebSearchProvider::Linkup, &linkup);
        assert_eq!(parsed.len(), 1);
        assert_eq!(
            (parsed[0].title.as_str(), parsed[0].url.as_str()),
            ("Linkup", "https://b")
        );

        let tavily = json!({ "results": [
            { "title": "", "url": "https://c", "content": "é".repeat(SEARCH_CONTENT_CHARS + 10) }
        ]});
        let parsed = parse_search(WebSearchProvider::Tavily, &tavily);
        assert_eq!(parsed[0].title, "https://c");
        assert_eq!(parsed[0].content.chars().count(), SEARCH_CONTENT_CHARS);
    }

    #[test]
    fn reports_a_page_the_backend_could_not_fetch() {
        let exa = json!({ "results": [], "statuses": [
            { "id": "https://x", "status": "error", "error": { "tag": "CRAWL_NOT_FOUND" } }
        ]});
        let err = parse_fetch(WebSearchProvider::Exa, "https://x", &exa).unwrap_err();
        assert!(err.to_string().contains("CRAWL_NOT_FOUND"), "{err}");

        let tavily = json!({ "results": [], "failed_results": [{ "url": "https://x", "error": "blocked" }] });
        let err = parse_fetch(WebSearchProvider::Tavily, "https://x", &tavily).unwrap_err();
        assert!(err.to_string().contains("blocked"), "{err}");

        let linkup = json!({ "markdown": "a".repeat(FETCH_CONTENT_CHARS + 1) });
        let page = parse_fetch(WebSearchProvider::Linkup, "https://x", &linkup).unwrap();
        assert!(page.truncated);
        assert_eq!(page.content.len(), FETCH_CONTENT_CHARS);
    }
}
