// Polls /proc/loadavg + nproc on each minikube node every intervalMs and
// writes one JSONL row per node per tick. Used to compute *saturation* (load /
// ncpu) and *oversaturation* (max(0, load/ncpu - 1)) in the dashboard. PSI is
// not available in the minikube kernel and cpu.stat throttling is meaningless
// without limits.cpu, so loadavg is the only saturation signal we have.
//
// Output line: {"ts": ms, "node": "wm-sim-k8s-4node-m02", "load1": 44.2,
//                "load5": 32.1, "load15": 24.4, "ncpu": 4}

import { MinikubeProvisioner } from "./k8s_provisioner.ts";

export type NodeLoadPoller = {
  cont: { value: boolean };
  done: Promise<void>;
};

export function startNodeLoadPoller(
  prov: MinikubeProvisioner,
  outPath: string,
  opts: { intervalMs?: number } = {},
): NodeLoadPoller {
  const intervalMs = opts.intervalMs ?? 2000;
  const cont = { value: true };
  const f = Deno.openSync(outPath, { write: true, create: true, truncate: true });
  const enc = new TextEncoder();

  const done = (async () => {
    // Discover nodes once at startup. The poll loop reuses this list. New
    // nodes joining mid-bench are rare (we don't auto-scale the cluster).
    let nodes: { name: string; ip: string }[] = [];
    try {
      const r = await prov.kubectl([
        "get", "nodes",
        "-o", "jsonpath={range .items[*]}{.metadata.name}{\"|\"}{.status.addresses[?(@.type==\"InternalIP\")].address}{\"\\n\"}{end}",
      ]);
      if (r.code === 0) {
        for (const line of r.stdout.split("\n")) {
          if (!line.trim()) continue;
          const [name, ip] = line.split("|");
          if (name && ip) nodes.push({ name: name.trim(), ip: ip.trim() });
        }
      }
    } catch (e) {
      console.warn(`[node-load] node discovery failed: ${(e as Error).message}`);
    }

    while (cont.value) {
      const startMs = Date.now();
      // Poll all nodes in parallel — one ssh per node per tick.
      await Promise.all(nodes.map(async ({ name, ip }) => {
        try {
          // Each minikube node has its own ssh key under ~/.minikube/machines.
          const keyPath = `${Deno.env.get("HOME")}/.minikube/machines/${name}/id_rsa`;
          const proc = new Deno.Command("ssh", {
            args: [
              "-o", "StrictHostKeyChecking=no",
              "-o", "UserKnownHostsFile=/dev/null",
              "-o", "ConnectTimeout=2",
              "-o", "LogLevel=ERROR",
              "-i", keyPath,
              `docker@${ip}`,
              // procs_running is the runnable count (CPU-bound queue) — does
              // NOT include D-state procs (disk/network wait). loadavg counts
              // both, so loadavg/ncpu was conflating CPU-starved processes
              // with PG backends waiting on disk I/O.
              "cat /proc/loadavg && cat /proc/stat | grep ^procs_running && nproc",
            ],
            stdout: "piped",
            stderr: "null",
          });
          const out = await proc.output();
          const text = new TextDecoder().decode(out.stdout).trim();
          const lines = text.split("\n");
          if (lines.length < 3) return;
          const loadParts = lines[0].split(" ");
          const load1 = parseFloat(loadParts[0]);
          const load5 = parseFloat(loadParts[1]);
          const load15 = parseFloat(loadParts[2]);
          // "procs_running N" — instantaneous count of runnable processes
          // (current + queued for CPU). Excludes D-state.
          const procsRunning = parseInt(lines[1].split(/\s+/)[1] ?? "");
          const ncpu = parseInt(lines[2].trim());
          if (!Number.isFinite(load1) || !Number.isFinite(ncpu)) return;
          const row = {
            ts: startMs,
            node: name,
            load1,
            load5,
            load15,
            procs_running: Number.isFinite(procsRunning) ? procsRunning : null,
            ncpu,
          };
          f.writeSync(enc.encode(JSON.stringify(row) + "\n"));
        } catch (_e) { /* skip this node this tick */ }
      }));
      const elapsed = Date.now() - startMs;
      if (cont.value && elapsed < intervalMs) {
        await new Promise((r) => setTimeout(r, intervalMs - elapsed));
      }
    }
    f.close();
  })();

  return { cont, done };
}
