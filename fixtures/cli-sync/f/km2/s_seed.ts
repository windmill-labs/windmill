// pipeline
// schedule "5 * * * *"
// retry 3
// Source: schedule-triggered seed that writes two S3 outputs (parquet + json)
// the rest of the km2 matrix consumes. Both paths are literal { s3: "..." }
// strings so the static asset parser picks them up as writes.

import * as wmill from "windmill-client"

type RawEvent = { id: number; category: "alpha" | "beta" | "gamma" | "delta"; value: number; ts: string }

const CATS: RawEvent["category"][] = ["alpha", "beta", "gamma", "delta"]

export async function main(count = 100) {
  const now = new Date().toISOString()
  const rows: RawEvent[] = Array.from({ length: count }, (_, i) => ({
    id: i,
    category: CATS[i % CATS.length],
    value: Math.round(Math.random() * 1000) / 10,
    ts: now,
  }))

  // Output #1: JSON blob — generic S3 object
  await wmill.writeS3File(
    { s3: "pipelines/km2/raw_events.json" },
    JSON.stringify(rows, null, 2),
    undefined,
    "application/json"
  )

  // Output #2: Parquet — exercised by the DuckDB read_parquet() path.
  // We don't have a TS parquet writer; instead, dump CSV alongside and let
  // the duckdb step rewrite to parquet. Keeping the same path declared as
  // an s3 write so the asset parser still records the output.
  const csv = ["id,category,value,ts", ...rows.map((r) => `${r.id},${r.category},${r.value},${r.ts}`)].join("\n")
  await wmill.writeS3File(
    { s3: "pipelines/km2/raw_events.csv" },
    csv,
    undefined,
    "text/csv"
  )

  return { written: rows.length, paths: ["pipelines/km2/raw_events.json", "pipelines/km2/raw_events.csv"] }
}
