# Snowflake

Arguments use `?` placeholders.

Name the parameters by adding comments before the statement:

```sql
-- ? name1 (text)
-- ? name2 (number) = 0
SELECT * FROM users WHERE name = ? AND age > ?;
```

## Receiving an S3Object as a script parameter

Declare the arg with type `(s3object)`. Windmill renders an S3 file picker for
it, downloads the file, and binds it as JSON text — Parquet/CSV files are
decoded server-side into a JSON array of records, JSON/JSONL pass through.
Wrap the bind with `PARSE_JSON(?)` and walk it with `LATERAL FLATTEN`:

```sql
-- ? file (s3object)
SELECT
  v.value:id::NUMBER AS id,
  v.value:name::STRING AS name
FROM LATERAL FLATTEN(input => PARSE_JSON(?)) v;
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
