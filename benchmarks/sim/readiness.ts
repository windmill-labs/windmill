// Pre-bench readiness check. Catches the "fire bench on broken cluster" trap
// that wasted multiple runs this session — pods Pending, samplers in
// CrashLoopBackOff, queue full of leftover jobs from a prior bench, PG not
// responding fast enough. Each of those silently produced a useless report.
//
// Usage from main.ts:
//   const r = await checkReadiness(prov);
//   if (!r.ready) {
//     console.error("[bench] cluster NOT ready:");
//     for (const issue of r.issues) console.error("  - " + issue);
//     Deno.exit(2);
//   }
//
// Optional `waitForReady` polls every 5s up to a timeout, printing transient
// issues until they clear. Use it when the cluster might be mid-reconcile
// (e.g. right after a helm upgrade).

import { MinikubeProvisioner } from "./k8s_provisioner.ts";

export type ReadinessReport = {
  ready: boolean;
  issues: string[];
  details: {
    samplers_running: number;
    samplers_total: number;
    workers_ready: number;
    workers_total: number;
    pg_phase: string;
    pg_responsive: boolean;
    queue_depth: number;
    toxiproxy_ready: boolean;
    app_ready: boolean;
  };
};

type PodSnapshot = {
  metadata?: { name?: string };
  spec?: { nodeName?: string };
  status?: {
    phase?: string;
    containerStatuses?: Array<{
      ready?: boolean;
      restartCount?: number;
      state?: { running?: unknown; waiting?: { reason?: string }; terminated?: unknown };
    }>;
  };
};

async function listPods(
  prov: MinikubeProvisioner,
  namespace: string,
  selector: string,
): Promise<PodSnapshot[]> {
  const res = await prov.kubectl([
    "-n", namespace, "get", "pods", "-l", selector, "-o", "json",
  ]);
  if (res.code !== 0) return [];
  try {
    return (JSON.parse(res.stdout) as { items?: PodSnapshot[] }).items ?? [];
  } catch {
    return [];
  }
}

function countReady(pods: PodSnapshot[]): number {
  return pods.filter((p) => p.status?.containerStatuses?.[0]?.ready === true).length;
}

function countRunning(pods: PodSnapshot[]): number {
  return pods.filter((p) => p.status?.phase === "Running").length;
}

export async function checkReadiness(
  prov: MinikubeProvisioner,
  opts: {
    expectSamplers?: number;
    expectWorkers?: number;
    namespace?: string;
    samplerNamespace?: string;
    requireEmptyQueue?: boolean;
  } = {},
): Promise<ReadinessReport> {
  const namespace = opts.namespace ?? "default";
  const samplerNs = opts.samplerNamespace ?? "kube-system";
  const expectSamplers = opts.expectSamplers ?? 4;
  const requireEmptyQueue = opts.requireEmptyQueue ?? true;

  const issues: string[] = [];

  // --- Samplers ---
  // Not just "Running" — must have been Running long enough for kubelet's
  // log buffer to have stable data covering the start of the bench. A
  // sampler that was just-deployed 5s ago is technically Running, but its
  // stdout buffer hasn't reached the kubelet log file yet, so a kubectl-
  // logs --since-time at bench-end gets nothing for the first ~30s of the
  // bench window. Require startTime to be at least MIN_STABLE_S in the past.
  const MIN_STABLE_S = 30;
  const samplerPods = await listPods(prov, samplerNs, "app=wm-sim-cpu-sampler");
  const samplersRunning = countRunning(samplerPods);
  if (samplersRunning < expectSamplers) {
    const bad = samplerPods
      .filter((p) => p.status?.phase !== "Running")
      .map((p) => {
        const reason = (p.status as { containerStatuses?: Array<{ state?: { waiting?: { reason?: string } } }> } | undefined)
          ?.containerStatuses?.[0]?.state?.waiting?.reason;
        return `${p.metadata?.name}${reason ? `(${reason})` : ""}`;
      })
      .join(", ");
    issues.push(
      `samplers ${samplersRunning}/${expectSamplers} Running — not Running: ${bad || "(none listed)"}`,
    );
  } else {
    const nowMs = Date.now();
    const tooYoung = samplerPods.filter((p) => {
      const startStr = (p.status as { startTime?: string } | undefined)?.startTime;
      if (!startStr) return true;
      const ageS = (nowMs - Date.parse(startStr)) / 1000;
      return ageS < MIN_STABLE_S;
    });
    if (tooYoung.length > 0) {
      const tags = tooYoung.map((p) => {
        const startStr = (p.status as { startTime?: string } | undefined)?.startTime;
        const ageS = startStr ? Math.floor((nowMs - Date.parse(startStr)) / 1000) : -1;
        return `${p.metadata?.name}(age=${ageS}s)`;
      }).join(", ");
      issues.push(
        `samplers Running but not yet stable (need ≥ ${MIN_STABLE_S}s uptime): ${tags}`,
      );
    }
  }

  // --- Workers ---
  const workerPods = await listPods(prov, namespace, "app=windmill-workers");
  const workersReady = countReady(workerPods);
  const workersTotal = workerPods.length;
  if (opts.expectWorkers !== undefined) {
    if (workersReady < opts.expectWorkers) {
      issues.push(`workers ${workersReady}/${opts.expectWorkers} ready (${workersTotal} pods total)`);
    }
  } else if (workersReady < workersTotal) {
    issues.push(`workers ${workersReady}/${workersTotal} ready`);
  }

  // --- Worker Deployments: rollout must be fully complete ---
  // Pod-level readiness above passes during rolling-updates (200 ready while
  // 50 old pods still being torn down). That worker churn floods the kernel's
  // cgroup_mutex and starves the CPU sampler — m04 had a 96s data gap last
  // bench because of exactly this. Bench MUST wait until each deployment's
  // status reflects: observedGeneration == metadata.generation AND
  // updatedReplicas == spec.replicas AND availableReplicas == spec.replicas
  // AND no leftover replicas from the old ReplicaSet.
  const deplRes = await prov.kubectl([
    "-n", namespace, "get", "deploy", "-l", "app=windmill-workers", "-o", "json",
  ]);
  if (deplRes.code === 0) {
    try {
      const list = JSON.parse(deplRes.stdout) as {
        items?: Array<{
          metadata?: { name?: string; generation?: number };
          spec?: { replicas?: number };
          status?: {
            observedGeneration?: number;
            updatedReplicas?: number;
            availableReplicas?: number;
            replicas?: number;
          };
        }>;
      };
      for (const d of list.items ?? []) {
        const name = d.metadata?.name ?? "?";
        const gen = d.metadata?.generation ?? 0;
        const obsGen = d.status?.observedGeneration ?? -1;
        const desired = d.spec?.replicas ?? 0;
        const updated = d.status?.updatedReplicas ?? 0;
        const avail = d.status?.availableReplicas ?? 0;
        const total = d.status?.replicas ?? 0;
        if (obsGen < gen) {
          issues.push(`deploy/${name} controller behind: observedGen=${obsGen} < gen=${gen}`);
          continue;
        }
        if (updated < desired) {
          issues.push(`deploy/${name} rollout incomplete: updated=${updated}/${desired}`);
        }
        if (avail < desired) {
          issues.push(`deploy/${name} rollout incomplete: available=${avail}/${desired}`);
        }
        if (total > desired) {
          issues.push(`deploy/${name} old replicas not yet terminated: total=${total} > desired=${desired}`);
        }
      }
    } catch (e) {
      issues.push(`worker deploy parse failed: ${(e as Error).message}`);
    }
  } else {
    issues.push(`worker deploy lookup failed (code ${deplRes.code})`);
  }

  // --- PG pod + responsiveness ---
  const pgPods = await listPods(prov, namespace, "app=windmill-postgresql-demo-app");
  const pgPhase = pgPods[0]?.status?.phase ?? "absent";
  if (pgPhase !== "Running") {
    issues.push(`PG phase=${pgPhase}`);
  }
  let pgResponsive = false;
  let queueDepth = -1;
  if (pgPods.length > 0 && pgPhase === "Running") {
    // Force a fast statement_timeout so a wedged PG doesn't hang the check.
    const r = await prov.kubectl([
      "-n", namespace, "exec", pgPods[0].metadata!.name!, "--",
      "psql", "-U", "postgres", "-d", "windmill", "-tAc",
      "SET statement_timeout=3000; SELECT count(*) FROM v2_job_queue;",
    ]);
    if (r.code === 0) {
      const last = r.stdout.trim().split("\n").pop() ?? "";
      const n = parseInt(last);
      if (Number.isFinite(n)) {
        pgResponsive = true;
        queueDepth = n;
        if (requireEmptyQueue && n > 0) {
          issues.push(`queue has ${n} leftover job(s) — drain or DELETE FROM v2_job_queue before benching`);
        }
      } else {
        issues.push(`PG returned non-numeric queue count: ${last.slice(0, 60)}`);
      }
    } else {
      issues.push(`PG psql failed (statement_timeout=3s): ${(r.stderr || r.stdout).slice(0, 100)}`);
    }
  }

  // --- toxiproxy + app ---
  const toxPods = await listPods(prov, namespace, "app=toxiproxy");
  const toxReady = countReady(toxPods) > 0;
  if (!toxReady) issues.push(`toxiproxy not Ready (${toxPods.length} pod(s))`);

  const appPods = await listPods(prov, namespace, "app=windmill-app");
  const appReady = countReady(appPods) > 0;
  if (!appReady) issues.push(`windmill-app not Ready (${appPods.length} pod(s))`);

  return {
    ready: issues.length === 0,
    issues,
    details: {
      samplers_running: samplersRunning,
      samplers_total: samplerPods.length,
      workers_ready: workersReady,
      workers_total: workersTotal,
      pg_phase: pgPhase,
      pg_responsive: pgResponsive,
      queue_depth: queueDepth,
      toxiproxy_ready: toxReady,
      app_ready: appReady,
    },
  };
}

// Poll checkReadiness every N seconds until ready or timeout. Transient
// issues that clear on a subsequent tick aren't treated as failures.
export async function waitForReady(
  prov: MinikubeProvisioner,
  opts: {
    timeoutMs?: number;
    pollMs?: number;
    expectSamplers?: number;
    expectWorkers?: number;
    requireEmptyQueue?: boolean;
  } = {},
): Promise<ReadinessReport> {
  const timeoutMs = opts.timeoutMs ?? 180_000;
  const pollMs = opts.pollMs ?? 5_000;
  const start = Date.now();
  let last: ReadinessReport | undefined;
  while (true) {
    last = await checkReadiness(prov, {
      expectSamplers: opts.expectSamplers,
      expectWorkers: opts.expectWorkers,
      requireEmptyQueue: opts.requireEmptyQueue,
    });
    if (last.ready) return last;
    if (Date.now() - start >= timeoutMs) return last;
    console.log(`[readiness] not ready (${last.issues.length} issue(s)) — polling again in ${pollMs / 1000}s`);
    for (const issue of last.issues) console.log(`  - ${issue}`);
    await new Promise((r) => setTimeout(r, pollMs));
  }
}
