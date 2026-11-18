# Data metrics: declared measures and dimensions

## What this is

A way to declare, on the DuckDB script that materializes a DuckLake table, what the
table's canonical aggregations are:

```sql
-- materialize ducklake://sales/orders
-- measure revenue = sum(amount) where not is_refund
-- measure order_count = count(*)
-- dimension region = region
-- dimension month = date_trunc('month', ordered_at)
SELECT ...
```

Those declarations are catalogued at deploy and read back by two consumers: the
script editor (a drawer that composes a `SELECT` you can insert) and agents (a tool
that lists what is declared, so they stop guessing).

That is the whole feature. It records definitions and hands them to whoever is
writing the query. It does not rewrite, compile, or execute anything.

## What it deliberately does not do

**It does not enforce anything.** An agent or a person can ignore a declared
measure and write `sum(amount)` without the refund filter, and nothing stops them.
The bet is that most wrong numbers come from not knowing the rule, not from
defying it, and that an informed writer is most of the value.

**It is not a semantic layer.** Declarations are single-table. There is no join
planner and no fan-out-safe aggregation across grains. To combine tables, a
developer materializes a mart with a pipeline step and declares measures on that.

**There is no query-time indirection.** What you deploy is what runs. Compiling a
`{{ metric(...) }}` token into frozen SQL at deploy and substituting it on the
worker would break that: the script in the editor would stop being the script that
executes, and job logs would show SQL appearing nowhere in the source.

## Why no compiler

A compiler buys two things over handing definitions to the caller: byte-identical
SQL with no LLM in the loop, and enforcement. Both matter only once several
independent consumers depend on the same number being reproducible, which is a
situation this codebase has no evidence of yet. Until then the machinery costs a
token grammar, a deploy-time freeze keyed to script hashes, a worker substitution
path, and a second way for the editor and the runtime to disagree.

If that evidence shows up, the catalog is the right foundation to build it on: the
declarations are already structured and already validated.

## Authoring

Both annotations live in the leading comment header, next to `// materialize`:

- `// measure <name> = <aggregate> [where <predicate>]`
- `// dimension <name> = <expression>`

`expr` and `filter` are the author's own SQL, stored verbatim. The predicate is
kept separate from the aggregate rather than folded into it, because a reader
renders it as `expr FILTER (WHERE filter)`. That is what lets two measures with
different predicates sit under one `GROUP BY`; a shared `WHERE` cannot express it.

Declarations attach to the table the script materializes, so a script with no
ducklake `// materialize` target contributes nothing: there is no table to hang
them on.

## The catalog

`data_metric` holds one row per declaration, replaced wholesale for a script path
on every deploy, so it always describes deployed state. It mirrors how the `asset`
table is maintained.

Persisting rather than parsing on demand is what makes folder-scoped listing cheap.
Reading a table's declarations is a point lookup either way, but "everything
declared under `f/analytics`" would otherwise mean fetching and parsing every
script body in the folder on each agent call. Both access patterns are single index
scans:

- `idx_data_metric_table` for the editor drawer
- `idx_data_metric_folder` for the agent tool, using `text_pattern_ops` so a
  `LIKE 'f/analytics/%'` prefix becomes a range scan. This database's collation is
  not `C`, and under a locale collation the planner will not use a default-opclass
  index for prefix matching (measured at 50k rows: seq scan with the default
  opclass, index range scan with `text_pattern_ops`).

### Canonical table paths

A producer's `// materialize ducklake://sales/orders` omits the schema, while a
consumer reading `sales.main.orders` records its asset as `sales/main.orders`.
Both are canonicalized to `<lake>/<schema>.<table>` with `main` as the default, or
a consumer could never resolve the table it reads.

### Authorization

`data_metric` has no RLS. Reads are filtered by an `EXISTS` against `script` on the
authed connection, so `script`'s existing folder, group and user policies decide
what a caller sees. The producing script path is therefore part of the key: a
DuckLake path has no folder to authorize against.

## Consumers

**Script editor drawer.** A Metrics trigger sits in the editor bar of every DuckDB
script, and in the compact Helpers menu below the width threshold. It lists the
tables that declare metrics so they can be browsed, composes a plain `SELECT`
client-side from a measure/dimension selection, and offers three things to do with
it: copy it (a complete query attaching the lake under `dl`), run it in an embedded
REPL against the lake, or append it to the script (reusing whatever alias the
script already attaches, since a repeated `ATTACH` would not run). The output is
ordinary editable SQL with no link back to the catalog.

**Agent tool.** The same endpoint is exposed with `x-mcp-tool`, so an agent can ask
what a table or a folder declares and use the declared `expr`/`filter` instead of
inventing an aggregate.

## Limitations and follow-ups

- Declarations only exist for materialized DuckLake tables. There is no way to
  declare a measure over a Postgres table or an API result.
- The drawer's REPL runs a preview job per execution, so it is a place to check a
  metric rather than a dashboard. There is no charting or filter builder.
- The drawer composes SQL in TypeScript while an agent composes its own. They agree
  semantically because both read the same declaration, but not byte for byte. That
  only becomes a problem if identical SQL is ever required, which is the compiler
  above.
- Deploy hard-rejects three things, all because a reader executes the stored text:
  an unsafe lake/table path and a measure/dimension body that is not a single SQL
  expression (both are stored SQL injection), and a filtered measure whose body is
  not a single aggregate call (`sum(a)/count(b) where …` would apply `FILTER` to
  only part of the expression and silently produce the wrong number). Everything
  else is advisory:
  missing-column and non-aggregate-measure warnings come from the separate
  `check_schema_contracts` endpoint, which the editor calls fire-and-forget on save.
  A CLI or direct-API deploy skips those warnings, and a declaration whose SQL is
  otherwise wrong is only discovered when someone runs it.
