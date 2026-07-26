/**
 * Result serialization and file I/O for benchmark results.
 */

import type { BenchmarkResult } from "../types.ts";

/** Save a benchmark result to a JSON file. */
export async function saveResult(
  result: BenchmarkResult,
  outputDir: string,
): Promise<string> {
  const filename = `${result.competitor}_competitor_benchmark.json`;
  const filepath = `${outputDir}/${filename}`;

  // Append to existing array if file exists, otherwise create new
  let data: BenchmarkResult[] = [];
  try {
    const existing = await Deno.readTextFile(filepath);
    data = JSON.parse(existing);
  } catch (_) {
    // File doesn't exist, start fresh
  }
  data.push(result);
  await Deno.writeTextFile(filepath, JSON.stringify(data, null, 2));
  console.log(`Results saved to ${filepath}`);
  return filepath;
}

/** Generate a flat comparison summary for graphing. */
export async function saveSummary(
  results: BenchmarkResult[],
  outputDir: string,
): Promise<string> {
  const filepath = `${outputDir}/competitor_comparison_benchmark.json`;
  const summary = results.flatMap((r) => [
    {
      competitor: r.competitor,
      metric: "cold_start_ms",
      value: r.cold_start.latency_ms,
      ts: r.timestamp,
    },
    {
      competitor: r.competitor,
      metric: "single_latency_median_ms",
      value: r.single_latency.median_ms,
      ts: r.timestamp,
    },
    {
      competitor: r.competitor,
      metric: "single_latency_p95_ms",
      value: r.single_latency.p95_ms,
      ts: r.timestamp,
    },
    {
      competitor: r.competitor,
      metric: "throughput_per_second",
      value: r.throughput.per_second,
      ts: r.timestamp,
    },
    {
      competitor: r.competitor,
      metric: "step_overhead_ms",
      value: r.step_overhead.per_step_ms,
      ts: r.timestamp,
    },
  ]);
  await Deno.writeTextFile(filepath, JSON.stringify(summary, null, 2));
  console.log(`Summary saved to ${filepath}`);
  return filepath;
}

/** Get approximate CPU count for environment metadata. */
export function getCpuCount(): number {
  return navigator.hardwareConcurrency ?? 0;
}

/** Get machine identifier from environment or hostname. */
export async function getMachineId(): Promise<string> {
  const github = Deno.env.get("RUNNER_NAME");
  if (github) return github;

  try {
    const cmd = new Deno.Command("hostname", { stdout: "piped" });
    const { stdout } = await cmd.output();
    return new TextDecoder().decode(stdout).trim();
  } catch (_) {
    return "unknown";
  }
}
