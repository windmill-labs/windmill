/**
 * Main benchmark orchestrator.
 *
 * Runs cold start, single latency, throughput, and step overhead tests
 * against one or more competitor adapters sequentially.
 */

import type { CompetitorAdapter, BenchmarkResult } from "./types.ts";
import {
  warmup,
  collectLatencySamples,
  measureThroughput,
  computeStats,
} from "./lib/timing.ts";
import { saveResult, saveSummary, getCpuCount, getMachineId } from "./lib/results.ts";
import { composeLogs } from "./lib/docker.ts";

export interface HarnessOptions {
  latencySamples: number;
  throughputBatch: number;
  warmupCount: number;
  outputDir: string;
}

const DEFAULTS: HarnessOptions = {
  latencySamples: 50,
  throughputBatch: 100,
  warmupCount: 5,
  outputDir: ".",
};

export async function runBenchmark(
  adapter: CompetitorAdapter,
  opts: Partial<HarnessOptions> = {},
): Promise<BenchmarkResult> {
  const options = { ...DEFAULTS, ...opts };
  const machine = await getMachineId();
  const numCpus = getCpuCount();

  console.log(`\n${"=".repeat(60)}`);
  console.log(`Benchmarking: ${adapter.name}`);
  console.log(`${"=".repeat(60)}`);

  // 1. Setup
  console.log(`[${adapter.name}] Setting up containers...`);
  try {
    await adapter.setup();
  } catch (e) {
    console.error(`[${adapter.name}] Setup failed, dumping logs...`);
    try {
      console.error(await composeLogs(adapter.composeFile));
    } catch (_) {
      // ignore
    }
    throw e;
  }

  let version = "unknown";
  try {
    version = await adapter.getVersion();
  } catch (_) {
    // non-critical
  }
  console.log(`[${adapter.name}] Version: ${version}`);

  // 2. Deploy workflow
  console.log(`[${adapter.name}] Deploying workflow...`);
  await adapter.deployWorkflow();

  // 3. Cold start
  console.log(`[${adapter.name}] Measuring cold start...`);
  const coldStartResult = await adapter.triggerOne();
  const coldStartMs = coldStartResult.latencyMs;
  console.log(`[${adapter.name}] Cold start: ${coldStartMs.toFixed(1)}ms`);

  // 4. Warmup
  console.log(`[${adapter.name}] Warming up (${options.warmupCount} runs)...`);
  await warmup(adapter, options.warmupCount);

  // 5. Single execution latency
  console.log(
    `[${adapter.name}] Collecting ${options.latencySamples} latency samples...`,
  );
  const samples = await collectLatencySamples(adapter, options.latencySamples);
  const latencyStats = computeStats(samples);
  console.log(
    `[${adapter.name}] Latency: median=${latencyStats.median_ms}ms p95=${latencyStats.p95_ms}ms mean=${latencyStats.mean_ms}ms`,
  );

  // 6. Throughput
  console.log(
    `[${adapter.name}] Measuring throughput (batch of ${options.throughputBatch})...`,
  );
  const { totalMs, perSecond } = await measureThroughput(
    adapter,
    options.throughputBatch,
  );
  console.log(
    `[${adapter.name}] Throughput: ${perSecond.toFixed(2)} workflows/s (${totalMs.toFixed(0)}ms total)`,
  );

  // 7. Step overhead
  const stepOverheadMs = Math.round((latencyStats.median_ms / 3) * 100) / 100;
  console.log(`[${adapter.name}] Step overhead: ${stepOverheadMs}ms/step`);

  // 8. Teardown
  console.log(`[${adapter.name}] Tearing down...`);
  await adapter.teardown();

  // 9. Build result
  const result: BenchmarkResult = {
    competitor: adapter.name,
    timestamp: Date.now(),
    environment: {
      machine,
      competitor_version: version,
      num_cpus: numCpus,
    },
    cold_start: { latency_ms: coldStartMs },
    single_latency: latencyStats,
    throughput: {
      batch_size: options.throughputBatch,
      total_ms: totalMs,
      per_second: perSecond,
    },
    step_overhead: { per_step_ms: stepOverheadMs },
  };

  await saveResult(result, options.outputDir);
  return result;
}

export async function runAllBenchmarks(
  adapters: CompetitorAdapter[],
  opts: Partial<HarnessOptions> = {},
): Promise<BenchmarkResult[]> {
  const results: BenchmarkResult[] = [];

  for (const adapter of adapters) {
    try {
      const result = await runBenchmark(adapter, opts);
      results.push(result);
    } catch (e) {
      console.error(`\n[${adapter.name}] BENCHMARK FAILED:`, e);
      // Continue with next competitor
      try {
        await adapter.teardown();
      } catch (_) {
        // best effort cleanup
      }
    }
  }

  if (results.length > 0) {
    const outputDir = opts.outputDir ?? ".";
    await saveSummary(results, outputDir);
  }

  // Print summary table
  console.log(`\n${"=".repeat(60)}`);
  console.log("RESULTS SUMMARY");
  console.log(`${"=".repeat(60)}`);
  console.log(
    `${"Competitor".padEnd(15)} ${"Cold Start".padEnd(12)} ${"Median".padEnd(12)} ${"P95".padEnd(12)} ${"Throughput".padEnd(15)} ${"Step OH".padEnd(10)}`,
  );
  console.log("-".repeat(76));
  for (const r of results) {
    console.log(
      `${r.competitor.padEnd(15)} ${(r.cold_start.latency_ms.toFixed(1) + "ms").padEnd(12)} ${(r.single_latency.median_ms.toFixed(1) + "ms").padEnd(12)} ${(r.single_latency.p95_ms.toFixed(1) + "ms").padEnd(12)} ${(r.throughput.per_second.toFixed(2) + "/s").padEnd(15)} ${(r.step_overhead.per_step_ms.toFixed(1) + "ms").padEnd(10)}`,
    );
  }

  return results;
}
