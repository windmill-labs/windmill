# Windmill benchmark sim

Provisioning layer on top of the existing benchmark harness. Reuses everything
in `benchmarks/` (workload kinds, `runBenchmark`, graph primitives) and adds:
declarative topology, per-node resource limits, optional per-node DB latency,
optional CPU pinning for benchmark accuracy, and per-worker metrics collection.

## Three node modes

Every node in a topology runs in one of three modes:

| Mode | What it is | How it's provisioned |
|------|-----------|----------------------|
| `native` | Windmill `MODE=standalone` — API server + workers in one process. The harness pushes jobs at its API. | One docker (or podman) container per node, `--cpus` / `--memory` / optional `--cpuset-cpus`. |
| `agent` | Windmill `MODE=worker` — worker-only, no API. Models a worker fleet that scales separately from the API. | Same as `native`, minus the API port mapping. Needs a peer `native` node so the harness has somewhere to push. |
| `k8s` | A k3d cluster with Windmill deployed via the helm chart (upstream or a locally-cloned copy). | `k3d cluster create` provisions k3s in containers, then `helm install` deploys Windmill onto it. Planned for M5 — schema and design are in place. |

The provisioner is a single class that handles all three. The container
runtime is configurable via `SIM_CONTAINER_CMD` (defaults to `docker`,
swap to `podman` if you prefer). For k8s mode, `k3d` and `helm` need to be
on `PATH`.

## Layered design

```
sim.ts                          (this layer)
  → loads suite + topology
  → SimProvisioner: PG + toxiproxy (if needed) + per-node provisioning
  → calls existing runBenchmark() against the chosen API node
  → Collector queries v2_job_completed / v2_job_queue from outside the cluster
  → writes JSON + markdown report
  → teardown
        │
        ▼
benchmarks/benchmark_oneoff.ts  (existing, unchanged)
  → login, createBenchScript, push jobs, poll for completion
        │
        ▼
Windmill instance(s)             (containers or k8s pods)
  → BENCHMARK_KIND env loads fixtures via backend bench.rs
  → workers run normal pull loop
        │
        ▼
Postgres container               (shared by every node)
```

## Two config files per run

1. **Topology** (e.g. `topologies/single_node.json`) — pure infra description.
2. **Suite** (e.g. `configs/baseline.json`) — workload, optionally references
   a topology by relative path.

Suites without a `topology` reference are first-class and continue to run via
`benchmark_suite.ts` against any existing Windmill at `--host`. Suites that
declare a topology run via `sim.ts` (this directory), which provisions infra
before delegating to the same underlying runner.

## Example topologies

- `topologies/single_node.json` — 1 PG + 1 native node. Baseline smoke test.
- `topologies/heterogeneous_latency.json` — 1 native + 1 agent at 50ms DB RTT.
  The "does the pull race self-level?" experiment.
- `topologies/k8s_3node.json` — k3d cluster shape: 3 worker pods. Helm chart
  selection is passed on the CLI (see below); the topology only describes the
  cluster.

## k8s mode: chart selection via CLI

The topology specifies cluster shape (pods, workers_per_pod, cluster_agents).
The helm chart itself is selected at the sim CLI:

```sh
# Upstream chart
deno run -A sim.ts -c configs/k8s.json \
  --helm-chart windmill/windmill \
  --helm-repo https://windmill-labs.github.io/windmill-helm-charts

# Locally-cloned chart (relative to cwd)
deno run -A sim.ts -c configs/k8s.json \
  --helm-chart ../windmill-helm-charts/charts/windmill

# Inline value overrides
deno run -A sim.ts -c configs/k8s.json --helm-chart windmill/windmill \
  --helm-set worker.replicaCount=5 \
  --helm-set windmill.numWorkers=2 \
  --helm-values-file ./extra-values.yaml
```

## Prerequisites

- A container runtime: `docker` (default) or `podman` (set
  `SIM_CONTAINER_CMD=podman`).
- A Windmill image built with the `benchmark` Cargo feature, tagged
  `windmill-bench:latest`. Override the tag via `SIM_WM_IMAGE`. The image
  must honour `DATABASE_URL`, `NUM_WORKERS`, `MODE` (`standalone` or
  `worker`), `BENCHMARK_KIND`, and `WORKER_TAGS`.
- Postgres 17 image (default `postgres:17`, override via `SIM_PG_IMAGE`).
- For k8s mode only: `k3d` and `helm` on `PATH`.

## Run

```sh
cd benchmarks/sim
deno run -A sim.ts -c configs/baseline.json
```

Useful flags:

- `--factor N` — multiply every benchmark's `jobs` count by N.
- `--keep-infra` — leave containers running after the run.
- `--snapshot-interval <ms>` — queue-depth sampling cadence (default 1000).

## Outputs

Each run writes two files in the working directory:

- `sim_results_<ts>.json` — raw structured data: workload results,
  per-worker stats, queue snapshots, derived throughput.
- `sim_results_<ts>.md` — human-readable summary: throughput table,
  per-worker table sorted by jobs handled, **utilization spread** headline,
  ASCII sparkline of queue depth over time.

The headline metric for the question this sim is built to answer ("are
workers balanced?") is the utilization spread line. A small spread (e.g.
±5pp) means the queue is self-balancing. A large spread (e.g. close
worker 95%, far worker 40%) is the smoking gun for uneven pull dynamics.

## Resource validation

`sim.ts` refuses to run if the topology's total CPU or memory exceeds the
host's available budget after the `reserved` buffer. `host_resources` is
optional in the topology — when omitted, the validator falls back to the
live host's logical core count and `MemTotal`. Declare it explicitly when
you want the topology to be portable to a specific target host.

Set `pin_cpus: true` on the topology to assign each container a disjoint
`--cpuset-cpus` range. This eliminates CFS time-slicing between containers
entirely; useful when two nodes' throughput numbers must not be
artificially correlated.

## Tests

```sh
nix develop --command bash -c "cd benchmarks && deno test -A sim/*_test.ts"
```

38 unit tests cover the validator, config loader, proxy planner, schema
modes, and report writer. The provisioner and Postgres collector are
intentionally not unit-tested — they need real infrastructure to exercise.

## Roadmap

- **M1 ✅**: scaffold + single-node baseline + container provisioner +
  resource validator.
- **M2 ✅**: multi-node + per-node toxiproxy latency.
- **M3 ✅**: metrics collection from Postgres (per-worker utilization,
  queue depth over time).
- **M4 ✅**: markdown + JSON report. SVG graphs deferred until needed.
- **M5**: k3d-based k8s mode with helm install (upstream or local chart).
- **M6+**: chaos timeline, assertion engine, resilience test corpus.
