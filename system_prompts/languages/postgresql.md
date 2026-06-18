# PostgreSQL

Arguments are obtained directly in the statement with `$1::{type}`, `$2::{type}`, etc.

Name the parameters by adding comments at the beginning of the script (without specifying the type):

```sql
-- $1 name1
-- $2 name2 = default_value
SELECT * FROM users WHERE name = $1::TEXT AND age > $2::INT;
```

## Receiving an S3Object as a script parameter

Declare the arg with type `(s3object)`. Windmill renders an S3 file picker for
it, downloads the file, and binds it as a `jsonb` parameter — Parquet/CSV files
are decoded server-side into a JSON array of records, JSON/JSONL pass through.
Consume with `jsonb_to_recordset` (or any `jsonb` API):

```sql
-- $1 file (s3object)
SELECT *
FROM jsonb_to_recordset($1::jsonb) AS r(id INT, name TEXT);
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
being buffered as the script return value.
