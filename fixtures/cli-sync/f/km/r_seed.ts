// pipeline
// Seed/reset entrypoint: regenerates every km_real dataset from nil so the
// whole f/km pipeline (r_enrich -> r_aggregate -> r_publish -> r_to_datatable
// -> r_to_ducklake) can run end-to-end against a clean object store.
// No `// on` / `// schedule` trigger on purpose: run this manually before an
// e2e run. Each dataset is produced with the SAME transform as the step that
// would normally emit it, so every downstream step gets consistent input.
//
// writeS3File is called directly with literal { s3: "..." } paths (no helper,
// no template literals) so static asset detection picks up all four outputs.

import * as wmill from "windmill-client"

const CATS = ["alpha", "beta", "gamma", "delta"]

type RawEvent = { id: number; category: string; value: number; ts: string }
type Enriched = RawEvent & { bucket: "low" | "mid" | "high" }
type SummaryRow = { category: string; count: number; avg_value: number; computed_at: string }

export async function main(count = 100) {
  const now = new Date().toISOString()

  // 1. raw_events.json — same generation as f/km/r_ingest
  const raw: RawEvent[] = Array.from({ length: count }, (_, i) => ({
    id: i,
    category: CATS[Math.floor(Math.random() * CATS.length)],
    value: Math.round(Math.random() * 1000) / 10,
    ts: now,
  }))
  await wmill.writeS3File(
    { s3: "pipelines/km_real/raw_events.json" },
    JSON.stringify(raw, null, 2),
    undefined,
    "application/json"
  )

  // 2. enriched.json — same transform as f/km/r_enrich
  const bucketOf = (v: number): Enriched["bucket"] =>
    v < 25 ? "low" : v < 75 ? "mid" : "high"
  const enriched: Enriched[] = raw.map((e) => ({ ...e, bucket: bucketOf(e.value) }))
  await wmill.writeS3File(
    { s3: "pipelines/km_real/enriched.json" },
    JSON.stringify(enriched, null, 2),
    undefined,
    "application/json"
  )

  // 3. summary.json — same aggregation as f/km/r_aggregate
  const byCat: Record<string, { count: number; sum: number }> = {}
  for (const e of enriched) {
    const k = e.category
    if (!byCat[k]) byCat[k] = { count: 0, sum: 0 }
    byCat[k].count++
    byCat[k].sum += e.value
  }
  const summary: SummaryRow[] = Object.entries(byCat).map(([category, s]) => ({
    category,
    count: s.count,
    avg_value: Number((s.sum / s.count).toFixed(2)),
    computed_at: now,
  }))
  await wmill.writeS3File(
    { s3: "pipelines/km_real/summary.json" },
    JSON.stringify(summary, null, 2),
    undefined,
    "application/json"
  )

  // 4. report.json — same shape as f/km/r_publish (final pipeline artifact)
  const report = {
    generated_at: now,
    total_events: summary.reduce((a, r) => a + r.count, 0),
    categories: summary,
    top: summary.slice().sort((a, b) => b.avg_value - a.avg_value).slice(0, 3),
  }
  await wmill.writeS3File(
    { s3: "pipelines/km_real/report.json" },
    JSON.stringify(report, null, 2),
    undefined,
    "application/json"
  )

  return {
    seeded: [
      "pipelines/km_real/raw_events.json",
      "pipelines/km_real/enriched.json",
      "pipelines/km_real/summary.json",
      "pipelines/km_real/report.json",
    ],
    raw_events: raw.length,
    groups: summary.length,
  }
}
