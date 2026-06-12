// In-bench OOMKilled poller. The end-of-bench scan in `oom_events.ts` reads
// `lastState.terminated` from every pod, but that field only holds the MOST
// RECENT termination — a pod that OOMKilled mid-bench and then recovered into
// Running state by end-of-bench shows no OOMKill at all in the final scan,
// silently zeroing out the L2 cgroup OOM panel.
//
// This poller closes the gap: every N seconds it lists all pods, watches for
// `restartCount` increases on any container whose `lastState.terminated.reason`
// is "OOMKilled", and appends a JSONL row per newly-seen OOMKill. End-of-bench
// `collectOomEvents` reads this JSONL and merges it with the final scan so the
// L2 cgroup OOM panel reflects every kill that fired during the window.
//
// Dedupe key: `<pod>|<container>|<finishedAt>` — restartCount can climb across
// multiple polls before the next OOMKill, but finishedAt is unique per kill.

import { MinikubeProvisioner } from "./k8s_provisioner.ts";

export type OomPoller = {
  // Mutate this to false to stop the loop. The polling promise resolves when
  // the in-flight kubectl call finishes after the flip.
  cont: { value: boolean };
  done: Promise<void>;
};

type ContainerStatus = {
  name?: string;
  restartCount?: number;
  lastState?: {
    terminated?: {
      reason?: string;
      finishedAt?: string;
    };
  };
};

type PodSnapshot = {
  metadata?: { name?: string };
  spec?: { nodeName?: string };
  status?: { containerStatuses?: ContainerStatus[] };
};

export function startOomPoller(
  prov: MinikubeProvisioner,
  outPath: string,
  opts: { intervalMs?: number } = {},
): OomPoller {
  const intervalMs = opts.intervalMs ?? 2000;
  const cont = { value: true };
  const f = Deno.openSync(outPath, { write: true, create: true, truncate: true });
  const enc = new TextEncoder();
  // Dedupe: a kill is uniquely identified by where + when it finished.
  const seen = new Set<string>();

  const done = (async () => {
    while (cont.value) {
      const startMs = Date.now();
      try {
        const res = await prov.kubectl(["get", "pods", "-A", "-o", "json"]);
        if (res.code === 0 && res.stdout) {
          let parsed: { items?: PodSnapshot[] } = {};
          try { parsed = JSON.parse(res.stdout); } catch { /* transient — retry next tick */ }
          for (const p of parsed.items ?? []) {
            const podName = p.metadata?.name;
            const node = p.spec?.nodeName ?? "unknown";
            if (!podName) continue;
            for (const cs of p.status?.containerStatuses ?? []) {
              const term = cs.lastState?.terminated;
              if (term?.reason !== "OOMKilled") continue;
              const finishedAt = term.finishedAt;
              if (!finishedAt) continue;
              const key = `${podName}|${cs.name ?? "?"}|${finishedAt}`;
              if (seen.has(key)) continue;
              seen.add(key);
              const ts = Date.parse(finishedAt);
              const row = JSON.stringify({
                ts_ms: Number.isFinite(ts) ? ts : startMs,
                source: "cgroup",
                victim: podName,
                container: cs.name,
                node,
              });
              f.writeSync(enc.encode(row + "\n"));
            }
          }
        }
      } catch (_e) {
        // Transient kubectl errors tolerated — bench should not die because
        // one poll failed. Next iteration retries.
      }
      const elapsed = Date.now() - startMs;
      if (cont.value && elapsed < intervalMs) {
        await new Promise((r) => setTimeout(r, intervalMs - elapsed));
      }
    }
    f.close();
  })();

  return { cont, done };
}
