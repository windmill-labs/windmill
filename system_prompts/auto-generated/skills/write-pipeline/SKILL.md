---
name: write-pipeline
description: MUST use when creating or modifying a data pipeline, a set of scripts marked `pipeline` and wired together by `on` / `materialize` annotations.
---

# Data pipeline authoring

A **data pipeline** is NOT a flow. A flow is one runnable that orchestrates steps internally. A data pipeline is a set of **independent scripts**, each deployed on its own, that form a DAG by reading and writing shared **storage assets** (DuckLake tables, data tables, S3 objects, volumes, resources) and by declaring execution **triggers**. The pipeline is visualized and edited at `/pipeline/<folder>`; every node is a normal workspace script that happens to carry pipeline annotations. When the user asks for a "data pipeline" (or to "ingest / transform / materialize" data across steps), build pipeline-annotated scripts — do NOT build a flow.

## Default to DuckDB + DuckLake

A pipeline node that produces a table should almost always be a **`duckdb`** node that materializes its output into a **DuckLake** table with `-- materialize ducklake://<name>/<table>` (in a DuckDB node the annotation uses SQL `--` comment syntax; write the body as a bare `SELECT` and let the runtime do the write). DuckLake is the default lakehouse store for pipelines and is the shape the pipeline editor is built around, so prefer it unless the work specifically calls for something else:

- `postgresql` / data tables — only for row-level, OLTP-style mutations against an existing Postgres data table (frequent single-row upserts/updates, transactional reads that an app queries live).
- `bun` / `python3` — only for work that doesn't map to SQL: calling an external API, wrangling files, arbitrary glue. When such a node produces tabular data for downstream steps, write it **straight into DuckLake** (or a data table) from the same script — see "Ingesting from Python / TypeScript" below — rather than inventing a parallel store.

Do not spread a pipeline across postgres, S3, and DuckLake when one DuckLake lake would do; a consistent DuckLake lakehouse is the goal.

## Ingesting from Python / TypeScript

A node that fetches data in code (an API, a SaaS export, a scraper) writes the rows **directly** into their DuckLake table with `wmill.ducklake()`, or into a data table with `wmill.datatable()`. DuckLake already stores every table as parquet files in the workspace object storage, so **never stage the raw payload first** — no intermediate JSON / CSV / parquet file in S3 and no second "load the raw file" node. One ingestion node, one table.

Pass the rows as a query argument (a list of dicts / array of objects is sent as JSON) and let SQL unnest them; declare the column types so the table gets a real schema:

```python
# pipeline
# on schedule
import wmill

def main():
    rows = fetch_charges()  # list[dict]
    lake = wmill.ducklake()
    lake.query("""
        CREATE OR REPLACE TABLE stripe_charges AS
        SELECT unnest(from_json($rows, '[{"id":"VARCHAR","amount":"BIGINT","created":"BIGINT"}]'), recursive := true)
    """, rows=rows).fetch()
```

```ts
// pipeline
// on schedule
import * as wmill from "windmill-client"

export async function main() {
  const rows = await fetchCharges()
  const lake = wmill.ducklake()
  await lake`CREATE OR REPLACE TABLE stripe_charges AS
    SELECT unnest(from_json(${rows}, '[{"id":"VARCHAR","amount":"BIGINT","created":"BIGINT"}]'), recursive := true)`.fetch()
}
```

- Use `INSERT INTO <table> SELECT ...` instead of `CREATE OR REPLACE` to append (an incremental pull), with a one-time `CREATE TABLE IF NOT EXISTS` first.
- Assign the client to a variable (`lake = wmill.ducklake()`) and keep the SQL a **string literal** (Python) or a template without `sql.raw` (TS): that is what lets the graph record the node's write to `ducklake://main/<table>`. A chained `wmill.ducklake().query(...)` or SQL built in a variable records no output, so the downstream edge never forms.
- `wmill.ducklake("<name>")` targets a non-default catalog; `wmill.datatable("<name>")` writes a Postgres data table the same way (Python `db.query(sql, *args)` with `$1` placeholders, TS the same template form) — use it only when the data belongs in Postgres (see above).
- The downstream node triggers on the table itself: `-- on ducklake://main/stripe_charges`.

## Storage prerequisites

A DuckLake pipeline only runs once the workspace has **object storage** (S3 / Azure Blob / GCS) **and a DuckLake catalog** configured — DuckLake tables and `s3://` assets can't be materialized or read without it. Check which DuckLake catalogs the workspace has before you build.
`wmill ducklake list` lists them.
Drafting the annotated scripts does not require storage, but the pipeline can't ingest, materialize, or read its assets until it exists. So if there is none (or the user hits "storage not configured" errors), say so and give the right next step **by role**:

- a workspace **admin** sets it up in Workspace settings → Object Storage (add an S3/Azure/GCS storage), then adds a DuckLake catalog on top of it;
- anyone **without admin rights** should ask a workspace admin to configure object storage + a DuckLake catalog.

Never hand back a DuckLake pipeline that cannot run without flagging the missing storage and pointing to who sets it up.

## What makes a script a pipeline node

A script joins the pipeline when its source begins with the `pipeline` annotation as a top-of-file comment, **written in the script's own comment syntax** — `//` for TS/JS (bun), `--` for SQL (DuckDB/Postgres), `#` for Python/Bash. So it's `-- pipeline` in a DuckDB node, `# pipeline` in a Python node, `// pipeline` in a bun node. Every annotation below uses that same prefix (the `//` shown is the TS form). All other wiring is expressed as annotation comments near the top of the file:

- `// on <ref>` — declares an execution-DAG **input** (what triggers/feeds this node). `<ref>` is either:
  - an **asset URI** (the node runs when that asset is produced upstream): `ducklake://main/orders`, `datatable://main/users`, `$res:f/folder/my_resource`, `volume://name/path`, or an S3 object (see the S3 storage-form rule below).
  - a **native trigger kind**: `schedule`, `webhook`, `email`, `kafka`, `mqtt`, `amqp`, `nats`, `postgres`, `sqs`, `gcp`, or `data_upload` (a user-uploaded S3 file). For these the actual trigger row (cron, topic, …) is created separately; the annotation only declares the binding. **`data_upload` is special**: there is no trigger row — the node instead declares an **`S3Object` input parameter** fed by the auto-generated upload picker; it never hard-codes a key. Any language can be the `data_upload` node:
    - Python (has `import wmill`): `def main(file: wmill.S3Object):` then `wmill.load_s3_file(file)`; TS (has `import * as wmill from "windmill-client"`): `export async function main(file: wmill.S3Object)`. Qualify the type as `wmill.S3Object` (or add `from wmill import S3Object` / `import { S3Object } from "windmill-client"`) — a bare `S3Object` is undefined.
    - **DuckDB** takes the s3object arg via a `-- $<name> (s3object)` declaration and reads it directly, so a single DuckDB node can ingest **and** materialize:
      ```
      -- pipeline
      -- on data_upload
      -- materialize ducklake://main/raw_uploads
      -- $file (s3object)
      SELECT * FROM read_csv($file)
      ```
- **Outputs** are inferred from what the body writes — a `CREATE TABLE`, a `wmill.writeS3File(...)`, a DuckLake/datatable write. To declare a managed output explicitly, use `// materialize <asset-uri>`.
- Optional badges: `// partitioned <daily|hourly|weekly|monthly|dynamic>`, `// freshness <duration>` (e.g. `1h`), `// tag <worker-tag>`, `// retry <count> [delay]`, `// data_test <kind> ...` (managed DuckLake targets only — deploy rejects it beside a `dbt://` one), `// measure <name> = <agg> [where <pred>]` and `// dimension <name> = <expr>` (see "Declared metrics" below).

## S3 object wiring (storage form matters)

An `s3://` URI's first slashes select the **storage**, not part of the key — get this wrong and the producer/consumer edge silently won't connect:

- `s3:///<key>` (**triple** slash, empty first segment) = the **default** workspace storage. A downstream node reading or triggering on that object uses `s3:///<key>` — e.g. DuckDB `-- on s3:///orders/2024.parquet` and `read_parquet('s3:///orders/2024.parquet')`.
- `s3://<storage>/<key>` (**double** slash, non-empty first segment) = a **named secondary** storage called `<storage>` — so `s3://ingest/x` means storage `ingest`, key `x`, NOT key `ingest/x`. Only use this when the object genuinely lives in a configured secondary storage; never invent a bucket/storage name for a default-storage object (it breaks the edge).

To make the producer side visible to lineage, a Python/TS node MUST pass the **`S3Object` form**, not a bare key string: Python `wmill.write_s3_file(wmill.S3Object(s3="<key>"), data)` (or the import-free dict `{"s3": "<key>"}`), TS `wmill.writeS3File({ s3: "<key>" }, data)`. That records the default-storage asset `/<key>`, which a downstream `s3:///<key>` reader connects to (same key both sides). A bare `write_s3_file("<key>", ...)` records **no** asset and produces **no** edge. Add `storage="<name>"` only for a named secondary storage.

The key must be a **string literal** — the graph parser is static and cannot follow a variable, f-string, or computed path, so `write_s3_file(wmill.S3Object(s3=key_var), ...)` records no edge. Inline the literal (`s3="events/user_events.parquet"`) on both the writing and reading node. The same rule applies to every asset URI in an annotation or SDK call (`ducklake://`, `datatable://`, `s3://`): write them literally, not via a variable.

## Materialize (the managed output)

> **A MANAGED `// materialize` is DuckDB-only**, and its target must be a DuckLake table (`ducklake://<name>/<table>`). Deploy **rejects** a `ducklake://` `// materialize` on any other language (`python3`, `bun`, `postgresql`). For a non-DuckDB node writing the lake, do **not** use `// materialize` — write the table itself with `wmill.ducklake()` (see "Ingesting from Python / TypeScript") and let the output be inferred. Use `duckdb` when a node should materialize a DuckLake table.
>
> The one target ANY language may declare (except a dbt script, whose writes come from its manifest) is a **warehouse relation**: `// materialize manual dbt://<warehouse>/<schema>/<name>`, where `<warehouse>` is a warehouse the workspace configures under Settings → dbt. `manual` is the only mode it has — nothing generates warehouse DDL, so the node issues its own write (a postgresql `CREATE TABLE` / `INSERT`, an SDK load, …) and the annotation records the outcome. Use it on an ingestion node whose output a dbt project reads as a `source`: the declared relation and the dbt model land on ONE graph node, and a downstream `// on dbt://<warehouse>/<schema>/<name>` fires when the ingestion node completes.

A managed `// materialize ducklake://<name>/<table>` tells the runtime to write the node's output table **for you**: write the body as a single `SELECT` and the runtime wraps it in the create/replace — do **not** also write your own `CREATE TABLE` / `INSERT`. (The opposite holds for the `dbt://` target above: there the node writes its own DDL and the strategies below do not apply.) Write strategy:

- no option → **replace** the whole table each run (full refresh; the only mode whose output columns may change);
- `// materialize <uri> append` → INSERT-append rows (incremental);
- `// materialize <uri> key=<col>` → merge/upsert on `<col>`.

`// materialize manual <uri>` opts **out** of managed writes — the script writes its own DDL and the annotation only records the output asset for lineage.

`materialize` pairs with partitioning for incremental pipelines: a `// partitioned <daily|hourly|weekly|monthly|dynamic>` node runs **once per partition** (append/merge into a fixed-schema table). The `{partition}` token, usable in any asset URI **and** in the body SQL, is replaced at run time by the current partition's **identity string**:

- To filter the source to the active slice on a time grain, use the runtime-injected macro: `WHERE wm_partition(<ts_col>) = {partition}`. `wm_partition(ts)` buckets a timestamp in exactly the identity format the runtime uses for daily/hourly/weekly/monthly, so it always matches; never hand-write a `strftime` format.
- Do NOT write `= TIMESTAMP {partition}`: the identity string is not a valid timestamp literal for hourly/weekly/monthly and errors at run time.
- For `dynamic` partitioning the identity is the caller-supplied key (not a timestamp, no macro), so filter on it directly: `WHERE <your_key_col> = {partition}`.

`materialize` is an output **declaration** on a node — not a command. There is no "materialize run".

## Declared metrics (`measure` / `dimension`)

On a node that materializes a DuckLake table, `// measure <name> = <aggregate> [where <predicate>]` names the canonical way to aggregate that table (e.g. `// measure revenue = sum(amount) where not is_refund`), and `// dimension <name> = <expr>` names a way to slice it (e.g. `// dimension region = region`, `// dimension month = date_trunc('month', ordered_at)`). They execute nothing: they are catalogued at deploy so the editor and other agents reuse the definition instead of re-deriving it and silently disagreeing.

- Keep the predicate in the `where` clause rather than folding it into the aggregate: it is rendered as `<agg> FILTER (WHERE <pred>)`, which is what lets two measures with different predicates share one GROUP BY.
- DuckLake-only, and only meaningful next to `// materialize`.
- Declare one when a number carries a judgement call someone else would get wrong (refunds excluded, test rows dropped, which column is the amount); do NOT blanket every table with measures — an obvious `count(*)` earns nothing.
- To use a metric another node declares, read that node and reuse its exact expression rather than guessing it.

## How to build one

1. Put every node in the **same folder**: `f/<folder>/<name>`. The folder is the pipeline.
2. Write each node as its own script. Default to `duckdb` materializing into DuckLake (see "Default to DuckDB + DuckLake" above); pick `postgresql`, `bun`, or `python3` only when that section says the work calls for it.
3. Start each body with `// pipeline`, then the `// on` input declarations, then the transform that writes the output.
4. **Chain nodes by asset URI**: read an upstream node's output asset, then `// on <that-same-uri>` in the downstream node so the edge forms. Reuse exact asset paths from existing nodes rather than inventing parallel ones.
5. Don't deploy nodes unless the user asks to. A pipeline only "runs" once its scripts are deployed and their triggers exist.

Locally:

- create each node with `wmill script new f/<folder>/<name> <language>`, then write its body;
- `wmill pipeline show <folder> --local` draws the graph from your working tree — check that every edge you meant to form is there;
- `wmill pipeline dev <folder>` live-previews the pipeline, and `wmill pipeline run <folder> --local` runs the cascade from local files without deploying (`--dry-run` prints the plan first);
- a trigger such as `// on schedule` only declares the binding: the schedule or trigger itself is created separately (see the `schedules` and `triggers` skills).

## Example (DuckDB → DuckLake, scheduled ingest + downstream transform)

Node `f/sales/orders_ingest` (runs on a schedule, materializes a DuckLake table):

```sql
-- pipeline
-- on schedule
-- materialize ducklake://main/orders
SELECT * FROM read_csv('s3:///raw/orders/*.csv')
```

Node `f/sales/orders_daily` (runs when `orders` is produced, writes a rollup):

```sql
-- pipeline
-- on ducklake://main/orders
-- materialize ducklake://main/orders_daily
SELECT date_trunc('day', ts) AS day, count(*) AS n
FROM ducklake.main.orders GROUP BY 1
```
