// pipeline
// on datatable://main/km_real_summary

import * as wmill from "windmill-client"

type Row = { category: string; event_count: number; avg_value: number }

export async function main() {
  const dt = wmill.datatable("main")
  const rows: Row[] = await dt`SELECT category, event_count, avg_value FROM km_real_summary`.fetch()

  const lake = wmill.ducklake("main")
  await lake`DROP TABLE IF EXISTS km_real_lake`.fetch()
  await lake`CREATE TABLE km_real_lake (
    category VARCHAR,
    event_count INT,
    avg_value DOUBLE,
    snapshotted_at TIMESTAMP
  )`.fetch()
  for (const r of rows) {
    // Same String() → ::cast workaround as r_to_datatable: the SDK
    // pre-casts JS numbers to BIGINT/DOUBLE PRECISION on the bind
    // side and the run_inline runner stringifies before binding,
    // which mismatches.
    await lake`INSERT INTO km_real_lake VALUES (${r.category}, ${String(r.event_count)}::INTEGER, ${String(r.avg_value)}::DOUBLE, now())`.fetch()
  }
  return { snapshotted: rows.length }
}
