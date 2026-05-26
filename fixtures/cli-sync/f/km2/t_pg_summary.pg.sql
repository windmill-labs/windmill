-- pipeline
-- on datatable://main/km2_clean
-- Postgres summary step: groups the cleaned events into a per-category
-- summary table. Showcases native CREATE TABLE … in postgresql language
-- (the asset parser maps unqualified table names to datatable://main/<name>
-- when the script runs against the datatable backend).

DROP TABLE IF EXISTS km2_summary;

CREATE TABLE km2_summary AS
SELECT
  category,
  count(*)                          AS event_count,
  avg(value)::double precision      AS avg_value,
  max(ts)                           AS computed_at
FROM km2_clean
GROUP BY category;
