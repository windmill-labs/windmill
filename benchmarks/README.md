# Benchmarks

Deno/TS benchmark suite for measuring Windmill job and flow execution throughput.

## Quick Start

```bash
# Install Deno
curl -fsSL https://deno.land/install.sh | sh

# Run a single benchmark
deno run -A benchmark_oneoff.ts --kind noop --jobs 10000

# Run the full suite
deno run -A benchmark_suite.ts -c suite_config.json

# Run WAC v2 benchmarks (workflow-as-code vs flow comparison)
deno run -A benchmark_suite.ts -c suite_wac.json
```

## Benchmark Kinds

### Script benchmarks
- `noop` — Empty jobs (measures pure scheduling overhead)
- `deno`, `bun`, `python`, `go`, `bash` — Language runtimes
- `nativets` — BunNative (no isolation)
- `dedicated`, `dedicated_nativets` — Dedicated worker mode

### Flow benchmarks
- `2steps` — 2-step flow (deno + identity)
- `bigscriptinflow` — Flow with large raw bash script
- `flow_seq_2_bun` — 2 sequential bun steps
- `flow_par_2_bun` — 2 parallel bun steps (branchall)
- `flow_seq_3_bun` — 3 sequential bun steps
- `flow:<path>` — Custom flow by path
- `script:<path>` — Custom script by path

### WAC v2 benchmarks (workflow-as-code)
- `wac_seq_2` — 2 sequential tasks
- `wac_par_2` — 2 parallel tasks (Promise.all)
- `wac_seq_3` — 3 sequential tasks
- `wac_inline_2` — 2 inline steps (no child jobs)

## Suite Configs

| File | Description |
|------|-------------|
| `suite_config.json` | Main benchmark suite (noop, languages, flows) |
| `suite_dedicated.json` | Dedicated worker benchmarks |
| `suite_dedicated_nativets.json` | Dedicated NativeTS benchmarks |
| `suite_wac.json` | WAC v2 vs flow comparison benchmarks |

## Interactive Benchmark Tool

```bash
deno run -A main.ts -e admin@windmill.dev -p changeme --host http://localhost:8000
```

Options: `--workers`, `--seconds`, `--maximum-throughput`, `--use-flows`, `--script-pattern`, `--export-json`, `--export-csv`

## Graph Generation

```bash
deno run -A benchmark_graphs.ts -c graphs_config.json
```

Generates SVG graphs from `*_benchmark.json` data files.

## CI

The GitHub Actions workflow (`.github/workflows/benchmark.yml`) runs hourly with 1/4/8 worker configurations plus WAC benchmarks. Results are committed to the `benchmarks` branch.

## Cluster benchmarks — sim mode

Provisioning + measuring throughput on a real multi-node Kubernetes cluster
(minikube + helm + per-pod cgroup sampling + PG analysis) is a separate
workflow on top of `main.ts`. See [`sim/README.md`](sim/README.md) for:

- bringing up the cluster (`wm_sim up`)
- firing a phased or flood workload from `workloads/`
- the consolidated dashboard with throughput, queue depth, per-node CPU +
  oversaturation, PG latency / conn counts, OOM events, restart events
- the JSONL poller pipeline + reliability fixes (sampler host-log, rollout-
  complete readiness, procs_running oversaturation)

Quick path:

```sh
# Provision cluster + deploy Windmill (foreground; Ctrl-C tears down).
# Prints `[helm] API reachable at http://127.0.0.1:<port>` — note the port.
# Port-forward stays alive as a child of wm_sim, so keep this terminal open.
wm_sim up \
  --topology    sim/topologies/k8s-4node.json \
  --helm        ../windmill-helm-charts/charts/windmill \
  --helm-values sim/values/smoke.yaml \
  --helm-values sim/values/local.yaml

# In another shell — fire bench against the wm_sim-managed port:
deno run -A main.ts \
  --host http://127.0.0.1:<port> \
  --token <admin-token> \
  --workload-config workloads/io_150ms_flood.json \
  --minikube-profile wm-sim-k8s-4node \
  --wait-ready 60

# Report appears at reports/<timestamp>/dashboard.svg
```
