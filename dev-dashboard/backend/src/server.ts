import {
  listWorktrees,
  getStatus,
  addWorktree,
  removeWorktree,
  openWorktree,
  closeWorktree,
  sendPrompt,
  type Profile,
} from "./workmux";
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
      const merged = worktrees.map(wt => {
        const st = status.find(s =>
          s.worktree.includes(wt.branch) || s.worktree.startsWith(wt.branch)
        );
        return { ...wt, status: st?.status ?? "", elapsed: st?.elapsed ?? "", title: st?.title ?? "" };
      });
      return jsonResponse(merged);
    }

    // POST /api/worktrees
    if (parts[0] === "worktrees" && parts.length === 1 && method === "POST") {
      const body = await req.json() as { branch?: string; prompt?: string; profile?: string };
      if (!body.branch) {
        return errorResponse("branch is required", 400);
      }
      const validProfiles = ["full", "agent-only", "agent-yolo"] as const;
      const profile = validProfiles.includes(body.profile as any) ? body.profile as Profile : "agent-only";
      const result = await addWorktree(body.branch, { prompt: body.prompt, profile });
      return jsonResponse({ message: result }, 201);
    }

    // DELETE /api/worktrees/:name
    if (parts[0] === "worktrees" && parts.length === 2 && method === "DELETE") {
      const name = decodeURIComponent(parts[1]);
      return jsonResponse({ message: await removeWorktree(name) });
    }

    // POST /api/worktrees/:name/open
    if (parts[0] === "worktrees" && parts.length === 3 && parts[2] === "open" && method === "POST") {
      const name = decodeURIComponent(parts[1]);
      return jsonResponse({ message: await openWorktree(name) });
    }

    // POST /api/worktrees/:name/close
    if (parts[0] === "worktrees" && parts.length === 3 && parts[2] === "close" && method === "POST") {
      const name = decodeURIComponent(parts[1]);
      return jsonResponse({ message: await closeWorktree(name) });
    }

    // POST /api/worktrees/:name/send
    if (parts[0] === "worktrees" && parts.length === 3 && parts[2] === "send" && method === "POST") {
      const name = decodeURIComponent(parts[1]);
      const body = await req.json() as { prompt?: string };
      if (!body.prompt) {
        return errorResponse("prompt is required", 400);
      }
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
console.log(`Dev Dashboard API running at http://localhost:${PORT}`);
