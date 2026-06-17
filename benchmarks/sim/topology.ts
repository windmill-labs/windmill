// Topology definition: PG + N Windmill nodes, each with configurable
// CPU/memory and optional per-node DB latency. Loaded from a standalone
// JSON file so suites can reference topologies by path.
//
// Every node runs exactly ONE Windmill worker. This is a hard rule. To scale
// up, spawn more nodes; do NOT stack workers in one process.
//
// A node runs in one of two modes, configurable per node:
//
//   - "native"           1 native worker connecting to PG directly via SQL.
//                        Maps to Windmill's MODE=worker by default — just a
//                        worker, no API server, no host port.
//                        Set `standalone: true` to colocate an API server in
//                        the same process (Windmill's MODE=standalone). That
//                        node gets a host port and becomes the user-facing
//                        entrypoint. Docker-based topologies need exactly one
//                        such node.
//
//   - "k8s"              A k3d-managed cluster. Helm decides server placement
//                        and replica count. Helm chart is selected via CLI
//                        flags.
//
// Windmill's MODE=agent (HTTP-only worker, no PG access) is a separate shape
// we don't model yet — deferred.

export type NativeMode = {
  mode: "native";
  // Defaults to false → MODE=worker (worker-only, no API, no host port,
  // connects to PG via SQL). When true → MODE=standalone (server + worker
  // in one process, exposed on a host port). Exactly one node per
  // docker-based topology must have this true.
  standalone?: boolean;
  // Only meaningful with `standalone: true`. When true, the node runs
  // MODE=server (API only, NO embedded worker) instead of MODE=standalone.
  // Use this when separate worker nodes do all the work and you don't want the
  // server's own worker competing for jobs (a standalone server's worker
  // ignores WORKER_TAGS restrictions and listens on the full default tag set).
  server_only?: boolean;
  tag?: string;
  // Caps the PG pool for this node via the DATABASE_CONNECTIONS env var.
  // Default Windmill behaviour is 5 per worker / 50 for a server; at high
  // worker counts that exhausts PG's max_connections, so large topologies
  // set this to 2-3 per worker.
  db_connections?: number;
};

// k8s mode is provisioned by the k3d provisioner (M5). The k3d cluster runs
// inside docker containers; the actual Windmill workload is deployed onto it
// via helm. `pods` and `workers_per_pod` are convenience shortcuts that the
// k3d provisioner maps onto helm values.
//
// Helm chart selection (upstream vs local, repo URL, version, value overrides)
// is *not* in the topology — it lives on the CLI (--helm-chart, --helm-repo,
// --helm-set, --helm-values-file). Topology describes the cluster shape;
// CLI describes what to deploy onto it.
export type K8sMode = {
  mode: "k8s";
  cluster_agents?: number;  // k3d cluster agent count (k3s nodes); defaults to 1.
                            // This shapes the underlying cluster, NOT the
                            // workload. The Windmill deployment scale (worker
                            // replica count, HPA, etc.) is owned by helm —
                            // pass it via --helm-set on the sim CLI.
  tag?: string;
};

// Helm config supplied at the CLI level for k8s-mode topologies.
export type HelmConfig = {
  chart?: string;          // local path (relative to cwd) OR chart name like "windmill/windmill"
  repo?: string;           // helm repo URL, used when chart is a name (not a path)
  version?: string;        // pin a specific chart version
  values?: Record<string, unknown>; // inline overrides from --helm-set
  values_files?: string[]; // paths from --helm-values-file
};

export type NodeMode = NativeMode | K8sMode;

export type NodeSpec = {
  id: string; // unique identifier, used for container naming and metrics keys
  cpu: number; // --cpus value (count of CPUs, not a fraction)
  memory: string; // --memory value (e.g. "4G", "512M")
  db_latency_ms?: number; // injected via toxiproxy; 0 or omitted means no proxy
} & NodeMode;

// Worker count per "node" in the topology. Hard rule for native/agent:
// NUM_WORKERS=1 per process. For k8s the worker count is whatever helm
// decides (configurable via --helm-set), so we don't know it at the topology
// layer — return null to signal "ask helm".
export function workerCount(n: NodeSpec): number | null {
  if (n.mode === "k8s") return null;
  return 1;
}

export type PostgresSpec = {
  cpu: number;
  memory: string;
  // PG max_connections. Default PG ships 100; large worker counts need more
  // (sum of per-node pools). When set, also bump `memory` since each backend
  // reserves a few MB.
  max_connections?: number;
  // shared_buffers etc. could go here later; defaults are fine for v1
};

export type ReservedHostResources = {
  cpus: number; // cores left untouched for harness + toxiproxy + host OS
  memory: string;
};

export type Topology = {
  name: string;
  pin_cpus?: boolean; // when true, provisioner assigns non-overlapping cpusets
  host_resources?: {
    cpus: number;
    memory: string;
    reserved?: ReservedHostResources;
  };
  postgres: PostgresSpec;
  nodes: NodeSpec[];
  // Note: container runtime (docker vs podman) is selected at runtime via the
  // SIM_CONTAINER_CMD env var, not declared in the topology. Topology is
  // workload-shape, not infrastructure-runtime.
};

// Parse a memory string like "4G" / "512M" / "1024K" / "1048576" into bytes.
// Docker accepts the same shorthand; we just need it for validation arithmetic.
export function parseMemory(value: string): number {
  const m = value.trim().match(/^(\d+(?:\.\d+)?)\s*([KMGT]?)([Bi]*)?$/i);
  if (!m) throw new Error(`Invalid memory value: ${value}`);
  const n = parseFloat(m[1]);
  const unit = m[2].toUpperCase();
  const mult: Record<string, number> = { "": 1, K: 1024, M: 1024 ** 2, G: 1024 ** 3, T: 1024 ** 4 };
  if (!(unit in mult)) throw new Error(`Invalid memory unit in ${value}`);
  return n * mult[unit];
}

export function formatBytes(bytes: number): string {
  const units = ["B", "K", "M", "G", "T"];
  let i = 0;
  let n = bytes;
  while (n >= 1024 && i < units.length - 1) { n /= 1024; i++; }
  return `${n.toFixed(2)}${units[i]}`;
}

export type ValidationIssue = { severity: "error" | "warn"; message: string };

// Detect the number of CPUs available to the harness process. On Linux,
// navigator.hardwareConcurrency reports logical cores. This is the same view
// the host kernel exposes; it does not respect cgroup limits, which is fine
// because the harness itself should not be containerised.
export function hostCores(): number {
  return navigator.hardwareConcurrency;
}

// Read total host memory from /proc/meminfo. Falls back to 0 if unavailable
// (e.g. on macOS dev machines); the validator treats 0 as "unknown" and only
// warns rather than blocks.
export async function hostMemoryBytes(): Promise<number> {
  try {
    const text = await Deno.readTextFile("/proc/meminfo");
    const m = text.match(/^MemTotal:\s+(\d+)\s+kB/m);
    if (!m) return 0;
    return parseInt(m[1], 10) * 1024;
  } catch (_) {
    return 0;
  }
}

// Sum requested CPU and memory across PG + all nodes, compare to host minus
// reserved buffer. Fail closed: any overcommit returns an error so the sim
// refuses to run rather than silently degrade benchmark accuracy.
export async function validateTopology(t: Topology): Promise<ValidationIssue[]> {
  const issues: ValidationIssue[] = [];

  const reserved = t.host_resources?.reserved ?? { cpus: 2, memory: "2G" };
  const declaredCpus = t.host_resources?.cpus ?? hostCores();
  const declaredMem = t.host_resources?.memory
    ? parseMemory(t.host_resources.memory)
    : await hostMemoryBytes();

  const totalCpu = t.postgres.cpu + t.nodes.reduce((s, n) => s + n.cpu, 0);
  const totalMem = parseMemory(t.postgres.memory) +
    t.nodes.reduce((s, n) => s + parseMemory(n.memory), 0);

  const cpuBudget = declaredCpus - reserved.cpus;
  if (totalCpu > cpuBudget) {
    issues.push({
      severity: "error",
      message: `CPU oversubscription: topology requests ${totalCpu} cores, ` +
        `host has ${declaredCpus} cores with ${reserved.cpus} reserved ` +
        `(budget ${cpuBudget}). Reduce node.cpu values or run on a larger host.`,
    });
  }

  if (declaredMem > 0) {
    const memBudget = declaredMem - parseMemory(reserved.memory);
    if (totalMem > memBudget) {
      issues.push({
        severity: "error",
        message: `Memory oversubscription: topology requests ${formatBytes(totalMem)}, ` +
          `host has ${formatBytes(declaredMem)} with ${reserved.memory} reserved ` +
          `(budget ${formatBytes(memBudget)}).`,
      });
    }
  } else {
    issues.push({
      severity: "warn",
      message: "Could not read host memory; skipping memory budget check. " +
        "Set host_resources.memory in the topology to enforce it.",
    });
  }

  const ids = new Set<string>();
  let nativeCount = 0;
  let standaloneCount = 0; // native nodes with standalone: true
  let k8sCount = 0;
  for (const n of t.nodes) {
    if (ids.has(n.id)) {
      issues.push({ severity: "error", message: `Duplicate node id: ${n.id}` });
    }
    ids.add(n.id);
    if (n.cpu <= 0) {
      issues.push({ severity: "error", message: `Node ${n.id} has non-positive cpu` });
    }
    // Use a typed-as-unknown probe because TS narrows away nodes that don't
    // match any union branch, leaving `never` and breaking property access.
    const probe = n as { id: string; mode?: string };
    if (!probe.mode) {
      issues.push({
        severity: "error",
        message: `Node ${probe.id} is missing a "mode" field (must be one of "native", "k8s")`,
      });
      continue;
    }
    if (probe.mode === "native") {
      nativeCount++;
      if ((n as NativeMode).standalone === true) standaloneCount++;
    } else if (probe.mode === "k8s") {
      k8sCount++;
    } else if (probe.mode === "agent") {
      issues.push({
        severity: "error",
        message: `Node ${probe.id}: mode "agent" is not supported yet. ` +
          `Use mode "native" (the default is worker-only); set ` +
          `\`standalone: true\` on the one node that should run an API server.`,
      });
    } else {
      issues.push({
        severity: "error",
        message: `Node ${probe.id} has unknown mode "${probe.mode}"`,
      });
    }

    // Reject the now-removed knobs explicitly so older configs fail loudly
    // instead of silently shipping a multi-worker setup the sim isn't built
    // to measure.
    const raw = n as unknown as Record<string, unknown>;
    if ("num_workers" in raw) {
      issues.push({
        severity: "error",
        message: `Node ${probe.id}: num_workers is no longer configurable. Every node runs exactly one worker.`,
      });
    }
    if (n.mode === "k8s") {
      if ("pods" in raw) {
        issues.push({
          severity: "error",
          message: `Node ${probe.id}: pods is no longer configurable in the topology. Helm owns workload scale — pass via --helm-set worker.replicaCount=N.`,
        });
      }
      if ("workers_per_pod" in raw) {
        issues.push({
          severity: "error",
          message: `Node ${probe.id}: workers_per_pod is no longer configurable. Helm owns workload scale.`,
        });
      }
    }
  }

  // Docker-based topologies must have exactly one standalone node — that's
  // the user-facing API entrypoint. Zero means there's no API; more than one
  // means the harness has no clear push target. k8s topologies are exempt
  // (helm provides its own service).
  if (k8sCount === 0) {
    if (standaloneCount === 0) {
      issues.push({
        severity: "error",
        message: "Topology has no `standalone: true` node. Exactly one " +
          "native node must set `standalone: true` to host the API server.",
      });
    } else if (standaloneCount > 1) {
      issues.push({
        severity: "error",
        message: `Topology has ${standaloneCount} nodes with \`standalone: true\`. ` +
          `Only one node should host the API server.`,
      });
    }
  }

  if (k8sCount > 0 && nativeCount > 0) {
    issues.push({
      severity: "error",
      message: "Mixed k8s and non-k8s nodes in one topology is not supported. " +
        "All nodes must share a provisioner (docker or k3d).",
    });
  }

  if (t.pin_cpus && totalCpu + reserved.cpus > declaredCpus) {
    issues.push({
      severity: "error",
      message: "pin_cpus requires sum(cpu) + reserved.cpus <= host cores so " +
        "every container gets a disjoint cpuset.",
    });
  }

  return issues;
}

export async function loadTopology(path: string): Promise<Topology> {
  const text = await Deno.readTextFile(path);
  const t = JSON.parse(text) as Topology;
  if (!t.name || !t.nodes || !t.postgres) {
    throw new Error(`Topology at ${path} is missing required fields (name, nodes, postgres)`);
  }
  return t;
}
