-- pipeline
-- on ducklake://main/km2_lake
-- DuckDB transform: reads from the DuckLake snapshot and dumps as parquet to
-- S3. Showcases ATTACH 'ducklake://main' AS lake + COPY … TO 's3://…' +
-- catalog-relative lake.<table> reference.

ATTACH 'ducklake://main' AS lake;

COPY (
  SELECT category, event_count, avg_value, snapshotted_at
  FROM lake.km2_lake
  ORDER BY avg_value DESC
) TO 's3://pipelines/km2/lake_snapshot.parquet' (FORMAT 'parquet');
