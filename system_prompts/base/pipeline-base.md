# Data pipeline authoring

A **data pipeline** is NOT a flow. A flow is one runnable that orchestrates steps internally. A data pipeline is a set of **independent scripts**, each deployed on its own, that form a DAG by reading and writing shared **storage assets** (DuckLake tables, data tables, S3 objects, volumes, resources) and by declaring execution **triggers**. The pipeline is visualized and edited at `/pipeline/<folder>`; every node is a normal workspace script that happens to carry pipeline annotations. When the user asks for a "data pipeline" (or to "ingest / transform / materialize" data across steps), build pipeline-annotated scripts — do NOT build a flow.

## Default to DuckDB + DuckLake

A pipeline node that produces a table should almost always be a **`duckdb`** node that materializes its output into a **DuckLake** table with `-- materialize ducklake://<name>/<table>` (in a DuckDB node the annotation uses SQL `--` comment syntax; write the body as a bare `SELECT` and let the runtime do the write). DuckLake is the default lakehouse store for pipelines and is the shape the pipeline editor is built around, so prefer it unless the work specifically calls for something else:

- `postgresql` / data tables — only for row-level, OLTP-style mutations against an existing Postgres data table (frequent single-row upserts/updates, transactional reads that an app queries live).
- `bun` / `python3` — only for non-tabular work that doesn't map to SQL: calling an external API, wrangling files, arbitrary glue. When such a node still produces tabular data for downstream steps, land it in DuckLake (write it with the wmill SDK / ducklake helpers) rather than inventing a parallel store.

Do not spread a pipeline across postgres, S3, and DuckLake when one DuckLake lake would do; a consistent DuckLake lakehouse is the goal.

## Storage prerequisites

A DuckLake pipeline only runs once the workspace has **object storage** (S3 / Azure Blob / GCS) **and a DuckLake catalog** configured — DuckLake tables and `s3://` assets can't be materialized or read without it. Check with the `list_ducklakes` tool before you build (it returns the configured DuckLake catalogs, or none). Drafting the annotated scripts does not require storage, but the pipeline can't ingest, materialize, or read its assets until it exists. So if `list_ducklakes` returns none (or the user hits "storage not configured" errors), say so and give the right next step **by role**:

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
- Optional badges: `// partitioned <daily|hourly|weekly|monthly|dynamic>`, `// freshness <duration>` (e.g. `1h`), `// tag <worker-tag>`, `// retry <count> [delay]`, `// data_test <kind> ...`.

## S3 object wiring (storage form matters)

An `s3://` URI's first slashes select the **storage**, not part of the key — get this wrong and the producer/consumer edge silently won't connect:

- `s3:///<key>` (**triple** slash, empty first segment) = the **default** workspace storage. A downstream node reading or triggering on that object uses `s3:///<key>` — e.g. DuckDB `-- on s3:///orders/2024.parquet` and `read_parquet('s3:///orders/2024.parquet')`.
- `s3://<storage>/<key>` (**double** slash, non-empty first segment) = a **named secondary** storage called `<storage>` — so `s3://ingest/x` means storage `ingest`, key `x`, NOT key `ingest/x`. Only use this when the object genuinely lives in a configured secondary storage; never invent a bucket/storage name for a default-storage object (it breaks the edge).

To make the producer side visible to lineage, a Python/TS node MUST pass the **`S3Object` form**, not a bare key string: Python `wmill.write_s3_file(wmill.S3Object(s3="<key>"), data)` (or the import-free dict `{"s3": "<key>"}`), TS `wmill.writeS3File({ s3: "<key>" }, data)`. That records the default-storage asset `/<key>`, which a downstream `s3:///<key>` reader connects to (same key both sides). A bare `write_s3_file("<key>", ...)` records **no** asset and produces **no** edge. Add `storage="<name>"` only for a named secondary storage.

The key must be a **string literal** — the graph parser is static and cannot follow a variable, f-string, or computed path, so `write_s3_file(wmill.S3Object(s3=key_var), ...)` records no edge. Inline the literal (`s3="events/user_events.parquet"`) on both the writing and reading node. The same rule applies to every asset URI in an annotation or SDK call (`ducklake://`, `datatable://`, `s3://`): write them literally, not via a variable.

## Materialize (the managed output)

> **`// materialize` is DuckDB-only**, and its target must be a DuckLake table (`ducklake://<name>/<table>`). Deploy **rejects** `// materialize` on any other language (`python3`, `bun`, `postgresql`) or a non-DuckLake target. For a non-DuckDB node, do **not** use `// materialize` — write the output via the SDK (`wmill.writeS3File(...)`, a postgresql `CREATE TABLE`, ducklake helpers, …) and let it be inferred. Use `duckdb` when a node should materialize a DuckLake table.

`// materialize <asset-uri>` tells the runtime to write the node's output table **for you**: write the body as a single `SELECT` and the runtime wraps it in the create/replace — do **not** also write your own `CREATE TABLE` / `INSERT`. Write strategy:

- no option → **replace** the whole table each run (full refresh; the only mode whose output columns may change);
- `// materialize <uri> append` → INSERT-append rows (incremental);
- `// materialize <uri> key=<col>` → merge/upsert on `<col>`.

`// materialize manual <uri>` opts **out** of managed writes — the script writes its own DDL and the annotation only records the output asset for lineage.

`materialize` pairs with partitioning for incremental pipelines: a `// partitioned <daily|hourly|weekly|monthly|dynamic>` node runs **once per partition** (append/merge into a fixed-schema table), and the `{partition}` token inside any asset URI is substituted with the current partition value at run time.

`materialize` is an output **declaration** on a node — not a command. There is no "materialize run".

## How to build one in chat

1. Put every node in the **same folder**: `f/<folder>/<name>`. The folder is the pipeline.
2. Author each node as a **script draft** with `write_script` (or `edit_script`). Default to `duckdb` materializing into DuckLake (see "Default to DuckDB + DuckLake" above); pick `postgresql`, `bun`, or `python3` only when that section says the work calls for it.
3. Start each body with `// pipeline`, then the `// on` input declarations, then the transform that writes the output.
4. **Chain nodes by asset URI**: read an upstream node's output asset, then `// on <that-same-uri>` in the downstream node so the edge forms. Reuse exact asset paths from existing nodes rather than inventing parallel ones.
5. Leave nodes as drafts unless the user asks to deploy. A pipeline only "runs" once its scripts are deployed and their triggers exist.

When the user already has the `/pipeline/<folder>` editor open, prefer the dedicated `build_pipeline_node` / `edit_pipeline_node` tools (they stage reviewable, canvas-highlighted proposals). Outside the editor, use the standard script-draft tools with the annotations above.

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
