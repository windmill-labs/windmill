//! Documentation search & page rendering.
//!
//! Direct port of the pure functions in the frontend's
//! `copilot/chat/docs/core.ts`, so the AI chat, the MCP `searchDocs`/`readDocsPage`
//! tools and the `wmill docs` CLI all return identical results from one place.
//! Operates over the vendored corpus parsed in [`super::corpus`].

use lazy_static::lazy_static;
use regex::Regex;
use serde::Serialize;
use std::collections::HashSet;

pub const DOCS_ORIGIN: &str = "https://www.windmill.dev";

// Above this size, return an outline of the page's headings instead of the full
// content, prompting the model to request a specific section.
const FULL_PAGE_CHAR_LIMIT: usize = 20_000;

// search result caps — keep the returned payload small (the whole point of search
// vs. dumping the index or full pages is token economy).
pub const SEARCH_MAX_PAGES: usize = 8;
const SEARCH_MAX_SNIPPETS_PER_PAGE: usize = 3;
const SEARCH_MAX_SNIPPET_CHARS: usize = 200;
// Each distinct query term triggers a full-corpus scan; cap it so a long,
// caller-controlled query can't multiply the scan cost without bound. Real
// queries are a handful of keywords, so this never truncates a useful search.
const MAX_QUERY_TERMS: usize = 24;

// ---------------------------------------------------------------------------
// Corpus record types
// ---------------------------------------------------------------------------

/// A single page extracted from llms-full.txt, keyed by its `Source:` URL.
pub struct DocsFullPage {
    pub url: String,
    pub title: String,
    pub body: String,
    /// `body` lowercased once at parse time, so the per-query full-corpus scan
    /// doesn't re-allocate a lowercase copy of every page on each request.
    pub body_lower: String,
}

/// A line of the llms.txt index: title, URL and one-line description.
pub struct DocsIndexEntry {
    pub title: String,
    pub url: String,
    pub description: String,
    /// `title`/`description` lowercased once at parse time (same rationale as
    /// `DocsFullPage::body_lower`).
    pub title_lower: String,
    pub description_lower: String,
}

#[derive(Serialize, Clone)]
pub struct DocsSearchResult {
    pub url: String,
    pub title: String,
    /// Higher = more relevant. Distinct query terms matched dominate raw occurrences.
    pub score: i64,
    pub snippets: Vec<String>,
}

// ---------------------------------------------------------------------------
// Corpus parsing
// ---------------------------------------------------------------------------

lazy_static! {
    // In llms-full.txt every page's `Source:` line is preceded by a category-header
    // lead-in: `...page body...\n\n---\n\n## <Category>\n\nSource: <url>`. Splitting
    // on `Source:` lines leaves that lead-in on the *previous* page, so strip a
    // trailing `---` + level-2-heading block to avoid mis-attributing the next
    // page's category title to the previous page.
    static ref TRAILING_LEAD_IN_RE: Regex =
        Regex::new(r"\n+-{3,}[ \t]*\n+#{2}[ \t]+.*[ \t]*\n*$").unwrap();
    // A line in llms.txt: `- [Title](https://.../page.md): question-phrased description`.
    static ref INDEX_ENTRY_RE: Regex =
        Regex::new(r"^\s*-\s*\[([^\]]+)\]\(([^)\s]+)\)\s*:?\s*(.*)$").unwrap();
    static ref ORDERING_PREFIX_RE: Regex = Regex::new(r"^\d+[_-]").unwrap();
    // Markdown inline link `](target "optional title")`.
    static ref MD_LINK_RE: Regex = Regex::new(r#"\]\(([^)\s]+?)(\s+"[^"]*")?\)"#).unwrap();
}

/// `^Source:\s*(\S+)\s*$` — returns the single non-whitespace URL token.
fn parse_source_line(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("Source:")?;
    let trimmed = rest.trim();
    if trimmed.is_empty() || trimmed.contains(char::is_whitespace) {
        return None;
    }
    Some(trimmed)
}

/// Splits the llms-full.txt corpus into per-page records keyed by the `Source:`
/// URL. Content before the first `Source:` line (the corpus preamble) is dropped.
pub fn parse_docs_full_text(full_text: &str) -> Vec<DocsFullPage> {
    let mut pages = Vec::new();
    let mut url: Option<String> = None;
    let mut buffer: Vec<&str> = Vec::new();

    for line in full_text.split('\n') {
        if let Some(u) = parse_source_line(line) {
            flush_page(&mut pages, &url, &buffer);
            url = Some(u.to_string());
            buffer.clear();
            continue;
        }
        if url.is_some() {
            buffer.push(line);
        }
    }
    flush_page(&mut pages, &url, &buffer);
    pages
}

fn flush_page(pages: &mut Vec<DocsFullPage>, url: &Option<String>, buffer: &[&str]) {
    let Some(url) = url else {
        return;
    };
    let joined = buffer.join("\n");
    let body = TRAILING_LEAD_IN_RE.replace(&joined, "");
    let body = body.trim();
    if !body.is_empty() {
        let title = first_heading(body).unwrap_or_else(|| url.clone());
        let body = body.to_string();
        let body_lower = body.to_lowercase();
        pages.push(DocsFullPage { url: url.clone(), title, body, body_lower });
    }
}

fn first_heading(body: &str) -> Option<String> {
    for line in body.split('\n') {
        if let Some((_, title)) = parse_heading_line(line, 6) {
            return Some(title);
        }
    }
    None
}

/// Parses the llms.txt index into per-page entries (title, URL, description).
pub fn parse_docs_index(index_text: &str) -> Vec<DocsIndexEntry> {
    let mut entries = Vec::new();
    for line in index_text.split('\n') {
        if let Some(caps) = INDEX_ENTRY_RE.captures(line) {
            let url = caps[2].trim();
            if !url.contains("/docs/") {
                continue;
            }
            let title = caps[1].trim().to_string();
            let description = caps[3].trim().to_string();
            entries.push(DocsIndexEntry {
                title_lower: title.to_lowercase(),
                description_lower: description.to_lowercase(),
                title,
                url: url.to_string(),
                description,
            });
        }
    }
    entries
}

// ---------------------------------------------------------------------------
// Headings, outline, section extraction
// ---------------------------------------------------------------------------

pub struct DocsHeading {
    pub level: usize,
    pub title: String,
    /// Byte offset of the start of the heading line within the document.
    pub start_index: usize,
}

/// `^(#{1,max_level})\s+(.*\S)\s*$` — heading level and trimmed title.
fn parse_heading_line(line: &str, max_level: usize) -> Option<(usize, String)> {
    let hashes = line.bytes().take_while(|&b| b == b'#').count();
    if hashes < 1 || hashes > max_level {
        return None;
    }
    let rest = &line[hashes..];
    // require at least one whitespace after the hashes (\s+)
    if !rest.starts_with(|c: char| c.is_whitespace()) {
        return None;
    }
    let title = rest.trim();
    if title.is_empty() {
        return None;
    }
    Some((hashes, title.to_string()))
}

fn match_fence(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let first = trimmed.chars().next()?;
    if first != '`' && first != '~' {
        return None;
    }
    let count = trimmed.chars().take_while(|&c| c == first).count();
    if count >= 3 {
        Some(std::iter::repeat(first).take(count).collect())
    } else {
        None
    }
}

/// Parses the markdown headings (`#`–`####`) of a docs page, ignoring any
/// heading-like lines inside fenced code blocks (which are common in samples).
pub fn parse_docs_headings(content: &str) -> Vec<DocsHeading> {
    let mut headings = Vec::new();
    let mut offset = 0usize;
    let mut fence_marker: Option<String> = None;

    for line in content.split('\n') {
        if let Some(fence) = match_fence(line) {
            match &fence_marker {
                None => fence_marker = Some(fence),
                Some(marker) if line.trim_start().starts_with(marker.as_str()) => {
                    fence_marker = None;
                }
                _ => {}
            }
            offset += line.len() + 1;
            continue;
        }

        if fence_marker.is_none() {
            if let Some((level, title)) = parse_heading_line(line, 4) {
                headings.push(DocsHeading { level, title, start_index: offset });
            }
        }
        offset += line.len() + 1;
    }
    headings
}

fn section_end_index(content: &str, headings: &[DocsHeading], index: usize) -> usize {
    let level = headings[index].level;
    // A section ends at the next heading of the same or higher (shallower) level.
    for h in &headings[index + 1..] {
        if h.level <= level {
            return h.start_index;
        }
    }
    content.len()
}

/// Builds a human-readable outline of a page's headings, with an approximate
/// size for each section. Used when a page is too large to return whole.
pub fn build_docs_outline(content: &str) -> String {
    let headings = parse_docs_headings(content);
    if headings.is_empty() {
        return "(no markdown headings found on this page)".to_string();
    }
    headings
        .iter()
        .enumerate()
        .map(|(i, h)| {
            let end = section_end_index(content, &headings, i);
            let approx = end.saturating_sub(h.start_index);
            let indent = "  ".repeat(h.level.saturating_sub(1));
            format!("{}- {} (~{} chars)", indent, h.title, approx)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Normalizes a heading title for tolerant, case/punctuation-insensitive matching.
fn normalize_heading_title(title: &str) -> String {
    // toLowerCase, replace /[^a-z0-9]+/g with ' ', trim
    let mut out = String::new();
    let mut prev_space = false;
    for ch in title.to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            prev_space = false;
        } else if !prev_space {
            out.push(' ');
            prev_space = true;
        }
    }
    out.trim().to_string()
}

/// Extracts the content of the section whose heading matches `section` (from the
/// heading up to the next heading of the same/higher level). Case-insensitive and
/// tolerant of minor punctuation differences. `None` when no heading matches.
pub fn extract_docs_section(content: &str, section: &str) -> Option<String> {
    let headings = parse_docs_headings(content);
    let target = normalize_heading_title(section);
    if target.is_empty() {
        return None;
    }
    let match_index = headings
        .iter()
        .position(|h| normalize_heading_title(&h.title) == target)
        .or_else(|| {
            // Fall back to a contains match so "Result streaming" matches "Result".
            headings
                .iter()
                .position(|h| normalize_heading_title(&h.title).contains(&target))
        })?;

    let start = headings[match_index].start_index;
    let end = section_end_index(content, &headings, match_index);
    Some(content[start..end].trim().to_string())
}

/// Decides what to return for read_docs_page: a requested section, the full page,
/// or an outline asking the model to pick a section.
pub fn render_docs_page_result(content: &str, section: Option<&str>) -> String {
    if let Some(section) = section {
        if let Some(extracted) = extract_docs_section(content, section) {
            return extracted;
        }
        return format!(
            "No section matching \"{}\" was found on this page. Available sections:\n\n{}",
            section,
            build_docs_outline(content)
        );
    }

    if content.len() <= FULL_PAGE_CHAR_LIMIT {
        return content.to_string();
    }

    format!(
        "This documentation page is large. Below is its list of sections with approximate sizes.\n\
         Call the docs page-reading tool again with the same `url` argument and a `section` set to one of these headings to read that section.\n\n{}",
        build_docs_outline(content)
    )
}

// ---------------------------------------------------------------------------
// Ranking
// ---------------------------------------------------------------------------

/// Splits a query into distinct, lowercased, non-empty terms (insertion order),
/// capped at `MAX_QUERY_TERMS`.
fn tokenize_query(query: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for term in query.to_lowercase().split_whitespace() {
        if seen.insert(term.to_string()) {
            out.push(term.to_string());
            if out.len() >= MAX_QUERY_TERMS {
                break;
            }
        }
    }
    out
}

fn count_occurrences(haystack: &str, needle: &str) -> usize {
    if needle.is_empty() {
        return 0;
    }
    haystack.matches(needle).count()
}

struct Scored {
    res: DocsSearchResult,
    distinct_terms: usize,
    order: usize,
}

/// Prefer pages that cover every query term; sort by score desc then input order;
/// take the top `max_pages`.
fn finalize_pool(mut scored: Vec<Scored>, term_count: usize, max_pages: usize) -> Vec<DocsSearchResult> {
    let has_full = scored.iter().any(|s| s.distinct_terms == term_count);
    if has_full {
        scored.retain(|s| s.distinct_terms == term_count);
    }
    scored.sort_by(|a, b| b.res.score.cmp(&a.res.score).then(a.order.cmp(&b.order)));
    scored.into_iter().take(max_pages).map(|s| s.res).collect()
}

/// Ranks docs pages for a keyword query. Score is `distinctTermsMatched`
/// (dominant) then total occurrences. Each result carries up to
/// `SEARCH_MAX_SNIPPETS_PER_PAGE` of its most term-dense lines.
pub fn search_docs_pages(pages: &[DocsFullPage], query: &str, max_pages: usize) -> Vec<DocsSearchResult> {
    let terms = tokenize_query(query);
    if terms.is_empty() {
        return Vec::new();
    }

    let mut scored = Vec::new();
    for (order, page) in pages.iter().enumerate() {
        let mut distinct = 0usize;
        let mut occurrences = 0usize;
        for term in &terms {
            let count = count_occurrences(&page.body_lower, term);
            if count > 0 {
                distinct += 1;
                occurrences += count;
            }
        }
        if distinct == 0 {
            continue;
        }
        scored.push(Scored {
            res: DocsSearchResult {
                url: page.url.clone(),
                title: page.title.clone(),
                // distinctTerms dominates so a page matching all terms always
                // outranks one matching fewer, regardless of raw occurrences.
                score: distinct as i64 * 1_000_000 + occurrences as i64,
                snippets: select_snippets(
                    &page.body,
                    &terms,
                    SEARCH_MAX_SNIPPETS_PER_PAGE,
                    SEARCH_MAX_SNIPPET_CHARS,
                ),
            },
            distinct_terms: distinct,
            order,
        });
    }
    finalize_pool(scored, terms.len(), max_pages)
}

/// Ranks index entries by matching query terms against each entry's title and
/// description (title matches weigh more). The description becomes the result's
/// single snippet. Recovers "named feature" discovery that full-text grep misses.
pub fn search_docs_index(entries: &[DocsIndexEntry], query: &str, max_pages: usize) -> Vec<DocsSearchResult> {
    let terms = tokenize_query(query);
    if terms.is_empty() {
        return Vec::new();
    }

    let mut scored = Vec::new();
    for (order, entry) in entries.iter().enumerate() {
        let mut distinct = 0usize;
        let mut score = 0i64;
        for term in &terms {
            let in_title = entry.title_lower.contains(term.as_str());
            let in_desc = entry.description_lower.contains(term.as_str());
            if in_title || in_desc {
                distinct += 1;
                score += if in_title { 5 } else { 0 } + if in_desc { 1 } else { 0 };
            }
        }
        if distinct == 0 {
            continue;
        }
        scored.push(Scored {
            res: DocsSearchResult {
                url: entry.url.clone(),
                title: entry.title.clone(),
                score: distinct as i64 * 1_000_000 + score,
                snippets: if entry.description.is_empty() {
                    Vec::new()
                } else {
                    vec![entry.description.clone()]
                },
            },
            distinct_terms: distinct,
            order,
        });
    }
    finalize_pool(scored, terms.len(), max_pages)
}

/// Picks the most term-dense lines of a page body as snippets, in document order,
/// deduped, each trimmed to `max_chars` around the first matched term.
fn select_snippets(body: &str, terms: &[String], max_snippets: usize, max_chars: usize) -> Vec<String> {
    struct LineHit {
        text: String,
        distinct: usize,
        order: usize,
    }
    let mut hits = Vec::new();
    for (order, line) in body.split('\n').enumerate() {
        let lower = line.to_lowercase();
        let distinct = terms.iter().filter(|t| lower.contains(t.as_str())).count();
        if distinct == 0 {
            continue;
        }
        let text = make_snippet(line, terms, max_chars);
        if !text.is_empty() {
            hits.push(LineHit { text, distinct, order });
        }
    }
    hits.sort_by(|a, b| b.distinct.cmp(&a.distinct).then(a.order.cmp(&b.order)));

    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for hit in hits {
        if !seen.insert(hit.text.clone()) {
            continue;
        }
        result.push(hit.text);
        if result.len() >= max_snippets {
            break;
        }
    }
    result
}

/// Collapses a matched line to a single-line snippet of at most `max_chars`,
/// windowed around the first matched term (with ellipses) when the line is long.
/// Operates on `char`s so multibyte content can't split mid-codepoint.
fn make_snippet(line: &str, terms: &[String], max_chars: usize) -> String {
    let collapsed = line.split_whitespace().collect::<Vec<_>>().join(" ");
    let chars: Vec<char> = collapsed.chars().collect();
    if chars.len() <= max_chars {
        return collapsed;
    }

    let lower = collapsed.to_lowercase();
    let mut first_index: Option<usize> = None;
    for term in terms {
        if let Some(byte_idx) = lower.find(term.as_str()) {
            let char_idx = lower[..byte_idx].chars().count();
            first_index = Some(first_index.map_or(char_idx, |f| f.min(char_idx)));
        }
    }

    let total = chars.len();
    match first_index {
        None => {
            let slice: String = chars[..max_chars].iter().collect();
            format!("{}…", slice.trim_end())
        }
        Some(fi) => {
            let start = fi.saturating_sub(max_chars / 3);
            let end = (start + max_chars).min(total);
            let prefix = if start > 0 { "…" } else { "" };
            let suffix = if end < total { "…" } else { "" };
            let slice: String = chars[start..end].iter().collect();
            format!("{}{}{}", prefix, slice.trim(), suffix)
        }
    }
}

/// Strips the `.md` suffix and trailing slash so index/body URLs dedupe.
pub fn canonical_search_url(url: &str) -> String {
    let stripped = strip_md_suffix(url);
    stripped.strip_suffix('/').unwrap_or(stripped).to_string()
}

fn strip_md_suffix(s: &str) -> &str {
    if s.len() >= 3 && s[s.len() - 3..].eq_ignore_ascii_case(".md") {
        &s[..s.len() - 3]
    } else {
        s
    }
}

/// Merges full-text (body) results with index-description results. Body matches
/// come first; index-only matches fill remaining slots — so a named feature
/// surfaced only by its index entry still appears even when body grep missed it.
pub fn merge_docs_search_results(
    body_results: Vec<DocsSearchResult>,
    index_results: Vec<DocsSearchResult>,
    max_pages: usize,
) -> Vec<DocsSearchResult> {
    let mut seen: HashSet<String> =
        body_results.iter().map(|r| canonical_search_url(&r.url)).collect();
    let mut merged = body_results;
    for entry in index_results {
        let key = canonical_search_url(&entry.url);
        if seen.insert(key) {
            merged.push(entry);
        }
    }
    merged.truncate(max_pages);
    merged
}

/// Renders search results as the string returned to the model.
pub fn format_docs_search_results(query: &str, results: &[DocsSearchResult]) -> String {
    if results.is_empty() {
        return format!(
            "No documentation pages matched \"{}\". Try fewer or more general keywords (a single distinctive term often works best).",
            query
        );
    }

    let blocks = results
        .iter()
        .map(|r| {
            let mut lines = vec![format!("## {}", r.title), format!("Source: {}", r.url)];
            for snippet in &r.snippets {
                lines.push(format!("  - {}", snippet));
            }
            lines.join("\n")
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    format!(
        "Found {} documentation page(s) matching \"{}\", most relevant first:\n\n{}\n\n\
         Cite the exact \"Source\" URL when referencing a page. If these snippets are not enough, call the docs page-reading tool with a Source URL as its `url` argument to read the full page or a section.",
        results.len(),
        query,
        blocks
    )
}

// ---------------------------------------------------------------------------
// URL normalization & link sanitization
// ---------------------------------------------------------------------------

/// Strips docusaurus numeric ordering prefixes (`13_`, `8-`) from each path
/// segment so it matches the published route.
fn strip_docs_path_prefixes(path: &str) -> String {
    path.split('/')
        .map(|seg| ORDERING_PREFIX_RE.replace(seg, "").into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// Normalizes a user/model-supplied docs reference to a fully-qualified `.md` URL
/// on the docs origin. Accepts a full URL, `/docs/...`, or `docs/...`.
pub fn normalize_docs_url(input: &str) -> String {
    let mut value = input.trim().to_string();

    if is_http_url(&value) {
        // Strip the origin so we re-anchor to DOCS_ORIGIN and normalize the path.
        if let Ok(parsed) = url::Url::parse(&value) {
            value = parsed.path().to_string();
        }
    }

    // Drop any query string or hash fragment.
    value = value.split('#').next().unwrap().split('?').next().unwrap().to_string();

    if !value.starts_with('/') {
        value = format!("/{}", value);
    }
    // Strip a trailing slash (but keep the leading one).
    if value.len() > 1 && value.ends_with('/') {
        value.pop();
    }

    value = strip_docs_path_prefixes(&value);
    if let Some(stripped) = value.strip_suffix(".mdx") {
        value = format!("{}.md", stripped);
    }
    if !value.ends_with(".md") {
        value = format!("{}.md", value);
    }

    format!("{}{}", DOCS_ORIGIN, value)
}

/// The canonical published URL a model should cite for a docs page (the `.md`
/// fetch URL without the suffix).
pub fn canonical_docs_page_url(path: &str) -> String {
    strip_md_suffix(&normalize_docs_url(path)).to_string()
}

fn is_http_url(s: &str) -> bool {
    let lower = s.to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

/// Rewrites relative/source-file doc links inside raw page markdown to canonical
/// published URLs, so the model never echoes a docusaurus source path into its
/// answer as a broken link. Non-doc links (external, images, anchors) and `../`
/// cross-directory links are left untouched.
pub fn sanitize_docs_markdown_links(content: &str, page_url: &str) -> String {
    let base = url::Url::parse(page_url).ok();
    MD_LINK_RE
        .replace_all(content, |caps: &regex::Captures| {
            let whole = caps.get(0).unwrap().as_str();
            let target = &caps[1];
            let title = caps.get(2).map(|m| m.as_str()).unwrap_or("");

            // Only rewrite links to docusaurus source files (.md/.mdx); leave
            // images, external URLs and bare anchors untouched.
            if !is_md_target(target) {
                return whole.to_string();
            }
            // `../` cross-directory links are authored against the docusaurus
            // source tree, whose depth differs from the published URL, so strict
            // resolution is unreliable. Leave them for the canonical-URL header.
            if has_parent_traversal(target) {
                return whole.to_string();
            }
            let Some(base) = &base else {
                return whole.to_string();
            };
            let Ok(resolved) = base.join(target) else {
                return whole.to_string();
            };
            if resolved.scheme() != "https"
                || resolved.host_str() != Some("www.windmill.dev")
                || !resolved.path().starts_with("/docs/")
            {
                return whole.to_string();
            }
            let pathname = strip_docs_path_prefixes(resolved.path());
            let pathname = strip_md_or_mdx(&pathname);
            let hash = resolved.fragment().map(|f| format!("#{}", f)).unwrap_or_default();
            format!("]({}{}{}{})", DOCS_ORIGIN, pathname, hash, title)
        })
        .into_owned()
}

/// `\.mdx?($|[#?])` — the target points at a markdown source file.
fn is_md_target(target: &str) -> bool {
    for marker in [".md", ".mdx"] {
        if let Some(idx) = target.to_ascii_lowercase().find(marker) {
            let after = &target[idx + marker.len()..];
            // `.md` must not be a prefix of `.mdx` here: only accept when the
            // extension is followed by end / `#` / `?`.
            if after.is_empty() || after.starts_with('#') || after.starts_with('?') {
                return true;
            }
        }
    }
    false
}

/// `(^|/)\.\./` — the target contains a parent-directory traversal segment.
fn has_parent_traversal(target: &str) -> bool {
    target == ".." || target.starts_with("../") || target.contains("/../")
}

fn strip_md_or_mdx(path: &str) -> &str {
    if let Some(stripped) = path.strip_suffix(".mdx") {
        stripped
    } else {
        strip_md_suffix(path)
    }
}

#[cfg(test)]
mod tests;
