// `wmill pipeline dev [folder]` — live-preview a data pipeline from local files.
//
// The pipeline analog of `wmill dev` / `wmill app dev`: watch a folder of
// `// pipeline` scripts, rebuild the asset graph from the working tree on every
// save, and push it over a WebSocket to the `/pipeline_dev` page, which renders
// the same graph editor the UI uses and runs the cascade via preview (no deploy).
// Editing stays in the user's own editor; the page live-reloads.

import { Command } from "@cliffy/command";
import { colors } from "@cliffy/ansi/colors";
import * as http from "node:http";
import * as fs from "node:fs";
import * as path from "node:path";
import { createHash, randomBytes } from "node:crypto";
import process from "node:process";
import * as open from "open";
import { WebSocket, WebSocketServer } from "ws";
import * as log from "../../core/log.ts";
import { GlobalOptions } from "../../types.ts";
import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace } from "../../core/context.ts";
import {
  mergeConfigWithConfigFile,
  type SyncOptions,
} from "../../core/conf.ts";
import { listSyncCodebases } from "../../utils/codebase.ts";
import { resolveBindPort } from "../../utils/port-probe.ts";
import { getConfigDirPath } from "../../../windmill-utils-internal/src/config/config.ts";
import { buildLocalPipelineGraph, workspaceRoot } from "./localGraph.ts";

const PORT = 3201;
// Bind loopback only: each WS frame ships the folder's full script source
// (`scripts[].content` + `temp_script_refs`) with no auth, so it must not be
// reachable from the LAN. The webview connects via `ws://localhost`, and an SSH
// `-L` / devbox port-forward targets 127.0.0.1 on the host, so both still work.
const LISTEN_HOST = "127.0.0.1";

interface PipelineDevOpts extends GlobalOptions, SyncOptions {
  port?: number;
  open?: boolean;
  defaultTs?: "bun" | "deno";
  frontend?: string;
}

// The WS token gates access to local source. Persist it under the user-private
// config dir (0600) so a `pipeline dev` restart reuses it — an already-open
// `/pipeline_dev` page auto-reconnecting with the token from its URL then
// survives the restart, instead of every upgrade being rejected by
// `verifyClient` until the freshly printed URL is reopened. Scoped by
// remote+workspace+root+folder+port (NOT port alone): a same-session restart
// reconnects, but any *different* session — another folder, workspace, remote, or
// local checkout on the same port — gets a different token, so a stale tab can't
// reconnect and receive another session's source. Falls back to an ephemeral
// token if the config dir can't be read/written.
async function stableWsToken(
  remote: string,
  workspaceId: string,
  root: string,
  folder: string,
  port: number,
): Promise<string> {
  // Hash the tuple (NUL-delimited so no value can spoof the boundary) → a
  // collision-resistant, filesystem-safe key. A plain sanitized join would let
  // different values collide (`a/b` and `a_b` → same name), which would reuse a
  // token across sessions and reintroduce the cross-session leak.
  const key = createHash("sha256")
    .update(`${remote}\0${workspaceId}\0${root}\0${folder}\0${port}`)
    .digest("hex")
    .slice(0, 32);
  let tokenFile: string | undefined;
  try {
    tokenFile = path.join(await getConfigDirPath(), `pipeline-dev-${key}.token`);
    const existing = fs.readFileSync(tokenFile, "utf-8").trim();
    if (existing) return existing;
  } catch {
    // no reusable token file yet (or config dir unavailable) — mint a fresh one
  }
  const token = randomBytes(24).toString("base64url");
  if (tokenFile) {
    try {
      fs.writeFileSync(tokenFile, token, { mode: 0o600 });
    } catch {
      // best-effort persistence; fall back to the in-memory token
    }
  }
  return token;
}

// Resolve the target folder: explicit arg, else auto-detect when cwd sits inside
// `f/<folder>/…` (the supported `cd f/my_pipeline && wmill pipeline dev` form).
function resolveFolder(root: string, folderArg?: string): string | undefined {
  if (folderArg) return folderArg.replace(/^f\//, "").replace(/\/$/, "");
  const rel = path.relative(root, process.cwd()).replaceAll("\\", "/");
  if (rel === "" || rel.startsWith("..")) return undefined;
  const segs = rel.split("/");
  if (segs[0] === "f" && segs[1]) return segs[1];
  return undefined;
}

async function dev(opts: PipelineDevOpts, folderArg?: string) {
  const root = workspaceRoot();
  const folder = resolveFolder(root, folderArg);
  if (!folder) {
    log.error(
      colors.red(
        "Could not determine the pipeline folder. Pass it explicitly " +
          "(`wmill pipeline dev <folder>`) or run from inside an `f/<folder>` directory.",
      ),
    );
    process.exit(1);
  }
  const folderDir = path.join(root, "f", folder);
  if (!fs.existsSync(folderDir)) {
    log.error(colors.red(`Folder not found on disk: ${folderDir}`));
    process.exit(1);
  }

  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);
  const merged = await mergeConfigWithConfigFile(opts);
  const codebases = await listSyncCodebases(merged);

  // Resolve relative imports from local (not-yet-deployed) content for previews.
  // Snapshot at startup like `wmill dev`; restart to refresh. Degrades to
  // undefined on older backends.
  let tempScriptRefs: Record<string, string> | undefined;
  try {
    const { buildPreviewTempScriptRefs } = await import(
      "../generate-metadata/generate-metadata.ts"
    );
    tempScriptRefs = await buildPreviewTempScriptRefs(
      workspace,
      merged,
      codebases,
      { kind: "all" },
    );
  } catch {
    // best-effort
  }

  const EMPTY_GRAPH = { runnables: [], assets: [], edges: [], triggers: [] };
  async function buildBundle() {
    const { graph, scripts } = await buildLocalPipelineGraph({
      root,
      folder: folder!,
      defaultTs: merged.defaultTs,
    });
    return {
      type: "pipeline" as const,
      folder,
      graph,
      scripts,
      temp_script_refs: tempScriptRefs,
    };
  }

  // Don't let a transient build error (e.g. a half-written file) abort startup —
  // serve an empty graph and recover on the next save.
  let current: Awaited<ReturnType<typeof buildBundle>>;
  try {
    current = await buildBundle();
  } catch (e: any) {
    log.error(colors.red(`Initial graph build failed: ${e.message}`));
    current = { type: "pipeline", folder, graph: EMPTY_GRAPH, scripts: [], temp_script_refs: tempScriptRefs };
  }
  log.info(
    colors.blue(
      `Watching f/${folder} — ${current.scripts.length} pipeline script(s)`,
    ),
  );

  const clients = new Set<WebSocket>();
  function broadcast() {
    const msg = JSON.stringify(current);
    for (const ws of clients) {
      if (ws.readyState === WebSocket.OPEN) ws.send(msg);
    }
  }

  // Debounced rebuild on any change under the folder.
  let timer: ReturnType<typeof setTimeout> | undefined;
  const watcher = fs.watch(folderDir, { recursive: true });
  watcher.on("change", (_ev, filename) => {
    if (filename && filename.toString().endsWith(".lock")) return;
    if (timer) clearTimeout(timer);
    timer = setTimeout(async () => {
      timer = undefined;
      try {
        current = await buildBundle();
        log.info(colors.cyan(`↻ rebuilt graph (${current.scripts.length} scripts)`));
        broadcast();
      } catch (e: any) {
        log.error(colors.red(`Failed to rebuild pipeline graph: ${e.message}`));
      }
    }, 150);
  });
  watcher.on("error", (e) => log.error(colors.red(`Watcher error: ${e.message}`)));

  const port = await resolveBindPort(opts.port ?? PORT, "wmill pipeline dev", {
    info: (m) => log.info(m),
    warn: (m) => log.warn(m),
  });

  const server = http.createServer((_req, res) => {
    res.writeHead(200);
    res.end();
  });
  // Loopback bind keeps the LAN out, but any browser tab can still open a
  // `ws://localhost:<port>` connection — and each frame ships the folder's full
  // script source. Gate the upgrade on an unguessable per-session token (carried
  // in the dev-page URL) so a stray page on the predictable dev port can't
  // exfiltrate the source. base64url → safe as a query value.
  // Stable across restarts (scoped to remote+workspace+root+folder+port) so an
  // already-open page reconnects after a CLI restart (see stableWsToken), not
  // just after a transient WS drop.
  const wsToken = await stableWsToken(
    workspace.remote,
    workspace.workspaceId,
    root,
    folder,
    port,
  );
  const wss = new WebSocketServer({
    server,
    verifyClient: (info) => {
      try {
        const u = new URL(info.req.url ?? "", "http://localhost");
        return u.searchParams.get("token") === wsToken;
      } catch {
        return false;
      }
    },
  });
  wss.on("connection", (ws: WebSocket) => {
    clients.add(ws);
    // Push the current bundle immediately so the page renders without waiting
    // for the first file change.
    try {
      ws.send(JSON.stringify(current));
    } catch {
      // ignore
    }
    ws.on("close", () => clients.delete(ws));
    ws.on("error", () => clients.delete(ws));
  });

  // The `/pipeline_dev` page is served by the frontend, not the backend. By
  // default we open it on the workspace remote, but that 404s on a remote whose
  // deployed frontend predates this route — `--frontend` points the page at a
  // locally-run frontend (`REMOTE=<remote> npm run dev`) while the API/token
  // still target the remote. Normalize to a single trailing slash either way.
  const pageBase = (opts.frontend ?? workspace.remote).replace(/\/?$/, "/");
  const url =
    `${pageBase}pipeline_dev?workspace=${workspace.workspaceId}` +
    `&wm_token=${workspace.token}&folder=${encodeURIComponent(folder)}&port=${port}` +
    `&ws_token=${encodeURIComponent(wsToken)}`;

  server.listen(port, LISTEN_HOST, () => {
    log.info(colors.green.bold(`🚀 Pipeline dev server on ws://localhost:${port}/ws`));
    log.info(colors.gray(`Open: ${url}`));
    if (opts.open !== false) {
      open.default(url).catch((e: any) =>
        log.warn(colors.yellow(`Failed to open browser: ${e.message}`)),
      );
    }
  });

  process.on("SIGINT", () => {
    log.info(colors.yellow("\n🛑 Shutting down…"));
    watcher.close();
    for (const ws of clients) ws.close();
    server.close();
    process.exit(0);
  });
}

const command = new Command()
  .description(
    "Live-preview a data pipeline from local files: watch an `f/<folder>` of `// pipeline` scripts, push the working-tree graph to the dev page, and run the cascade via preview (no deploy).",
  )
  .arguments("[folder:string]")
  .option("--port <port:number>", "Port for the dev WebSocket server.")
  .option("--no-open", "Do not open the browser automatically.")
  .option(
    "--frontend <origin:string>",
    "Origin serving the /pipeline_dev page (e.g. http://localhost:3000 for a locally-run frontend). Defaults to the workspace remote; use it when the remote's deployed frontend predates the dev page.",
  )
  .action(dev as any);

export default command;
