-- pipeline
-- schedule "0 0 * * *"
-- partitioned daily
-- freshness 1h
-- Standalone partitioned showcase: scheduled daily, declared as
-- `partitioned daily`, and a `freshness 1h` SLA. Reads the seed parquet
-- and writes a per-day ducklake table.

ATTACH 'ducklake://main' AS lake;

DROP TABLE IF EXISTS lake.km2_daily;

CREATE TABLE lake.km2_daily AS
SELECT
  CAST(ts AS DATE)               AS day,
  category,
  count(*)                       AS event_count,
  avg(value)                     AS avg_value
FROM read_parquet('s3://pipelines/km2/clean.parquet')
GROUP BY 1, 2;
