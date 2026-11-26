# Competitor Benchmarks

Performance benchmarks comparing Windmill's workflow-as-code against Temporal, Inngest, Restate, and Kestra.

## What's Measured

All platforms run the **same logical workflow**: 3 sequential steps, each returning a trivial integer. This isolates orchestration overhead from actual compute.

| Metric | Description |
|--------|-------------|
| **Cold start** | First execution latency after fresh deploy |
| **Single latency** | Median + P95 of individual end-to-end execution times |
| **Throughput** | Concurrent workflow completions per second |
| **Step overhead** | Per-step orchestration overhead (median_latency / 3) |

## Quick Start

```bash
# Install Deno
curl -fsSL https://deno.land/install.sh | sh

# Run all competitors (requires Docker)
deno run -A competitor_suite.ts

# Run specific competitors
deno run -A competitor_suite.ts --competitors windmill,temporal

# Custom parameters
deno run -A competitor_suite.ts \
  --competitors windmill,inngest,restate \
  --latency-samples 100 \
  --throughput-batch 200 \
  --output-dir ./results

# Generate graphs from results
deno run -A competitor_graphs.ts \
  -c competitor_graphs_config.json \
  --results-dir ./results
```

## Prerequisites

- [Deno](https://deno.land/) v2+
- [Docker](https://www.docker.com/) with Docker Compose v2
- [Node.js](https://nodejs.org/) 20+ (for Temporal client subprocess)
- ~8GB RAM (competitors run sequentially, one at a time)

## Workflow Equivalence

Each platform implements the same 3-step sequential workflow:

| Platform | Implementation |
|----------|---------------|
| **Windmill** | `task()` + `workflow()` from `windmill-client` |
| **Temporal** | `proxyActivities()` with 3 sequential activity calls |
| **Inngest** | `inngest.createFunction()` with 3 `step.run()` calls |
| **Restate** | `restate.workflow()` with 3 `ctx.run()` calls |
| **Kestra** | YAML flow with 3 `io.kestra.plugin.core.debug.Return` tasks |

## Architecture

```
competitor_suite.ts          CLI entrypoint
  └── competitor_harness.ts  Orchestrator (setup → deploy → test → teardown)
        ├── lib/docker.ts    Docker Compose lifecycle
        ├── lib/timing.ts    Latency/throughput measurement
        ├── lib/results.ts   JSON output + statistics
        └── {competitor}/
              ├── adapter.ts         CompetitorAdapter implementation
              ├── docker-compose.yml Container stack
              └── workflow.*         Workflow definition
```

Each competitor gets its own Docker Compose stack. Competitors run sequentially with full teardown between runs to avoid resource contention.

## Output

Results are written to `./results/` as JSON:
- `{competitor}_competitor_benchmark.json` — Per-competitor detailed results
- `competitor_comparison_benchmark.json` — Flat summary for graphing
- `competitor_*.svg` — Bar chart visualizations

## CI

The GitHub Actions workflow (`.github/workflows/benchmark-competitors.yml`) runs weekly on Monday. Results are committed to the `benchmarks` branch.
