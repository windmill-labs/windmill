# Ingestion (EL) in Windmill pipelines

How to land external data in the lake with Windmill pipelines — the
extract-load story that sits in front of the transform/materialize layer
described in [`ducklake-materialization.md`](./ducklake-materialization.md).
Everything here rides on mechanics that already exist (scripts, `wmill`
state, native triggers, `// materialize`); this page is the deliberate
framing, plus the ingestion templates that ship in the pipeline editor.

## The shape: extract lands raw, the engine loads

The canonical ingestion pipeline is **two nodes**:

```
[entry script: any language]  →  s3:///pipelines/<folder>/landing/<x>.jsonl  →  [DuckDB loader: -- materialize]  →  ducklake://…
        E (extract)                       raw landing object                          L (load, managed)
```

- The **entry script** talks to the outside world — REST pagination, a dlt
  source, an SFTP drop — in whatever language fits, and writes the raw batch
  to a landing object in workspace storage (`wmill.writeS3File` /
  `wmill.write_s3_file(wmill.S3Object(...))`).
- The **loader** is a DuckDB script with `-- on s3:///…/landing/<x>.jsonl` +
  `-- materialize ducklake://… [key=…|append]`. The asset trigger re-runs it
  whenever a new batch lands; the managed materialize gives the write
  idempotent strategies, DuckLake snapshots, the partition grid, backfill,
  and `-- data_test` — none of which a hand-rolled INSERT gets.

Why not one polyglot script that writes the lake directly?

- `// materialize` is **DuckDB-only** — deploy rejects the annotation on any
  other language, managed or `manual`.
- The SDK materialize helpers (`wmill.upsertPartition` / TS,
  `lake.upsert_partition(...)` / Python) work and even record partition state
  in pipeline context, but they build their SQL **inside the SDK**, so the
  asset parser cannot see the write — the node deploys with no output edge in
  the graph, which breaks lineage, cascade scheduling, and the backfill UI's
  producer lookup. Use them for ad-hoc jobs, not pipeline nodes.
- Literal SQL through `wmill.ducklake("main").query("INSERT INTO t …")` *is*
  parser-visible (the SQL string is fed to the SQL asset parser), but then
  you own idempotency, schema bootstrap, and partition bookkeeping yourself.

The landing file is not overhead — it is the raw zone. It decouples source
flakiness from load semantics, gives you a replayable artifact, and keeps the
row-crunching in vectorized DuckDB instead of a Python loop.

## Choosing the extract engine (a.k.a. "isn't Python slow?")

Order of preference:

1. **DuckDB-native pull — no scripting in the data path at all.** If the
   source is a database Windmill can attach (`ATTACH 'datatable://…'` for a
   Postgres-backed datatable), a single DuckDB node reads the source table
   directly (vectorized, predicate-pushdown via the postgres scanner) and
   materializes in the same statement. This is the `Postgres → lake`
   template. There is no landing file and no per-row interpreter work.
2. **A plain fetch loop (TypeScript/Bun or Python).** For REST APIs the
   bottleneck is the API — rate limits, page sizes, network latency — not the
   language. A Bun `fetch` loop shuttling JSON to the landing object adds
   negligible overhead; all heavy lifting happens in the DuckDB loader.
3. **dlt (Python).** Reach for dlt when you want its **source ecosystem**:
   `RESTClient` with pluggable paginators/auth for quick custom sources, or a
   verified source for a SaaS API you'd rather not reverse-engineer. That
   value is in extraction; keep dlt out of the load path (its own
   normalize/load pipeline duplicates — more slowly and outside the graph —
   what the managed loader already does). The template iterates dlt page
   streams straight into the landing file.

Rule of thumb: Python/dlt is fine wherever the source itself caps throughput
(almost every external API). If extract CPU ever becomes the bottleneck, the
source is probably a database or file store — use engine 1, or land files
directly and let the loader `read_parquet`/`read_json_auto` them.

For **streaming/CDC sources**, don't poll at all — see "Triggers" below.

## Cursor state: incremental extraction

`wmill.get_state()` / `wmill.set_state(v)` persist a value **per script
path** (it lives in the script's state resource, visible in the script's
Details pane). That is the incremental cursor:

```python
last_id = wmill.get_state() or 0          # first run → full ingest
rows = [r for r in fetch_pages() if r["id"] > last_id]
...
wmill.set_state(max(r["id"] for r in rows))
```

Guidelines:

- **Set the cursor only after the landing write succeeds** (the templates do
  this) — a failed run then simply re-extracts the same window.
- Prefer a **monotonic source column** (`id`, `updated_at`). For `updated_at`
  cursors, overlap the window slightly (`>=` or a lookback interval) and let
  a `key=` merge loader dedup — clock skew and late commits otherwise drop
  rows.
- The **lookback-window variant needs no state at all**: re-read everything
  touched in the last N days and merge by key (idempotent). This is the only
  option for pure-DuckDB entries (no `get_state` there) and is remarkably
  robust — the `Postgres → lake` template uses it.

### Resetting the cursor (re-ingest from scratch)

1. Clear the state: run once with `wmill.set_state(None)` (or edit/delete the
   state in the script's Details pane).
2. Run the entry script — it extracts from the epoch and lands a full batch.
3. The loader's strategy decides what happens on load:
   - `key=` **merge**: safe — existing rows are upserted, history intact.
   - **append**: a full re-ingest **duplicates** the table; drop/rename the
     table first (or switch the loader to `key=` for the backfill).
   - **replace** (no option): the slice/table is rebuilt — always safe.

Re-*loading* without re-extracting is cheaper: re-run just the loader (the
landing file still holds the last batch), or use the partition grid's
backfill for partitioned targets.

## Schema drift: what happens when the source adds a column

The landing file always carries whatever the source sent —
`read_json_auto` picks the new column up on the next load. What happens then
depends on the materialize strategy (see
[`ducklake-materialization.md`](./ducklake-materialization.md) §codegen):

| Strategy | On a new/removed column |
|---|---|
| **replace**, unpartitioned (default) | Re-derives the schema every run (`CREATE OR REPLACE TABLE`) — drift just flows through. |
| **replace, partitioned** | Table is `CREATE TABLE IF NOT EXISTS` — schema frozen at first run; a drifted batch **fails the run** (column-count mismatch). |
| **`key=` merge** | Frozen at first run; drifted batch fails the run. |
| **append** | Frozen at first run; drifted batch fails the run. |
| **`key=… history` (SCD2)** | Frozen at first run *by design* — closed versions can't be reshaped. Manual rebuild required. |

A failing run is the intended behavior for frozen-schema strategies: it's
your schema contract firing. Recovery options, in order of typicality:

- **Project explicitly** in the loader (`SELECT id, name, … FROM
  read_json_auto(…)`) so new source columns are ignored until you opt in —
  the batch keeps loading through drift.
- **Evolve the table**: `ALTER TABLE lake.t ADD COLUMN …` (DuckLake tracks
  schema evolution in the catalog), then widen the SELECT.
- **Rebuild**: switch to replace for one run, or drop the table and let the
  next run bootstrap the new shape (append/merge lose nothing upstream — the
  source of truth is the source; SCD2 loses accumulated history, so prefer
  ALTER there).

`read_json_auto` also *infers* types per batch — a batch where a column is
all-null can infer a different type. Pinning the loader with an explicit
projection (or `columns={…}`) freezes the read side too; worth doing once a
pipeline matures past the prototype stage.

## Triggers: what feeds an entry node

An entry script declares its trigger kind as a marker annotation
(`# on schedule`, `// on kafka`, …); the actual trigger row (cron expression,
broker/topic, …) is created in the trigger editor after deploy — the graph
shows a red placeholder node until it exists.

- **`on schedule`** — the default for polling APIs/databases. Pair the
  schedule interval with the cursor window (lookback ≥ 2× interval).
- **`on kafka` / `on nats` / `on mqtt` / `on sqs` / `on gcp`** — streaming
  sources push each event batch as a run argument; the entry script appends
  it to the landing object (or straight to an `append` loader). No cursor
  needed — the broker's offset tracking replaces it.
- **`on postgres`** — true CDC: the native Postgres trigger uses logical
  replication to fire on INSERT/UPDATE/DELETE. Use it when you need
  event-level latency; use the pull template (`updated_at` window) when
  minute-level freshness is fine and you don't want replication slots.
- **`on data_upload`** — manual/UI-first ingestion: the run form renders an
  S3 picker and the uploaded file *is* the landing object.
- The **loader** is always asset-triggered (`-- on s3:///…`): it runs when a
  batch lands, regardless of which of the above produced it.

## The ingestion templates

The pipeline editor's `+` menu has an "Ingestion templates" section
(`frontend/src/lib/components/assets/AssetGraph/pipelineTemplates.ts`,
`INGESTION_TEMPLATES`). Each pick seeds ready-to-deploy draft node(s) wired
per this page:

| Template | Scripts | Pattern |
|---|---|---|
| **Postgres → lake** | 1 DuckDB | `ATTACH 'datatable://…'`, `updated_at` lookback window → `-- materialize … key=id` (merge). Stateless-incremental, fully vectorized. |
| **REST API → lake** | Bun entry + DuckDB loader | `fetch` pagination, id cursor in state → JSONL landing → `-- materialize … append`. |
| **REST API → lake (dlt)** | Python entry + DuckDB loader | dlt `RESTClient` pagination, cursor in state → JSONL landing → `-- materialize … key=id` (merge). |

The entry bodies point at a public demo API (jsonplaceholder) so they run
end-to-end the moment they're deployed; swap the URL/params and the cursor
column for your source.

## Gotchas worth knowing

- **S3 paths have two spellings.** SDK object forms (`{s3: "k"}`,
  `S3Object(s3="k")`) take a bare key and canonicalize to `/k`; annotation and
  DuckDB URI forms use `s3:///k` (triple slash = default storage) which
  canonicalizes the same. A double-slash `s3://k` URI canonicalizes to `k` —
  a *different* asset — so lineage between an SDK write and a DuckDB read
  silently breaks. Stick to bare-key SDK forms + triple-slash URIs.
- **Python: never pass a bare string key to `write_s3_file`/`load_s3_file`.**
  The SDK only parses `s3://…`-prefixed strings; a bare key becomes
  `S3Object(s3="")` and the upload lands under an auto-generated key. Use
  `wmill.S3Object(s3="…")`.
- **Don't write an empty landing batch.** A zero-row extraction should
  return early (the templates do) — overwriting the landing file with an
  empty array re-triggers the loader for nothing, and `read_json_auto` on an
  empty file errors the run.
- **`append` loaders assume disjoint batches.** The cursor guarantees that;
  a manual re-run of the loader on an already-loaded landing file duplicates
  rows. If operators may re-run freely, prefer `key=` merge.
