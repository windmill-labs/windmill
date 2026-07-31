//! Parity tests ported from the frontend `copilot/chat/docs/core.test.ts`.

use super::*;

const SAMPLE: &str = "# Jobs\n\nIntro text about jobs.\n\n## Job kinds\n\nSome kinds.\n\n## Result\n\n### Result of jobs that failed\n\n```\n{ \"error\": \"boom\" }\n```\n\n### Result streaming\n\n#### Returning a stream directly\n\n```python\n# Returning a stream directly is a comment heading that must be ignored\ndef main():\n    pass\n```\n\n## Retention policy\n\nFinal section.\n";

// Mirrors the llms-full.txt layout: a corpus preamble, then per-page blocks each
// introduced by a `---` + `## <Category>` lead-in followed by a `Source:` line.
const SAMPLE_FULL: &str = "# Windmill\n\n> Preamble blurb that precedes the first Source line and must be ignored.\n\n## Browser automation\n\nSource: https://www.windmill.dev/docs/advanced/browser_automation\n\n# Browser automation\n\nBy default, a worker group named `reports` handles jobs with the `chromium` tag.\nThe chromium binary will be available on these workers at /usr/bin/chromium.\nYou can disable the sandbox by passing the --no-sandbox flag.\n\n---\n\n## Worker groups\n\nSource: https://www.windmill.dev/docs/core_concepts/worker_groups\n\n# Worker groups\n\nWorker groups let you assign tags to workers.\nSet the chromium tag on a worker so it can run browser jobs.\n\n---\n\n## Scheduling\n\nSource: https://www.windmill.dev/docs/core_concepts/scheduling\n\n# Scheduling\n\nUse cron expressions to schedule scripts and flows.\n";

#[test]
fn parses_headings_and_ignores_fenced_blocks() {
    let titles: Vec<String> = parse_docs_headings(SAMPLE)
        .iter()
        .map(|h| format!("{}:{}", h.level, h.title))
        .collect();
    assert_eq!(
        titles,
        vec![
            "1:Jobs",
            "2:Job kinds",
            "2:Result",
            "3:Result of jobs that failed",
            "3:Result streaming",
            "4:Returning a stream directly",
            "2:Retention policy",
        ]
    );
}

#[test]
fn heading_start_index_points_at_the_heading_line() {
    for h in parse_docs_headings(SAMPLE) {
        let at = &SAMPLE[h.start_index..];
        assert!(at.starts_with(&"#".repeat(h.level)));
        assert!(at[h.level..].trim_start().starts_with(&h.title));
    }
}

#[test]
fn handles_tilde_fences() {
    let content = "# Title\n\n~~~\n# not a heading\n~~~\n\n## Real\n";
    let titles: Vec<String> = parse_docs_headings(content).iter().map(|h| h.title.clone()).collect();
    assert_eq!(titles, vec!["Title", "Real"]);
}

#[test]
fn extracts_section_up_to_next_same_or_higher_heading() {
    let section = extract_docs_section(SAMPLE, "Result").unwrap();
    assert!(section.contains("## Result"));
    assert!(section.contains("### Result of jobs that failed"));
    assert!(section.contains("### Result streaming"));
    assert!(!section.contains("## Retention policy"));
}

#[test]
fn extract_section_is_case_and_punctuation_tolerant() {
    let section = extract_docs_section(SAMPLE, "retention-policy!").unwrap();
    assert!(section.contains("## Retention policy"));
    assert!(section.contains("Final section."));
}

#[test]
fn extract_section_returns_none_when_missing() {
    assert!(extract_docs_section(SAMPLE, "Nonexistent section").is_none());
}

#[test]
fn build_outline_lists_headings_with_indent() {
    let outline = build_docs_outline(SAMPLE);
    assert!(outline.contains("- Jobs (~"));
    assert!(outline.contains("  - Job kinds (~"));
    assert!(outline.contains("    - Result of jobs that failed (~"));
}

#[test]
fn build_outline_handles_no_headings() {
    assert_eq!(
        build_docs_outline("just some text\nwith no headings"),
        "(no markdown headings found on this page)"
    );
}

#[test]
fn render_page_returns_whole_small_page() {
    assert_eq!(render_docs_page_result(SAMPLE, None), SAMPLE);
}

#[test]
fn render_page_returns_outline_for_large_page() {
    let large = format!("# Big\n\n{}\n\n## Tail\n\nmore", "x".repeat(25_000));
    let result = render_docs_page_result(&large, None);
    assert!(result.contains("This documentation page is large"));
    assert!(result.contains("same `url` argument"));
    assert!(!result.contains("same path"));
    assert!(!result.contains("read_docs_page"));
    assert!(result.contains("- Big (~"));
    assert!(result.contains("- Tail (~"));
}

#[test]
fn render_page_returns_requested_section() {
    let result = render_docs_page_result(SAMPLE, Some("Job kinds"));
    assert!(result.contains("## Job kinds"));
    assert!(result.contains("Some kinds."));
}

#[test]
fn render_page_missing_section_returns_outline_note() {
    let result = render_docs_page_result(SAMPLE, Some("Does not exist"));
    assert!(result.contains("No section matching \"Does not exist\" was found"));
    assert!(result.contains("- Jobs (~"));
}

#[test]
fn normalize_docs_url_cases() {
    assert_eq!(normalize_docs_url("/docs/core_concepts/jobs"), "https://www.windmill.dev/docs/core_concepts/jobs.md");
    assert_eq!(normalize_docs_url("docs/core_concepts/jobs"), "https://www.windmill.dev/docs/core_concepts/jobs.md");
    assert_eq!(
        normalize_docs_url("https://www.windmill.dev/docs/core_concepts/jobs#result?foo=bar"),
        "https://www.windmill.dev/docs/core_concepts/jobs.md"
    );
    assert_eq!(normalize_docs_url("/docs/core_concepts/jobs.md"), "https://www.windmill.dev/docs/core_concepts/jobs.md");
    assert_eq!(normalize_docs_url("/docs/core_concepts/jobs/"), "https://www.windmill.dev/docs/core_concepts/jobs.md");
    assert_eq!(normalize_docs_url("/docs/flows/13_flow_branches"), "https://www.windmill.dev/docs/flows/flow_branches.md");
    assert_eq!(normalize_docs_url("/docs/flows/13_flow_branches.mdx"), "https://www.windmill.dev/docs/flows/flow_branches.md");
}

#[test]
fn canonical_docs_page_url_cases() {
    assert_eq!(canonical_docs_page_url("/docs/flows/flow_editor"), "https://www.windmill.dev/docs/flows/flow_editor");
    assert_eq!(canonical_docs_page_url("/docs/flows/14_retries.md"), "https://www.windmill.dev/docs/flows/retries");
}

#[test]
fn sanitize_markdown_links_cases() {
    let page = "https://www.windmill.dev/docs/flows/flow_editor.md";
    assert_eq!(
        sanitize_docs_markdown_links("See [retries](./14_retries.mdx) for more.", page),
        "See [retries](https://www.windmill.dev/docs/flows/retries) for more."
    );
    assert_eq!(
        sanitize_docs_markdown_links("[handling](./8_error_handling.mdx)", page),
        "[handling](https://www.windmill.dev/docs/flows/error_handling)"
    );
    assert_eq!(
        sanitize_docs_markdown_links("[branch all](./13_flow_branches.mdx#branch-all)", page),
        "[branch all](https://www.windmill.dev/docs/flows/flow_branches#branch-all)"
    );
    // images & external links untouched
    let external = "![diagram](./assets/flow_example.png) and [site](https://example.com/page.md)";
    assert_eq!(sanitize_docs_markdown_links(external, page), external);
    // bare anchor untouched
    assert_eq!(sanitize_docs_markdown_links("[top](#introduction)", page), "[top](#introduction)");
    // ../ cross-directory links untouched
    let parent = "[handling](../core_concepts/8_error_handling.mdx)";
    assert_eq!(sanitize_docs_markdown_links(parent, page), parent);
    let parent2 = "[retries](../../flows/14_retries.md)";
    assert_eq!(sanitize_docs_markdown_links(parent2, page), parent2);
}

#[test]
fn parse_full_text_splits_pages_and_drops_preamble() {
    let pages = parse_docs_full_text(SAMPLE_FULL);
    assert_eq!(
        pages.iter().map(|p| p.url.clone()).collect::<Vec<_>>(),
        vec![
            "https://www.windmill.dev/docs/advanced/browser_automation",
            "https://www.windmill.dev/docs/core_concepts/worker_groups",
            "https://www.windmill.dev/docs/core_concepts/scheduling",
        ]
    );
    assert_eq!(
        pages.iter().map(|p| p.title.clone()).collect::<Vec<_>>(),
        vec!["Browser automation", "Worker groups", "Scheduling"]
    );
    // The next page's "## Worker groups" lead-in must not leak into this body.
    let browser = pages.iter().find(|p| p.url.ends_with("/browser_automation")).unwrap();
    assert!(!browser.body.contains("Worker groups"));
    assert!(!browser.body.contains("---"));
}

#[test]
fn search_pages_ranks_more_occurrences_first() {
    let pages = parse_docs_full_text(SAMPLE_FULL);
    let results = search_docs_pages(&pages, "chromium", 5);
    assert_eq!(
        results.iter().map(|r| r.url.clone()).collect::<Vec<_>>(),
        vec![
            "https://www.windmill.dev/docs/advanced/browser_automation",
            "https://www.windmill.dev/docs/core_concepts/worker_groups",
        ]
    );
    assert!(!results[0].snippets.is_empty());
    assert!(results[0].snippets.join("\n").contains("chromium"));
}

#[test]
fn search_prefers_pages_covering_all_terms() {
    let pages = parse_docs_full_text(SAMPLE_FULL);
    // Only browser_automation contains "sandbox"; worker_groups has "chromium" but
    // not "sandbox". A full-coverage page exists, so partial matches are dropped.
    let results = search_docs_pages(&pages, "chromium sandbox", 5);
    assert_eq!(
        results.iter().map(|r| r.url.clone()).collect::<Vec<_>>(),
        vec!["https://www.windmill.dev/docs/advanced/browser_automation"]
    );
}

#[test]
fn merge_dedupes_index_against_body_by_canonical_url() {
    let body = vec![DocsSearchResult {
        url: "https://www.windmill.dev/docs/a".to_string(),
        title: "A".to_string(),
        score: 10,
        snippets: vec![],
    }];
    let index = vec![
        DocsSearchResult {
            url: "https://www.windmill.dev/docs/a.md".to_string(),
            title: "A".to_string(),
            score: 5,
            snippets: vec![],
        },
        DocsSearchResult {
            url: "https://www.windmill.dev/docs/b.md".to_string(),
            title: "B".to_string(),
            score: 5,
            snippets: vec![],
        },
    ];
    let merged = merge_docs_search_results(body, index, SEARCH_MAX_PAGES);
    assert_eq!(
        merged.iter().map(|r| r.url.clone()).collect::<Vec<_>>(),
        vec!["https://www.windmill.dev/docs/a", "https://www.windmill.dev/docs/b.md"]
    );
}

#[test]
fn empty_query_returns_no_results() {
    let pages = parse_docs_full_text(SAMPLE_FULL);
    assert!(search_docs_pages(&pages, "   ", 5).is_empty());
}

#[test]
fn format_search_results_no_matches() {
    let out = format_docs_search_results("zzz", &[]);
    assert!(out.contains("No documentation pages matched \"zzz\""));
}

#[test]
fn format_search_results_uses_caller_neutral_followup_guidance() {
    let out = format_docs_search_results(
        "jobs",
        &[DocsSearchResult {
            url: "https://www.windmill.dev/docs/core_concepts/jobs".to_string(),
            title: "Jobs".to_string(),
            score: 1,
            snippets: vec!["Jobs run scripts and flows.".to_string()],
        }],
    );

    assert!(out.contains("Source: https://www.windmill.dev/docs/core_concepts/jobs"));
    assert!(out.contains("Source URL as its `url` argument"));
    assert!(!out.contains("read_docs_page"));
    assert!(!out.contains("readDocsPage"));
}
