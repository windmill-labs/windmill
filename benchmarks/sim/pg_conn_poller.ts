// PG connection-count poller. Every interval (default 1s), runs
//   SELECT state, count(*) FROM pg_stat_activity GROUP BY state
// against the bundled PG. Writes one JSONL line per poll with per-state
// breakdown so the dashboard can plot active vs idle vs idle-in-transaction
// over time. Answers "is the spike PG connection-count vs PG query-rate?"
// — pairs naturally with the pg_latency panel.
//
// Output:
//   {"ts": <ms>, "active": N, "idle": M, "idle_in_xact": K, "total": T}

import { MinikubeProvisioner } from "./k8s_provisioner.ts";

export type PgConnPoller = {
  cont: { value: boolean };
  done: Promise<void>;
};

export function startPgConnPoller(
  prov: MinikubeProvisioner,
  outPath: string,
  opts: { intervalMs?: number; namespace?: string; pgPodSelector?: string } = {},
): PgConnPoller {
  const intervalMs = opts.intervalMs ?? 1000;
  const namespace = opts.namespace ?? "default";
  const selector = opts.pgPodSelector ?? "app=windmill-postgresql-demo-app";
  const cont = { value: true };
  const f = Deno.openSync(outPath, { write: true, create: true, truncate: true });
  const enc = new TextEncoder();

  const done = (async () => {
    let podName = "";
    try {
      const r = await prov.kubectl([
        "-n", namespace, "get", "pods", "-l", selector,
        "-o", "jsonpath={.items[0].metadata.name}",
      ]);
      if (r.code === 0) podName = r.stdout.trim();
    } catch (_e) { /* retry in loop */ }

    while (cont.value) {
      const startMs = Date.now();
      let row: Record<string, unknown> = { ts: startMs };
      try {
        if (!podName) {
          const r = await prov.kubectl([
            "-n", namespace, "get", "pods", "-l", selector,
            "-o", "jsonpath={.items[0].metadata.name}",
          ]);
          if (r.code === 0) podName = r.stdout.trim();
        }
        if (podName) {
          const r = await prov.kubectl([
            "-n", namespace, "exec", podName, "--",
            "psql", "-U", "postgres", "-d", "windmill", "-tAc",
            // Statement timeout so a wedged PG can't stall the poller.
            "SET statement_timeout=2000; SELECT coalesce(state,'unknown') || ':' || count(*) FROM pg_stat_activity GROUP BY state",
          ]);
          if (r.code === 0) {
            let active = 0, idle = 0, idle_in_xact = 0, unknown = 0;
            for (const line of r.stdout.split("\n")) {
              if (!line.includes(":")) continue;
              const [state, countStr] = line.split(":");
              const n = parseInt(countStr.trim());
              if (!Number.isFinite(n)) continue;
              if (state === "active") active = n;
              else if (state === "idle") idle = n;
              else if (state === "idle in transaction") idle_in_xact = n;
              else unknown += n;
            }
            row = { ts: startMs, active, idle, idle_in_xact, unknown, total: active + idle + idle_in_xact + unknown };
          } else {
            row = { ts: startMs, err: (r.stderr || r.stdout).slice(0, 200) };
          }
        }
      } catch (e) {
        row = { ts: startMs, err: (e as Error).message };
      }
      f.writeSync(enc.encode(JSON.stringify(row) + "\n"));
      const elapsed = Date.now() - startMs;
      if (cont.value && elapsed < intervalMs) {
        await new Promise((r) => setTimeout(r, intervalMs - elapsed));
      }
    }
    f.close();
  })();

  return { cont, done };
}
