-- pipeline
-- on s3:///pipelines/km2/raw_events.csv
-- freshness 1h
-- DuckDB transform: reads the seed CSV from S3, normalizes types, and
-- materializes into a Postgres-backed datatable. Showcases ATTACH
-- 'datatable://main' AS pg + CREATE TABLE pg.<table> AS — the asset parser
-- resolves the pg-prefixed name back to datatable://main/<table>.

ATTACH 'datatable://main' AS pg;

DROP TABLE IF EXISTS pg.km2_clean;
CREATE TABLE pg.km2_clean AS
SELECT
  CAST(id AS INTEGER)        AS id,
  CAST(category AS VARCHAR)  AS category,
  CAST(value AS DOUBLE)      AS value,
  CAST(ts AS TIMESTAMP)      AS ts
FROM read_csv('s3://pipelines/km2/raw_events.csv', header=true, auto_detect=true);

-- Also write back to S3 as parquet so the duckdb→s3 inference is exercised.
COPY (
  SELECT * FROM pg.km2_clean
) TO 's3://pipelines/km2/clean.parquet' (FORMAT 'parquet');
