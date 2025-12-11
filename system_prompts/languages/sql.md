# SQL Languages

## PostgreSQL

Arguments are obtained directly in the statement with `$1::{type}`, `$2::{type}`, etc.

Name the parameters by adding comments at the beginning of the script:
```sql
-- $1 name1
-- $2 name2 = default_value
SELECT * FROM users WHERE name = $1::TEXT AND age > $2::INT;
```

## MySQL

Arguments use `?` placeholders.

Name the parameters by adding comments before the statement:
```sql
-- ? name1 (text)
-- ? name2 (int) = 0
SELECT * FROM users WHERE name = ? AND age > ?;
```

## BigQuery

Arguments use `@name` syntax.

Name the parameters by adding comments before the statement:
```sql
-- @name1 (string)
-- @name2 (int64) = 0
SELECT * FROM users WHERE name = @name1 AND age > @name2;
```

## Snowflake

Arguments use `?` placeholders.

Name the parameters by adding comments before the statement:
```sql
-- ? name1 (text)
-- ? name2 (number) = 0
SELECT * FROM users WHERE name = ? AND age > ?;
```

## Microsoft SQL Server (MSSQL)

Arguments use `@P1`, `@P2`, etc.

Name the parameters by adding comments before the statement:
```sql
-- @P1 name1 (varchar)
-- @P2 name2 (int) = 0
SELECT * FROM users WHERE name = @P1 AND age > @P2;
```

## DuckDB

Arguments are defined with comments and used with `$name` syntax:
```sql
-- $name (text) = default
-- $age (integer)
SELECT * FROM users WHERE name = $name AND age > $age;
```

### Ducklake Integration
Attach Ducklake for data lake operations:
```sql
-- Main ducklake
ATTACH 'ducklake' AS dl;

-- Named ducklake
ATTACH 'ducklake://my_lake' AS dl;

-- Then query
SELECT * FROM dl.schema.table;
```

### External Database Connections
Connect to external databases using resources:
```sql
ATTACH '$res:path/to/resource' AS db (TYPE postgres);
SELECT * FROM db.schema.table;
```

### S3 File Operations
Read files from S3 storage:
```sql
-- Default storage
SELECT * FROM read_csv('s3:///path/to/file.csv');

-- Named storage
SELECT * FROM read_csv('s3://storage_name/path/to/file.csv');

-- Parquet files
SELECT * FROM read_parquet('s3:///path/to/file.parquet');
```
