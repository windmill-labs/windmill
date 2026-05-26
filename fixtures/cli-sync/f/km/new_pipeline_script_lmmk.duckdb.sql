-- pipeline
-- on s3:///pipelines/km_real/summary.json
-- More: partitioned daily, freshness 1h, retry 3, tag heavy — https://www.windmill.dev/docs/pipelines/annotations
-- Upstream: s3:///pipelines/km_real/summary.json
-- Output: datatable://main/calm_metrics_njfi

ATTACH 'datatable://main' AS pg;

CREATE TABLE IF NOT EXISTS pg.km_real_summary AS
SELECT * FROM read_parquet('s3:///pipelines/km_real/summary.json');
