/**
 * Shared types for the competitor benchmark framework.
 *
 * Every competitor implements CompetitorAdapter. The harness handles
 * all timing, statistics, and result serialization.
 */

export interface CompetitorAdapter {
  readonly name: string;
  readonly composeFile: string;

  /** Start containers, wait for health checks. */
  setup(): Promise<void>;

  /** Deploy/register the 3-step sequential workflow. */
  deployWorkflow(): Promise<void>;

  /** Trigger one workflow execution. Returns wall-clock ms and result. */
  triggerOne(): Promise<{ latencyMs: number; result: unknown }>;

  /** Trigger N concurrent workflow executions. Returns wall-clock ms for all to complete. */
  triggerBatch(n: number): Promise<{ totalMs: number; results: unknown[] }>;

  /** Tear down all containers. */
  teardown(): Promise<void>;

  /** Return the competitor server version string. */
  getVersion(): Promise<string>;
}

export interface LatencyStats {
  samples: number[];
  median_ms: number;
  p95_ms: number;
  mean_ms: number;
  stdev_ms: number;
}

export interface BenchmarkResult {
  competitor: string;
  timestamp: number;
  environment: {
    machine: string;
    competitor_version: string;
    num_cpus: number;
  };
  cold_start: {
    latency_ms: number;
  };
  single_latency: LatencyStats;
  throughput: {
    batch_size: number;
    total_ms: number;
    per_second: number;
  };
  step_overhead: {
    per_step_ms: number;
  };
}

export interface SuiteConfig {
  competitors: string[];
  latency_samples: number;
  throughput_batch: number;
  warmup_count: number;
}
