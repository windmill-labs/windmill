import { Command } from "@cliffy/command";
import * as log from "../../core/log.ts";
import { sep as SEP } from "node:path";
import { yamlParseFile } from "../../utils/yaml.ts";
import { stringify as yamlStringify } from "yaml";
import { WebSocket, WebSocketServer } from "ws";

import * as getPort from "get-port";
import * as http from "node:http";
import * as https from "node:https";
import * as open from "open";
import { access, readFile, readdir, realpath, unlink, writeFile } from "node:fs/promises";
import { watch } from "node:fs";
import { getTypeStrFromPath, GlobalOptions } from "../../types.ts";
import { ignoreF } from "../sync/sync.ts";
import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace } from "../../core/context.ts";
import {
  GLOBAL_CONFIG_OPT,
  SyncOptions,
  mergeConfigWithConfigFile,
} from "../../core/conf.ts";
import { exts, removeExtensionToPath } from "../script/script.ts";
import { inferContentTypeFromFilePath } from "../../utils/script_common.ts";
import { OpenFlow } from "../../../gen/types.gen.ts";
import { FlowFile } from "../flow/flow.ts";
import { replaceInlineScripts, replaceAllPathScriptsWithLocal } from "../../../windmill-utils-internal/src/inline-scripts/replacer.ts";
import { extractInlineScripts, extractCurrentMapping } from "../../../windmill-utils-internal/src/inline-scripts/extractor.ts";
import { parseMetadataFile } from "../../utils/metadata.ts";
import {
  getFolderSuffix,
  getFolderSuffixWithSep,
  getMetadataFileName,
  extractFolderPath,
  getNonDottedPaths,
  hasFolderSuffix,
  loadNonDottedPathsSetting,
} from "../../utils/resource_folders.ts";
import * as path from "node:path";
import * as fs from "node:fs";
import { listSyncCodebases } from "../../utils/codebase.ts";
import { createPreviewLocalScriptReader } from "../../utils/local_path_scripts.ts";

const PORT = 3001;

// PathScript snapshot/restore for flow round-trip
const TAG_KEY = "_originalPathScript" as const;

function walkFlowModules(modules: any[], visitor: { onModule: (m: any) => void }) {
  for (const module of modules) {
    if (!module.value) continue;
    const val = module.value;
    if (val.type === "forloopflow" || val.type === "whileloopflow") {
      walkFlowModules(val.modules, visitor);
    } else if (val.type === "branchall") {
      for (const branch of val.branches ?? []) {
        walkFlowModules(branch.modules, visitor);
      }
    } else if (val.type === "branchone") {
      for (const branch of val.branches ?? []) {
        walkFlowModules(branch.modules, visitor);
      }
      if (val.default) {
        walkFlowModules(val.default, visitor);
      }
    } else {
      visitor.onModule(module);
    }
  }
}

function walkFlow(flowValue: any, visitor: { onModule: (m: any) => void }) {
  if (flowValue?.modules) walkFlowModules(flowValue.modules, visitor);
  if (flowValue?.failure_module) walkFlowModules([flowValue.failure_module], visitor);
  if (flowValue?.preprocessor_module) walkFlowModules([flowValue.preprocessor_module], visitor);
}

function snapshotPathScripts(flowValue: any) {
  walkFlow(flowValue, {
    onModule(module) {
      if (module.value.type === "script") {
        module[TAG_KEY] = JSON.parse(JSON.stringify(module.value));
      }
    },
  });
}

function tagReplacedPathScripts(flowValue: any) {
  walkFlow(flowValue, {
    onModule(module) {
      if (module[TAG_KEY] && module.value.type === "rawscript") {
        module.value[TAG_KEY] = module[TAG_KEY];
        delete module[TAG_KEY];
      } else if (module[TAG_KEY]) {
        delete module[TAG_KEY];
      }
    },
  });
}

function restorePathScripts(flowValue: any) {
  walkFlow(flowValue, {
    onModule(module) {
      if (module.value[TAG_KEY]) {
        module.value = module.value[TAG_KEY];
      }
    },
  });
}

export interface DevOpts {
  proxyPort?: number;
  path?: string;
}

export async function dev(opts: GlobalOptions & SyncOptions & DevOpts) {
  // Auto-detect flow folder: if no --path and cwd is a flow folder, resolve path and chdir to workspace root
  if (!opts.path) {
    const cwd = process.cwd();
    const cwdBasename = path.basename(cwd);

    // Need to init nonDottedPaths before checking suffix
    await loadNonDottedPathsSetting();

    if (hasFolderSuffix(cwdBasename, "flow")) {
      GLOBAL_CONFIG_OPT.noCdToRoot = true;

      // Find workspace root
      let searchDir = cwd;
      let workspaceRoot: string | undefined;
      while (true) {
        const wmillYaml = path.join(searchDir, "wmill.yaml");
        if (fs.existsSync(wmillYaml)) {
          workspaceRoot = searchDir;
          break;
        }
        const parentDir = path.dirname(searchDir);
        if (parentDir === searchDir) break;
        searchDir = parentDir;
      }

      if (workspaceRoot) {
        const relPath = path.relative(workspaceRoot, cwd).replaceAll("\\", "/");
        const flowSuffix = getFolderSuffix("flow");
        opts.path = relPath.endsWith(flowSuffix)
          ? relPath.slice(0, -flowSuffix.length)
          : relPath;
        opts.proxyPort = opts.proxyPort ?? 3100;
        log.info(`Detected flow folder, path: ${opts.path}`);
        process.chdir(workspaceRoot);
      }
    }
  }

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
        // Snapshot PathScript modules before replacement, then tag after
        snapshotPathScripts(localFlow);
        const localScriptReader = createPreviewLocalScriptReader({
          exts,
          defaultTs: opts.defaultTs,
          codebases,
        });
        await replaceAllPathScriptsWithLocal(localFlow.value, localScriptReader, log);
        tagReplacedPathScripts(localFlow);
        const flowSuffix = getFolderSuffix("flow");
        const wmFlowPath = localPath.endsWith(flowSuffix + "/")
          ? localPath.slice(0, -(flowSuffix.length + 1))
          : localPath.replace(/\/$/, "");
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

  // Normalize a windmill path by stripping any trailing flow/app suffix
  function normalizeWmPath(p: string): string {
    const flowSuffix = getFolderSuffix("flow");
    let result = p.replace(/\/$/, "");
    if (result.endsWith(flowSuffix)) {
      result = result.slice(0, -flowSuffix.length);
    }
    return result;
  }

  // Load a resource by its windmill path (e.g., "u/admin/my_script" or "f/my_flow")
  async function loadWmPath(wmPath: string): Promise<LastEditScript | LastEditFlow | undefined> {
    wmPath = normalizeWmPath(wmPath);
    // Try as flow
    const flowDir = wmPath + getFolderSuffix("flow") + "/";
    const flowYaml = flowDir + "flow.yaml";
    try {
      await access(flowYaml);
      const localFlow = (await yamlParseFile(flowYaml)) as FlowFile;
      await replaceInlineScripts(
        localFlow.value.modules,
        async (p: string) => await readFile(flowDir + p, "utf-8"),
        log,
        flowDir,
        SEP,
        undefined,
      );
      snapshotPathScripts(localFlow);
      const localScriptReader = createPreviewLocalScriptReader({
        exts,
        defaultTs: opts.defaultTs,
        codebases,
      });
      await replaceAllPathScriptsWithLocal(localFlow.value, localScriptReader, log);
      tagReplacedPathScripts(localFlow);
      const edit: LastEditFlow = {
        type: "flow",
        flow: localFlow,
        uriPath: flowDir,
        path: wmPath,
      };
      currentLastEdit = edit;
      return edit;
    } catch {
      // Not a flow, try as script
    }

    // Try as script
    for (const ext of exts) {
      const filePath = wmPath + ext;
      try {
        await access(filePath);
        const content = await readFile(filePath, "utf-8");
        const lang = inferContentTypeFromFilePath(filePath, opts.defaultTs);
        const typed = (await parseMetadataFile(removeExtensionToPath(filePath), undefined))?.payload;
        const edit: LastEditScript = {
          type: "script",
          content,
          path: wmPath,
          language: lang,
          tag: typed?.tag,
          lock: typed?.lock,
        };
        currentLastEdit = edit;
        return edit;
      } catch {
        continue;
      }
    }

    log.error(`Could not find file for path: ${wmPath}`);
    return undefined;
  }

  // Handle flow edits from the dev UI — write changes back to disk
  async function handleFlowRoundTrip(data: { flow: any; uriPath: string }) {
    if (!data.uriPath || !data.flow?.value) return;

    let flowDir = data.uriPath;
    if (!flowDir.endsWith("/")) flowDir += "/";
    if (flowDir.includes("://")) {
      flowDir = new URL(flowDir).pathname;
    }

    restorePathScripts(data.flow.value);

    const flowYamlPath = flowDir + "flow.yaml";
    let currentModules: any[] | undefined;
    try {
      const currentFlow = (await yamlParseFile(flowYamlPath)) as FlowFile;
      currentModules = currentFlow.value?.modules;
    } catch {
      // flow.yaml doesn't exist yet or is invalid
    }

    const inlineScriptMapping: Record<string, string> = {};
    extractCurrentMapping(currentModules, inlineScriptMapping);

    const allExtracted = extractInlineScripts(
      data.flow.value.modules ?? [],
      inlineScriptMapping,
      "/",
      opts.defaultTs ?? "bun",
      undefined,
      { skipInlineScriptSuffix: getNonDottedPaths() },
    );

    for (const s of allExtracted) {
      const filePath = flowDir + s.path;
      let needsWrite = true;
      try {
        const existing = await readFile(filePath, "utf-8");
        if (existing === s.content) needsWrite = false;
      } catch {
        // File doesn't exist
      }
      if (needsWrite) {
        await writeFile(filePath, s.content, "utf-8");
        log.info(`Wrote inline script: ${filePath}`);
      }
    }

    const flowYaml = yamlStringify(data.flow);
    await writeFile(flowYamlPath, flowYaml, "utf-8");
    log.info(`Wrote flow: ${flowYamlPath}`);

    // Clean up orphaned inline script files
    const extractedPaths = new Set(allExtracted.map((s) => s.path));
    try {
      const dirFiles = await readdir(flowDir);
      for (const file of dirFiles) {
        if (file === "flow.yaml" || file === "flow.json" || file.startsWith(".")) continue;
        if (!extractedPaths.has(file)) {
          await unlink(flowDir + file);
          log.info(`Removed orphaned file: ${flowDir + file}`);
        }
      }
    } catch {
      // Directory read failed
    }
  }

  const connectedClients: Set<WebSocket> = new Set();

  // Function to send a message to all connected clients
  function broadcastChanges(lastEdit: LastEditScript | LastEditFlow) {
    for (const client of connectedClients.values()) {
      client.send(JSON.stringify(lastEdit));
    }
  }

  function setupDevWs(wss: WebSocketServer) {
    wss.on("connection", (ws: WebSocket) => {
      connectedClients.add(ws);
      console.log("New dev client connected");

      ws.on("close", () => {
        connectedClients.delete(ws);
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
        } else if (data.type === "flow") {
          handleFlowRoundTrip(data).catch((err) => {
            log.error(`Failed to write flow changes: ${err}`);
          });
        } else if (data.type === "loadWmPath") {
          loadWmPath(data.path).then((edit) => {
            if (edit && ws.readyState === WebSocket.OPEN) {
              ws.send(JSON.stringify(edit));
            }
          }).catch((err) => {
            log.error(`Failed to load path ${data.path}: ${err}`);
          });
        }
      });
    });
  }

  // --- Reverse proxy (when --proxy-port is set) ---

  async function startProxyServer(proxyPort: number) {
    const remote = new URL(workspace.remote);
    const isHttps = remote.protocol === "https:";
    const remoteHost = remote.hostname;
    const remotePort = remote.port ? parseInt(remote.port) : (isHttps ? 443 : 80);
    const httpModule = isHttps ? https : http;

    const devWss = new WebSocketServer({ noServer: true });
    setupDevWs(devWss);

    const proxyWss = new WebSocketServer({ noServer: true });

    const proxyServer = http.createServer((clientReq, clientRes) => {
      const parsedUrl = new URL(clientReq.url ?? "/", `http://localhost`);
      if (parsedUrl.pathname === "/" || parsedUrl.pathname === "") {
        let devUrl = `/dev?workspace=${workspace.workspaceId}&local=true&wm_token=${workspace.token}&port=${proxyPort}`;
        if (opts.path) {
          devUrl += `&path=${opts.path}`;
        }
        clientRes.writeHead(302, { Location: devUrl });
        clientRes.end();
        return;
      }

      const fwdHeaders: Record<string, string | string[] | undefined> = {
        ...clientReq.headers,
        host: remote.host,
      };
      delete fwdHeaders["connection"];
      delete fwdHeaders["keep-alive"];
      delete fwdHeaders["transfer-encoding"];
      delete fwdHeaders["accept-encoding"];

      const proxyOpts: http.RequestOptions = {
        hostname: remoteHost,
        port: remotePort,
        path: clientReq.url,
        method: clientReq.method,
        headers: fwdHeaders,
      };

      const proxyReq = httpModule.request(proxyOpts, (proxyRes) => {
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

    // WebSocket upgrades
    proxyServer.on("upgrade", (req, socket, head) => {
      const pathname = req.url?.split("?")[0] ?? "";

      if (pathname === "/ws_dev" || pathname === "/ws") {
        devWss.handleUpgrade(req, socket, head, (ws) => {
          devWss.emit("connection", ws, req);
        });
        return;
      }

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

      socket.destroy();
    });

    return new Promise<void>((resolve) => {
      proxyServer.listen(proxyPort, () => {
        console.log(`Dev proxy listening on http://localhost:${proxyPort}`);
        resolve();
      });
    });
  }

  // --- Legacy server (direct WebSocket, no proxy) ---

  async function startLegacyServer() {
    const server = http.createServer((_req, res) => {
      res.writeHead(200);
      res.end();
    });
    const wss = new WebSocketServer({ server });
    setupDevWs(wss);

    const port = await getPort.default({ port: PORT });
    const url =
      `${workspace.remote}dev?workspace=${workspace.workspaceId}&local=true&wm_token=${workspace.token}` +
      (port === PORT ? "" : `&port=${port}`);

    console.log(`Go to ${url}`);
    try {
      open.openApp(open.apps.browser, { arguments: [url] }).catch((error) => {
        console.error(
          `Failed to open browser, please navigate to ${url}, error: ${error}`
        );
      });
      console.log("Opened browser for you");
    } catch (error) {
      console.error(
        `Failed to open browser, please navigate to ${url}, ${error}`
      );
    }

    console.log(
      "Dev server will automatically point to the last script edited locally"
    );

    server.listen(port, () => {
      console.log(`Server listening on port ${port}`);
    });
  }

  // --- Start ---

  // If --path is set, load it immediately
  if (opts.path) {
    await loadWmPath(opts.path);
  }

  const startServer = opts.proxyPort
    ? () => startProxyServer(opts.proxyPort!)
    : () => startLegacyServer();

  await Promise.all([startServer(), watchChanges()]);
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
    "Port for a localhost reverse proxy to the remote Windmill server"
  )
  .option(
    "--path <path:string>",
    "Watch a specific windmill path (e.g., u/admin/my_script or f/my_flow)"
  )
  .action(dev as any);

export default command;
