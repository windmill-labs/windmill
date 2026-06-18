/**
 * Port collision detection + fallback for `wmill dev` and `wmill app dev`.
 *
 * Why this exists: Node's default listen() has platform-dependent dual-stack
 * behaviour. If the requested IPv4 binding (0.0.0.0:N) is already taken by
 * another process, Node may silently fall back to IPv6-only ([::1]:N). The OS
 * then routes new `localhost` connections to the older IPv4 listener, so the
 * user opens http://localhost:N and sees the wrong server with no signal that
 * anything is wrong. Bit us in practice: a leftover `wmill dev --proxy-port 4000`
 * served traffic for a freshly-started `wmill app dev --port 4000`.
 *
 * The fix: probe both stacks before binding. Treat the port as taken if either
 * 0.0.0.0 or :: refuses the bind. On collision, walk upward to the next free
 * port and log the shift prominently.
 */

import { createServer } from "node:net";
import { execSync } from "node:child_process";

type Host = "0.0.0.0" | "::";

/**
 * Try to bind a fresh server to (port, host) and immediately close it.
 *
 * Returns false ONLY when the port is genuinely held by another process
 * (EADDRINUSE) or denied by permissions (EACCES). Other errors — most
 * importantly EAFNOSUPPORT / EADDRNOTAVAIL on the IPv6 probe when the host
 * has no IPv6 stack at all — return true: the stack we're probing simply
 * isn't reachable, which is functionally indistinguishable from "free" for
 * the dual-stack collision check.
 */
function isPortFree(port: number, host: Host): Promise<boolean> {
  return new Promise((resolve) => {
    const s = createServer();
    s.once("error", (err: NodeJS.ErrnoException) => {
      const code = err.code ?? "";
      // Anything that means "another process is holding this port" → not free.
      // Anything else (no IPv6 stack on this host, etc.) → treat as free so we
      // don't false-alarm on IPv4-only containers.
      resolve(code !== "EADDRINUSE" && code !== "EACCES");
    });
    s.once("listening", () => s.close(() => resolve(true)));
    s.listen(port, host);
  });
}

/**
 * A port counts as free only if BOTH IPv4 and IPv6 stacks accept the bind.
 * If either is held by another process, the OS may route `localhost` traffic
 * to that other process even when our listener succeeds on the free stack.
 *
 * Probes sequentially, not in parallel: on Linux the default is
 * `net.ipv6.bindv6only=0`, which makes a `bind(::, port)` socket also occupy
 * the IPv4 stack on the same port. Running both probes concurrently then
 * causes one to lose the race with EADDRINUSE on a port that is actually
 * free, producing false negatives. Sequential keeps each probe's bind fully
 * released before the next starts.
 */
async function isPortFreeOnBothStacks(port: number): Promise<boolean> {
  if (!(await isPortFree(port, "0.0.0.0"))) return false;
  if (!(await isPortFree(port, "::"))) return false;
  return true;
}

/**
 * Best-effort lookup of the PID + command currently bound to <port>. Returns
 * undefined if nothing is found, the lookup fails, or the platform tooling
 * isn't installed. Never throws.
 */
function findPortHolder(port: number): { pid: number; command: string } | undefined {
  // macOS + Linux: lsof. -nP avoids DNS / port-name lookups, -sTCP:LISTEN
  // narrows to the listening socket.
  try {
    const out = execSync(`lsof -nP -iTCP:${port} -sTCP:LISTEN -F pc 2>/dev/null`, {
      encoding: "utf-8",
      stdio: ["ignore", "pipe", "ignore"],
    });
    // -F pc emits records like:
    //   p91418
    //   cbun
    let pid: number | undefined;
    let cmd: string | undefined;
    for (const line of out.split("\n")) {
      if (line.startsWith("p")) pid = parseInt(line.slice(1), 10);
      else if (line.startsWith("c")) cmd = line.slice(1);
      if (pid && cmd) return { pid, command: cmd };
    }
  } catch {
    // lsof missing or no holder — fall through.
  }

  // Linux fallback: ss. -ltnp lists listening TCP sockets with PID/command.
  try {
    const out = execSync(`ss -ltnp 2>/dev/null | awk '$4 ~ /:${port}$/ { print $NF }'`, {
      encoding: "utf-8",
      stdio: ["ignore", "pipe", "ignore"],
    }).trim();
    // Format: users:(("bun",pid=91418,fd=23))
    const m = out.match(/\("([^"]+)",pid=(\d+)/);
    if (m) return { pid: parseInt(m[2], 10), command: m[1] };
  } catch {
    /* fall through */
  }

  return undefined;
}

/**
 * Resolve the port we should actually bind to.
 *
 * Walks upward from `requested` until a port is free on both stacks, capped
 * at +20 to avoid silently scanning the whole 4xxx range. On shift, logs a
 * prominent warning naming the holder if we can find it.
 *
 * Returns the chosen port (== requested when it was already free).
 */
export async function resolveBindPort(
  requested: number,
  flagLabel: string,
  log: { info: (msg: string) => void; warn: (msg: string) => void },
): Promise<number> {
  const MAX_SHIFT = 20;
  for (let port = requested; port < requested + MAX_SHIFT; port++) {
    if (await isPortFreeOnBothStacks(port)) {
      if (port !== requested) {
        const holder = findPortHolder(requested);
        const holderHint = holder
          ? ` (held by PID ${holder.pid} \`${holder.command}\`)`
          : "";
        log.warn(
          `Port ${requested} is already in use${holderHint}. Using port ${port} instead.`,
        );
        log.info(
          `If you need port ${requested} stable (e.g. a launch.json entry pinned to it), stop the holder and re-run with ${flagLabel} ${requested}.`,
        );
      }
      return port;
    }
  }
  throw new Error(
    `Could not find a free port in the range ${requested}-${requested + MAX_SHIFT - 1}. Stop a holder or pass ${flagLabel} <other>.`,
  );
}

/**
 * The host string we bind to. Explicit IPv4 — `localhost` resolves to
 * 127.0.0.1 first on every platform we care about, and binding both stacks
 * relies on platform-specific IPV6_V6ONLY behaviour we don't want to debug.
 */
export const BIND_HOST = "0.0.0.0" as const;
