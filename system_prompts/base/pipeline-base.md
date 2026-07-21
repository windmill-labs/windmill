# Data pipeline authoring

A **data pipeline** is NOT a flow. A flow is one runnable that orchestrates steps internally. A data pipeline is a set of **independent scripts**, each deployed on its own, that form a DAG by reading and writing shared **storage assets** (DuckLake tables, data tables, S3 objects, volumes, resources) and by declaring execution **triggers**. The pipeline is visualized and edited at `/pipeline/<folder>`; every node is a normal workspace script that happens to carry pipeline annotations. When the user asks for a "data pipeline" (or to "ingest / transform / materialize" data across steps), build pipeline-annotated scripts — do NOT build a flow.

## What makes a script a pipeline node

A script joins the pipeline when its source begins with the `pipeline` annotation as a top-of-file comment, **written in the script's own comment syntax** — `//` for TS/JS (bun), `--` for SQL (DuckDB/Postgres), `#` for Python/Bash. So it's `-- pipeline` in a DuckDB node, `# pipeline` in a Python node, `// pipeline` in a bun node. Every annotation below uses that same prefix (the `//` shown is the TS form). All other wiring is expressed as annotation comments near the top of the file:

- `// on <ref>` — declares an execution-DAG **input** (what triggers/feeds this node). `<ref>` is either:
  - an **asset URI** (the node runs when that asset is produced upstream): `ducklake://main/orders`, `datatable://main/users`, `s3://<key>`, `$res:f/folder/my_resource`, `volume://name/path`.
  - a **native trigger kind**: `schedule`, `webhook`, `email`, `kafka`, `mqtt`, `amqp`, `nats`, `postgres`, `sqs`, `gcp`, or `data_upload` (a user-uploaded S3 file). For these the actual trigger row (cron, topic, …) is created separately; the annotation only declares the binding.
- **Outputs** are inferred from what the body writes — a `CREATE TABLE`, a `wmill.writeS3File(...)`, a DuckLake/datatable write. To declare a managed output explicitly, use `// materialize <asset-uri>`.
- Optional badges: `// partitioned <daily|hourly|weekly|monthly|dynamic>`, `// freshness <duration>` (e.g. `1h`), `// tag <worker-tag>`, `// retry <count> [delay]`, `// data_test <kind> ...`.

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
2. Author each node as a **script draft** with `write_script` (or `edit_script`), language chosen for the work: `duckdb` or `postgresql` for SQL-shaped data work, `bun`/`python3` for general transforms. SQL-heavy lakehouse steps usually use `duckdb`.
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
SELECT * FROM read_csv('s3://raw/orders/*.csv')
```

Node `f/sales/orders_daily` (runs when `orders` is produced, writes a rollup):

```sql
-- pipeline
-- on ducklake://main/orders
-- materialize ducklake://main/orders_daily
SELECT date_trunc('day', ts) AS day, count(*) AS n
FROM ducklake.main.orders GROUP BY 1
```
