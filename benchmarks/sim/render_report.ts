// Post-run report renderer for the k8s sim.
//
// Reads measurement artifacts from a reports dir (cpu_samples.tsv,
// pods.json, throughput_samples.json) and writes:
//   - <metric>.svg per panel (throughput, node_cpu, node_memory, workers_per_node)
//   - dashboard.svg composed via dashboard.ts (brand-styled grid)
//   - report.md with summary + embedded SVG refs
//
// All four panels are time-series (multi-series, one line per node). Reuses
// benchmarks/graph.ts:drawGraphMulti as the chart primitive (same code path
// the CI bench uses for trend graphs).

import { drawGraphMulti, drawBars, drawDonut } from "../graph.ts";
import { computeOversatPct } from "./util_metrics.ts";
import {
  sampleJobParams,
  type WorkloadConfig,
  type DistSpec,
} from "../workloads/distribution.ts";
import {
  renderDashboard,
  type DashboardMeta,
  type DashboardPanel,
  type DataPointMulti,
} from "./dashboard.ts";
import type { PodEntry } from "./pod_inventory.ts";
import type { OomEvent } from "./oom_events.ts";
import { renderSvgToPdf } from "./svg_to_pdf.ts";

type CpuSample = {
  ts_ms: number;       // wall clock (CLOCK_REALTIME) — used for x-axis placement.
  mono_ms: number;     // monotonic (CLOCK_MONOTONIC) — used for delta math.
                       // 0 for samples emitted by the pre-monotonic sampler;
                       // consumers fall back to ts_ms in that case.
  node: string;
  pod_slice: string;
  container_scope: string; // "-" when the row is the pod-level cgroup
  cpu_usec: number;
  mem_bytes: number | null;
};

type ThroughputSample = {
  ts: number;
  processed: number;
  sum: number;
  queue: number;
};

// kubectl logs --prefix=true prepends `[pod/<pod>/<container>] ` to each line.
const PREFIX_RE = /^\[pod\/[^\]]+\]\s+/;

function parseCpuSamples(text: string): CpuSample[] {
  // Two on-wire formats are accepted (auto-detected per row by column count):
  //   v1 (legacy, 6 cols): ts_ns node pod_slice container_scope usage_usec mem_bytes
  //   v2 (current, 7 cols): ts_ns mono_ns node pod_slice container_scope usage_usec mem_bytes
  // v2 added a monotonic timestamp column to dodge NTP wall-clock backwards
  // jumps that produced impossible >100% per-VM CPU spikes in v1 reports.
  const out: CpuSample[] = [];
  for (const raw of text.split("\n")) {
    if (!raw) continue;
    const line = raw.replace(PREFIX_RE, "");
    const parts = line.split(/\s+/);
    if (parts[0] === "ts_ns") continue; // header line(s) — one per sampler pod
    let ts_ns: number, mono_ns: number, node: string, pod_slice: string,
        container_scope: string, cpu_usec: number, memField: string;
    if (parts.length >= 7) {
      ts_ns = Number(parts[0]);
      mono_ns = Number(parts[1]);
      node = parts[2];
      pod_slice = parts[3];
      container_scope = parts[4];
      cpu_usec = Number(parts[5]);
      memField = parts[6];
    } else if (parts.length >= 6) {
      ts_ns = Number(parts[0]);
      mono_ns = 0; // unavailable in v1
      node = parts[1];
      pod_slice = parts[2];
      container_scope = parts[3];
      cpu_usec = Number(parts[4]);
      memField = parts[5];
    } else {
      continue;
    }
    if (!Number.isFinite(ts_ns)) continue;
    const mem_bytes = memField === "-" ? null : Number(memField);
    out.push({
      ts_ms: Math.floor(ts_ns / 1_000_000),
      mono_ms: Number.isFinite(mono_ns) ? Math.floor(mono_ns / 1_000_000) : 0,
      node,
      pod_slice,
      container_scope,
      cpu_usec,
      mem_bytes,
    });
  }
  return out;
}

const NODE_ROOT_TAG = "__node_root__";

// Node-root rows — one per node per tick from the VM's root cgroup
// (`/sys/fs/cgroup/cpu.stat` / `memory.current`). Covers everything on the
// VM: kubelet, containerd, system daemons, every pod. This is "VM CPU/mem".
function nodeRootOnly(samples: CpuSample[]): CpuSample[] {
  return samples.filter((s) => s.pod_slice === NODE_ROOT_TAG);
}

// `kubepods-besteffort-pod<uid>.slice` → uid (with underscores → hyphens).
function uidFromPodSlice(slice: string): string | null {
  const m = slice.match(/-pod([0-9a-f_]{30,})\.slice$/i);
  if (!m) return null;
  return m[1].replace(/_/g, "-");
}

// Per-node CPU rate as % of the VM (0-100). Computed from consecutive deltas
// of the node-root cgroup `usage_usec`, then divided by `cpusPerNode` so 100%
// = whole VM saturated regardless of core count. Pass cpusPerNode=1 to keep
// the raw "% of one core" units (values can exceed 100% on multi-core VMs).
function buildNodeCpuSeries(
  samples: CpuSample[],
  cpusByNode: Map<string, number>,
  fallbackCpus = 1,
): DataPointMulti[] {
  const grouped = new Map<string, CpuSample[]>();
  for (const s of samples) {
    const k = `${s.node}|${s.pod_slice}`;
    let arr = grouped.get(k);
    if (!arr) { arr = []; grouped.set(k, arr); }
    arr.push(s);
  }
  const buckets = new Map<string, Map<number, number>>();
  for (const [k, arr] of grouped) {
    // Sort by mono_ms when available (monotonic, NTP-immune); fall back to
    // wall ts_ms for v1 (pre-monotonic) sampler output. Mixing v1/v2 within
    // one series is not expected — sampler-pod rollouts get matched format.
    const useMono = arr.length > 0 && arr[0].mono_ms > 0;
    arr.sort((a, b) => useMono ? a.mono_ms - b.mono_ms : a.ts_ms - b.ts_ms);
    const node = k.split("|")[0];
    const cpus = Math.max(1, cpusByNode.get(node) ?? fallbackCpus);
    let m = buckets.get(node);
    if (!m) { m = new Map(); buckets.set(node, m); }
    for (let i = 1; i < arr.length; i++) {
      const prev = arr[i - 1];
      const cur = arr[i];
      // Delta math uses MONO (immune to NTP wall-clock corrections); x-axis
      // placement still uses wall ts_ms so this panel aligns with throughput,
      // OOM event markers, etc. on the shared timeline.
      const dWall_ms = useMono ? cur.mono_ms - prev.mono_ms : cur.ts_ms - prev.ts_ms;
      if (dWall_ms <= 0) continue;
      const dCpu_us = cur.cpu_usec - prev.cpu_usec;
      // % of one core: dCpu(us) / dWall(us) * 100 = dCpu / dWall_ms / 10.
      // % of VM: divide by this node's core count so a fully-saturated VM reads 100.
      const pct = dCpu_us / dWall_ms / 10 / cpus;
      const acc = m.get(cur.ts_ms) ?? 0;
      m.set(cur.ts_ms, acc + pct);
    }
  }
  return flattenBuckets(buckets);
}

// Per-node arithmetic mean CPU %, starting from the first sample where the
// value crossed `thresholdPct` (treated as "the moment work started on this
// node"). Falls back to the full series mean if the threshold was never hit.
// Used to bake "avg from first job" into the Node CPU legend.
function perNodeAvgFromFirstJob(
  data: DataPointMulti[],
  thresholdPct: number,
): Map<string, number> {
  const byNode = new Map<string, number[]>();
  for (const d of data) {
    let arr = byNode.get(d.kind);
    if (!arr) { arr = []; byNode.set(d.kind, arr); }
    arr.push(d.value);
  }
  const out = new Map<string, number>();
  for (const [node, vals] of byNode) {
    const start = vals.findIndex((v) => v >= thresholdPct);
    const slice = start >= 0 ? vals.slice(start) : vals;
    if (slice.length === 0) continue;
    out.set(node, slice.reduce((a, b) => a + b, 0) / slice.length);
  }
  return out;
}

// Trim the topology's verbose `wm-sim-k8s-4node-m02` to a chart-friendly
// `m02` (control plane stays as "n0" by convention). Keeps the legend
// readable when avg numbers are baked into the same string.
function shortNode(full: string): string {
  const m = full.match(/-m(\d+)$/);
  if (m) return `m${m[1]}`;
  return full.endsWith("-4node") ? "n0" : full;
}

// Per-node memory (MiB) — `memory.current` from the node-root row.
function buildNodeMemSeries(samples: CpuSample[]): DataPointMulti[] {
  const buckets = new Map<string, Map<number, number>>();
  for (const s of samples) {
    if (s.mem_bytes === null) continue;
    let m = buckets.get(s.node);
    if (!m) { m = new Map(); buckets.set(s.node, m); }
    const acc = m.get(s.ts_ms) ?? 0;
    m.set(s.ts_ms, acc + s.mem_bytes);
  }
  return flattenBuckets(buckets, (b) => b / (1024 * 1024));
}

// PG cgroup memory over time, from per-pod sampler rows. Filters samples to
// the postgres pod (matching by name → UID via pods.json). Y axis in GiB
// because PG memory grows into multi-GiB territory and MiB digits get noisy.
function buildPgMemSeries(samples: CpuSample[], pods: PodEntry[]): DataPointMulti[] {
  const pgUids = new Set<string>();
  for (const p of pods) {
    if (p.name.startsWith("windmill-postgresql")) pgUids.add(p.uid);
  }
  if (pgUids.size === 0) return [];
  // Single series labelled "pg" — cleanest legend.
  const buckets = new Map<string, Map<number, number>>();
  const m = new Map<number, number>();
  buckets.set("pg", m);
  for (const s of samples) {
    if (s.mem_bytes === null) continue;
    const uid = uidFromPodSlice(s.pod_slice);
    if (!uid || !pgUids.has(uid)) continue;
    const acc = m.get(s.ts_ms) ?? 0;
    m.set(s.ts_ms, acc + s.mem_bytes);
  }
  return flattenBuckets(buckets, (b) => b / (1024 ** 3));
}

// Classify a pod name into a coarse app-type bucket. With 80+ worker pods
// individually plotted, the legend becomes unreadable; rolling up by family
// keeps the chart to ~5 lines.
function podAppType(name: string): string {
  if (name.startsWith("windmill-workers-")) return "workers";
  if (name.startsWith("windmill-postgresql")) return "postgres";
  if (name.startsWith("windmill-app")) return "app";
  if (name.startsWith("toxiproxy")) return "toxiproxy";
  if (name.startsWith("wm-sim-cpu-sampler")) return "sampler";
  return "other";
}

// Per-pod cgroup memory over time. With 80+ worker pods, plotting EVERY pod
// as its own line makes the legend unreadable, so we keep the top-N by peak
// memory as named series and roll the rest into a single "others (sum)"
// aggregate. Worker pod names get shortened to their last hash chunk for the
// legend (`windmill-workers-default-6fd...-2fhx9` → `wk-2fhx9`).
function buildPodMemSeries(
  samples: CpuSample[],
  pods: PodEntry[],
  topN = 20,
): DataPointMulti[] {
  const uidToName = new Map<string, string>();
  for (const p of pods) uidToName.set(p.uid, p.name);

  // First pass: build per-pod timeline + remember peak
  const perPod = new Map<string, Map<number, number>>();
  const peakByPod = new Map<string, number>();
  for (const s of samples) {
    if (s.mem_bytes === null) continue;
    const uid = uidFromPodSlice(s.pod_slice);
    if (!uid) continue;
    const name = uidToName.get(uid);
    if (!name) continue;
    let m = perPod.get(name);
    if (!m) { m = new Map(); perPod.set(name, m); }
    const next = (m.get(s.ts_ms) ?? 0) + s.mem_bytes;
    m.set(s.ts_ms, next);
    if (next > (peakByPod.get(name) ?? 0)) peakByPod.set(name, next);
  }

  // Pick top-N pods by peak; everyone else aggregates into "others (sum)".
  const ranked = [...peakByPod.entries()].sort((a, b) => b[1] - a[1]);
  const topNames = new Set(ranked.slice(0, topN).map(([n]) => n));

  const out = new Map<string, Map<number, number>>();
  const others = new Map<number, number>();
  for (const [name, timeline] of perPod) {
    if (topNames.has(name)) {
      out.set(shortPodName(name), timeline);
    } else {
      for (const [ts, v] of timeline) {
        others.set(ts, (others.get(ts) ?? 0) + v);
      }
    }
  }
  if (others.size > 0) out.set(`others (sum of ${ranked.length - topN} pods)`, others);
  return flattenBuckets(out, (b) => b / (1024 ** 3));
}

// Trim verbose pod names down to a chart-legend friendly form.
function shortPodName(name: string): string {
  if (name.startsWith("windmill-workers-default-")) {
    return "wk-" + name.split("-").slice(-1)[0];
  }
  if (name.startsWith("wm-sim-cpu-sampler-")) {
    return "sampler-" + name.split("-").slice(-1)[0];
  }
  if (name.startsWith("windmill-postgresql")) return "postgres";
  if (name.startsWith("windmill-app")) return "app";
  if (name.startsWith("toxiproxy")) return "toxiproxy";
  return name;
}

// Bar bins: per (node, app-type) pod count, for the "Pod inventory by node"
// panel. Order: node ascending, then type by count descending within node.
function buildPodInventoryBins(pods: PodEntry[]): { label: string; count: number; color: string }[] {
  const buckets = new Map<string, Map<string, number>>();
  for (const p of pods) {
    if (!p.node) continue;
    let perType = buckets.get(p.node);
    if (!perType) { perType = new Map(); buckets.set(p.node, perType); }
    const type = podAppType(p.name);
    perType.set(type, (perType.get(type) ?? 0) + 1);
  }
  // Color per node so all bars belonging to the same node share a hue. Reuses
  // the same colorbrewer ramp d3 uses for line charts so the legend visually
  // matches Node CPU / Node memory if put side-by-side.
  const NODE_PALETTE = ["#e41a1c", "#377eb8", "#4daf4a", "#984ea3", "#ff7f00"];
  const nodes = [...buckets.keys()].sort();
  const colorByNode = new Map<string, string>();
  nodes.forEach((n, i) => colorByNode.set(n, NODE_PALETTE[i % NODE_PALETTE.length]));
  const out: { label: string; count: number; color: string }[] = [];
  for (const node of nodes) {
    const nodeShort = node.replace(/^wm-sim-k8s-4node-?/, "") || "n0";
    const perType = buckets.get(node)!;
    const types = [...perType.entries()].sort((a, b) => b[1] - a[1]);
    for (const [type, count] of types) {
      out.push({ label: `${nodeShort}: ${type}`, count, color: colorByNode.get(node)! });
    }
  }
  return out;
}

// Workers per node — Ready count from the kubectl-based pod_timeline.jsonl.
// This is the authoritative version: it sees CrashLoopBackOff pods as NOT
// ready (which the cgroup sampler can't, because the failed container's slice
// often persists in /sys/fs/cgroup for several seconds after the crash).
// Format of each JSONL row:
//   {"ts": <ms>, "pods": [{"name","node","phase","ready"}, ...]}
function buildReadyWorkersSeriesFromTimeline(text: string): DataPointMulti[] {
  const out: DataPointMulti[] = [];
  for (const line of text.split("\n")) {
    if (!line.trim()) continue;
    let row: { ts: number; pods: Array<{ name: string; node: string; ready: boolean }> };
    try {
      row = JSON.parse(line);
    } catch {
      continue;
    }
    const perNode = new Map<string, number>();
    for (const p of row.pods) {
      if (!p.name.startsWith("windmill-workers-") || !p.ready) continue;
      perNode.set(p.node, (perNode.get(p.node) ?? 0) + 1);
    }
    for (const [node, count] of perNode) {
      out.push({ value: count, date: new Date(row.ts), kind: node });
    }
  }
  out.sort((a, b) => a.date.getTime() - b.date.getTime());
  return out;
}

// Pod restart timeline — for each tick, count the cumulative number of
// container restarts across all worker pods, grouped by node. Detects
// EVERY pod restart regardless of reason (preemption, OOM, liveness probe
// failure, helm rollouts) — fills the gap between the L0/L1/L2 panels
// (which only catch specific kill modes) and the actual "how many workers
// died this bench" question.
//
// Uses pod_timeline.jsonl which now includes restartCount per pod. The
// number plotted is CUMULATIVE restart count per node since bench start
// — climbs as restarts accumulate, so the slope shows the kill rate.
// Count of restart events per pod-family during the bench window, derived
// purely from pod_timeline.jsonl's restartCount transitions. This is the
// ground-truth source for "how many things got killed/restarted" — it
// catches what containerd-Status-based collectors miss (e.g. toxiproxy's
// "Error" exit 137 that isn't labeled OOMKilled). Each (pod, container)
// whose restartCount went up between two ticks counts as one event.
function buildRestartEventBins(text: string): { label: string; count: number }[] {
  const counts = new Map<string, number>();   // family -> restart events
  const last = new Map<string, number>();     // pod_name -> last seen restartCount
  for (const line of text.split("\n")) {
    if (!line.trim()) continue;
    let row: { ts: number; pods: Array<{ name: string; restarts?: number }> };
    try { row = JSON.parse(line); } catch { continue; }
    for (const p of row.pods) {
      const rc = p.restarts ?? 0;
      const prev = last.get(p.name);
      if (prev !== undefined && rc > prev) {
        const family = podAppType(p.name);
        counts.set(family, (counts.get(family) ?? 0) + (rc - prev));
      }
      last.set(p.name, rc);
    }
  }
  return [...counts.entries()]
    .map(([label, count]) => ({ label, count }))
    .sort((a, b) => b.count - a.count);
}

function buildRestartTimelineSeries(text: string): DataPointMulti[] {
  // Track the first-seen restart count per pod, so the plotted value is
  // RESTARTS DURING THE BENCH WINDOW (not the pre-existing baseline).
  const baseline = new Map<string, number>();
  const out: DataPointMulti[] = [];
  for (const line of text.split("\n")) {
    if (!line.trim()) continue;
    let row: { ts: number; pods: Array<{ name: string; node: string; restarts?: number }> };
    try {
      row = JSON.parse(line);
    } catch {
      continue;
    }
    const perNode = new Map<string, number>();
    for (const p of row.pods) {
      if (!p.name.startsWith("windmill-workers-")) continue;
      const restarts = p.restarts ?? 0;
      const base = baseline.get(p.name);
      if (base === undefined) {
        baseline.set(p.name, restarts);
        continue;
      }
      const delta = Math.max(0, restarts - base);
      if (delta > 0) {
        perNode.set(p.node, (perNode.get(p.node) ?? 0) + delta);
      }
    }
    for (const [node, count] of perNode) {
      out.push({ value: count, date: new Date(row.ts), kind: node });
    }
  }
  out.sort((a, b) => a.date.getTime() - b.date.getTime());
  return out;
}

// Workers per node over time — count of distinct UIDs at each tick whose pod
// name starts with "windmill-workers-" (i.e. our worker Deployment).
// Fallback used when pod_timeline.jsonl is missing.
function buildWorkersSeries(samples: CpuSample[], pods: PodEntry[]): DataPointMulti[] {
  const uid2name = new Map<string, string>();
  for (const p of pods) uid2name.set(p.uid, p.name);
  const isWorker = (uid: string): boolean => {
    const n = uid2name.get(uid);
    return !!n && n.startsWith("windmill-workers-");
  };
  const buckets = new Map<string, Map<number, Set<string>>>();
  for (const s of samples) {
    const uid = uidFromPodSlice(s.pod_slice);
    if (!uid || !isWorker(uid)) continue;
    let m = buckets.get(s.node);
    if (!m) { m = new Map(); buckets.set(s.node, m); }
    let set = m.get(s.ts_ms);
    if (!set) { set = new Set(); m.set(s.ts_ms, set); }
    set.add(uid);
  }
  const out: DataPointMulti[] = [];
  for (const [node, m] of buckets) {
    for (const [ts, set] of m) {
      out.push({ value: set.size, date: new Date(ts), kind: node });
    }
  }
  out.sort((a, b) => a.date.getTime() - b.date.getTime());
  return out;
}

// Rolling-window throughput in jobs/s. For each sample at time T, look back
// `windowS` seconds and divide processed-delta by the window.
//
// Two normalizations: (1) skip windows shorter than MIN_DWALL_S — sub-100ms
// dt over a noisy processed counter produces huge fake spikes (50 j/s real
// throughput appearing as 30,000+ j/s peaks because dt was 5ms). (2) floor
// negative deltas at 0 — `processed = sum - queue_length` is non-monotonic
// due to a sampling race between the two counters, so adjacent samples can
// show Δprocessed < 0 even though no jobs were "un-processed". Both make
// the chart show the real shape without auto-scaling to glitch spikes.
const MIN_DWALL_S = 0.5;
function buildThroughputSeries(
  samples: ThroughputSample[],
  windowS = 1,
): DataPointMulti[] {
  const out: DataPointMulti[] = [];
  for (let i = 0; i < samples.length; i++) {
    const cur = samples[i];
    const target_ts = cur.ts - windowS * 1000;
    let j = i;
    while (j > 0 && samples[j - 1].ts >= target_ts) j--;
    const ref = samples[j];
    const dWall_s = (cur.ts - ref.ts) / 1000;
    if (dWall_s < MIN_DWALL_S) continue;
    const dProcessed = cur.processed - ref.processed;
    const rate = Math.max(0, dProcessed / dWall_s);
    out.push({ value: rate, date: new Date(cur.ts), kind: "processed" });
  }
  return out;
}

// Queue depth (pending jobs) over time — straight from the bench sampler.
function buildQueueSeries(samples: ThroughputSample[]): DataPointMulti[] {
  return samples.map((s) => ({ value: s.queue, date: new Date(s.ts), kind: "queue" }));
}

// Rolling-average smoothing for multi-series time data. For each series
// (grouped by .kind), each output point at time T = mean of input values
// whose timestamps fall in (T - windowS, T]. Preserves x-axis placement so
// the series still aligns with verticalLines / horizontalLines.
function smoothMultiSeries(data: DataPointMulti[], windowS: number): DataPointMulti[] {
  if (windowS <= 0 || data.length === 0) return data;
  const windowMs = windowS * 1000;
  const byKind = new Map<string, DataPointMulti[]>();
  for (const d of data) {
    let arr = byKind.get(d.kind);
    if (!arr) { arr = []; byKind.set(d.kind, arr); }
    arr.push(d);
  }
  const out: DataPointMulti[] = [];
  for (const [kind, arr] of byKind) {
    arr.sort((a, b) => a.date.getTime() - b.date.getTime());
    let head = 0;
    for (let i = 0; i < arr.length; i++) {
      const cur = arr[i].date.getTime();
      const cutoff = cur - windowMs;
      while (head < i && arr[head].date.getTime() < cutoff) head++;
      let sum = 0;
      for (let j = head; j <= i; j++) sum += arr[j].value;
      out.push({ value: sum / (i - head + 1), date: arr[i].date, kind });
    }
  }
  return out;
}

function flattenBuckets(
  buckets: Map<string, Map<number, number>>,
  transform: (v: number) => number = (v) => v,
): DataPointMulti[] {
  const out: DataPointMulti[] = [];
  for (const [node, m] of buckets) {
    for (const [ts, v] of m) {
      out.push({ value: transform(v), date: new Date(ts), kind: node });
    }
  }
  out.sort((a, b) => a.date.getTime() - b.date.getTime());
  return out;
}

export type RenderInput = {
  outDir: string;
  topology: string;
  walltimeS: number;
  finalThroughput?: number;
  // Cores per node — used to scale CPU to "% of VM" (0-100) instead of raw
  // "% of one core" (0-N×100). Pass a map for heterogeneous clusters (e.g. a
  // 2-vCPU control plane + 4-vCPU workers); otherwise pass a single number
  // applied to every node, or leave undefined for "% of one core".
  cpusPerNode?: number | Record<string, number>;
};

export async function renderReport(input: RenderInput): Promise<void> {
  const { outDir, topology, walltimeS } = input;
  const cpusByNode = new Map<string, number>();
  let fallbackCpus = 1;
  if (typeof input.cpusPerNode === "number") {
    fallbackCpus = input.cpusPerNode;
  } else if (input.cpusPerNode) {
    for (const [n, c] of Object.entries(input.cpusPerNode)) cpusByNode.set(n, c);
  }

  const cpuRaw = await safeReadText(`${outDir}/cpu_samples.tsv`);
  const podsRaw = await safeReadText(`${outDir}/pods.json`) ?? "[]";
  const thrRaw = await safeReadText(`${outDir}/throughput_samples.json`); // optional

  // Shared relative-time origin: 0s on every panel = bench start (from
  // meta.json's bench_start_ms). Without this, each chart anchors at its
  // own earliest sample and panels drift by tens of seconds.
  let benchStartMs: number | undefined;
  try {
    const metaEarly = await safeReadText(`${outDir}/meta.json`);
    if (metaEarly) {
      const parsed = JSON.parse(metaEarly) as { bench_start_ms?: unknown };
      if (typeof parsed.bench_start_ms === "number" && Number.isFinite(parsed.bench_start_ms)) {
        benchStartMs = parsed.bench_start_ms;
      }
    }
  } catch { /* fall back to per-chart origin */ }

  if (!cpuRaw) {
    console.warn(`[report] no cpu_samples.tsv at ${outDir} — skipping report`);
    return;
  }

  // Compute phase boundary timestamps for phased workloads.
  //
  // PULL semantics — not push: a boundary marks when the FIRST job of the
  // next phase actually starts processing (not when its pushers started
  // pushing). When push N+1 begins, its first job sits behind everything
  // from phase ≤N in the FIFO queue. Workers only pull phase N+1's first
  // job after `sum_at_phase_N_push_end` jobs have been processed. So:
  //   pull_boundary_{N+1} = earliest ts where `processed >= sum_at_t_push_end_N`
  // For N phases we emit N-1 boundary lines.
  const wlRaw = await safeReadText(`${outDir}/workload.json`);
  const phaseBoundaries: Date[] = [];
  if (wlRaw && thrRaw) {
    try {
      const wl = JSON.parse(wlRaw) as { phases?: Array<{ duration_s: number }> };
      const thrSamples = JSON.parse(thrRaw) as ThroughputSample[];
      if (wl.phases && wl.phases.length > 1 && thrSamples.length > 0) {
        const tStart = thrSamples[0].ts;
        let acc = 0;
        for (let i = 0; i < wl.phases.length - 1; i++) {
          acc += wl.phases[i].duration_s;
          const pushBoundaryMs = tStart + acc * 1000;
          // 1) sum (total pushed) at the push boundary
          const atPush = thrSamples.find((s) => s.ts >= pushBoundaryMs);
          if (!atPush) continue;
          const targetProcessed = atPush.sum;
          // 2) ts when processed catches up to targetProcessed — that's when
          //    the first job of the next phase actually starts running
          const pullSample = thrSamples.find((s) => s.processed >= targetProcessed);
          if (pullSample) phaseBoundaries.push(new Date(pullSample.ts));
        }
      }
    } catch { /* ignore */ }
  }
  // Two variants — Throughput gets the labeled boundaries (P1>P2 etc.),
  // every other chart gets the lines only. Eliminates the perception of
  // duplicate labels when 9 charts each rendered the same 3 labels.
  const lineOpts = phaseBoundaries.length > 0 ? phaseBoundaries : undefined;
  const lineOptsNoLabels = phaseBoundaries.length > 0
    ? { dates: phaseBoundaries, hideLabels: true }
    : undefined;

  // Push window — translucent shaded zone covering the time when the bench
  // is actively pushing jobs. Starts at the first phase with pushers > 0
  // (skipping leading idle phases) and ends at the sum of all phase
  // durations. Drawn on Throughput / Queue depth / Node CPU so it's
  // visually obvious "this is when jobs were being pushed" vs "drain only".
  let pushZones: { from: Date; to: Date; label?: string }[] | undefined;
  // Use thrRaw directly here — `thr` isn't initialized yet at this point in
  // the function (this code was added above its declaration). Earlier this
  // was a hard TDZ error and the whole report render failed.
  if (wlRaw && thrRaw) {
    try {
      const wl = JSON.parse(wlRaw) as { phases?: Array<{ duration_s: number; pushers: number }> };
      const earlyThr = JSON.parse(thrRaw) as ThroughputSample[];
      if (wl.phases && wl.phases.length > 0 && earlyThr.length > 0) {
        const tStart = earlyThr[0].ts;
        let acc = 0;
        let pushStartS: number | null = null;
        for (const p of wl.phases) {
          if (pushStartS === null && (p.pushers ?? 0) > 0) {
            pushStartS = acc;
          }
          acc += p.duration_s;
        }
        const pushEndS = acc;
        if (pushStartS !== null && pushEndS > pushStartS) {
          pushZones = [{
            from: new Date(tStart + pushStartS * 1000),
            to: new Date(tStart + pushEndS * 1000),
            label: "push window",
          }];
        }
      }
    } catch { /* leave zones undefined */ }
  }
  const cpu = parseCpuSamples(cpuRaw);
  const nodeCpu = nodeRootOnly(cpu);
  const pods: PodEntry[] = JSON.parse(podsRaw);
  const thr: ThroughputSample[] = thrRaw ? JSON.parse(thrRaw) : [];

  const scaled = cpusByNode.size > 0 || fallbackCpus > 1;
  const cpuYLabel = scaled ? "[% of VM]" : "[% of 1 core]";

  const panels: DashboardPanel[] = [];
  if (thr.length > 0) {
    // 7.5-second rolling window. 5s was still bumpy on P3 (etl_storm)
    // because the bench's queue-count polls become irregular when the API
    // is under load, so per-sample Δt varies and 5 s isn't long enough to
    // average it out. 7.5 s smooths the curve without losing phase shape
    // (each phase is 8-15 s, so we still see them as distinct shapes).
    panels.push({ title: "Throughput", yLabel: "[jobs/s]", data: buildThroughputSeries(thr, 7.5), verticalLines: lineOpts, shadedZones: pushZones });
    panels.push({ title: "Queue depth", yLabel: "[pending jobs]", data: buildQueueSeries(thr), verticalLines: lineOptsNoLabels, shadedZones: pushZones });
  }
  // Node CPU panel — clamped at 150% so sampler noise can't compress the rest
  // of the chart, and per-node avg-from-first-job baked into the legend so you
  // can read "is this cluster actually being used" at a glance.
  // 2-second rolling average applied so the per-sample 100ms-tick noise is
  // smoothed out. Title carries the smoothing window so it's not invisible.
  const CPU_SMOOTH_S = 2;
  const cpuSeries = smoothMultiSeries(buildNodeCpuSeries(nodeCpu, cpusByNode, fallbackCpus), CPU_SMOOTH_S);
  const avgsByNode = perNodeAvgFromFirstJob(cpuSeries, 5);
  // Flag the node that hosts PG so it stands out in the legend — when the user
  // asks "how much CPU does PG pull at this scale" they need to read it off
  // a specific line, not guess which of m02/m03/m04 has the database.
  const pgNodes = new Set<string>();
  for (const p of pods) {
    if (p.name.startsWith("windmill-postgresql")) pgNodes.add(p.node);
  }
  const cpuSeriesLabeled = cpuSeries.map((d) => {
    const avg = avgsByNode.get(d.kind);
    const pgFlag = pgNodes.has(d.kind) ? " [PG]" : "";
    const tag = avg !== undefined ? `${shortNode(d.kind)}${pgFlag} avg ${avg.toFixed(0)}%` : `${shortNode(d.kind)}${pgFlag}`;
    return { ...d, kind: tag };
  });
  // Horizontal reference line at 100% (= one whole VM saturated). Without
  // it, eyes can't tell whether a 90% reading is "lots of headroom" or
  // "about to peg" because the y-axis goes to 150 (the clamp).
  const cpuHLines = scaled ? [{ y: 100, label: "100% (full VM)" }] : undefined;
  // yMax was 150 (room for sampler noise spikes); dropped to 110 so the
  // idle 0-3% range isn't compressed to a 2px band at the bottom of the
  // chart. The 100% horizontal reference line still shows the ceiling.
  const nodeCpuSvg = drawGraphMulti(
    cpuSeriesLabeled,
    `Node CPU (${CPU_SMOOTH_S}s smoothed)`,
    cpuYLabel,
    110,
    lineOpts,
    cpuHLines,
    pushZones,
    "[PG]",
    undefined,
    benchStartMs,
  );
  panels.push({ title: "Node CPU", yLabel: cpuYLabel, data: [], svg: nodeCpuSvg, verticalLines: lineOpts });

  // Util group: one panel per node, layered:
  //   1) orange oversaturation area BEHIND  — max(0, load1/ncpu - 1) × 100,
  //      capped at 100% so the panel y-axis stays comparable across nodes
  //   2) CPU util area on top — same per-node util series as Node CPU, capped
  //      at 100% (% of VM)
  // The combo answers "is this node maxed AND is there extra queued work".
  // cols: 2 → 2 panels per row, wraps when nodes >2.
  const nodeLoadRaw = await safeReadText(`${outDir}/node_load.jsonl`);
  if (nodeLoadRaw) {
    // Oversaturation is NOT capped — let the y-axis autoscale up to whatever
    // peak the bench produced. CPU util IS capped at 100% (it's already
    // "% of VM" so values >100 are sampler noise). The panel y-axis then
    // covers [0, max(100, peak_oversat)] which is what we want: CPU util
    // visible against the 100% ceiling, oversaturation shown at full magnitude.
    // Oversaturation = (procs_running - ncpu) / ncpu × 100, lower-clamped at 0.
    // procs_running is the runnable count from /proc/stat — actual CPU run-
    // queue pressure. Earlier we used load1 which includes D-state procs and
    // wildly overestimated oversaturation when PG backends were in disk wait.
    // Fallback to load1 only if procs_running is missing (older poller schema).
    const loadByNode = new Map<string, { ts: number; oversatPct: number }[]>();
    for (const line of nodeLoadRaw.split("\n")) {
      if (!line.trim()) continue;
      try {
        const r = JSON.parse(line) as {
          ts: number;
          node: string;
          load1: number;
          ncpu: number;
          procs_running?: number | null;
        };
        if (!Number.isFinite(r.ts)) continue;
        const oversat = computeOversatPct(r.ncpu, {
          procs_running: r.procs_running,
          load1: r.load1,
        });
        const arr = loadByNode.get(r.node) ?? [];
        arr.push({ ts: r.ts, oversatPct: oversat });
        loadByNode.set(r.node, arr);
      } catch { /* ignore malformed */ }
    }
    const cpuUtilByNode = new Map<string, { ts: number; utilPct: number }[]>();
    for (const d of cpuSeries) {
      const arr = cpuUtilByNode.get(d.kind) ?? [];
      arr.push({ ts: d.date.getTime(), utilPct: Math.min(100, d.value) });
      cpuUtilByNode.set(d.kind, arr);
    }
    const utilGroup = {
      index: 150,
      label: "Util — per node: CPU util (capped 100%, blue, 2s smoothed) + oversaturation behind (orange, (procs_running − ncpu) / ncpu × 100, 5s smoothed)",
      cols: 2,
    };
    const sortedNodes = [...cpuUtilByNode.keys()].sort();
    for (const nodeName of sortedNodes) {
      const utilPts = cpuUtilByNode.get(nodeName) ?? [];
      const loadPts = loadByNode.get(nodeName) ?? [];
      // 5s rolling smooth on oversaturation — procs_running is sampled at 2Hz
      // and bounces noisily as workers fire. CPU util is already 2s-smoothed
      // (CPU_SMOOTH_S above), leave as-is.
      const OVERSAT_SMOOTH_S = 5;
      const oversatRaw: DataPointMulti[] = loadPts.map((p) => ({
        value: p.oversatPct, date: new Date(p.ts), kind: "oversaturation",
      }));
      const oversatSmoothed = smoothMultiSeries(oversatRaw, OVERSAT_SMOOTH_S);
      const data: DataPointMulti[] = [];
      for (const p of utilPts) data.push({ value: p.utilPct, date: new Date(p.ts), kind: "cpu util" });
      for (const d of oversatSmoothed) data.push(d);
      if (data.length === 0) continue;
      panels.push({
        title: `${shortNode(nodeName)}${pgNodes.has(nodeName) ? " [PG]" : ""} — CPU + oversat`,
        yLabel: "[%]",
        data,
        // No yMax → panel autoscales to max(oversat).
        // No shadedZones → push window removed per user request.
        verticalLines: lineOptsNoLabels,
        horizontalLines: [{ y: 100, label: "100% (full VM)" }],
        phaseGroup: utilGroup,
        areaFills: [
          // Oversaturation drawn first (backmost). Orange fills the full
          // height to the curve, so above the CPU-util area it shows as the
          // "extra queued work" band.
          { kind: "oversaturation", color: "#ff8c00", opacity: 0.55 },
          // CPU util drawn on top, SOLID — gives a crisp "CPU is busy here"
          // floor. Oversaturation above the util ceiling shows on top of it.
          { kind: "cpu util", color: "#377eb8", opacity: 1.0 },
        ],
        lineColorOverrides: {
          "oversaturation": "#ff8c00",
          "cpu util": "#377eb8",
        },
      });
    }
  }

  // Node memory gets its own boxed section: time-series + per-phase
  // average. User explicitly wanted the per-phase avg view so memory
  // pressure can be compared phase-to-phase (e.g. is etl_storm consistently
  // worse than morning_rush) at a glance.
  // Section uses cols=2 so the time-series (colSpan:2) hogs the first
  // row, and the per-phase bars below pair up 2 per row.
  // Section order (by index ascending): 100 memory, 200 pg latency, 300+
  // workload phases, 400 kill events.
  const nodeMemGroup = { index: 100, label: "Node memory — over time + per-phase averages", cols: 2 };
  const nodeMemSeries = buildNodeMemSeries(nodeCpu);
  panels.push({ title: "Node memory (over time)", yLabel: "[MiB]", data: nodeMemSeries, verticalLines: lineOptsNoLabels, phaseGroup: nodeMemGroup, colSpan: 2 });

  // Per-phase per-node memory averages. Read phase windows from workload.json
  // (push boundaries are clean enough for "what did each phase typically
  // pull?"; pull-based boundaries from phaseBoundaries[] are also OK but
  // they shift behind push by the queue lag).
  const wlForMemAvg = wlRaw ? (() => {
    try { return JSON.parse(wlRaw) as { phases?: Array<{ name?: string; duration_s: number }> }; }
    catch { return null; }
  })() : null;
  const phaseDefs = wlForMemAvg?.phases ?? [];
  if (phaseDefs.length > 0 && thr.length > 0) {
    const tStart = thr[0].ts;
    let acc = 0;
    const phaseWindows = phaseDefs.map((p, i) => {
      const start = tStart + acc * 1000;
      acc += p.duration_s;
      const end = tStart + acc * 1000;
      return { name: p.name ?? `phase${i + 1}`, index: i, start, end };
    });
    // Per-(node, phase): mean memory_mib. Then for each phase emit a bar
    // chart showing node→mean (skipping cp because we never care about
    // its memory and it makes the bars unreadable).
    const memByNodeTs = new Map<string, Map<number, number>>();
    for (const s of nodeCpu) {
      if (s.mem_bytes === null) continue;
      let m = memByNodeTs.get(s.node);
      if (!m) { m = new Map(); memByNodeTs.set(s.node, m); }
      m.set(s.ts_ms, s.mem_bytes);
    }
    const workerNodes = [...memByNodeTs.keys()]
      .filter((n) => !n.endsWith("4node") || n.endsWith("m02") || n.endsWith("m03") || n.endsWith("m04"))
      .sort();
    for (const w of phaseWindows) {
      const bins: { label: string; count: number }[] = [];
      for (const node of workerNodes) {
        const samples = memByNodeTs.get(node)!;
        let sum = 0;
        let n = 0;
        for (const [ts, v] of samples) {
          if (ts >= w.start && ts <= w.end) {
            sum += v;
            n++;
          }
        }
        if (n === 0) continue;
        const meanMiB = sum / n / (1024 * 1024);
        const nodeShort = node.replace(/^wm-sim-k8s-4node-?/, "") || "n0";
        bins.push({ label: nodeShort, count: Math.round(meanMiB) });
      }
      const svg = drawBars(
        bins.length > 0 ? bins : [{ label: "(no data)", count: 0 }],
        `Phase ${w.index + 1} (${w.name})`,
        "avg mem [MiB] per node",
      );
      panels.push({
        title: `Phase ${w.index + 1} (${w.name})`,
        yLabel: "[MiB]",
        data: [],
        svg,
        phaseGroup: nodeMemGroup,
      });
    }
  }
  // PG memory over time (cgroup memory.current). Sampled by the per-pod sampler.
  // The single most-watched signal for "is PG about to die" in our benches.
  const pgMemSeries = buildPgMemSeries(cpu, pods);
  if (pgMemSeries.length > 0) {
    panels.push({ title: "PG memory", yLabel: "[GiB]", data: pgMemSeries, verticalLines: lineOptsNoLabels });
  }

  // PG response latency over time — from pg_latency.jsonl, written by the
  // 1Hz poller. Flat near zero when PG is healthy; spikes when the
  // connection pool / backend forks / autovacuum stalls under load.
  const pgLatRaw = await safeReadText(`${outDir}/pg_latency.jsonl`);
  if (pgLatRaw) {
    // Split into TWO panels in their own boxed group because the y-axis
    // scales differ by ~3 orders of magnitude:
    //   - pg query time (psql `\timing on`) is sub-ms when healthy
    //   - kubectl-exec roundtrip is ~200ms baseline (control-plane overhead)
    // On a shared axis the pg-query line is a flat zero. Splitting lets
    // each have its own auto-scale.
    const pgQueryPts: DataPointMulti[] = [];
    const kubectlPts: DataPointMulti[] = [];
    for (const line of pgLatRaw.split("\n")) {
      if (!line.trim()) continue;
      try {
        const r = JSON.parse(line) as { ts: number; latency_ms: number; pg_query_ms?: number };
        if (Number.isFinite(r.ts) && Number.isFinite(r.latency_ms)) {
          kubectlPts.push({ value: r.latency_ms, date: new Date(r.ts), kind: "kubectl roundtrip" });
        }
        if (Number.isFinite(r.ts) && Number.isFinite(r.pg_query_ms)) {
          pgQueryPts.push({ value: r.pg_query_ms!, date: new Date(r.ts), kind: "pg query" });
        }
      } catch { /* ignore malformed line */ }
    }
    if (pgQueryPts.length > 0 || kubectlPts.length > 0) {
      const PG_LAT_SMOOTH_S = 1;
      const pgQuerySmoothed = smoothMultiSeries(pgQueryPts, PG_LAT_SMOOTH_S);
      const kubectlSmoothed = smoothMultiSeries(kubectlPts, PG_LAT_SMOOTH_S);
      const pgLatGroup = { index: 200, label: "PG response latency — pure PG query vs kubectl-exec roundtrip" };
      if (pgQuerySmoothed.length > 0) {
        panels.push({
          title: `PG query time — psql \\timing (4Hz, ${PG_LAT_SMOOTH_S}s smoothed)`,
          yLabel: "[ms]",
          data: pgQuerySmoothed,
          verticalLines: lineOptsNoLabels,
          shadedZones: pushZones,
          phaseGroup: pgLatGroup,
        });
      }
      if (kubectlSmoothed.length > 0) {
        panels.push({
          title: `kubectl exec roundtrip (4Hz, ${PG_LAT_SMOOTH_S}s smoothed)`,
          yLabel: "[ms]",
          data: kubectlSmoothed,
          verticalLines: lineOptsNoLabels,
          shadedZones: pushZones,
          phaseGroup: pgLatGroup,
        });
      }
    }
  }

  // PG connection counts over time — pg_stat_activity broken down by state.
  // Tells you when sqlx pools saturate (active climbs to N_workers * pool_size),
  // and exposes leaks (idle_in_transaction climbing without bound).
  const pgConnRaw = await safeReadText(`${outDir}/pg_connections.jsonl`);
  if (pgConnRaw) {
    const pts: DataPointMulti[] = [];
    for (const line of pgConnRaw.split("\n")) {
      if (!line.trim()) continue;
      try {
        const r = JSON.parse(line) as { ts: number; active?: number; idle?: number; idle_in_xact?: number; total?: number };
        if (!Number.isFinite(r.ts)) continue;
        const d = new Date(r.ts);
        if (Number.isFinite(r.active)) pts.push({ value: r.active!, date: d, kind: "active" });
        if (Number.isFinite(r.idle)) pts.push({ value: r.idle!, date: d, kind: "idle" });
        if (Number.isFinite(r.idle_in_xact)) pts.push({ value: r.idle_in_xact!, date: d, kind: "idle_in_xact" });
        if (Number.isFinite(r.total)) pts.push({ value: r.total!, date: d, kind: "total" });
      } catch { /* ignore malformed line */ }
    }
    if (pts.length > 0) {
      const pgConnGroup = { index: 250, label: "PG connections — pg_stat_activity by state (1Hz)" };
      panels.push({
        title: "PG connections over time",
        yLabel: "[#]",
        data: pts,
        verticalLines: lineOptsNoLabels,
        shadedZones: pushZones,
        phaseGroup: pgConnGroup,
      });
    }
  }

  // Pod memory PEAK as a bar chart — one bar per pod, 90° rotated labels
  // so all ~100 pods fit horizontally. A time-series with 80+ lines would
  // be unreadable; peak-per-pod tells you "which pods got biggest" at a
  // glance, which is what matters for OOM analysis.
  {
    const uidToName = new Map<string, string>();
    for (const p of pods) uidToName.set(p.uid, p.name);
    const peakByPod = new Map<string, number>();
    for (const s of cpu) {
      if (s.mem_bytes === null) continue;
      const uid = uidFromPodSlice(s.pod_slice);
      if (!uid) continue;
      const name = uidToName.get(uid);
      if (!name) continue;
      const cur = (peakByPod.get(name) ?? 0);
      const next = s.mem_bytes;
      if (next > cur) peakByPod.set(name, next);
    }
    const bins = [...peakByPod.entries()]
      .map(([name, bytes]) => ({ label: shortPodName(name), count: Math.round(bytes / (1024 * 1024)) }))
      .sort((a, b) => b.count - a.count);
    if (bins.length > 0) {
      const svg = drawBars(
        bins,
        "Pod memory peak (per pod)",
        `[MiB] — ${bins.length} pods sorted by peak`,
        undefined,
        { rotateDeg: -90, fontSize: "7px" },
      );
      panels.push({ title: "Pod memory peak (per pod)", yLabel: "[MiB]", data: [], svg });
    }
  }

  // Pod inventory by node — bar chart, one bar per (node, app-type) pair.
  // Quick read on how the cluster's pod families are distributed:
  // "m02 has PG + 1 worker + sampler", "m03 has toxi + 35 workers + sampler",
  // etc. Captured from end-of-bench pods.json so it reflects the final
  // (post-preemption) layout.
  {
    const bins = buildPodInventoryBins(pods);
    const svg = drawBars(
      bins.length > 0 ? bins : [{ label: "(no pods)", count: 0 }],
      "Pod inventory by node",
      `${pods.length} pods across ${new Set(pods.map((p) => p.node).filter(Boolean)).size} node(s)`,
    );
    panels.push({ title: "Pod inventory by node", yLabel: "[count]", data: [], svg });
  }

  // Failed jobs over time — cumulative count from the per-bench failed_jobs.jsonl
  // file populated at end-of-bench. Same phase boundary lines as the other
  // time-series so a spike can be attributed to the right phase. Always
  // rendered (even when the file is missing or empty) — a flat line at 0 is a
  // positive signal that no jobs failed, which we want on every dashboard.
  const failedRaw = await safeReadText(`${outDir}/failed_jobs.jsonl`);
  const failures: Array<{ ts_ms: number; category?: string; exit_code?: number }> = [];
  if (failedRaw) {
    for (const line of failedRaw.split("\n")) {
      if (!line.trim()) continue;
      try {
        const r = JSON.parse(line);
        if (Number.isFinite(r.ts_ms)) failures.push({ ts_ms: r.ts_ms, category: r.category, exit_code: r.exit_code });
      } catch { /* ignore */ }
    }
  }

  // Failed jobs by failure category — the missing-context panel: catches the
  // "1000+ jobs failed, what killed them" question. Exit 137 / SIGKILL is
  // the smoking gun for "deno subprocess SIGKILLed by something OTHER than
  // cgroup OOM" (most often: worker's heartbeat-reaper killing its own deno
  // child because the worker→app ping timed out).
  {
    const byCat = new Map<string, number>();
    for (const f of failures) {
      const cat = f.category ?? "unknown";
      byCat.set(cat, (byCat.get(cat) ?? 0) + 1);
    }
    const bins = [...byCat.entries()]
      .map(([label, count]) => ({ label, count }))
      .sort((a, b) => b.count - a.count);
    const svg = drawBars(
      bins.length > 0 ? bins : [{ label: "(no failures)", count: 0 }],
      "Failed jobs by category",
      `${failures.length} total failure(s)`,
    );
    panels.push({ title: "Failed jobs by category", yLabel: "[count]", data: [], svg });
  }
  if (failures.length > 0) {
    failures.sort((a, b) => a.ts_ms - b.ts_ms);
    const series: DataPointMulti[] = failures.map((f, i) => ({
      value: i + 1,
      date: new Date(f.ts_ms),
      kind: "failed",
    }));
    panels.push({ title: "Failed jobs (cumulative)", yLabel: "[count]", data: series, verticalLines: lineOptsNoLabels });
  } else {
    const t0 = thr.length > 0 ? thr[0].ts : Date.now();
    const t1 = thr.length > 0 ? thr[thr.length - 1].ts : t0 + 1000;
    panels.push({
      title: "Failed jobs (cumulative)",
      yLabel: "[count]",
      data: [
        { value: 0, date: new Date(t0), kind: "failed" },
        { value: 0, date: new Date(t1), kind: "failed" },
      ],
      verticalLines: lineOptsNoLabels,
    });
  }

  // Workers per node — prefer kubectl-based pod_timeline.jsonl (truthy Ready
  // signal) over the cgroup-based fallback (counts cgroup slices regardless of
  // pod health, which makes a crashlooping fleet look fully staffed).
  const timelineRaw = await safeReadText(`${outDir}/pod_timeline.jsonl`);
  const workersSeries = timelineRaw
    ? buildReadyWorkersSeriesFromTimeline(timelineRaw)
    : buildWorkersSeries(cpu, pods);
  panels.push({ title: "Workers per node (Ready)", yLabel: "[count]", data: workersSeries, verticalLines: lineOptsNoLabels });

  // Pod restart timeline — cumulative count of container restarts per node,
  // captured by the 1Hz pod_timeline poller. Catches kills the L0/L1/L2
  // panels miss: helm rollouts, liveness probe failures, generic exit-137
  // SIGKILLs, anything that increments containerStatus.restartCount.
  if (timelineRaw) {
    const restartSeries = buildRestartTimelineSeries(timelineRaw);
    panels.push({
      title: "Worker pod restarts (cumulative)",
      yLabel: "[count]",
      data: restartSeries,
      verticalLines: lineOpts,
    });
  }

  // Kill events captured during the bench window. Three discrete panels —
  // always rendered, even when zero, so absence is a positive signal:
  //   - L1: kubelet evictions (graceful, under node memory pressure)
  //   - L2a: cgroup OOM kills (per-pod limit hit — limits.memory)
  //   - L2b: node-kernel OOM kills (whole VM OOM, kernel picks a process)
  // L1 events carry memory-at-kill in the kubelet message; L2 events don't
  // (kernel dmesg doesn't log RSS), so we only render bytes on the L1 panel.
  // Empty-state panels show a "no kills" bar so the slot is visible in the grid.
  const oomRaw = await safeReadText(`${outDir}/oom_events.json`);
  let events: OomEvent[] = [];
  if (oomRaw) {
    try {
      events = JSON.parse(oomRaw);
    } catch (e) {
      console.warn(`[report] couldn't parse oom_events.json: ${(e as Error).message}`);
    }
  }
  const l0Events  = events.filter(e => e.source === "scheduler_preemption");
  const l1Events  = events.filter(e => e.source === "kubelet_eviction");
  const l2cEvents = events.filter(e => e.source === "cgroup");
  const l2kEvents = events.filter(e => e.source === "node_kernel");

  // L0/L1/L2 kill panels: grouped into ONE row inside a boxed section so the
  // four kill modes can be compared side-by-side. Reusing phaseGroup with a
  // high index ensures these render AFTER the phase distribution sections.
  //
  // The cascade (when does each fire):
  //   L0 — Scheduler preemption  (priorityClass collision; before pod runs)
  //   L1 — Kubelet eviction       (node memory/disk pressure; pod-level)
  //   L2 — cgroup OOM kill        (container hits its limits.memory)
  //   L2 — node-kernel OOM kill   (subprocess inside a cgroup gets oom-killed)
  const oomGroup = { index: 400, label: "Kill events — L0 preempt / L1 evict / L2 cgroup / L2 node-kernel" };

  // L0 — Scheduler preemptions (new — was missing entirely)
  {
    const bins = l0Events.length > 0
      ? buildOomBins(l0Events)
      : [{ label: "(none)", count: 0 }];
    const xLabel = l0Events.length > 0
      ? `victim (${l0Events.length} preempt(s))`
      : "victim (no preemptions)";
    const svg = drawBars(bins, "L0 — Scheduler preemptions", xLabel);
    panels.push({ title: "L0 — Scheduler preemptions", yLabel: "[count]", data: [], svg, phaseGroup: oomGroup });
  }

  // L1 — Kubelet evictions
  {
    const bins = l1Events.length > 0
      ? buildL1Bins(l1Events)
      : [{ label: "(none)", count: 0 }];
    const totalBytes = l1Events.reduce((a, e) => a + (e.bytes_at_kill ?? 0), 0);
    const sizeStr = totalBytes > 0 ? `, ${(totalBytes / (1024 ** 3)).toFixed(1)} GiB total` : "";
    const xLabel = l1Events.length > 0
      ? `pod (${l1Events.length} evict(s)${sizeStr})`
      : "pod (no evictions)";
    const svg = drawBars(bins, "L1 — Kubelet evictions", xLabel);
    panels.push({ title: "L1 — Kubelet evictions", yLabel: "[count]", data: [], svg, phaseGroup: oomGroup });
  }

  // L2a — cgroup OOM kills (per-pod limits.memory)
  {
    const bins = l2cEvents.length > 0
      ? buildOomBins(l2cEvents)
      : [{ label: "(none)", count: 0 }];
    const xLabel = l2cEvents.length > 0
      ? `victim (${l2cEvents.length} cgroup OOM(s))`
      : "victim (no cgroup OOMs)";
    const svg = drawBars(bins, "L2 — cgroup OOM kills", xLabel);
    panels.push({ title: "L2 — cgroup OOM kills", yLabel: "[count]", data: [], svg, phaseGroup: oomGroup });
  }

  // L2b — node-kernel OOM kills (whole-VM OOM)
  {
    const bins = l2kEvents.length > 0
      ? buildOomBins(l2kEvents)
      : [{ label: "(none)", count: 0 }];
    const xLabel = l2kEvents.length > 0
      ? `victim (${l2kEvents.length} node-kernel OOM(s))`
      : "victim (no node-kernel OOMs)";
    const svg = drawBars(bins, "L2 — node-kernel OOM kills", xLabel);
    panels.push({ title: "L2 — node-kernel OOM kills", yLabel: "[count]", data: [], svg, phaseGroup: oomGroup });
  }

  // Restart events from pod_timeline.jsonl. Counts every restartCount
  // increment per pod-family during the bench window. THIS IS THE CATCH-ALL
  // — works regardless of how containerd labels the termination (Error,
  // OOMKilled, ContainerCannotRun, whatever), how dmesg parsing performed,
  // or whether kubelet logs were reachable. Verified on the previous bench
  // to catch 4 toxiproxy restarts that all the other panels missed.
  if (timelineRaw) {
    const bins = buildRestartEventBins(timelineRaw);
    const total = bins.reduce((a, b) => a + b.count, 0);
    const xLabel = total > 0
      ? `pod family (${total} restart event(s) total)`
      : "pod family (no restarts in window)";
    const svg = drawBars(
      bins.length > 0 ? bins : [{ label: "(no restarts)", count: 0 }],
      "Restart events (from pod_timeline)",
      xLabel,
    );
    panels.push({ title: "Restart events (from pod_timeline)", yLabel: "[count]", data: [], svg, phaseGroup: oomGroup });
  }

  // Container terminations — the ground-truth source. Reads pods.json
  // (captured at end of bench) directly: any container with restartCount > 0
  // whose lastState.terminated.finishedAt falls inside the bench window is
  // counted, regardless of containerd's "reason" label (which often says
  // "Error" for what was actually cgroup-OOM exit 137, dropping it off the
  // L1/L2 panels). This is what answers "did anything die during the bench"
  // without depending on event collectors, dmesg parsing, or kubelet log
  // reachability.
  // benchStartMs from meta.json is the shared chart-time origin (declared at
  // top of function). For the bench-window terminations check below we want
  // the actual first/last throughput sample — those bookend "during the
  // bench" more precisely than meta.json's wall-clock start (which can be
  // a few seconds before any throughput data exists).
  const termWindowStartMs = thr.length > 0 ? thr[0].ts : 0;
  const benchEndMs = thr.length > 0 ? thr[thr.length - 1].ts : Date.now();
  type Term = { pod: string; container: string; reason: string; exitCode: number; ts: number };
  const terms: Term[] = [];
  for (const p of pods) {
    for (const c of p.containers ?? []) {
      const t = c.lastState?.terminated;
      if (!t || !t.finishedAt) continue;
      const ts = Date.parse(t.finishedAt);
      if (!Number.isFinite(ts) || ts < termWindowStartMs || ts > benchEndMs + 5000) continue;
      terms.push({
        pod: p.name,
        container: c.name,
        reason: t.reason ?? "?",
        exitCode: t.exitCode ?? 0,
        ts,
      });
    }
  }
  {
    const counts = new Map<string, number>();
    for (const t of terms) {
      // Group by pod-deployment prefix (strip the random hash tail) +
      // reason+exit so we can see "60 worker pods, Error exit 137" as one bar
      // instead of 60 separate bars.
      const family = t.pod.replace(/-[a-z0-9]{5,10}$/, "").replace(/-[a-f0-9]{8,}$/, "");
      const key = `${family} (${t.reason}/${t.exitCode})`;
      counts.set(key, (counts.get(key) ?? 0) + 1);
    }
    const bins = counts.size > 0
      ? [...counts.entries()]
        .map(([label, count]) => ({ label, count }))
        .sort((a, b) => b.count - a.count)
      : [{ label: "(no terminations in bench window)", count: 0 }];
    const xLabel = terms.length > 0
      ? `container (${terms.length} termination(s) in window)`
      : "container (no terminations)";
    const svg = drawBars(bins, "Container terminations (all reasons)", xLabel);
    panels.push({ title: "Container terminations (all reasons)", yLabel: "[count]", data: [], svg, phaseGroup: oomGroup });
  }

  // Workload distribution panels — only when the bench saved a workload.json
  // (i.e. `--script-pattern random` was used). Synthesises 10k samples from
  // each distribution and renders bar/histogram panels.
  const workloadRaw = await safeReadText(`${outDir}/workload.json`);
  if (workloadRaw) {
    try {
      const cfg = JSON.parse(workloadRaw) as WorkloadConfig;
      panels.push(...buildWorkloadPanels(cfg));
    } catch (e) {
      console.warn(`[report] couldn't parse workload.json: ${(e as Error).message}`);
    }
  }

  // Per-panel SVG.
  for (const p of panels) {
    if (p.data.length === 0 && !p.svg) continue;
    const svg = p.svg ?? drawGraphMulti(p.data, p.title, p.yLabel, undefined, p.verticalLines);
    await Deno.writeTextFile(`${outDir}/${slug(p.title)}.svg`, svg);
  }

  // Run-context summary — pinned at the top so you can tell at a glance
  // which bench/topology/workload produced this dashboard. Includes paths
  // so you can re-derive everything from the report dir alone.
  const summary: Array<{ label: string; value: string }> = [];
  const metaJsonRaw = await safeReadText(`${outDir}/meta.json`);
  if (metaJsonRaw) {
    try {
      const m = JSON.parse(metaJsonRaw) as {
        topology?: string;
        host?: string;
        workspace?: string;
        bench_cmd?: string;
        workload_path?: string;
        helm_values?: string[];
      };
      if (m.topology) summary.push({ label: "Topology", value: m.topology });
      if (m.workload_path) summary.push({ label: "Workload", value: m.workload_path });
      if (m.host) summary.push({ label: "Host", value: m.host });
      if (m.workspace) summary.push({ label: "Workspace", value: m.workspace });
      if (m.helm_values?.length) summary.push({ label: "Helm values", value: m.helm_values.join(", ") });
      if (m.bench_cmd) summary.push({ label: "Bench cmd", value: m.bench_cmd });
    } catch { /* fall through */ }
  }
  // Always also show the report dir path so you know where to find raw data
  // (cpu_samples.tsv, pg.log, pgbadger.html, JSONLs).
  summary.push({ label: "Report dir", value: outDir });

  // Phased workload summary — list each phase with its time window, pusher
  // count, and dominant mode, so reading the dashboard side-by-side with the
  // throughput chart, you can attribute dips/spikes to the right phase.
  if (workloadRaw) {
    try {
      const wcfg = JSON.parse(workloadRaw) as { phases?: Array<{
        name?: string;
        duration_s: number;
        pushers: number;
        mode?: { weights?: Record<string, number> };
      }> };
      if (Array.isArray(wcfg.phases) && wcfg.phases.length > 0) {
        let t = 0;
        for (let i = 0; i < wcfg.phases.length; i++) {
          const p = wcfg.phases[i];
          const t0 = t;
          const t1 = t + p.duration_s;
          t = t1;
          const weights = p.mode?.weights ?? {};
          const total = Object.values(weights).reduce((a, b) => a + b, 0);
          const top = Object.entries(weights).sort((a, b) => b[1] - a[1])[0];
          const modeStr = top && total > 0
            ? `${(top[1] / total * 100).toFixed(0)}% ${top[0]}`
            : "—";
          summary.push({
            label: `Phase ${i + 1} ${p.name ?? ""}`.trim(),
            value: `${t0}-${t1}s | ${p.pushers} pushers | mode: ${modeStr}`,
          });
        }
      }
    } catch { /* ignore — fall through to no summary */ }
  }

  // Composed dashboard.
  const meta: DashboardMeta = {
    topology,
    suite: outDir,
    generated: new Date().toISOString(),
    walltime_s: walltimeS,
    jobs_completed: thr.length > 0 ? thr[thr.length - 1].processed : 0,
    throughput_per_s: input.finalThroughput ?? 0,
    summary: summary.length > 0 ? summary : undefined,
  };
  await Deno.writeTextFile(
    `${outDir}/dashboard.svg`,
    renderDashboard(meta, panels, { xRelativeOriginMs: benchStartMs }),
  );
  // Best-effort PDF render alongside the SVG. Some viewers prefer PDF; this
  // gives both for free. Failures don't abort the report.
  try {
    await renderSvgToPdf(`${outDir}/dashboard.svg`, `${outDir}/dashboard.pdf`);
  } catch (e) {
    console.warn(`[report] PDF render failed (svg still written): ${(e as Error).message}`);
  }

  // Markdown report.
  const md: string[] = [];
  md.push(`# ${topology}`);
  md.push("");
  md.push(`- Wall time: ${walltimeS.toFixed(1)}s`);
  if (input.finalThroughput !== undefined) {
    md.push(`- Final throughput: ${input.finalThroughput.toFixed(2)} jobs/s`);
  }
  md.push(`- Generated: ${meta.generated}`);
  md.push("");
  md.push("## Dashboard");
  md.push("");
  md.push("![Dashboard](./dashboard.svg)");
  md.push("");
  md.push("## Panels");
  md.push("");
  for (const p of panels) {
    if (p.data.length === 0 && !p.svg) continue;
    md.push(`### ${p.title}`);
    md.push("");
    md.push(`![${p.title}](./${slug(p.title)}.svg)`);
    md.push("");
  }
  await Deno.writeTextFile(`${outDir}/report.md`, md.join("\n") + "\n");

  const rendered = panels.filter((p) => p.data.length > 0 || p.svg).length;
  console.log(`[report] dashboard.svg + report.md + ${rendered} panel SVG(s) -> ${outDir}`);
}

// Synthesise 10k samples from a single distribution and bucket them into
// histogram bins. For categorical distributions, returns one bin per category
// in declaration order (counts proportional to weights × N).
function sampleDist(spec: DistSpec, n = 10000): { values: number[]; categorical: { label: string; count: number }[] | null } {
  if (spec.dist === "categorical") {
    const total = Object.values(spec.weights).reduce((a, b) => a + b, 0);
    return {
      values: [],
      categorical: Object.entries(spec.weights).map(([label, w]) => ({
        label,
        count: Math.round((w / total) * n),
      })),
    };
  }
  // Reuse sampleJobParams's underlying sampler by constructing a throwaway
  // config: pick one parameter at a time.
  const dummyCat: DistSpec = { dist: "categorical", weights: { sleep: 1 } };
  const cfg = {
    ram_mb: spec,
    duration_ms: { dist: "uniform", min: 0, max: 1 } as DistSpec,
    mode: dummyCat,
  };
  const values: number[] = [];
  for (let i = 0; i < n; i++) values.push(sampleJobParams(cfg).ram_mb);
  return { values, categorical: null };
}

// Bucket continuous samples into ~20 bins.  Uses log-space bins when values
// span >2 orders of magnitude (typical for lognormal) so the heavy tail
// doesn't squash the bulk of the distribution into one bar.
function histogramBins(values: number[], binCount = 20): { label: string; count: number }[] {
  if (values.length === 0) return [];
  const min = Math.min(...values);
  const max = Math.max(...values);
  if (min === max) return [{ label: String(min), count: values.length }];

  // Log binning kicks in when min > 0 and we span >100x — otherwise linear.
  const useLog = min > 0 && max / min > 100;
  const edges: number[] = [];
  if (useLog) {
    const logMin = Math.log(min);
    const logMax = Math.log(max);
    const step = (logMax - logMin) / binCount;
    for (let i = 0; i <= binCount; i++) edges.push(Math.exp(logMin + i * step));
  } else {
    const step = (max - min) / binCount;
    for (let i = 0; i <= binCount; i++) edges.push(min + i * step);
  }

  const bins = Array.from({ length: binCount }, (_, i) => ({
    label: String(Math.round((edges[i] + edges[i + 1]) / 2)),
    count: 0,
  }));
  for (const v of values) {
    // Binary search would be tidier but binCount is tiny.
    let idx = binCount - 1;
    for (let i = 0; i < binCount; i++) {
      if (v < edges[i + 1]) { idx = i; break; }
    }
    bins[idx].count++;
  }
  return bins;
}

// One bar per (victim, source) pair, sorted by count desc so the noisiest
// victims are leftmost. cgroup OOMs are the "right kind" — show them with the
// pod name as-is. Node-kernel OOMs name the process (e.g. "deno") — tagged
// so the source distinction stays visible at a glance.
// One bar per evicted pod, label includes total bytes evicted across all
// eviction events for that pod. Sorted by event count desc — most-frequently
// evicted at the left.
function buildL1Bins(events: OomEvent[]): { label: string; count: number }[] {
  const counts = new Map<string, number>();
  const bytes = new Map<string, number>();
  for (const e of events) {
    counts.set(e.victim, (counts.get(e.victim) ?? 0) + 1);
    if (e.bytes_at_kill) {
      bytes.set(e.victim, (bytes.get(e.victim) ?? 0) + e.bytes_at_kill);
    }
  }
  return [...counts.entries()]
    .map(([pod, count]) => {
      const b = bytes.get(pod);
      const sizeStr = b
        ? b >= 1024 ** 3
          ? `${(b / 1024 ** 3).toFixed(1)}G`
          : `${(b / 1024 ** 2).toFixed(0)}M`
        : "";
      const label = sizeStr ? `${pod} (${sizeStr})` : pod;
      return { label, count };
    })
    .sort((a, b) => b.count - a.count);
}

function buildOomBins(events: OomEvent[]): { label: string; count: number }[] {
  const counts = new Map<string, number>();
  for (const e of events) {
    const tag = e.source === "cgroup" ? "cgroup" : "kernel";
    const label = `${e.victim} (${tag})`;
    counts.set(label, (counts.get(label) ?? 0) + 1);
  }
  return [...counts.entries()]
    .map(([label, count]) => ({ label, count }))
    .sort((a, b) => b.count - a.count);
}

// Cast helper — TS would otherwise narrow `WorkloadConfig` to either branch
// at the use site and complain. The phased path is checked first so a
// `phases:` field shorts out the static-config interpretation.
function workloadPhases(cfg: WorkloadConfig): Array<{ name?: string; ram_mb?: DistSpec; duration_ms?: DistSpec; mode?: DistSpec; duration_s?: number; pushers?: number }> | null {
  const maybePhased = cfg as unknown as { phases?: Array<{ name?: string; ram_mb?: DistSpec; duration_ms?: DistSpec; mode?: DistSpec; duration_s?: number; pushers?: number }> };
  return Array.isArray(maybePhased.phases) ? maybePhased.phases : null;
}

function buildOneStaticDistributionPanels(
  cfg: { ram_mb?: DistSpec; duration_ms?: DistSpec; mode?: DistSpec },
  titlePrefix: string,
  phaseGroup?: { index: number; label: string },
): DashboardPanel[] {
  const out: DashboardPanel[] = [];
  for (const [field, baseLabel, xLabel] of [
    ["ram_mb",      "RAM distribution",      "ram_mb"],
    ["duration_ms", "Duration distribution", "duration_ms"],
    ["mode",        "Mode distribution",     "mode"],
  ] as const) {
    const spec = cfg[field];
    if (!spec) continue;
    const { values, categorical } = sampleDist(spec);
    // For phased panels the section header already names the phase; keep
    // the per-panel title concise so titles don't get truncated in the
    // narrower per-panel slot.
    const label = phaseGroup
      ? baseLabel
      : titlePrefix ? `${titlePrefix} — ${baseLabel}` : baseLabel;
    let svg: string;
    if (categorical) {
      svg = drawDonut(categorical, label);
    } else {
      const stats = {
        min: Math.min(...values),
        max: Math.max(...values),
        avg: values.reduce((a, b) => a + b, 0) / values.length,
      };
      svg = drawBars(histogramBins(values), label, xLabel, stats);
    }
    out.push({ title: label, yLabel: "[count]", data: [], svg, phaseGroup });
  }
  return out;
}

function buildWorkloadPanels(cfg: WorkloadConfig): DashboardPanel[] {
  const phases = workloadPhases(cfg);
  if (phases) {
    // Phased — emit three distribution panels per phase, all tagged with the
    // same phaseGroup so the dashboard renders them as a boxed single-row
    // section. The user explicitly wants per-phase comparison, which means
    // ALL panels for one phase must be on one row without wrapping.
    const out: DashboardPanel[] = [];
    phases.forEach((p, i) => {
      const label = `Phase ${i + 1}${p.name ? ` — ${p.name}` : ""}`;
      // Index offset of 300 so phase sections render AFTER nodeMem (100) +
      // PG latency (200), and BEFORE kill events (400).
      out.push(...buildOneStaticDistributionPanels(p, label, { index: 300 + i, label }));
    });
    return out;
  }
  return buildOneStaticDistributionPanels(cfg as { ram_mb?: DistSpec; duration_ms?: DistSpec; mode?: DistSpec }, "");
}

async function safeReadText(path: string): Promise<string> {
  try { return await Deno.readTextFile(path); } catch { return ""; }
}

function slug(s: string): string {
  return s.toLowerCase().replace(/\s+/g, "_").replace(/[^a-z0-9_-]/g, "");
}
