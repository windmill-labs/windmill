// pipeline
// on datatable://main/km2_summary
// on ducklake://main/km2_lake
// AND-join: TS transform that waits for BOTH the Postgres summary AND the
// DuckLake snapshot before firing. Demonstrates multi-input asset triggers
// joining at this node. Writes two outputs in different kinds so both write
// edges land on the canvas.

import * as wmill from "windmill-client"

type SummaryRow = { category: string; event_count: number; avg_value: number }
type LakeRow = { category: string; avg_value: number; snapshotted_at: string }

export async function main() {
  const pg = wmill.datatable("main")
  const lake = wmill.ducklake("main")

  const summary: SummaryRow[] = await pg`
    SELECT category, event_count, avg_value FROM km2_summary
  `.fetch()
  const snapshot: LakeRow[] = await lake`
    SELECT category, avg_value, snapshotted_at FROM km2_lake
  `.fetch()

  const byCat = new Map(snapshot.map((s) => [s.category, s]))
  const joined = summary.map((s) => ({
    ...s,
    lake_avg_value: byCat.get(s.category)?.avg_value ?? null,
    snapshotted_at: byCat.get(s.category)?.snapshotted_at ?? null,
  }))

  // Output #1: S3 JSON sink
  await wmill.writeS3File(
    { s3: "pipelines/km2/joined.json" },
    JSON.stringify(joined, null, 2),
    undefined,
    "application/json"
  )

  // Output #2: re-materialize the join into Postgres for downstream readers.
  await pg`DROP TABLE IF EXISTS km2_joined`.fetch()
  await pg`CREATE TABLE km2_joined (
    category text not null,
    event_count int not null,
    avg_value double precision not null,
    lake_avg_value double precision,
    snapshotted_at timestamptz
  )`.fetch()
  for (const r of joined) {
    await pg`INSERT INTO km2_joined (category, event_count, avg_value, lake_avg_value, snapshotted_at)
      VALUES (${r.category}, ${String(r.event_count)}::int, ${String(r.avg_value)}::double precision,
              ${r.lake_avg_value == null ? null : String(r.lake_avg_value)}::double precision,
              ${r.snapshotted_at})`.fetch()
  }

  return { joined: joined.length }
}
