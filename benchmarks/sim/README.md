# Windmill benchmark sim

Provisions a minikube-backed multi-node Kubernetes cluster, deploys Windmill
on it via Helm, runs workload benches against it, and produces a single SVG
dashboard with throughput, queue depth, per-node CPU + memory + oversaturation,
per-pod CPU/memory peaks, PG response latency + connection counts, OOM and
restart events, and pod inventory.

The goal: realistic, reproducible benches at scales where running Windmill
on bare containers (`docker run`) hides the production failure modes
(scheduler saturation, cgroup_mutex contention, PG backend thrash, kubelet
log rotation, OOM cascades). Everything here exists to make those
production-shape problems observable in 3 minutes from a clean checkout.

## What you actually get

Single CLI to bring up the cluster + deploy Windmill:

```sh
wm_sim up \
  --topology  sim/topologies/k8s-4node.json \
  --helm      ../windmill-helm-charts/charts/windmill \
  --helm-values sim/values/smoke.yaml \
  --helm-values sim/values/local.yaml      # gitignored; carries the EE license
```

`wm_sim up` opens its own port-forward to `svc/windmill-app` on a free local
port and prints the URL — look for a line like:

```
[helm] API reachable at http://127.0.0.1:38291 (port-forward svc/windmill-app)
```

The forward dies with the wm_sim process, so leave the wm_sim terminal open.
Then in a second shell, fire a bench against that URL:

```sh
cd benchmarks && deno run -A main.ts \
  --host http://127.0.0.1:<port from wm_sim output> \
  --token <admin-token> \
  --workload-config workloads/io_150ms_flood.json \
  --minikube-profile wm-sim-k8s-4node \
  --wait-ready 60
```

Bench runs ~3 min. Report lands at `benchmarks/reports/<timestamp>/`, with
`dashboard.svg` as the entry point. Open it in a browser.

If the port-forward dies mid-bench (kubelet sometimes drops it under heavy
load), restart it manually in a second shell with the same port:

```sh
kubectl --context wm-sim-k8s-4node port-forward -n default \
  svc/windmill-app <same port>:8000
```

## Prerequisites

- `nix develop` from the repo flake — provisions `minikube`, `kubectl`, `helm`,
  `pgbadger`, `deno`, and the `wm_sim` wrapper. **No host config required
  beyond the flake** with one exception: if you want multi-node KVM clusters
  on NixOS, libvirtd needs to be enabled at the system level — see
  `sim/nix/kvm2-driver.nix`.
- Root + libvirtd for the kvm2 driver. Single-node clusters work with the
  qemu2 driver without root, but multi-node needs kvm2.
- A clone of `windmill-helm-charts` next to this repo (`../windmill-helm-charts`).
  The vendored chart was deleted in favor of using the upstream chart with
  bench-specific overlays in `values/smoke.yaml`. The chart depends on a few
  knobs (PriorityClass support, `oomImmune`, `maxConnections`) that may not
  be in upstream yet — see the windmill-helm-charts PR.

## Topology + values layout

```
sim/
  topologies/
    k8s-4node.json          # 1 control plane + 3 workers (4 vCPU, 20 GiB each)
    k8s-3node.json          # control plane + 2 workers
    ...                     # several other shapes
  values/
    smoke.yaml              # bench-tuned helm overlay (worker reqs, PG sizing,
                            # priorityClass=wm-critical, maxConnections, etc.)
    local.example.yaml      # template — copy to local.yaml, fill in your
                            # EE license key. local.yaml is gitignored.
```

`smoke.yaml` is the source of truth for chart config across benches. Tune
worker replica counts, PG memory, max_connections, etc. there.

## Architecture

```
wm_sim up
  │
  ▼ provision
  ┌────────────────────────────────────────────────────────────────┐
  │ k8s_provisioner: minikube start + add-nodes + per-node sizing  │
  │ image_cache:     pre-load images so bringup is offline-safe    │
  │ helm_deploy:     helm upgrade --install windmill via smoke.yaml│
  │ toxiproxy_k8s:   per-node toxiproxy DaemonSet for latency inj  │
  │ cpu_sampler_k8s: per-node sampler DaemonSet (10Hz cpu.stat)    │
  └────────────────────────────────────────────────────────────────┘
  │
  ▼ benchmarks/main.ts (separate shell)
  ┌────────────────────────────────────────────────────────────────┐
  │ readiness:        wait for samplers stable, workers Ready,    │
  │                   PG responsive, queue empty,                 │
  │                   *deployment rollout complete* — mid-rollout │
  │                   pod churn starves the sampler under         │
  │                   cgroup_mutex contention                     │
  │ start pollers:    pod_timeline, oom, pg_latency, pg_conn,     │
  │                   node_load (all write JSONL into report dir) │
  │ truncate sampler  /var/log/wm-sim-cpu-sampler/sampler.tsv on   │
  │   host log:       each node so this run gets a clean file     │
  │ push jobs:        pushers → API → workers pull → PG queue     │
  │ stop pollers:     finalize JSONL                              │
  │ collect samples:  scp host log from each node (fallback:      │
  │                   kubectl logs)                               │
  │ collect pg.log:   kubectl logs PG pod → pgBadger HTML report  │
  │ render_report:    JSONL + samples → dashboard.svg + report.md │
  └────────────────────────────────────────────────────────────────┘
  │
  ▼ output: benchmarks/reports/<ISO timestamp>/
```

## Measurement subsystem

| Signal | Source | Cadence | File in report |
|---|---|---|---|
| Per-pod CPU + memory | DS sampler reading cgroup cpu.stat / memory.current | 10 Hz | `cpu_samples.tsv` |
| Workers Ready per node | `kubectl get pods` | 1 Hz | `pod_timeline.jsonl` |
| OOM events | `kubectl get events` + node-kernel dmesg | live | `oom_events_live.jsonl` |
| PG query latency | `psql -c "\timing on" -c "SELECT 1"` | 4 Hz | `pg_latency.jsonl` |
| PG connections by state | `SELECT state, count(*) FROM pg_stat_activity` | 1 Hz | `pg_connections.jsonl` |
| Node loadavg + procs_running | ssh `/proc/loadavg`, `/proc/stat` | 0.5 Hz | `node_load.jsonl` |
| Throughput (jobs/s sent + processed) | bench worker `postMessage` | sub-second | `throughput_samples.json` |
| Failed jobs | `/api/w/{ws}/jobs/completed/list` | post-bench | `failed_jobs.jsonl` |
| Pod inventory + node placement | `kubectl get pods -o json` | post-bench | `pods.json` |
| PG slow-query analysis | PG log → pgBadger | post-bench, background | `pgbadger.html` |

Key reliability fixes (don't undo them):

- **Sampler host file**: the sampler dual-writes to stdout AND a hostPath log
  file. `kubectl logs --since-time=X` loses early data when kubelet rotates
  log files under heavy benches (m04 was missing the first 90-120s of every
  busy run). Collector now scp's the host file as primary source.
- **Readiness rollout check**: `kubectl get deploy windmill-workers-default`
  is checked for `status.updatedReplicas == spec.replicas`. Mid-rolling-update
  pod churn floods the kernel's `cgroup_mutex`, starving every other process
  on the node — including the sampler. Pod-readiness alone isn't enough.
- **Oversaturation uses procs_running**, not load1. loadavg includes D-state
  procs (PG backends waiting on disk I/O, kernel mutex contention) which has
  nothing to do with CPU starvation but inflates "oversaturation" by 5-10x.
  `procs_running` from `/proc/stat` is the runnable count.

## Dashboard panels

Rendered into `dashboard.svg`. Layout is roughly:

| Section | Panels |
|---|---|
| **Run context** | Topology, workload, helm values, bench cmd, report dir |
| **Throughput** | Jobs sent + processed over time, with push-window shading and phase boundaries |
| **Queue depth** | Pending jobs in `v2_job_queue` |
| **Node memory** | Per-node memory time-series + per-phase average bar chart |
| **PG latency** | Pure SQL time (psql `\timing`) + kubectl-exec roundtrip (separate axes) |
| **PG connections** | active / idle / idle_in_xact over time |
| **Util group** | One panel per node, 2 per row. Solid blue CPU util (capped 100%) on top of translucent orange oversaturation. 100% reference line. PG-host node flagged `[PG]` |
| **Node CPU** | Multi-line per-node CPU, with PG-host node tinted via stroke + label |
| **Pod inventory** | Bar chart of pod count by node and pod-type donut |
| **Failed jobs** | Cumulative + categorized bars |
| **Restart events** | Pod restarts detected from `pod_timeline.jsonl` |
| **OOM events** | L0 scheduler preemptions, L1 kubelet evictions, L2 cgroup + node-kernel kills |

All time-series share an x-axis origin (`bench_start_ms` written into
`meta.json`) so the same x-coordinate on every panel means the same wall-clock
moment. Tick labels are relative seconds ("0s, 30s, …"), not wall-clock.

## Workloads

`benchmarks/workloads/*.json`. Each is a phased JSON spec the bench reads at
startup. Phases run sequentially, each with its own pusher count, mode mix,
duration range, and ram_mb distribution.

Examples:

| Workload | Shape | Use case |
|---|---|---|
| `io_4phase.json` | idle → 2.5s → 500ms → 150ms IO jobs | Steady-state vs saturation in one bench |
| `io_150ms_flood.json` | 60 pushers, 150ms IO, 120s | Worker-host CFS saturation regime |
| `io_300ms_flood.json` | 60 pushers, 300ms IO, 120s | Lower context-switch rate |
| `io_1s_flood.json`, `io_2s_flood.json` | Same shape, longer jobs | Near-theoretical worker scaling |
| `ops_day.json` | Mixed phases simulating a day | Realistic load profile |
| `cpu_heavy.json`, `mixed.json`, `burst.json` | Non-IO and bursty shapes | CPU-bound and spikiness |

## Agent workers (HTTP+JWT path)

`smoke.yaml` declares a second worker group `agent` with `mode: agent`. Agent
workers reach the windmill API over HTTP+JWT instead of holding a direct
sqlx pool to PG. Useful for measuring how much of the bench cap is PG
contention vs. the HTTP-mediated path.

The JWT lives in a K8s Secret `windmill-agent-token` (not in any committed
yaml). Create it once:

```sh
kubectl --context wm-sim-k8s-4node create secret generic windmill-agent-token \
  --from-literal=token='jwt_agent_…'
```

## Tests

```sh
cd benchmarks && deno test --allow-import --no-check \
  sim/util_metrics_test.ts sim/util_panel_snapshot_test.ts
```

- `sim/util_metrics_test.ts` — 8 tests for `computeOversatPct`, the formula
  the util panel uses. Specifically guards the procs_running > load1 fallback,
  the clamp-at-zero, invalid-ncpu inputs.
- `sim/util_panel_snapshot_test.ts` — 5 assertions on the rendered util-panel
  SVG: orange-behind-blue z-order, 100% reference line, relative-time ticks
  (no wall-clock leak), phase-boundary dashed verticals, shared-origin override.
- `sim/topology_test.ts`, `sim/toxiproxy_test.ts` — earlier coverage of the
  validator + proxy planner.

Pre-existing typecheck errors in `graph.ts` (untyped d3 callback args,
implicit `any`) trip `deno test` without `--no-check`. The functional tests
pass regardless.

## Operational notes

- Cluster name / minikube profile is `wm-sim-k8s-4node` by default. Pass
  `--minikube-profile <name>` to `main.ts` if you renamed it.
- Default kubectl context comes from minikube. Use
  `kubectl --context wm-sim-k8s-4node …` to be explicit.
- `wm_sim up` runs the port-forward as a child process. Killing wm_sim
  (Ctrl-C) kills the forward. The kubelet sometimes drops the forward under
  heavy bench load; if a bench fails partway with "connection refused",
  start a replacement forward manually with `kubectl port-forward` on the
  same local port and re-fire.
- `minikube stop` is safe — VM disks persist, etcd survives, helm releases
  + Secrets all come back when you `minikube start`. `wm_sim up` does
  `minikube delete` first, so it's destructive — only use it for clean reprovisions.

## Code layout

```
sim/
  sim.ts                  # wm_sim CLI entry (cliffy)
  k8s_provisioner.ts      # minikube up + node sizing
  helm_deploy.ts          # helm upgrade --install
  image_cache.ts          # pre-load images so bringup is offline-safe
  cpu_sampler_k8s.ts      # sampler DaemonSet + collector
  toxiproxy_k8s.ts        # per-node toxiproxy DaemonSet
  pg_logging.ts           # ALTER SYSTEM + SIGHUP for verbose PG logs
  pgbadger.ts             # post-bench PG log → HTML
  readiness.ts            # pre-bench cluster health check
  pod_timeline.ts         # pod readiness poller
  pod_inventory.ts        # post-bench pod inventory snapshot
  oom_poller.ts           # live OOM event capture
  oom_events.ts           # post-bench OOM event parsing
  pg_latency_poller.ts    # 4Hz PG \timing on SELECT 1
  pg_conn_poller.ts       # pg_stat_activity by state
  node_load_poller.ts     # /proc/loadavg + /proc/stat procs_running
  failed_jobs.ts          # post-bench failed-jobs fetch
  util_metrics.ts         # pure helpers (oversat formula)
  render_report.ts        # JSONL → SVG dashboard + report.md
  dashboard.ts            # SVG layout primitives (grid, sections)
  svg_to_pdf.ts           # dashboard.svg → dashboard.pdf (best-effort)
  topology.ts             # topology JSON loader + validator
  values/                 # helm overlay yamls
  topologies/             # topology JSON files
  nix/kvm2-driver.nix     # kvm2 driver overlay for nixpkgs
```

The bench runner itself is `benchmarks/main.ts` (one level up). It calls
into `sim/` for measurement + report rendering.
