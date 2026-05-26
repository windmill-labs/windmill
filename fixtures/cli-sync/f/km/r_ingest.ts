// pipeline
// schedule "0 * * * *"

import * as wmill from "windmill-client"

export async function main() {
  const cats = ["alpha", "beta", "gamma", "delta"]
  const events = Array.from({ length: 100 }, (_, i) => ({
    id: i,
    category: cats[Math.floor(Math.random() * cats.length)],
    value: Math.round(Math.random() * 1000) / 10,
    ts: new Date().toISOString()
  }))
  await wmill.writeS3File(
    { s3: "pipelines/km_real/raw_events.json" },
    JSON.stringify(events, null, 2),
    undefined,
    "application/json"
  )
  return { count: events.length }
}
