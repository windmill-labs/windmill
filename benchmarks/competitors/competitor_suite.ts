/**
 * CLI wrapper for the competitor benchmark harness.
 *
 * Usage:
 *   deno run -A competitor_suite.ts
 *   deno run -A competitor_suite.ts --competitors windmill,temporal
 *   deno run -A competitor_suite.ts --latency-samples 100 --throughput-batch 200
 */

import { Command } from "https://deno.land/x/cliffy@v0.25.7/command/mod.ts";
import { runAllBenchmarks } from "./competitor_harness.ts";
import type { CompetitorAdapter, SuiteConfig } from "./types.ts";

// Import all adapters
import { windmillAdapter } from "./windmill/adapter.ts";
import { temporalAdapter } from "./temporal/adapter.ts";
import { inngestAdapter } from "./inngest/adapter.ts";
import { restateAdapter } from "./restate/adapter.ts";
import { kestraAdapter } from "./kestra/adapter.ts";
import { prefectAdapter } from "./prefect/adapter.ts";
import { airflowAdapter } from "./airflow/adapter.ts";

const ALL_ADAPTERS: Record<string, CompetitorAdapter> = {
  windmill: windmillAdapter,
  temporal: temporalAdapter,
  inngest: inngestAdapter,
  restate: restateAdapter,
  kestra: kestraAdapter,
  prefect: prefectAdapter,
  airflow: airflowAdapter,
};

async function main({
  competitors,
  configPath,
  latencySamples,
  throughputBatch,
  warmupCount,
  outputDir,
}: {
  competitors?: string;
  configPath?: string;
  latencySamples?: number;
  throughputBatch?: number;
  warmupCount?: number;
  outputDir: string;
}) {
  let config: Partial<SuiteConfig> = {};

  // Load config file if provided
  if (configPath) {
    config = JSON.parse(await Deno.readTextFile(configPath));
  }

  // CLI flags override config file
  const competitorNames = competitors
    ? competitors.split(",").map((c) => c.trim())
    : config.competitors ?? Object.keys(ALL_ADAPTERS);

  const adapters: CompetitorAdapter[] = [];
  for (const name of competitorNames) {
    const adapter = ALL_ADAPTERS[name];
    if (!adapter) {
      console.error(`Unknown competitor: ${name}. Available: ${Object.keys(ALL_ADAPTERS).join(", ")}`);
      Deno.exit(1);
    }
    adapters.push(adapter);
  }

  console.log(`Competitors: ${adapters.map((a) => a.name).join(", ")}`);
  console.log(`Output directory: ${outputDir}`);

  // Ensure output directory exists
  try {
    await Deno.mkdir(outputDir, { recursive: true });
  } catch (_) {
    // already exists
  }

  await runAllBenchmarks(adapters, {
    latencySamples: latencySamples ?? config.latency_samples ?? 50,
    throughputBatch: throughputBatch ?? config.throughput_batch ?? 100,
    warmupCount: warmupCount ?? config.warmup_count ?? 5,
    outputDir,
  });
}

await new Command()
  .name("competitor-bench")
  .description(
    "Run workflow-as-code performance benchmarks against competitor platforms.",
  )
  .version("1.0.0")
  .option(
    "--competitors <list:string>",
    "Comma-separated competitor names (default: all)",
  )
  .option(
    "-c --config-path <path:string>",
    "Path to suite config JSON",
  )
  .option(
    "--latency-samples <n:number>",
    "Number of single-execution latency samples",
  )
  .option(
    "--throughput-batch <n:number>",
    "Number of concurrent executions for throughput test",
  )
  .option(
    "--warmup-count <n:number>",
    "Number of warmup executions before latency test",
  )
  .option(
    "--output-dir <path:string>",
    "Directory to write result JSON files",
    { default: "./results" },
  )
  .action(main)
  .parse();
