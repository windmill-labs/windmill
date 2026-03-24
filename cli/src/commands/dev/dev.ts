import { Command } from "@cliffy/command";
import * as log from "../../core/log.ts";
import { sep as SEP } from "node:path";
import { yamlParseFile } from "../../utils/yaml.ts";
import { WebSocket, WebSocketServer } from "ws";

import * as getPort from "get-port";
import * as http from "node:http";
import * as https from "node:https";
import * as open from "open";
import { readFile, realpath } from "node:fs/promises";
import { watch } from "node:fs";
import { getTypeStrFromPath, GlobalOptions } from "../../types.ts";
import { ignoreF } from "../sync/sync.ts";
import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace } from "../../core/context.ts";
import {
  SyncOptions,
  mergeConfigWithConfigFile,
} from "../../core/conf.ts";
import { exts, removeExtensionToPath } from "../script/script.ts";
import { inferContentTypeFromFilePath } from "../../utils/script_common.ts";
import { OpenFlow } from "../../../gen/types.gen.ts";
import { FlowFile } from "../flow/flow.ts";
import { replaceInlineScripts, replaceAllPathScriptsWithLocal } from "../../../windmill-utils-internal/src/inline-scripts/replacer.ts";
import { parseMetadataFile } from "../../utils/metadata.ts";
import {
  getFolderSuffixWithSep,
  getMetadataFileName,
  extractFolderPath,
} from "../../utils/resource_folders.ts";
import { listSyncCodebases } from "../../utils/codebase.ts";
import { createPreviewLocalScriptReader } from "../../utils/local_path_scripts.ts";

const PORT = 3001;
const PROXY_PORT = 3100;

async function dev(opts: GlobalOptions & SyncOptions & { proxyPort?: number }) {
  opts = await mergeConfigWithConfigFile(opts);
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  log.info("Started dev mode");
  let currentLastEdit: LastEditScript | LastEditFlow | undefined = undefined;

  const fsWatcher = watch(".", { recursive: true });
  const base = await realpath(".");
  const ignore = await ignoreF(opts);
  const codebases = await listSyncCodebases(opts);

  const changesTimeouts: Record<string, ReturnType<typeof setTimeout>> = {};
  function watchChanges() {
    return new Promise<void>((_resolve, _reject) => {
      fsWatcher.on("change", (_eventType, filename) => {
        if (!filename) return;
        const filePath = typeof filename === "string" ? filename : filename.toString();
        const key = filePath;
        if (changesTimeouts[key]) {
          clearTimeout(changesTimeouts[key]);
        }
        changesTimeouts[key] = setTimeout(async () => {
          delete changesTimeouts[key];
          await loadPaths([filePath]).catch((error) => {
            log.error(
              `Failed to reload ${filePath}: ${error instanceof Error ? error.message : error}`
            );
          });
        }, 100);
      });
      fsWatcher.on("error", (err) => {
        _reject(err);
      });
    });
  }

  const flowFolderSuffix = getFolderSuffixWithSep("flow");
  const flowMetadataFile = getMetadataFileName("flow", "yaml");
  async function loadPaths(pathsToLoad: string[]) {
    const paths = pathsToLoad.filter((path) =>
      exts.some(
        (ext) => path.endsWith(ext) || path.endsWith(flowFolderSuffix + flowMetadataFile)
      )
    );
    if (paths.length == 0) {
      return;
    }
    const nativePath = (await realpath(paths[0])).replace(base + SEP, "");
    const cpath = nativePath.replaceAll("\\", "/");
    if (!ignore(nativePath, false)) {
      const typ = getTypeStrFromPath(cpath);
      log.info("Detected change in " + cpath + " (" + typ + ")");
      if (typ == "flow") {
        const localPath = extractFolderPath(cpath, "flow")!;
        const localFlow = (await yamlParseFile(
          localPath + "flow.yaml"
        )) as FlowFile;
        await replaceInlineScripts(
          localFlow.value.modules,
          async (path: string) => await readFile(localPath + path, "utf-8"),
          log,
          localPath,
          SEP,
          undefined,
        );
        // Replace PathScript modules with local file content so dev mode uses local versions
        const localScriptReader = createPreviewLocalScriptReader({
          exts,
          defaultTs: opts.defaultTs,
          codebases,
        });
        await replaceAllPathScriptsWithLocal(localFlow.value, localScriptReader, log);
        const wmFlowPath = localPath.replace(/\.flow\/$/, "").replace(/\/$/, "");
        currentLastEdit = {
          type: "flow",
          flow: localFlow,
          uriPath: localPath,
          path: wmFlowPath,
        };
        log.info("Updated " + wmFlowPath);
        broadcastChanges(currentLastEdit);
      } else if (typ == "script") {
        const content = await readFile(cpath, "utf-8");
        const splitted = cpath.split(".");
        const wmPath = splitted[0];
        const lang = inferContentTypeFromFilePath(cpath, opts.defaultTs);
        const typed =
          (await parseMetadataFile(
            removeExtensionToPath(cpath),
            undefined,
          )
          )?.payload


        currentLastEdit = {
          type: "script",
          content,
          path: wmPath,
          language: lang,
          tag: typed?.tag,
          lock: typed?.lock,
        };
        log.info("Updated " + wmPath);
        broadcastChanges(currentLastEdit);
      }
    }
  }
  type LastEditScript = {
    type: "script";
    content: string;
    path: string;
    language: string;
    tag?: string;
    lock?: string;

  };

  type LastEditFlow = {
    type: "flow";
    flow: OpenFlow;
    uriPath: string;
    path: string;
  };

  // Map each connected client to its optional watchPath filter
  const clientWatchPaths: Map<WebSocket, string | undefined> = new Map();

  function getEditPath(lastEdit: LastEditScript | LastEditFlow): string {
    return lastEdit.path;
  }

  function normalizePath(p: string): string {
    return p.replace(/\.flow\/?$/, "").replace(/\/$/, "");
  }

  // Send file changes to clients, filtered by their watchPath
  function broadcastChanges(lastEdit: LastEditScript | LastEditFlow) {
    const editPath = normalizePath(getEditPath(lastEdit));
    const msg = JSON.stringify(lastEdit);
    for (const [client, watchPath] of clientWatchPaths.entries()) {
      if (watchPath === undefined || normalizePath(watchPath) === editPath) {
        client.send(msg);
      }
    }
  }

  function setupDevWs(wss: WebSocketServer) {
    wss.on("connection", (ws: WebSocket) => {
      clientWatchPaths.set(ws, undefined);
      console.log("New dev client connected");

      ws.on("open", () => {
        if (currentLastEdit) {
          // Send the current state to the new client
          const watchPath = clientWatchPaths.get(ws);
          if (watchPath === undefined || normalizePath(watchPath) === normalizePath(getEditPath(currentLastEdit))) {
            ws.send(JSON.stringify(currentLastEdit));
          }
        }
      });

      ws.on("close", () => {
        clientWatchPaths.delete(ws);
        console.log("Dev client disconnected");
      });

      ws.on("message", (message: WebSocket.RawData) => {
        let data;
        try {
          data = JSON.parse(message.toString());
        } catch (e) {
          console.log("Received invalid JSON: " + message + " " + e);
          return;
        }

        if (data.type === "load") {
          loadPaths([data.path]);
        } else if (data.type === "setWatch") {
          const path = data.path as string;
          clientWatchPaths.set(ws, path);
          console.log(`Client watching: ${path}`);
          ws.send(JSON.stringify({ type: "watchSet", path }));
          // Send current state for the watched path if available
          if (currentLastEdit && normalizePath(getEditPath(currentLastEdit)) === normalizePath(path)) {
            ws.send(JSON.stringify(currentLastEdit));
          }
        }
      });
    });
  }

  async function startLegacyServer(): Promise<number> {
    const server = http.createServer((_req, res) => {
      res.writeHead(200);
      res.end();
    });
    const wss = new WebSocketServer({ server });
    setupDevWs(wss);

    const port = await getPort.default({ port: PORT });
    return new Promise((resolve) => {
      server.listen(port, () => {
        console.log(`Legacy dev server listening on port ${port}`);
        resolve(port);
      });
    });
  }

  async function startProxyServer(remoteUrl: string, proxyPort: number, wsPort: number) {
    const remote = new URL(remoteUrl);
    const isHttps = remote.protocol === "https:";
    const remoteHost = remote.hostname;
    const remotePort = remote.port ? parseInt(remote.port) : (isHttps ? 443 : 80);
    const httpModule = isHttps ? https : http;

    // Dev WebSocket server (handles /ws_dev path)
    const devWss = new WebSocketServer({ noServer: true });
    setupDevWs(devWss);

    // Separate WSS for proxied WebSocket connections (not tracked as dev clients)
    const proxyWss = new WebSocketServer({ noServer: true });

    const proxyServer = http.createServer((clientReq, clientRes) => {
      const proxyOpts: http.RequestOptions = {
        hostname: remoteHost,
        port: remotePort,
        path: clientReq.url,
        method: clientReq.method,
        headers: {
          ...clientReq.headers,
          host: remote.host,
        },
      };

      const proxyReq = httpModule.request(proxyOpts, (proxyRes) => {
        // Rewrite Set-Cookie domain to localhost
        const setCookie = proxyRes.headers["set-cookie"];
        if (setCookie) {
          proxyRes.headers["set-cookie"] = setCookie.map((cookie) =>
            cookie.replace(/domain=[^;]+/gi, "domain=localhost")
          );
        }
        clientRes.writeHead(proxyRes.statusCode ?? 502, proxyRes.headers);
        proxyRes.pipe(clientRes, { end: true });
      });

      proxyReq.on("error", (err) => {
        console.error("Proxy error:", err.message);
        clientRes.writeHead(502);
        clientRes.end("Bad Gateway");
      });

      clientReq.pipe(proxyReq, { end: true });
    });

    // Handle WebSocket upgrades
    proxyServer.on("upgrade", (req, socket, head) => {
      const pathname = req.url?.split("?")[0] ?? "";

      if (pathname === "/ws_dev") {
        // Handle locally: dev file-change WebSocket
        devWss.handleUpgrade(req, socket, head, (ws) => {
          devWss.emit("connection", ws, req);
        });
        return;
      }

      // Proxy all other WebSocket paths to remote
      if (pathname.startsWith("/ws/") || pathname.startsWith("/ws_mp/") || pathname.startsWith("/ws_debug/")) {
        const wsProtocol = isHttps ? "wss" : "ws";
        const remoteWsUrl = `${wsProtocol}://${remote.host}${req.url}`;
        const remoteWs = new WebSocket(remoteWsUrl, {
          headers: {
            ...req.headers,
            host: remote.host,
          },
        });

        remoteWs.on("open", () => {
          proxyWss.handleUpgrade(req, socket, head, (clientWs) => {
            clientWs.on("message", (data) => {
              if (remoteWs.readyState === WebSocket.OPEN) {
                remoteWs.send(data);
              }
            });
            remoteWs.on("message", (data) => {
              if (clientWs.readyState === WebSocket.OPEN) {
                clientWs.send(data);
              }
            });
            clientWs.on("close", () => remoteWs.close());
            remoteWs.on("close", () => clientWs.close());
          });
        });

        remoteWs.on("error", (err) => {
          console.error("WebSocket proxy error:", err.message);
          socket.destroy();
        });
        return;
      }

      // Unknown WS path — destroy
      socket.destroy();
    });

    return new Promise<void>((resolve) => {
      proxyServer.listen(proxyPort, () => {
        console.log(`Dev proxy listening on http://localhost:${proxyPort}`);
        resolve();
      });
    });
  }

  async function startApp() {
    const wsPort = await startLegacyServer();

    const proxyPort = opts.proxyPort ?? PROXY_PORT;
    await startProxyServer(workspace.remote, proxyPort, wsPort);

    const legacyUrl =
      `${workspace.remote}dev?workspace=${workspace.workspaceId}&local=true&wm_token=${workspace.token}` +
      (wsPort === PORT ? "" : `&port=${wsPort}`);

    const proxyUrl =
      `http://localhost:${proxyPort}/dev?workspace=${workspace.workspaceId}&local=true&wm_token=${workspace.token}`;

    console.log(`\nLegacy dev URL: ${legacyUrl}`);
    console.log(`Proxy dev URL:  ${proxyUrl}`);
    console.log(`\nTo watch a specific file: ${proxyUrl}&path=<wm_path>`);

    try {
      open.openApp(open.apps.browser, { arguments: [legacyUrl] }).catch((error) => {
        console.error(
          `Failed to open browser, please navigate to ${legacyUrl}, error: ${error}`
        );
      });
      console.log("Opened browser for you");
    } catch (error) {
      console.error(
        `Failed to open browser, please navigate to ${legacyUrl}, ${error}`
      );
    }

    console.log(
      "Dev server will automatically point to the last script edited locally"
    );
  }

  await Promise.all([startApp(), watchChanges()]);
  console.log("Stopped dev mode");
}

const command = new Command()
  .description("Launch a dev server that will spawn a webserver with HMR")
  .option(
    "--includes <pattern...:string>",
    "Filter paths givena glob pattern or path"
  )
  .option(
    "--proxy-port <port:number>",
    "Port for the localhost reverse proxy (default: 3100)"
  )
  .action(dev as any);

export default command;
