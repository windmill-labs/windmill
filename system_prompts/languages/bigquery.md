# BigQuery

Arguments use `@name` syntax.

Name the parameters by adding comments before the statement:

```sql
-- @name1 (string)
-- @name2 (int64) = 0
SELECT * FROM users WHERE name = @name1 AND age > @name2;
```

## Receiving an S3Object as a script parameter

Declare the arg with type `(s3object)`. Windmill renders an S3 file picker for
it, downloads the file, and binds it as a `STRING` JSON parameter — Parquet/CSV
files are decoded server-side into a JSON array of records, JSON/JSONL pass
through. Consume with `JSON_EXTRACT_ARRAY` / `JSON_VALUE`:

```sql
-- @file (s3object)
SELECT
  CAST(JSON_VALUE(row, '$.id') AS INT64) AS id,
  JSON_VALUE(row, '$.name') AS name
FROM UNNEST(JSON_EXTRACT_ARRAY(@file)) AS row;
```

## Streaming query results to S3

Add a `-- s3` directive at the top of the script to stream the result set to S3
instead of returning rows. Windmill writes the file and returns its `S3Object`
as the script result.

```sql
-- s3 prefix=exports/users format=parquet
SELECT id, name FROM users;
```

All keys are optional: `prefix` (object key prefix), `storage` (named storage —
omit to use the workspace default), `format` (`json` (default), `parquet`, or
`csv`). Use this for large result sets — rows stream directly to S3 instead of
being buffered, bypassing the 10000-row return cap.
