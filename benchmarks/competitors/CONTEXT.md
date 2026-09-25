# Competitor Benchmark Framework — Context for Continuation

## Goal

Reproducible performance benchmarks comparing Windmill's workflow-as-code (WAC) against competitors, for SEO comparison pages. All platforms run an equivalent **3-step sequential workflow** with inline step execution (no subprocess/child job dispatch). This isolates orchestration overhead from actual compute.

## Current Competitors (7)

| Competitor | Version Tested | Adapter | Workflow Type |
|---|---|---|---|
| **Windmill** | CE v1.673.0 | `windmill/adapter.ts` | `step()` from `windmill-client` (inline, no child jobs) |
| **Temporal** | auto-setup:latest | `temporal/adapter.ts` | `proxyActivities()` with 3 sequential activities |
| **Inngest** | inngest:latest | `inngest/adapter.ts` | `step.run()` × 3 via Express app |
| **Restate** | restate:latest | `restate/adapter.ts` | `ctx.run()` × 3 via Node.js endpoint |
| **Kestra** | 1.3.7 | `kestra/adapter.ts` | `io.kestra.plugin.core.debug.Return` × 3 |
| **Prefect** | 3.6.25 | `prefect/adapter.ts` | `@task` × 3 with `.serve()` runner |
| **Airflow** | 2.10.5 | `airflow/adapter.ts` | `PythonOperator` × 3 with LocalExecutor |

## Latest Results (local dev machine, 2026-04-03)

| Competitor | Cold Start | Median Latency | P95 Latency | Throughput | Step Overhead | Workers |
|---|---|---|---|---|---|---|
| Restate | 15ms | 4.1ms | 5.0ms | 2,057/s | 1.4ms | event loop |
| **Windmill** | **106ms** | **103ms** | **105ms** | **152/s** | **34ms** | 10 |
| Temporal | 166ms | 119ms | 283ms | 39/s | 40ms | 10 |
| Kestra | 410ms | 177ms | 326ms | 68/s | 59ms | JVM threads |
| Inngest | 313ms | 260ms | 261ms | 90/s | 87ms | 10 |
| Airflow | 3,858ms | 3,792ms | 4,277ms | 7.4/s | 1,264ms | LocalExecutor |
| Prefect | 2,764ms | 9,679ms | 12,631ms | 1.3/s | 3,226ms | .serve() |

## Architecture

```
benchmarks/competitors/
├── types.ts                    # CompetitorAdapter interface + BenchmarkResult types
├── competitor_harness.ts       # Main orchestrator: for each competitor → setup → deploy → test → teardown
├── competitor_suite.ts         # CLI wrapper (cliffy): --competitors, --latency-samples, --throughput-batch, --output-dir
├── competitor_graphs.ts        # SVG bar chart generation (D3 + JSDOM)
├── competitor_suite_config.json
├── competitor_graphs_config.json
├── lib/
│   ├── docker.ts               # composeUp(), composeDown(), waitForHealth(), composeLogs()
│   ├── timing.ts               # measureLatency(), warmup(), collectLatencySamples(), computeStats()
│   └── results.ts              # saveResult(), saveSummary(), getCpuCount(), getMachineId()
├── .gitignore                  # excludes node_modules/ and results/
├── {competitor}/
│   ├── adapter.ts              # implements CompetitorAdapter
│   ├── docker-compose.yml      # isolated container stack
│   ├── workflow.*              # equivalent workflow definition
│   └── Dockerfile.*            # (some competitors need custom app/worker images)
└── results/                    # (gitignored) JSON output from benchmark runs
```

## CompetitorAdapter Interface

```typescript
interface CompetitorAdapter {
  readonly name: string;
  readonly composeFile: string;
  setup(): Promise<void>;           // docker compose up + health wait
  deployWorkflow(): Promise<void>;  // register/create the 3-step workflow
  triggerOne(): Promise<{ latencyMs: number; result: unknown }>;
  triggerBatch(n: number): Promise<{ totalMs: number; results: unknown[] }>;
  teardown(): Promise<void>;        // docker compose down -v
  getVersion(): Promise<string>;
}
```

## Harness Flow (per competitor)

1. `setup()` — `docker compose up -d` + wait for health endpoint
2. `deployWorkflow()` — register the 3-step workflow via REST API / SQL / CLI
3. **Cold start** — `triggerOne()` immediately after deploy
4. **Warmup** — `triggerOne()` × N, discard results
5. **Single latency** — `triggerOne()` × N, collect `performance.now()` timings → compute median, P95, mean, stdev
6. **Throughput** — `triggerBatch(N)` (concurrent `Promise.all`), measure total wall-clock time
7. **Step overhead** — median_latency / 3 (trivial steps, so overhead ≈ latency)
8. `teardown()` — `docker compose down -v`
9. Save JSON results

## How to Run

```bash
cd benchmarks/competitors

# Single competitor
deno run -A competitor_suite.ts --competitors windmill --latency-samples 50 --throughput-batch 100 --output-dir ./results

# All competitors
deno run -A competitor_suite.ts --latency-samples 100 --throughput-batch 200 --warmup-count 10 --output-dir ./results

# Generate SVG graphs
deno run -A --allow-import competitor_graphs.ts -c competitor_graphs_config.json --results-dir ./results
```

## Key Decisions & Gotchas

### Windmill
- Uses `step()` (inline), NOT `task()` (child jobs). Other competitors don't create separate jobs per step either, so this is the fair comparison.
- `WORKER_GROUP=main` + `NUM_WORKERS=10` + `I_ACK_NUM_WORKERS_IS_UNSAFE=1` + `SLEEP_QUEUE=50`. Without `SLEEP_QUEUE=50`, the queue poll interval scales to `50ms × NUM_WORKERS / 2 = 250ms`, killing latency. See `backend/windmill-worker/src/worker.rs:318-329`.
- `WORKER_TAGS=bun,flow,dependency` — must include `bun` since WAC scripts deploy as bun language.
- Cannot use `WORKER_GROUP=native` or `NATIVE_MODE=true` because native mode forces `NUM_WORKERS=8` and doesn't support the `bun` tag needed for WAC scripts.

### Temporal
- Needs a **separate worker container** (Dockerfile.worker) because the Temporal TypeScript SDK has native gRPC bindings that don't work in Deno.
- The adapter uses `node -e` subprocess calls to run Temporal client operations (connect, start workflow, get result).
- `npm install` in `temporal/` is needed before running (the adapter does this in `setup()`).
- `maxConcurrentWorkflowTaskExecution: 10` and `maxConcurrentActivityTaskExecution: 10` in worker.ts to match Windmill's 10 workers.
- Cold start is high (~6s) because `temporalio/auto-setup` does DB schema migration on first boot. Not representative of steady-state.

### Inngest
- The dev server's event run status API (`/v1/events/{id}/runs`) **caches responses for 15 seconds**. Must add `?ts=${Date.now()}` cache-buster to polling requests, otherwise each step appears to take 5 seconds.
- `--queue-workers 10` caps concurrency to match other competitors.
- `--tick 10` (10ms) for fast queue polling, `--poll-interval 1` for fast app sync.
- The Express app needs `express.json()` middleware or Inngest SDK returns 500 "Missing body".
- `--no-discovery` is set because auto-discovery is unreliable in Docker networks.

### Restate
- Extremely fast (4ms median) because it runs workflow steps **in-process** in the app's Node.js event loop. No queue hop, no network round-trip between steps. This is a fundamental architectural difference from Windmill/Temporal/Inngest.
- Uses `send` + `attach` pattern: POST `/benchmark/{id}/run/send` then GET `/restate/workflow/benchmark/{id}/attach`.
- The app must be registered with the Restate admin API: POST `http://admin:9070/deployments` with `{"uri": "http://app:9080"}`.
- Ports remapped to 8085/9075 to avoid conflict with port 8080 (often in use).

### Kestra
- Kestra 1.3.x has **mandatory basic auth** that cannot be disabled via config. The `/api/v1/flows` endpoint always returns 401.
- Workaround: deploy the flow via **direct SQL insertion** into Kestra's Postgres `flows` table. Key format is `main_{namespace}_{id}_{revision}`. The `value` JSONB must include `tenantId: "main"`, `source`, `deleted: false`, etc. (see existing tutorial flows for format).
- Trigger via the **webhook endpoint** (`/api/v1/executions/webhook/{namespace}/{flowId}/{key}`) which is public.
- Poll execution status via SQL: `SELECT value->>'state' FROM executions WHERE key = '{executionId}'`.
- Uses `io.kestra.plugin.core.debug.Return` tasks (not shell tasks) to avoid subprocess overhead.

### Prefect
- Very slow (~9.7s/workflow) because `.serve()` mode spawns a new subprocess per flow run. Prefect is designed for data pipeline tasks, not lightweight orchestration.
- The worker container runs `python flow.py` which calls `.serve(name="benchmark-deployment")` — this both registers the deployment and runs a worker loop.
- Deployment ID must be fetched via `GET /api/deployments/name/{flow_name}/{deployment_name}` before triggering.
- No auth required on self-hosted Prefect OSS server.

### Airflow
- Slow (~3.8s/workflow) due to scheduler overhead and PythonOperator subprocess execution.
- DAG file is volume-mounted to `./dags/`. Scheduler detects it after `DAG_DIR_LIST_INTERVAL` (set to 5s).
- DAGs are paused by default — must PATCH `/api/v1/dags/{dag_id}` with `{"is_paused": false}`.
- All API calls require Basic Auth: `Authorization: Basic YWRtaW46YWRtaW4=` (admin:admin).
- `airflow-init` container runs DB migration + user creation, then exits. Webserver and scheduler depend on it via `service_completed_successfully`.
- Webserver port remapped to 8090 to avoid conflict with 8080.

### Docker / Infrastructure
- All custom app/worker images must be **pre-built** before running (`docker build -t {name} -f Dockerfile ...`). Docker Compose's `build:` directive times out in some environments.
- Compose files use pre-built image references (e.g., `image: temporal-benchmark-worker`) not `build:` blocks.
- Docker Compose `--wait` flag was removed from `composeUp()` because it caused timeout failures. Each adapter handles its own readiness checking via `waitForHealth()` or custom polling.
- Competitors run sequentially with full `docker compose down -v` between runs to avoid resource contention and port conflicts.
- **Disk space**: Running all 7 competitors needs ~15GB for Docker images. Kestra (3.3GB) and Airflow (1.5GB) are the largest. Use `docker system prune -af` between runs if tight on space.

### Graphs
- `competitor_graphs.ts` generates SVG bar charts using D3 + JSDOM (same stack as existing `benchmarks/graph.ts`).
- Reads from `results/competitor_comparison_benchmark.json` (flat summary format).
- Config in `competitor_graphs_config.json` defines 5 charts: cold start, median latency, P95, throughput, step overhead.
- D3 callback params need explicit `any` types to pass Deno type checking.

## CI

`.github/workflows/benchmark-competitors.yml`:
- Runs weekly (Monday 6AM UTC) + manual dispatch
- `ubicloud-standard-8` runner
- Pre-pulls all Docker images in parallel
- Sequential competitor runs
- Results committed to `benchmarks` branch

## Potential Improvements

1. **Dagster adapter** — Another popular Python DAG-based orchestrator (competitor to Airflow/Prefect)
2. **Hatchet adapter** — Rising Postgres-backed task orchestrator (YC W24)
3. **Multi-worker scaling test** — Run with 1, 4, 8, 16 workers to show scaling curves
4. **Windmill `task()` benchmark** — Add a separate test using `task()` (child jobs) for users who need per-step isolation
5. **Payload size test** — Steps that pass non-trivial data (1KB, 100KB, 1MB) to measure serialization overhead
6. **Error/retry test** — Steps that fail and retry to measure recovery overhead
7. **Long-running workflow test** — 100+ steps to measure checkpoint overhead at scale
8. **Graph improvements** — Add error bars, include version numbers in chart labels
9. **Prefect worker pool mode** — Test with `Process` work pool instead of `.serve()` for potentially better throughput
10. **Airflow CeleryExecutor** — Test with Celery + Redis for parallel task execution instead of LocalExecutor
