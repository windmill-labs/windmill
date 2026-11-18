//! The vendored documentation snapshot, embedded into the binary and parsed once.
//!
//! `docs_snapshot/*.gz` are refreshed by `docs_snapshot/fetch.sh`. Embedding them
//! lets docs search work with no runtime egress (including air-gapped instances).

use std::io::Read;
use std::sync::OnceLock;

use flate2::read::GzDecoder;

use super::search::{
    canonical_docs_page_url, canonical_search_url, parse_docs_full_text, parse_docs_index,
    DocsFullPage, DocsIndexEntry,
};

const LLMS_FULL_GZ: &[u8] =
    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/docs_snapshot/llms-full.txt.gz"));
const LLMS_INDEX_GZ: &[u8] =
    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/docs_snapshot/llms.txt.gz"));

pub struct DocsCorpus {
    /// Every docs page, keyed by its `Source:` URL (from llms-full.txt).
    pub pages: Vec<DocsFullPage>,
    /// The curated page index with one-line descriptions (from llms.txt).
    pub index: Vec<DocsIndexEntry>,
}

impl DocsCorpus {
    /// Finds the page whose `Source:` URL matches a model/CLI-supplied path or URL
    /// after canonicalization (origin re-anchored, `.md` and ordering prefixes
    /// stripped).
    pub fn find_page(&self, path: &str) -> Option<&DocsFullPage> {
        let key = canonical_search_url(&canonical_docs_page_url(path));
        self.pages.iter().find(|p| canonical_search_url(&p.url) == key)
    }
}

static CORPUS: OnceLock<DocsCorpus> = OnceLock::new();

fn decompress(bytes: &[u8]) -> String {
    let mut out = String::new();
    if let Err(e) = GzDecoder::new(bytes).read_to_string(&mut out) {
        // The embedded snapshot is valid gzip text; a decode failure is a
        // build-time packaging error, so failing closed to an empty corpus
        // (docs search returns "no matches") is acceptable.
        tracing::error!("failed to decompress embedded docs snapshot: {e}");
        return String::new();
    }
    out
}

/// Returns the parsed docs corpus, decompressing and parsing the embedded
/// snapshot once on first access.
pub fn corpus() -> &'static DocsCorpus {
    CORPUS.get_or_init(|| {
        let pages = parse_docs_full_text(&decompress(LLMS_FULL_GZ));
        let index = parse_docs_index(&decompress(LLMS_INDEX_GZ));
        DocsCorpus { pages, index }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_corpus_parses_to_non_empty() {
        let corpus = corpus();
        assert!(corpus.pages.len() > 50, "expected many pages, got {}", corpus.pages.len());
        assert!(corpus.index.len() > 50, "expected many index entries, got {}", corpus.index.len());
        assert!(corpus
            .pages
            .iter()
            .all(|p| p.url.starts_with("https://www.windmill.dev/docs/") && !p.body.is_empty()));
    }

    #[test]
    fn find_page_matches_by_canonical_url() {
        let corpus = corpus();
        // A page that is expected to exist in the published docs.
        let by_path = corpus.find_page("/docs/core_concepts/worker_groups");
        let by_url = corpus.find_page("https://www.windmill.dev/docs/core_concepts/worker_groups.md");
        assert!(by_path.is_some());
        assert_eq!(by_path.map(|p| &p.url), by_url.map(|p| &p.url));
    }
}
