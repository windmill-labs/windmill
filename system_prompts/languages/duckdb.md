# DuckDB

Arguments are defined with comments and used with `$name` syntax:

```sql
-- $name (text) = default
-- $age (integer)
SELECT * FROM users WHERE name = $name AND age > $age;
```

## Ducklake Integration

Attach Ducklake for data lake operations:

```sql
-- Main ducklake
ATTACH 'ducklake' AS dl;

-- Named ducklake
ATTACH 'ducklake://my_lake' AS dl;

-- Then query
SELECT * FROM dl.schema.table;
```

## External Database Connections

Connect to external databases using resources:

```sql
ATTACH '$res:path/to/resource' AS db (TYPE postgres);
SELECT * FROM db.schema.table;
```

## S3 File Operations

Read files from S3 storage:

```sql
-- Default storage
SELECT * FROM read_csv('s3:///path/to/file.csv');

-- Named storage
SELECT * FROM read_csv('s3://storage_name/path/to/file.csv');

-- Parquet files
SELECT * FROM read_parquet('s3:///path/to/file.parquet');

-- JSON files
SELECT * FROM read_json('s3:///path/to/file.json');
```

### Receiving an S3Object as a script parameter

Declare the arg with type `(s3object)`. Windmill renders an S3 file picker for it
and binds the arg as the bare `s3://storage/key` URI, which DuckDB's reader
functions consume directly:

```sql
-- $file (s3object)
SELECT * FROM read_parquet($file);
```

Works with any DuckDB reader: `read_csv($file)`, `read_json($file)`, etc.

### Writing query results to S3

DuckDB writes to S3 natively via `COPY ... TO`:

```sql
COPY (SELECT * FROM users) TO 's3:///exports/users.parquet' (FORMAT PARQUET);
```

Use this instead of the `-- s3` streaming directive supported by the other SQL
dialects — that directive is not available in DuckDB.
