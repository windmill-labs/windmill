// pipeline
// on s3:///pipelines/km_real/summary.json

import * as wmill from "windmill-client"

export async function main() {
  const buf = await wmill.loadS3File({ s3: "pipelines/km_real/summary.json" })
  const summary = JSON.parse(new TextDecoder().decode(buf))
  const total = summary.reduce((a: number, r: any) => a + r.count, 0)
  const report = {
    generated_at: new Date().toISOString(),
    total_events: total,
    categories: summary,
    top: summary.slice().sort((a: any, b: any) => b.avg_value - a.avg_value).slice(0, 3)
  }
  await wmill.writeS3File(
    { s3: "pipelines/km_real/report.json" },
    JSON.stringify(report, null, 2),
    undefined,
    "application/json"
  )
  return report
}
