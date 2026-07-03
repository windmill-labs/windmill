# Vendored docs snapshot

`llms.txt.gz` (curated page index) and `llms-full.txt.gz` (full corpus, every docs
page concatenated and delimited by `Source:` lines) are a gzipped snapshot of
`https://www.windmill.dev/llms.txt` and `/llms-full.txt`.

They are embedded into the binary by `../src/docs/corpus.rs` (`include_bytes!`) and
decompressed/parsed once at first use. This lets in-product docs search
(`GET /api/docs/search`, `GET /api/docs/page`) — used by the AI chat, the MCP
`searchDocs`/`readDocsPage` tools, and the `wmill docs` CLI — work with **no
runtime network egress**, including on air-gapped instances.

The tradeoff is staleness: the snapshot is pinned to whatever was published when
`fetch.sh` was last run. Refresh on each release:

```bash
./fetch.sh        # re-downloads and re-gzips both files
git add llms.txt.gz llms-full.txt.gz
```
