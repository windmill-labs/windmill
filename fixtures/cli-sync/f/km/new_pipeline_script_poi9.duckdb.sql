-- pipeline
-- schedule "0 * * * *"
-- More: partitioned daily, freshness 1h, retry 3 — https://www.windmill.dev/docs/pipelines/annotations
-- Output: datatable://main/km_rsi30c8

-- DataTable is exposed as the 'pg' attached database in duckdb scripts.
CREATE TABLE IF NOT EXISTS pg.km_rsi30c8 AS
SELECT * FROM (SELECT 1 AS placeholder);
