import { file } from "bun";
import * as path from "path";
import {
  listWorktrees,
  getStatus,
  addWorktree,
  removeWorktree,
  openWorktree,
  closeWorktree,
  sendPrompt,
} from "./workmux";
import {
  attach,
  detach,
  write,
  resize,
  getScrollback,
  setCallbacks,
  clearCallbacks,
} from "./terminal";

const PORT = parseInt(process.env.DASHBOARD_PORT || "5111");
const PUBLIC_DIR = path.join(import.meta.dir, "..", "public");

function jsonResponse(data: unknown, status = 200): Response {
  return new Response(JSON.stringify(data), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}

function errorResponse(message: string, status = 500): Response {
  return jsonResponse({ error: message }, status);
}

async function serveStatic(pathname: string): Promise<Response> {
  if (pathname === "/" || pathname === "") pathname = "/index.html";

  const filePath = path.join(PUBLIC_DIR, pathname);

  if (!filePath.startsWith(PUBLIC_DIR)) {
    return new Response("Forbidden", { status: 403 });
  }

  const f = file(filePath);
  if (await f.exists()) {
    return new Response(f);
  }
  return new Response("Not Found", { status: 404 });
}

interface WsData {
  worktree: string;
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
      const upgraded = server.upgrade(req, { data: { worktree } });
      if (upgraded) return undefined as unknown as Response;
      return new Response("WebSocket upgrade failed", { status: 400 });
    }

    if (url.pathname.startsWith("/api/")) {
      return handleApi(req, url);
    }

    return serveStatic(url.pathname);
  },

  websocket: {
    async open(ws) {
      const { worktree } = ws.data;
      const cols = 120;
      const rows = 30;

      try {
        await attach(worktree, cols, rows);

        const { onData, onExit } = makeCallbacks(ws);
        setCallbacks(worktree, onData, onExit);

        const scrollback = getScrollback(worktree);
        if (scrollback) {
          ws.send(JSON.stringify({ type: "scrollback", data: scrollback }));
        }
      } catch (err: unknown) {
        const message = err instanceof Error ? err.message : String(err);
        ws.send(JSON.stringify({ type: "error", message }));
        ws.close();
      }
    },

    message(ws, message) {
      try {
        const msg = JSON.parse(typeof message === "string" ? message : new TextDecoder().decode(message));
        const { worktree } = ws.data;

        switch (msg.type) {
          case "input":
            write(worktree, msg.data);
            break;
          case "resize":
            resize(worktree, msg.cols, msg.rows);
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
      const body = await req.json() as { branch?: string; prompt?: string; autoName?: boolean };
      if (!body.branch && !body.autoName) {
        return errorResponse("branch is required (or use autoName)", 400);
      }
      const result = await addWorktree(body.branch || "", {
        prompt: body.prompt,
        autoName: body.autoName,
      });
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

console.log(`Dev Dashboard running at http://localhost:${PORT}`);
