# Ask docs-tool variants — approaches & benchmark

The "ask" copilot answers Windmill questions by calling a docs tool, then writing
an answer. This note records the variants tried, why, and how they compare —
primarily on **token usage** (the main goal) without sacrificing answer quality.

The arm is selected by `DocsToolVariant` (`ask/core.ts`) and benchmarked via the
`ai_evals` `ask` mode (`bun run cli -- run ask --docs-tool <arm>`), which records
pass-rate and prompt/completion/total tokens per attempt.

## Variants

### `inkeep` (original)
One tool, `get_documentation(request)` → `POST /api/inkeep` (hosted RAG). Search
happens server-side; returns pre-selected passages truncated to 30k chars. No
client-side navigation. Requires the inkeep service configured on the backend.

### `llmstxt`
Two tools that navigate the docs like a filesystem:
- `list_docs_pages()` → returns the **entire** `llms.txt` index (~34 KB, 227
  pages, each `title + URL + question-phrased description`), unconditionally.
- `read_docs_page(path, section?)` → fetches a page's raw markdown; returns the
  full page if ≤ 20k chars, else an outline of headings to request a section.

Flow: list (discover by index) → read 1–3 pages. Discovery relies on matching the
question against index titles/descriptions.

### `search` (this experiment)
Search-first, designed to cut tokens by **replacing the index dump**:
- `search_docs(query)` → one cached fetch of `llms-full.txt` (~2.3 MB, every page
  concatenated, delimited by `Source: <url>` lines), grep the corpus for the
  query terms, return the top pages with small matching snippets + `Source:` URL.
- `read_docs_page(path, section?)` → same as above, for when a snippet isn't
  enough.

Flow: search (discover by body content) → optionally read. The `Source:` markers
give exact, citable URLs for free (no URL reconstruction).

Rationale: `list_docs_pages` dumps ~8.7k tokens of index on **every** query before
a page is even chosen; `read_docs_page` then adds full pages, all re-sent across
loop turns. Returning only small snippets should be dramatically leaner — *if*
keyword search can find the right page.

## Benchmark — `search` vs `llmstxt`

- Model: `claude-sonnet-4-5`, judge `claude-sonnet-4-6`
- 30 `ask` cases (lookup / concept / synthesis / niche / nodocs), 1 run each
- Backend proxy `:8050`, workspace `integration-tests`

| Arm | Pass rate | Avg tokens / case | Tokens / passed answer |
|---|---|---|---|
| `llmstxt` | **100%** (30/30) | 49,685 (48.6k prompt) | 49,685 |
| `search` (body-only, v1) | 86.7% (26/30) | **17,553** (16.7k prompt) | 17,429 |

**~65% fewer tokens — ~2.85× cheaper per correct answer.** The reduction holds in
every category:

```
category   | n | llmstxt pass  tok | search pass  tok
lookup     | 8 |   8/8    52,831  |  7/8   17,803
concept    | 8 |   8/8    51,797  |  6/8   17,175
synthesis  | 6 |   6/6    57,327  |  6/6   18,846
niche      | 5 |   5/5    39,329  |  4/5   18,507
nodocs     | 3 |   3/3    37,644  |  3/3   13,716
```

### The 4 cases `search` dropped (all passed by `llmstxt`)

1. **`ask-concept-agent-step` (judge 15) — the structural miss.** "An LLM step that
   decides which script to call." The model searched branch-centric terms
   (`branchone`, `branch predicate`) on a wrong prior and never found the
   `ai_agents` page — even though that page's body matches perfectly. `llmstxt`
   passes because dumping the full index shows a page titled *"AI agents"*; the
   model can't miss a named feature it didn't know to search for. **Search only
   returns what you think to search for — it amplifies the model's prior; the
   index corrects it.**
2. **`ask-lookup-retry-step` (judge 72).** Searched retry terms but read
   `/docs/openflow` instead of `/docs/flows/retries` — discovery landed wrong.
3. **`ask-concept-remember-value-between-runs` (judge 97).** Answer was good; it
   read/cited a sibling page (`resources_and_types`) instead of the expected
   `persistent_storage/within_windmill`. A pure citation miss.
4. **`ask-niche-key-value-store` (judge 72).** Read the **correct** page; citation
   passed. Borderline judge score — answer-quality/single-run noise, not a search
   failure.

Net: 2 genuine discovery misses, 1 citation-only, 1 noise.

### Conclusion

Search-first delivers the token win convincingly. Its one architectural weakness
is exactly the index's strength: surfacing **named features for vague conceptual
queries** (the "unknown unknowns"). Both failing pages were *findable* — `llms.txt`
has an `AI agents` entry and a `Retries` entry whose titles/descriptions match the
query — the model just never browsed them.

## Improvement — hybrid search (bodies + index descriptions)

Idea: have `search_docs` also match the `llms.txt` index **titles/descriptions**
(227 question-phrased lines), returning matching index entries alongside body
snippets. The index descriptions are semantic-rich, so a query like "AI LLM
integration" surfaces the *"AI agents"* entry even when the body grep is led
astray — closing the conceptual-discovery gap **without** dumping the full index
(only the few matched entries are returned, ~a few hundred tokens).

Implementation (`docs/core.ts`): `search_docs` now runs two searches and merges
them (`mergeDocsSearchResults`) — body matches first (concrete content hits),
then index-description matches fill remaining slots, deduped by canonical URL:
- `searchDocsPages(parseDocsFullText(llms-full.txt), q)` — full-text grep (≤5 pages)
- `searchDocsIndex(parseDocsIndex(llms.txt), q)` — title/description match (≤4),
  title matches weighted above description matches; the description is the snippet
- index fetch is best-effort, so a failure still leaves full-text results

### Results — three-way (same setup as above)

| Arm | Pass rate | Avg tokens / case | vs `llmstxt` |
|---|---|---|---|
| `llmstxt` | 100% (30/30) | 49,685 | — |
| `search` v1 (body-only) | 86.7% (26/30) | 17,553 | −65% tokens |
| **`search` v2 (hybrid)** | **96.7%** (29/30) | **19,215** | **−61% tokens** |

```
category   | llmstxt        -> v1 (body)      -> v2 (hybrid)
lookup     | 8/8  52,831    -> 7/8  17,803    -> 8/8  18,794
concept    | 8/8  51,797    -> 6/8  17,175    -> 7/8  16,249
synthesis  | 6/6  57,327    -> 6/6  18,846    -> 6/6  23,987
niche      | 5/5  39,329    -> 4/5  18,507    -> 5/5  23,603
nodocs     | 3/3  37,644    -> 3/3  13,716    -> 3/3  11,388
```

The hybrid recovered 3 of v1's 4 losses (lookup 7→8, niche 4→5, concept 6→7) for
~1.7k extra tokens/case — still 61% below `llmstxt`. Verified on `agent-step`: the
model issued the *same* branch-centric queries that failed in v1, but the index
match surfaced `ai_agents.md` and it read the right page (judge 15 → 95).

The one remaining failure (`ask-concept-remember-value-between-runs`, a citation
miss) **passed when re-run in isolation** — single-run flakiness on the strict
`answerIncludesAny` URL check, not a discovery failure.

### Takeaways

- Search-first (snippets instead of the index dump + full pages) is the token
  lever: ~60–65% fewer tokens per question, in every category.
- Pure body grep has one structural blind spot — it can't surface a **named
  feature the model didn't think to search for** ("unknown unknowns"). Matching
  the index titles/descriptions too closes that gap cheaply, because the
  descriptions are question-phrased and only the matched lines are returned.
- Net: **`search` v2 ≈ `llmstxt` answer quality at ~⅖ the token cost.**

### Haiku vs Sonnet (hybrid search vs llmstxt, both models)

| Model | Arm | Pass rate | Tokens / question |
|---|---|---|---|
| Sonnet | llmstxt | 100% (30/30) | 49,685 |
| Sonnet | search (hybrid) | 96.7% (29/30) | 19,215 |
| Haiku | llmstxt | 93.3% (28/30) | 49,514 |
| Haiku | search (hybrid) | 86.7% (26/30) | 15,950 |

- **Token cost is set by the tool, not the model:** llmstxt ≈ 49.5k on both,
  search ≈ 16–19k on both. The ~60–68% saving holds regardless of model.
- **Haiku is weaker on both arms** (93.3 vs 100 on llmstxt; 86.7 vs 96.7 on
  search) — search isn't uniquely bad on haiku.
- **Search costs more accuracy on the weaker model:** llmstxt→search penalty is
  −3.3pp (sonnet) vs −6.7pp (haiku); search-first leans on the model to pick good
  keywords and recognize the right page from snippets. Of haiku-search's 4 misses,
  ~2 are search-specific; `python-deps` fails haiku on *both* arms (model
  weakness) and `remember-value` is the same flaky citation check.
- **Sonnet + search (19k, 96.7%) beats Haiku + llmstxt (49.5k, 93.3%) on both
  axes** — switching the tool on the strong model beats switching to a cheaper
  model on the old tool.
- These are token *counts*, not dollars — Haiku's per-token price is far lower, so
  on $ basis Haiku+search is cheapest (at 86.7% accuracy).

### Caveats / not yet done

- 1 run per case, one model (sonnet), 30 cases — judge scores on single runs
  carry noise (see the flaky citation case). Multi-run would tighten this.
- `inkeep` not re-benchmarked here (separate hosted service; needs backend config).
- Result files: `ai_evals/results/*__ask.json` (look for `runModel` `ask:search` /
  `ask:llmstxt`). Re-run: `bun run cli -- run ask --docs-tool search --model sonnet`.
