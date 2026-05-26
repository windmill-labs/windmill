// pipeline
// on s3:///pipelines/km_real/enriched.json

import * as wmill from "windmill-client"

export async function main() {
  const buf = await wmill.loadS3File({ s3: "pipelines/km_real/enriched.json" })
  const events = JSON.parse(new TextDecoder().decode(buf))
  const byCat: Record<string, { count: number; sum: number }> = {}
  for (const e of events) {
    const k = e.category
    if (!byCat[k]) byCat[k] = { count: 0, sum: 0 }
    byCat[k].count++
    byCat[k].sum += e.value
  }
  const summary = Object.entries(byCat).map(([category, s]) => ({
    category,
    count: s.count,
    avg_value: Number((s.sum / s.count).toFixed(2)),
    computed_at: new Date().toISOString()
  }))
  await wmill.writeS3File(
    { s3: "pipelines/km_real/summary.json" },
    JSON.stringify(summary, null, 2),
    undefined,
    "application/json"
  )
  return { groups: summary.length }
}
