// Collect OOM kills that fired during the bench window, from two distinct
// sources:
//
//   - Node-level VM kernel OOM (`kubectl get events --field-selector
//     reason=SystemOOM`): fires when the sum of pod RSS on a node exceeds
//     the VM's RAM ceiling. The kernel picks a victim by oom_score across
//     all cgroups; the event message names the victim process (e.g. "deno").
//
//   - Per-container cgroup OOM (pod containerStatus
//     `lastState.terminated.reason == "OOMKilled"`): fires when one pod
//     exceeds its own `limits.memory`. Kills only processes in that pod's
//     cgroup; kubelet marks the container OOMKilled and restarts per policy.
//
// Output is a JSON file the renderer turns into a bar chart so heavy-bench
// reports show "what got killed and how many times" at a glance.

import { MinikubeProvisioner } from "./k8s_provisioner.ts";

export type OomEvent = {
  ts_ms: number;
  source: "node_kernel" | "cgroup" | "kubelet_eviction" | "scheduler_preemption";
  // process name from SystemOOM, or pod name from cgroup OOM / eviction /
  // preemption
  victim: string;
  node: string;
  // bytes the container was using at eviction time, parsed from the
  // Evicted message. Only present for `kubelet_eviction` source.
  bytes_at_kill?: number;
};

type RawEvent = {
  metadata?: { creationTimestamp?: string };
  lastTimestamp?: string;
  eventTime?: string;
  involvedObject?: { name?: string };
  message?: string;
  reason?: string;
};

type RawPod = {
  metadata?: { name?: string };
  spec?: { nodeName?: string };
  status?: {
    containerStatuses?: Array<{
      restartCount?: number;
      lastState?: {
        terminated?: {
          reason?: string;
          finishedAt?: string;
        };
      };
    }>;
  };
};

function parseTs(s?: string): number {
  if (!s) return 0;
  const ms = Date.parse(s);
  return Number.isFinite(ms) ? ms : 0;
}

// Pull "victim process: deno" out of the SystemOOM message text.
function parseVictim(msg: string): string {
  const m = msg.match(/victim process:\s*([^\s,]+)/);
  return m ? m[1] : "unknown";
}

// Read dmesg + uptime from a sampler pod (privileged + hostPID, so it sees
// the host kernel log buffer) and convert each "Memory cgroup out of memory:
// Killed process N (CMD)" line to an OomEvent. Catches subprocess kills
// inside a cgroup that don't surface as containerStatus OOMKilled (because
// PID1 survived) and don't surface as SystemOOM events (because the OOM
// scope was cgroup, not whole-VM). These are the ones that produce 500+
// failed jobs + low CPU util while every other source says "0 OOMs".
async function dmesgOomsFromSampler(
  prov: MinikubeProvisioner,
  samplerNamespace: string,
  samplerPod: string,
  nodeName: string,
  sinceMs: number,
): Promise<OomEvent[]> {
  // Two outputs in one exec, separated by a sentinel — saves a round-trip.
  const res = await prov.kubectl([
    "-n", samplerNamespace, "exec", samplerPod, "--", "sh", "-c",
    "cat /proc/uptime; echo '---DMESG---'; dmesg 2>/dev/null | grep 'Memory cgroup out of memory'",
  ]);
  if (res.code !== 0 || !res.stdout) return [];
  const parts = res.stdout.split("---DMESG---");
  if (parts.length < 2) return [];
  const uptime_s = parseFloat(parts[0].trim().split(/\s+/)[0] || "0");
  if (!Number.isFinite(uptime_s) || uptime_s <= 0) return [];
  const wallNowMs = Date.now();
  const bootWallMs = wallNowMs - uptime_s * 1000;

  // dmesg line format: `[ 7126.289518] Memory cgroup out of memory: Killed process 354780 (deno) total-vm:...`
  const re = /^\[\s*(\d+(?:\.\d+)?)\]\s+Memory cgroup out of memory:\s+Killed process \d+ \(([^)]+)\)/;
  const out: OomEvent[] = [];
  for (const line of parts[1].split("\n")) {
    const m = re.exec(line);
    if (!m) continue;
    const event_uptime_s = parseFloat(m[1]);
    const cmd = m[2];
    if (!Number.isFinite(event_uptime_s)) continue;
    const ts_ms = Math.round(bootWallMs + event_uptime_s * 1000);
    if (ts_ms < sinceMs) continue;
    out.push({ ts_ms, source: "node_kernel", victim: cmd, node: nodeName });
  }
  return out;
}

// Pull "was using 14821920Ki" (or similar size suffixes) out of an Evicted
// event message. kubelet formats this as `<digits><Ki|Mi|Gi>`. Returns bytes.
function parseEvictedBytes(msg: string): number | undefined {
  const m = msg.match(/was using\s+(\d+)(Ki|Mi|Gi)\b/);
  if (!m) return undefined;
  const n = parseInt(m[1]);
  if (!Number.isFinite(n)) return undefined;
  const mult = m[2] === "Gi" ? 1024 * 1024 * 1024 : m[2] === "Mi" ? 1024 * 1024 : 1024;
  return n * mult;
}

export async function collectOomEvents(
  prov: MinikubeProvisioner,
  outPath: string,
  opts: { sinceMs?: number } = {},
): Promise<void> {
  const since = opts.sinceMs ?? 0;
  const out: OomEvent[] = [];

  // 1) Node-level VM kernel OOMs (SystemOOM events).
  const evRes = await prov.kubectl([
    "get", "events",
    "-A",
    "--field-selector", "reason=SystemOOM",
    "-o", "json",
  ]);
  if (evRes.code === 0 && evRes.stdout) {
    let evJson: { items?: RawEvent[] } = {};
    try { evJson = JSON.parse(evRes.stdout); } catch { /* ignore parse errors */ }
    for (const e of evJson.items ?? []) {
      const ts = parseTs(e.eventTime ?? e.lastTimestamp ?? e.metadata?.creationTimestamp);
      if (ts < since) continue;
      out.push({
        ts_ms: ts,
        source: "node_kernel",
        victim: parseVictim(e.message ?? ""),
        node: e.involvedObject?.name ?? "unknown",
      });
    }
  }

  // 1b) Kubelet evictions (L1, fires BEFORE kernel OOM under graceful node
  // pressure). Has a usable memory-at-kill value in the message.
  const evictedRes = await prov.kubectl([
    "get", "events",
    "-A",
    "--field-selector", "reason=Evicted",
    "-o", "json",
  ]);
  if (evictedRes.code === 0 && evictedRes.stdout) {
    let evJson: { items?: RawEvent[] } = {};
    try { evJson = JSON.parse(evictedRes.stdout); } catch { /* ignore parse errors */ }
    for (const e of evJson.items ?? []) {
      const ts = parseTs(e.eventTime ?? e.lastTimestamp ?? e.metadata?.creationTimestamp);
      if (ts < since) continue;
      out.push({
        ts_ms: ts,
        source: "kubelet_eviction",
        victim: e.involvedObject?.name ?? "unknown",
        node: "unknown",  // event references the evicted pod, not the node;
                          // the message has it but parsing is brittle
        bytes_at_kill: parseEvictedBytes(e.message ?? ""),
      });
    }
  }

  // 1c) Scheduler preemptions (L0, fires when a higher-priority pod can't
  // fit and the scheduler picks a victim to evict). DIFFERENT from kubelet
  // eviction: kubelet evicts due to NODE PRESSURE (memory, disk); scheduler
  // preempts due to a PriorityClass collision. Common case in this cluster:
  // PG/sampler at wm-critical can't fit → workers (priority 0) get
  // preempted. Without this we miss the bulk of "worker dying" events.
  const preemptedRes = await prov.kubectl([
    "get", "events",
    "-A",
    "--field-selector", "reason=Preempted",
    "-o", "json",
  ]);
  if (preemptedRes.code === 0 && preemptedRes.stdout) {
    let evJson: { items?: RawEvent[] } = {};
    try { evJson = JSON.parse(preemptedRes.stdout); } catch { /* ignore parse errors */ }
    for (const e of evJson.items ?? []) {
      const ts = parseTs(e.eventTime ?? e.lastTimestamp ?? e.metadata?.creationTimestamp);
      if (ts < since) continue;
      out.push({
        ts_ms: ts,
        source: "scheduler_preemption",
        victim: e.involvedObject?.name ?? "unknown",
        node: "unknown",
      });
    }
  }

  // 2) Per-container cgroup OOMs (pod containerStatuses).
  // Catches at most ONE OOMKill per container — the most recent. For multi-
  // restart OOM patterns we merge the live poller's JSONL below.
  const podRes = await prov.kubectl([
    "get", "pods", "-A",
    "-o", "json",
  ]);
  if (podRes.code === 0 && podRes.stdout) {
    let podJson: { items?: RawPod[] } = {};
    try { podJson = JSON.parse(podRes.stdout); } catch { /* ignore parse errors */ }
    for (const p of podJson.items ?? []) {
      for (const cs of p.status?.containerStatuses ?? []) {
        const term = cs.lastState?.terminated;
        if (term?.reason !== "OOMKilled") continue;
        const ts = parseTs(term.finishedAt);
        if (ts < since) continue;
        out.push({
          ts_ms: ts,
          source: "cgroup",
          victim: p.metadata?.name ?? "unknown",
          node: p.spec?.nodeName ?? "unknown",
        });
      }
    }
  }

  // 3) Merge mid-bench OOMKills captured by the live poller. The poller
  // dedupes by (pod, container, finishedAt) and writes JSONL; we union with
  // the end-of-bench scan above so OOMs that fired AND recovered mid-bench
  // (which the lastState scan misses) still show up on the L2 cgroup panel.
  const liveJsonlPath = outPath.replace(/\.json$/, "_live.jsonl");
  try {
    const text = await Deno.readTextFile(liveJsonlPath);
    const seen = new Set(out.map((e) => `${e.victim}|${e.ts_ms}`));
    for (const line of text.split("\n")) {
      if (!line.trim()) continue;
      try {
        const r = JSON.parse(line) as OomEvent;
        if (r.ts_ms < since) continue;
        const key = `${r.victim}|${r.ts_ms}`;
        if (seen.has(key)) continue;
        seen.add(key);
        out.push(r);
      } catch { /* skip malformed line */ }
    }
  } catch { /* poller wasn't running or file missing — ignore */ }

  // 4) Per-node dmesg scan. Catches subprocess OOM kills inside a cgroup
  // (where the container's PID 1 survived → no containerStatus OOMKilled →
  // sources 2/3 miss it, and the OOM was cgroup-scope → no SystemOOM event
  // → source 1 misses it). This is what produces "lots of failed jobs +
  // low CPU util while every panel says 0 OOMs" — workers' deno
  // subprocesses get SIGKILLed mid-job, the worker container keeps polling.
  const samplerNs = "kube-system";
  const samplerSelector = "app=wm-sim-cpu-sampler";
  const samplerListRes = await prov.kubectl([
    "-n", samplerNs, "get", "pods", "-l", samplerSelector,
    "-o", "jsonpath={range .items[*]}{.metadata.name}|{.spec.nodeName}\\n{end}",
  ]);
  if (samplerListRes.code === 0 && samplerListRes.stdout) {
    const dmesgEvents: OomEvent[] = [];
    for (const line of samplerListRes.stdout.split("\n")) {
      if (!line.trim()) continue;
      const [samplerPod, nodeName] = line.split("|");
      if (!samplerPod || !nodeName) continue;
      try {
        const evs = await dmesgOomsFromSampler(prov, samplerNs, samplerPod.trim(), nodeName.trim(), since);
        dmesgEvents.push(...evs);
      } catch (e) {
        console.warn(`[oom] dmesg scan on ${nodeName} failed: ${(e as Error).message}`);
      }
    }
    // Dedupe across nodes — extremely close timestamps for the same victim
    // would be rare but defend against double-counting if a future change
    // adds dmesg multi-source overlap.
    const seenDmesg = new Set(out.map((e) => `${e.source}|${e.victim}|${e.ts_ms}`));
    for (const e of dmesgEvents) {
      const key = `${e.source}|${e.victim}|${e.ts_ms}`;
      if (seenDmesg.has(key)) continue;
      seenDmesg.add(key);
      out.push(e);
    }
  }

  out.sort((a, b) => a.ts_ms - b.ts_ms);
  await Deno.writeTextFile(outPath, JSON.stringify(out));
  const ks = out.filter(e => e.source === "node_kernel").length;
  const cs = out.filter(e => e.source === "cgroup").length;
  const es = out.filter(e => e.source === "kubelet_eviction").length;
  console.log(`[oom] ${out.length} kill event(s) captured (${es} L1 evicted, ${cs} L2 cgroup, ${ks} L2 node-kernel) -> ${outPath}`);
}
