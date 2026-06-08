// kubectl-based per-second pod timeline. Replaces the cgroup-derived
// workers-per-node series from the sampler (which had two known bugs: stale
// pods.json captured only at end of bench, and the sampler's 5s rescan
// interval missing short-lived pods). Polling kubectl every 1s gives us the
// authoritative pod inventory at each tick — slower resolution but accurate.
//
// Output: JSONL, one line per poll, each line:
//   {"ts": <ms>, "pods": [{"name":"...","node":"...","phase":"..."}, ...]}
//
// Stopping: caller flips the `cont` ref to false; the loop exits within ~1s.

import { MinikubeProvisioner } from "./k8s_provisioner.ts";

export type PodTimelinePoller = {
  // Mutate this to false to stop the loop. The polling promise resolves when
  // the in-flight kubectl call finishes after the flip.
  cont: { value: boolean };
  done: Promise<void>;
};

export function startPodTimeline(
  prov: MinikubeProvisioner,
  outPath: string,
  opts: { intervalMs?: number } = {},
): PodTimelinePoller {
  const intervalMs = opts.intervalMs ?? 1000;
  const cont = { value: true };
  const f = Deno.openSync(outPath, { write: true, create: true, truncate: true });
  const enc = new TextEncoder();

  const done = (async () => {
    while (cont.value) {
      const startMs = Date.now();
      try {
        // Capture name|node|phase|ready per pod. `ready` is the first
        // container's readiness gate — that's what kubelet uses for Service
        // routing decisions, and it's the truthy "this worker is actually
        // serving traffic" signal. Excluding the cgroup-still-around-but-
        // CrashLooping case that was making the old chart misleading.
        // CRITICAL: the newline delimiter MUST be the jsonpath string form
        // `{"\n"}` — not literal `\n`. Bare \n means "two-char string '\n'"
        // when emitted by kubectl, so JS split("\n") collapses everything to
        // one mangled row. AND in TypeScript source, the inside-string `\n`
        // must be written as `\\n` so the compiled runtime string preserves
        // the literal `\n` for kubectl to parse — passing a TS template with
        // raw `\n` would put a real newline INSIDE the quoted string,
        // breaking jsonpath's quoted-string parser. Use `\\n` here.
        const res = await prov.kubectl([
          "get", "pods", "-A",
          "-o", "jsonpath={range .items[*]}{.metadata.name}|{.spec.nodeName}|{.status.phase}|{.status.containerStatuses[0].ready}|{.status.containerStatuses[0].restartCount}{\"\\n\"}{end}",
        ]);
        if (res.code === 0) {
          const pods: Array<{ name: string; node: string; phase: string; ready: boolean; restarts: number }> = [];
          for (const line of res.stdout.split("\n")) {
            if (!line.trim()) continue;
            const [name, node, phase, ready, restartCount] = line.split("|");
            if (name && node) {
              pods.push({
                name,
                node,
                phase: phase ?? "",
                ready: ready === "true",
                // restartCount lets the renderer detect "pod was restarted
                // mid-bench" events (catches preemption, OOMKill, liveness
                // probe failure, helm rollouts — anything that increments
                // the kubelet's restart counter on the container).
                restarts: Number.isFinite(parseInt(restartCount)) ? parseInt(restartCount) : 0,
              });
            }
          }
          const row = JSON.stringify({ ts: startMs, pods });
          f.writeSync(enc.encode(row + "\n"));
        }
      } catch (_e) {
        // Transient kubectl errors are tolerated — bench should not die because
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
