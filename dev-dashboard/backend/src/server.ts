import {
  listWorktrees,
  getStatus,
  addWorktree,
  removeWorktree,
  openWorktree,
  closeWorktree,
  sendPrompt,
  readEnvLocal,
  type Profile,
  type Agent,
} from "./workmux";
import { reconcileForwarding, stopAll } from "./socat";
import {
  attach,
  detach,
  write,
  resize,
  getScrollback,
  setCallbacks,
  clearCallbacks,
  cleanupStaleSessions,
} from "./terminal";

const PORT = parseInt(process.env.DASHBOARD_PORT || "5111");

/** Map branch name → worktree directory using git worktree list. */
function getWorktreePaths(): Map<string, string> {
  const result = Bun.spawnSync(["git", "worktree", "list", "--porcelain"], { stdout: "pipe" });
  const output = new TextDecoder().decode(result.stdout);
  const paths = new Map<string, string>();
  let currentPath = "";
  for (const line of output.split("\n")) {
    if (line.startsWith("worktree ")) {
      currentPath = line.slice("worktree ".length);
    } else if (line.startsWith("branch ")) {
      // branch refs/heads/foo → "foo"
      const branch = line.slice("branch ".length).replace("refs/heads/", "");
      // Also map by directory basename (workmux uses basename as branch key)
      const basename = currentPath.split("/").pop() ?? "";
      paths.set(branch, currentPath);
      if (basename !== branch) paths.set(basename, currentPath);
    }
  }
  return paths;
}

/** Check if a port has a service responding (not just a TCP handshake). */
function isPortListening(port: number): Promise<boolean> {
  return new Promise((resolve) => {
    const timeout = setTimeout(() => { resolve(false); }, 1000);
    fetch(`http://127.0.0.1:${port}/`, { signal: AbortSignal.timeout(1000) })
      .then((res) => { clearTimeout(timeout); resolve(true); })
      .catch(() => { clearTimeout(timeout); resolve(false); });
  });
}

function jsonResponse(data: unknown, status = 200): Response {
  return new Response(JSON.stringify(data), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}

function errorResponse(message: string, status = 500): Response {
  return jsonResponse({ error: message }, status);
}

interface WsData {
  worktree: string;
  attached: boolean;
}

function makeCallbacks(ws: { send: (data: string) => void; readyState: number }) {
  return {
    onData: (data: string) => {
      if (ws.readyState <= 1) {
        ws.send(JSON.stringify({ type: "output", data }));
      }
    },
    onExit: (exitCode: number) => {
      if (ws.readyState <= 1) {
        ws.send(JSON.stringify({ type: "exit", exitCode }));
      }
    },
  };
}

Bun.serve<WsData>({
  port: PORT,

  async fetch(req, server) {
    const url = new URL(req.url);

    const wsMatch = url.pathname.match(/^\/ws\/(.+)$/);
    if (wsMatch) {
      const worktree = decodeURIComponent(wsMatch[1]);
      const upgraded = server.upgrade(req, { data: { worktree, attached: false } });
      if (upgraded) return undefined as unknown as Response;
      return new Response("WebSocket upgrade failed", { status: 400 });
    }

    if (url.pathname.startsWith("/api/")) {
      return handleApi(req, url);
    }

    return new Response("Not Found", { status: 404 });
  },

  websocket: {
    open(_ws) {
      // Wait for the client to send its actual dimensions before spawning
    },

    async message(ws, message) {
      try {
        const msg = JSON.parse(typeof message === "string" ? message : new TextDecoder().decode(message));
        const { worktree } = ws.data;

        switch (msg.type) {
          case "input":
            write(worktree, msg.data);
            break;
          case "resize":
            if (!ws.data.attached) {
              // First resize = client reporting actual dimensions. Spawn now.
              ws.data.attached = true;
              try {
                await attach(worktree, msg.cols, msg.rows);
                const { onData, onExit } = makeCallbacks(ws);
                setCallbacks(worktree, onData, onExit);
                const scrollback = getScrollback(worktree);
                if (scrollback) {
                  ws.send(JSON.stringify({ type: "scrollback", data: scrollback }));
                }
              } catch (err: unknown) {
                const errMsg = err instanceof Error ? err.message : String(err);
                ws.send(JSON.stringify({ type: "error", message: errMsg }));
                ws.close();
              }
            } else {
              resize(worktree, msg.cols, msg.rows);
            }
            break;
        }
      } catch {
        // Ignore malformed messages
      }
    },

    async close(ws) {
      clearCallbacks(ws.data.worktree);
      await detach(ws.data.worktree);
    },
  },
});

async function handleApi(req: Request, url: URL): Promise<Response> {
  const method = req.method;
  const parts = url.pathname.slice(5).split("/").filter(Boolean);

  try {
    // GET /api/worktrees
    if (parts[0] === "worktrees" && parts.length === 1 && method === "GET") {
      const [worktrees, status] = await Promise.all([listWorktrees(), getStatus()]);
      const wtPaths = getWorktreePaths();
      const merged = await Promise.all(worktrees.map(async (wt) => {
        const st = status.find(s =>
          s.worktree.includes(wt.branch) || s.worktree.startsWith(wt.branch)
        );
        const wtDir = wtPaths.get(wt.branch);
        const env = wtDir ? readEnvLocal(wtDir) : {};
        const backendPort = env.BACKEND_PORT ? parseInt(env.BACKEND_PORT) : null;
        const frontendPort = env.FRONTEND_PORT ? parseInt(env.FRONTEND_PORT) : null;
        const [backendRunning, frontendRunning] = await Promise.all([
          backendPort ? isPortListening(backendPort) : false,
          frontendPort ? isPortListening(frontendPort) : false,
        ]);
        return {
          ...wt,
          dir: wtDir ?? null,
          status: st?.status ?? "",
          elapsed: st?.elapsed ?? "",
          title: st?.title ?? "",
          profile: env.PROFILE || null,
          agentName: env.AGENT || null,
          backendPort,
          frontendPort,
          backendRunning,
          frontendRunning,
        };
      }));
      return jsonResponse(merged);
    }

    // POST /api/worktrees
    if (parts[0] === "worktrees" && parts.length === 1 && method === "POST") {
      const body = await req.json() as { branch?: string; prompt?: string; profile?: string; agent?: string };
      if (!body.branch) {
        return errorResponse("branch is required", 400);
      }
      const validProfiles = ["full", "agent-yolo"] as const;
      const validAgents = ["claude", "codex"] as const;
      const profile = validProfiles.includes(body.profile as any) ? body.profile as Profile : "full";
      const agent = validAgents.includes(body.agent as any) ? body.agent as Agent : "claude";
      console.log(`[worktree:add] branch=${body.branch} agent=${agent} profile=${profile}${body.prompt ? ` prompt="${body.prompt.slice(0, 80)}"` : ""}`);
      const result = await addWorktree(body.branch, { prompt: body.prompt, profile, agent });
      console.log(`[worktree:add] done branch=${body.branch}: ${result}`);
      return jsonResponse({ message: result }, 201);
    }

    // DELETE /api/worktrees/:name
    if (parts[0] === "worktrees" && parts.length === 2 && method === "DELETE") {
      const name = decodeURIComponent(parts[1]);
      console.log(`[worktree:rm] name=${name}`);
      const result = await removeWorktree(name);
      console.log(`[worktree:rm] done name=${name}: ${result}`);
      return jsonResponse({ message: result });
    }

    // POST /api/worktrees/:name/open
    if (parts[0] === "worktrees" && parts.length === 3 && parts[2] === "open" && method === "POST") {
      const name = decodeURIComponent(parts[1]);
      console.log(`[worktree:open] name=${name}`);
      return jsonResponse({ message: await openWorktree(name) });
    }

    // POST /api/worktrees/:name/close
    if (parts[0] === "worktrees" && parts.length === 3 && parts[2] === "close" && method === "POST") {
      const name = decodeURIComponent(parts[1]);
      console.log(`[worktree:close] name=${name}`);
      return jsonResponse({ message: await closeWorktree(name) });
    }

    // POST /api/worktrees/:name/send
    if (parts[0] === "worktrees" && parts.length === 3 && parts[2] === "send" && method === "POST") {
      const name = decodeURIComponent(parts[1]);
      const body = await req.json() as { prompt?: string };
      if (!body.prompt) {
        return errorResponse("prompt is required", 400);
      }
      console.log(`[worktree:send] name=${name} prompt="${body.prompt.slice(0, 80)}"`);
      return jsonResponse({ message: await sendPrompt(name, body.prompt) });
    }

    // GET /api/worktrees/:name/status
    if (parts[0] === "worktrees" && parts.length === 3 && parts[2] === "status" && method === "GET") {
      const name = decodeURIComponent(parts[1]);
      const status = await getStatus();
      const match = status.find(s => s.worktree.includes(name));
      return jsonResponse(match ?? { status: "unknown" });
    }

    return errorResponse("Not Found", 404);
  } catch (err: unknown) {
    const message = err instanceof Error ? err.message : String(err);
    console.error(`[api:error] ${method} ${url.pathname}: ${message}`);
    return errorResponse(message);
  }
}

// Ensure tmux server is running (needs at least one session to persist)
const tmuxCheck = Bun.spawnSync(["tmux", "list-sessions"], { stdout: "pipe", stderr: "pipe" });
if (tmuxCheck.exitCode !== 0) {
  Bun.spawnSync(["tmux", "new-session", "-d", "-s", "0"]);
  console.log("Started tmux session");
}

cleanupStaleSessions();

// Re-establish socat forwarding for any sandbox containers still running
const wtPathsForReconcile = getWorktreePaths();
reconcileForwarding((branch) => wtPathsForReconcile.get(branch));

// Clean shutdown: kill socat processes
process.on("SIGINT", () => { stopAll(); process.exit(0); });
process.on("SIGTERM", () => { stopAll(); process.exit(0); });

console.log(`Dev Dashboard API running at http://localhost:${PORT}`);
