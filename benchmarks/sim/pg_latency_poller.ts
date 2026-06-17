// PG response-time poller. Every interval (default 1s), runs a trivial
// `SELECT 1` against the bundled PG via `kubectl exec ... psql` and records
// how long it took. The point is to make PG's responsiveness visible OVER
// TIME — when the DB is healthy this is a flat line near 0; when PG gets
// contended (heavy bench load, connection storm, autovacuum stalling, etc.)
// the latency rises and you can correlate it with the throughput dip on the
// same x-axis.
//
// Output: JSONL, one line per poll
//   {"ts": <ms>, "latency_ms": <number>, "ok": true|false, "err"?: "..."}
//
// Why `SELECT 1` (vs a real queue query): we want PG's *infrastructure*
// latency, not query cost. A trivial query exposes connection-pool /
// backend-fork / network round-trip cost, which is exactly what bench
// pressure inflates. Switching to a heavier query muddies "is PG fast" with
// "is this query fast".

import { MinikubeProvisioner } from "./k8s_provisioner.ts";

export type PgLatencyPoller = {
  cont: { value: boolean };
  done: Promise<void>;
};

export function startPgLatencyPoller(
  prov: MinikubeProvisioner,
  outPath: string,
  opts: { intervalMs?: number; namespace?: string; pgPodSelector?: string } = {},
): PgLatencyPoller {
  const intervalMs = opts.intervalMs ?? 250;
  const namespace = opts.namespace ?? "default";
  const selector = opts.pgPodSelector ?? "app=windmill-postgresql-demo-app";
  const cont = { value: true };
  const f = Deno.openSync(outPath, { write: true, create: true, truncate: true });
  const enc = new TextEncoder();

  // Resolve the PG pod name once at startup. If PG restarts mid-bench the
  // name shouldn't change (StatefulSet), so caching is safe.
  const done = (async () => {
    let podName = "";
    try {
      const r = await prov.kubectl([
        "-n", namespace, "get", "pods", "-l", selector,
        "-o", "jsonpath={.items[0].metadata.name}",
      ]);
      if (r.code === 0) podName = r.stdout.trim();
    } catch (_e) { /* fall through; loop will retry */ }

    while (cont.value) {
      const startMs = Date.now();
      let ok = false;
      let err: string | undefined;
      let pgQueryMs: number | undefined;
      try {
        if (!podName) {
          // Retry pod lookup if the initial resolve failed.
          const r = await prov.kubectl([
            "-n", namespace, "get", "pods", "-l", selector,
            "-o", "jsonpath={.items[0].metadata.name}",
          ]);
          if (r.code === 0) podName = r.stdout.trim();
        }
        if (podName) {
          // Run with `\timing on` so we get TWO signals:
          //   1. Wall-clock around the whole kubectl exec → "kubectl
          //      roundtrip" (~200ms baseline = API hop + containerd exec +
          //      fork psql + open new pg conn). Useful as a cluster-control-
          //      plane health signal, not as a PG signal.
          //   2. The `Time: X.XXX ms` line psql emits → pure PG query
          //      latency (sub-millisecond when PG is happy; rises only
          //      under contention/lock waits).
          // Both are emitted as separate `kind` series so the chart shows
          // both lines on the same axis.
          const r = await prov.kubectl([
            "-n", namespace, "exec", podName, "--",
            "psql", "-U", "postgres", "-d", "windmill",
            "-c", "\\timing on",
            "-c", "SELECT 1",
          ]);
          ok = r.code === 0;
          if (!ok) err = (r.stderr || r.stdout).slice(0, 200);
          // Parse psql's "Time: 0.420 ms" line for the pure PG query time.
          const m = r.stdout.match(/Time:\s*([\d.]+)\s*ms/);
          if (m) {
            pgQueryMs = parseFloat(m[1]);
          }
        } else {
          err = "no PG pod";
        }
      } catch (e) {
        err = (e as Error).message;
      }
      const latency_ms = Date.now() - startMs;
      const row = JSON.stringify({ ts: startMs, latency_ms, pg_query_ms: pgQueryMs, ok, err });
      f.writeSync(enc.encode(row + "\n"));
      const elapsed = Date.now() - startMs;
      if (cont.value && elapsed < intervalMs) {
        await new Promise((r) => setTimeout(r, intervalMs - elapsed));
      }
    }
    f.close();
  })();

  return { cont, done };
}
