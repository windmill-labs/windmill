// pipeline
// on s3:///pipelines/km_real/summary.json

import * as wmill from "windmill-client"

type Row = { category: string; count: number; avg_value: number; computed_at: string }

export async function main() {
  const buf = await wmill.loadS3File({ s3: "pipelines/km_real/summary.json" })
  const rows: Row[] = JSON.parse(new TextDecoder().decode(buf))
  const sql = wmill.datatable("main")

  // Drop + recreate so column-name changes are picked up — IF NOT EXISTS
  // would leave a stale schema from earlier deploys.
  await sql`DROP TABLE IF EXISTS km_real_summary`.fetch()
  await sql`CREATE TABLE km_real_summary (
    id serial primary key,
    category text not null,
    event_count int not null,
    avg_value double precision not null,
    computed_at timestamptz default now()
  )`.fetch()

  // Workaround for the SDK + run_inline binding mismatch:
  // - SDK auto-injects ::BIGINT / ::DOUBLE PRECISION based on JS arg type
  // - run_inline serializes args as JSON strings
  // - sqlx refuses String → non-text bind
  // Wrapping numeric values in String() makes the SDK auto-cast as ::TEXT;
  // the explicit ::int / ::double precision in SQL then converts text →
  // numeric server-side. Cleaner bulk paths (jsonb_to_recordset) don't
  // help because the SDK strips chained casts on the bind site.
  for (const r of rows) {
    await sql`INSERT INTO km_real_summary (category, event_count, avg_value)
      VALUES (${r.category}, ${String(r.count)}::int, ${String(r.avg_value)}::double precision)`.fetch()
  }
  return { inserted: rows.length }
}
