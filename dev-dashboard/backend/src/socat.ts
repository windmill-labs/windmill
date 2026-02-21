/**
 * Manages socat port forwarding for sandbox containers.
 *
 * When a worktree runs inside a Docker sandbox, its ports are only reachable
 * via the container's bridge IP. socat forwards host ports to the container
 * so the browser (over SSH) can reach them.
 */

import { $ } from "bun";
import { readEnvLocal } from "./env";

interface ForwardingEntry {
  branch: string;
  containerIp: string;
  ports: { host: number; proc: ReturnType<typeof Bun.spawn> }[];
}

const registry = new Map<string, ForwardingEntry>();

/** Get the bridge IP of a running sandbox container for a worktree branch. */
async function getContainerIp(branch: string): Promise<string | null> {
  try {
    // Container names follow the pattern wm-{branch}-*
    const ps = await $`docker ps --filter name=wm-${branch}- --format {{.ID}}`.text();
    const containerId = ps.trim().split("\n")[0];
    if (!containerId) return null;
    const ip = (await $`docker inspect ${containerId} --format {{.NetworkSettings.IPAddress}}`.text()).trim();
    return ip || null;
  } catch {
    return null;
  }
}

/** Start socat forwarding for a sandbox worktree. Returns true if forwarding was started. */
export async function startForwarding(branch: string, wtDir: string): Promise<boolean> {
  // Don't double-start
  if (registry.has(branch)) return true;

  const containerIp = await getContainerIp(branch);
  if (!containerIp) {
    return false;
  }

  const env = readEnvLocal(wtDir);
  const backendPort = env.BACKEND_PORT ? parseInt(env.BACKEND_PORT) : null;
  const frontendPort = env.FRONTEND_PORT ? parseInt(env.FRONTEND_PORT) : null;

  const entry: ForwardingEntry = { branch, containerIp, ports: [] };

  for (const port of [backendPort, frontendPort]) {
    if (!port) continue;
    const proc = Bun.spawn([
      "socat",
      `TCP-LISTEN:${port},fork,reuseaddr`,
      `TCP:${containerIp}:${port}`,
    ], { stdout: "ignore", stderr: "pipe" });
    // Consume the exit promise so Bun reaps the child (prevents zombies)
    proc.exited.then(() => {});
    entry.ports.push({ host: port, proc });
    console.log(`[socat] forwarding :${port} → ${containerIp}:${port} (branch=${branch}, pid=${proc.pid})`);
  }

  if (entry.ports.length > 0) {
    registry.set(branch, entry);
  }
  return true;
}

/** Stop socat forwarding for a worktree. */
export function stopForwarding(branch: string): void {
  const entry = registry.get(branch);
  if (!entry) return;

  for (const { host, proc } of entry.ports) {
    try {
      proc.kill();
      console.log(`[socat] stopped :${host} (branch=${branch}, pid=${proc.pid})`);
    } catch {
      // Already exited
    }
  }
  registry.delete(branch);
}

/**
 * Reconcile socat forwarding on startup.
 * Kills any orphaned socat processes from a previous run, then starts
 * forwarding for any running sandbox containers.
 */
export async function reconcileForwarding(getWorktreeDir: (branch: string) => string | undefined): Promise<void> {
  try {
    // Kill orphaned socat processes from previous dashboard runs
    try {
      await $`pkill -f ${"socat TCP-LISTEN.*TCP:172\\."}`.quiet();
      console.log("[socat] reconcile: killed orphaned socat processes");
    } catch {
      // No orphans found (pkill exits non-zero when no match)
    }

    const ps = await $`docker ps --filter name=wm- --format {{.Names}}`.text();
    const names = ps.trim().split("\n").filter(Boolean);

    for (const name of names) {
      // Container name format: wm-{branch}-{pid}
      const match = name.match(/^wm-(.+)-\d+$/);
      if (!match) continue;
      const branch = match[1];

      if (registry.has(branch)) continue;

      const wtDir = getWorktreeDir(branch);
      if (!wtDir) {
        console.log(`[socat] reconcile: no worktree dir found for ${branch}, skipping`);
        continue;
      }

      console.log(`[socat] reconcile: starting forwarding for ${branch}`);
      await startForwarding(branch, wtDir);
    }
  } catch (err) {
    console.error(`[socat] reconcile failed:`, err);
  }
}

/** Stop all forwarding (for clean shutdown). */
export function stopAll(): void {
  for (const branch of [...registry.keys()]) {
    stopForwarding(branch);
  }
}
