# Local development for Data Pipelines

Status: **draft PR, validated end-to-end.** Both the headless CLI paths and the live browser
preview (`pipeline dev` → `/pipeline_dev`) have been exercised against a running EE + MinIO
stack — headless `pipeline run --local`, browser `Run` / `Run + downstream` cascades writing
real assets, live-reload on save, failure/cascade UX, parameterized run-forms, and the
`--frontend` flag. This document captures the design and the remaining follow-ups.

## What & why

Windmill "data pipelines" are folders of scripts marked with a `// pipeline` comment, wired
together by asset annotations (`// on <asset-uri>`, `// partitioned`, `// schedule`). Until now
they could only be built/run in the browser at `/pipeline/<folder>` (the `PipelineGraphEditor`),
and the CLI only inspected/ran the **deployed** workspace (`wmill pipeline list|show|run`).

This adds the **local edit → preview → run** loop, the pipeline analog of `wmill dev`
(flows/scripts) and `wmill app dev` (raw apps), usable both from a code editor and from an
agentic loop — building a pipeline from working-tree files, seeing the same graph the UI shows,
and running it, all **without deploying**.

## The key design decision (no backend changes)

Full body inference is obtained in the CLI from the **same wasm the frontend uses**:
`windmill-parser-wasm-asset` (`parse_assets_ts | parse_assets_py | parse_assets_sql`). That wasm
returns the entire serialized Rust `ParseAssetsOutput` — `assets` (with `r`/`w`/`rw` access)
**and** the parsed pipeline annotations (`in_pipeline`, `triggers`, `partition`, …) in one call.
The CLI already loads sibling wasm parsers via `loadParser()`; we just added the `-asset` dep.

Running local content reuses the existing preview API:
`runScriptPreview({ content, language, path, args: { _wmill_skip_asset_dispatch: true }, temp_script_refs })`
per node in topological order. Data flows through real asset storage; `_wmill_skip_asset_dispatch`
makes the client own the whole cascade so the backend dispatcher never double-fires.

⇒ No new backend endpoint, no Rust changes, no TS re-port of the annotation parser.

## Surfaces

### Headless CLI (agentic loop) — `cli/src/commands/pipeline/`
- `pipeline show <folder> --local` — render the DAG from working-tree files (fully offline).
- `pipeline run <folder> --local [--from/--to/--dry-run/--json]` — run the cascade via preview of
  local content, reusing the `boundedCascade.ts` topo/lineage engine. Scripts whose only trigger
  needs caller input or per-event fanout (`data_upload`/`webhook`/kafka/…) are skipped by default.
- `pipeline run … --upload <script>[:<param>]=<local-file|s3://storage/key>` — bind an object to a
  `data_upload`/`webhook` entry point so it (and its downstream) runs: a local path is uploaded to
  the workspace store; an `s3://<storage>/<key>` source binds an existing object (authority = named
  storage, `s3:///key` for the default store). The target S3Object arg is inferred when the script
  declares exactly one (else name it with `:<param>`). Repeatable.
- `pipeline docs <folder> [--local]` — write `PIPELINE.md` + `AGENTS.md`/`CLAUDE.md` pointers
  (graph + datatable schemas) so an editor/agent has the same context the UI surfaces.

### Browser live-preview — `pipeline dev` + `/pipeline_dev`
- `pipeline dev [folder]` — folder arg or cwd auto-detect; watches the folder, rebuilds the graph
  on each save, pushes `{type:'pipeline', folder, graph, scripts, temp_script_refs}` over a
  WebSocket (direct mode, default port 3201), opens the dev page.
- `/pipeline_dev` route → `PipelineDevView.svelte` renders the **same** `PipelineGraphEditor`
  (mode `view`) fed by the pushed graph; runs the cascade via preview. Editing stays in the
  user's editor; each save live-reloads.

## Files

**New**
- `cli/src/commands/pipeline/localGraph.ts` — the enabler. Walks `f/<folder>` scripts, wasm-infers
  each (`parse_assets_<lang>`), assembles the `/assets/graph`-shaped payload (`runnables`,
  `assets`, `edges`, `triggers`). Exports `buildLocalPipelineGraph`, `collectScripts`,
  `inferScriptAssets`, `workspaceRoot`, and the canonical graph types.
- `cli/src/commands/pipeline/docs.ts` — `pipeline docs`.
- `cli/src/commands/pipeline/dev.ts` — `pipeline dev` watcher + WS server.
- `cli/test/pipeline_local_graph_unit.test.ts` — unit tests for the builder.
- `frontend/src/lib/components/assets/AssetGraph/cascadeRun.ts` — reusable run primitives
  (`makeLaunch`, `makeWaitJobTerminal`, `runDownstreamCascade`, `runBoundedCascade`) wrapping
  `cascadeOrchestrator` + `graphTraversal`.
- `frontend/src/lib/components/assets/AssetGraph/PipelineDevView.svelte` + `frontend/src/routes/pipeline_dev/+page.svelte`.

**Modified**
- `cli/src/commands/pipeline/pipeline.ts` — `--local` on `show`/`run`; `show` split into graph
  acquisition + `renderGraph()` + deployed-only `enrichRootMarkers()`; registers `docs`/`dev`;
  graph types moved to `localGraph.ts`.
- `cli/package.json` (+ `windmill-parser-wasm-asset`), `cli/package-lock.json`.
- `system_prompts/auto-generated/*`, `cli/src/guidance/skills.gen.ts` — regenerated via
  `python system_prompts/generate.py` (required by AGENTS.md for CLI command changes).

## Language coverage

`inferScriptAssets` mirrors `frontend/src/lib/infer.ts:inferAssets`: ts (bun/deno/nativets),
python3, and SQL dialects all get wasm inference (SQL dialects route to `parse_assets_sql`; its
comment-header annotation scan is dialect-independent). `go`/`bash` have no wasm asset parser, so
they fall back to a minimal `// pipeline` + `// on` scan (annotation-only). Inferred asset paths
must match **exactly** to connect nodes. An S3 asset path is `<storage>/<key>` with an empty
storage segment for the workspace default: `parse_asset_syntax` (shared by the native and wasm
parsers) keeps the URI suffix verbatim, so the SDK object forms — TS `writeS3File({s3:"x"})` and
python `write_s3_file(S3Object(s3="x"))`, which resolve to `s3:///x` — the triple-slash annotation
`// on s3:///x`, and DuckDB `read_csv('s3:///x')` / `COPY ... TO 's3:///x'` all yield path `/x`
(leading slash significant). The named-storage form `s3://secondary/key` yields `secondary/key` —
a **different** object and a different node, so the storage distinction is never collapsed. In
practice: use the triple-slash form everywhere for default-storage objects and the producer and
consumer connect regardless of language; mixing in the no-slash form (`s3://x`) names a storage
called `x` and will not connect to a default-storage write.

## How to test

Run the CLI from source (`alias wmilld='bun run /home/rfiszel/windmill/cli/src/main.ts'`).

Headless (works directly against any remote, e.g. internal.windmill.dev / `data-pipelines`):
```
wmilld workspace add internal data-pipelines https://internal.windmill.dev/   # paste a token
# build an example (connected all-DuckDB DAG):
mkdir -p ~/pl-demo/f/demo_pipeline && cd ~/pl-demo && printf 'defaultTs: bun\n' > wmill.yaml
#   ingest.duckdb.sql:    -- pipeline\nCOPY (SELECT 1 id,'a' name) TO 's3://demo/raw.csv';
#   transform.duckdb.sql: -- pipeline\n-- on s3://demo/raw.csv\nCOPY (SELECT * FROM read_csv('s3://demo/raw.csv')) TO 's3://demo/clean.csv';
#   report.duckdb.sql:    -- pipeline\n-- on s3://demo/clean.csv\nSELECT count(*) FROM read_csv('s3://demo/clean.csv');
wmilld pipeline show demo_pipeline --local
wmilld pipeline run  demo_pipeline --local --dry-run
wmilld pipeline run  demo_pipeline --local          # writes real s3 assets; needs object storage
wmilld pipeline docs demo_pipeline --local   # --local: document the working tree (default path queries the deployed graph)
```
Or pull real pipelines: `wmilld sync pull --yes && wmilld pipeline list && wmilld pipeline show <folder> --local`.

Browser preview: against a remote whose deployed frontend predates the `/pipeline_dev` route, the
auto-opened `<remote>/pipeline_dev?…` 404s. Run THIS branch's frontend locally and point the dev
page at it with `--frontend`:
```
cd frontend && REMOTE=https://internal.windmill.dev npm run dev      # or a local backend
cd ~/pl-demo && wmilld pipeline dev demo_pipeline --frontend http://localhost:3000
```
`pipeline dev` then opens (and prints) the full URL — **copy that printed URL**, don't hand-write it.
It carries both `wm_token` (workspace auth) and `ws_token` (the per-session WS token); the dev
server rejects the WebSocket without `ws_token`, so the page would sit disconnected:
```
http://localhost:3000/pipeline_dev?workspace=<WS>&wm_token=<TOK>&folder=demo_pipeline&port=3201&ws_token=<WS_TOK>
```

## Validation status

- ✅ `pipeline show --local` verified offline against a fixture (renders a connected DAG).
- ✅ `localGraph` unit tests (7) + the CLI suite pass; CLI `tsc` clean; frontend `npm run check`
  0 errors on changed files; svelte-autofixer clean.
- ✅ CLI agent docs regenerated.
- ✅ Live `pipeline dev` → `/pipeline_dev` browser preview **exercised against a running EE + MinIO
  stack**: graph render + live-reload on save, single-node `Run` and `Run + downstream` cascades
  writing real assets, parameterized run-forms, failure/cascade UX, responsive layout, and the
  `--frontend` flag. Screenshots in the PR body.

## TODO for the takeover agent

(Items 1–2 below are **done**; left here for context.)

1. ~~Verify the browser preview end-to-end~~ — done (see Validation status; screenshots in PR).
2. ~~Dev-page route reachability on remotes~~ — done: added `--frontend <origin>` so `pipeline dev`
   can open the page on a locally-run frontend while the API/token target the remote.
3. **Route-page dedup**: optionally refactor `frontend/src/routes/(root)/(logged)/pipeline/[folder]/+page.svelte`
   to consume `cascadeRun.ts` (its `launchCascadeScript`/`runDraftAwareCascade`/`waitJobTerminal`
   are the source of the extraction) — eliminates duplication. Keep behavior identical.
4. **Proxy mode for `pipeline dev`**: currently direct-mode only. Port `dev/dev.ts`'s
   `startProxyServer` for embedders that need a localhost origin (e.g. Claude Code preview).
5. **`pipeline dev` editing**: the dev page is view+run only (editing stays in the user's editor).
   If in-browser editing with file round-trip is wanted, mirror flow-dev's `handleFlowRoundTrip`.
6. **Asset-path normalization**: the python parser resolves the
   `S3Object(s3=…, storage=…?)` constructor / dict-literal forms to the same canonical path as the
   TS `{s3, storage}` object form, so SDK writes and reads connect across ts/python. The
   SDK-form path keeps its default-storage leading slash (`/x`), matching the triple-slash URI
   forms (`s3:///x` in DuckDB and `// on s3:///x`); the no-slash `s3://x` form names a storage
   called `x` and is intentionally a distinct node (see Language coverage).

## Plan reference

Original plan: `/home/rfiszel/.claude/plans/tingly-wandering-whistle.md` (local to the author).
