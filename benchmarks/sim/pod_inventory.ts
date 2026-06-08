// Snapshot of pods alive on the cluster at teardown — written as JSON so the
// report renderer can map pod UIDs (the only thing the cgroup-based CPU
// sampler can extract) to human-meaningful names + nodes + the worker-group
// label.
//
// Shape (one entry per pod):
//   { uid, name, namespace, node, labels: {...} }

import type { MinikubeProvisioner } from "./k8s_provisioner.ts";

export type PodEntry = {
  uid: string;
  name: string;
  namespace: string;
  node: string;
  labels: Record<string, string>;
  // Captured so the renderer can show "what died during the bench" without
  // relying on the event-based oom_events.json (which misses containerd's
  // mislabeled "Error" exit 137 kills, dmesg-source flakiness, etc.).
  containers?: Array<{
    name: string;
    restartCount: number;
    lastState?: {
      terminated?: {
        reason?: string;
        exitCode?: number;
        finishedAt?: string;
      };
    };
  }>;
};

export async function capturePodInventory(
  prov: MinikubeProvisioner,
  outPath: string,
): Promise<PodEntry[]> {
  const res = await prov.kubectl([
    "get", "pods", "-A",
    "-o", "json",
  ]);
  if (res.code !== 0) {
    console.warn(`[inventory] kubectl get pods failed`);
    return [];
  }
  let parsed: { items?: Array<Record<string, unknown>> };
  try {
    parsed = JSON.parse(res.stdout);
  } catch (e) {
    console.warn(`[inventory] JSON parse failed: ${(e as Error).message}`);
    return [];
  }
  const entries: PodEntry[] = (parsed.items ?? []).map((p) => {
    const meta = (p.metadata as Record<string, unknown> | undefined) ?? {};
    const spec = (p.spec as Record<string, unknown> | undefined) ?? {};
    const status = (p.status as Record<string, unknown> | undefined) ?? {};
    const rawCs = (status.containerStatuses as Array<Record<string, unknown>> | undefined) ?? [];
    const containers = rawCs.map((cs) => {
      const ls = (cs.lastState as Record<string, unknown> | undefined) ?? {};
      const term = (ls.terminated as Record<string, unknown> | undefined);
      return {
        name: String(cs.name ?? ""),
        restartCount: Number(cs.restartCount ?? 0),
        lastState: term
          ? {
            terminated: {
              reason: term.reason as string | undefined,
              exitCode: term.exitCode as number | undefined,
              finishedAt: term.finishedAt as string | undefined,
            },
          }
          : undefined,
      };
    });
    return {
      uid: String(meta.uid ?? ""),
      name: String(meta.name ?? ""),
      namespace: String(meta.namespace ?? ""),
      node: String(spec.nodeName ?? ""),
      labels: (meta.labels as Record<string, string> | undefined) ?? {},
      containers,
    };
  });
  await Deno.writeTextFile(outPath, JSON.stringify(entries, null, 2));
  console.log(`[inventory] ${entries.length} pods captured -> ${outPath}`);
  return entries;
}
