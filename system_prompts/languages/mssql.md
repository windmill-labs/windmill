# Microsoft SQL Server (MSSQL)

Arguments use `@P1`, `@P2`, etc.

Name the parameters by adding comments before the statement:

```sql
-- @P1 name1 (varchar)
-- @P2 name2 (int) = 0
SELECT * FROM users WHERE name = @P1 AND age > @P2;
```

## Receiving an S3Object as a script parameter

Declare the arg with type `(s3object)`. Windmill renders an S3 file picker for
it, downloads the file, and binds it as `nvarchar(max)` JSON text — Parquet/CSV
files are decoded server-side into a JSON array of records, JSON/JSONL pass
through. Consume with `OPENJSON`:

```sql
-- @P1 file (s3object)
SELECT id, name
FROM OPENJSON(@P1)
WITH (id INT, name NVARCHAR(200));
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
