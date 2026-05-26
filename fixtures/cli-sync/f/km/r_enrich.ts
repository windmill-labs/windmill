// pipeline
// on s3:///pipelines/km_real/raw_events.json

import * as wmill from "windmill-client"

export async function main() {
  const buf = await wmill.loadS3File({ s3: "pipelines/km_real/raw_events.json" })
  const events = JSON.parse(new TextDecoder().decode(buf))
  const bucketOf = (v: number) => (v < 25 ? "low" : v < 75 ? "mid" : "high")
  const enriched = events.map((e: any) => ({ ...e, bucket: bucketOf(e.value) }))
  await wmill.writeS3File(
    { s3: "pipelines/km_real/enriched.json" },
    JSON.stringify(enriched, null, 2),
    undefined,
    "application/json"
  )
  return { in: events.length, out: enriched.length }
}
