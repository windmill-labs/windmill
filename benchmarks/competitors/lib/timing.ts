/**
 * Precision timing utilities for benchmark measurements.
 */

import type { CompetitorAdapter, LatencyStats } from "../types.ts";

/** Measure a single workflow execution latency in ms. */
export async function measureLatency(
  adapter: CompetitorAdapter,
): Promise<{ latencyMs: number; result: unknown }> {
  const start = performance.now();
  const { result } = await adapter.triggerOne();
  const latencyMs = performance.now() - start;
  return { latencyMs, result };
}

/** Run N warmup executions (results discarded). */
export async function warmup(
  adapter: CompetitorAdapter,
  count: number,
): Promise<void> {
  for (let i = 0; i < count; i++) {
    await adapter.triggerOne();
  }
}

/** Collect N latency samples sequentially. */
export async function collectLatencySamples(
  adapter: CompetitorAdapter,
  count: number,
): Promise<number[]> {
  const samples: number[] = [];
  for (let i = 0; i < count; i++) {
    const { latencyMs } = await measureLatency(adapter);
    samples.push(latencyMs);
  }
  return samples;
}

/** Measure throughput by triggering a batch and timing total completion. */
export async function measureThroughput(
  adapter: CompetitorAdapter,
  batchSize: number,
): Promise<{ totalMs: number; perSecond: number }> {
  const start = performance.now();
  await adapter.triggerBatch(batchSize);
  const totalMs = performance.now() - start;
  return {
    totalMs,
    perSecond: (batchSize / totalMs) * 1000,
  };
}

/** Compute statistics from a set of latency samples. */
export function computeStats(samples: number[]): LatencyStats {
  const sorted = [...samples].sort((a, b) => a - b);
  const n = sorted.length;
  const mean = sorted.reduce((a, b) => a + b, 0) / n;
  const variance = sorted.reduce((sum, v) => sum + (v - mean) ** 2, 0) / n;

  return {
    samples: sorted,
    median_ms: percentile(sorted, 50),
    p95_ms: percentile(sorted, 95),
    mean_ms: round(mean),
    stdev_ms: round(Math.sqrt(variance)),
  };
}

function percentile(sorted: number[], p: number): number {
  const idx = Math.ceil((p / 100) * sorted.length) - 1;
  return round(sorted[Math.max(0, idx)]);
}

function round(v: number): number {
  return Math.round(v * 100) / 100;
}
