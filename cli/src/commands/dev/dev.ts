import { Command } from "@cliffy/command";
import * as log from "../../core/log.ts";
import { sep as SEP } from "node:path";
import { yamlParseFile } from "../../utils/yaml.ts";
import { stringify as yamlStringify } from "yaml";
import { WebSocket, WebSocketServer } from "ws";

import * as http from "node:http";
import * as https from "node:https";
import * as open from "open";
import { access, readdir, realpath, stat, unlink, writeFile } from "node:fs/promises";
import { readTextFile } from "../../utils/utils.ts";
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
import { newPathAssigner } from "../../../windmill-utils-internal/src/path-utils/path-assigner.ts";
import { parseMetadataFile } from "../../utils/metadata.ts";
import {
  getMetadataFileName,
  extractFolderPath,
  getNonDottedPaths,
  loadNonDottedPathsSetting,
} from "../../utils/resource_folders.ts";
import * as path from "node:path";
import * as fs from "node:fs";
import { listSyncCodebases } from "../../utils/codebase.ts";
import { createPreviewLocalScriptReader } from "../../utils/local_path_scripts.ts";
import { resolveBindPort, BIND_HOST } from "../../utils/port-probe.ts";
import {
  snapshotPathScripts,
  tagReplacedPathScripts,
  restorePathScripts,
} from "./pathscript-restore.ts";

const PORT = 3001;

type WmPathItem = {
  path: string;
  kind: "flow" | "script" | "raw_app";
  summary?: string;
};

const FLOW_SUFFIXES = [".flow", "__flow"] as const;
const APP_SUFFIXES = [".app", "__app", ".raw_app", "__raw_app"] as const;

// Extensions the dev round-trip might have written into a flow folder as
// inline scripts. Derived from script.ts's `exts` so adding a new language
// there auto-extends orphan cleanup; otherwise stale inline scripts of that
// language would silently linger. Excludes `.yml` — user fixtures commonly
// use it in flow folders, and leaving a stale `.playbook.yml` inline script
// is preferable to deleting a fixture. `.js` is added explicitly for
// hand-written flows that aren't in the `exts` list.
//
// Anything else (README.md, fixtures, .env*, TODO.md…) is preserved during
// orphan cleanup so we don't trample user-added files.
const INLINE_SCRIPT_EXTS = new Set([
  // path.extname(".py") === "" (Node treats ".py" as a hidden filename, not
  // an extension), so prefix with a dummy character before extracting.
  ...exts.map((e) => path.extname("x" + e)).filter((e) => e !== ".yml"),
  ".js",
]);

function stripFolderSuffix(rel: string, suffixes: readonly string[]): string {
  for (const s of suffixes) {
    if (rel.endsWith(s)) return rel.slice(0, -s.length);
  }
  return rel;
}

function isFlowFolderName(name: string): boolean {
  return FLOW_SUFFIXES.some((s) => name.endsWith(s));
}

// Normalize a windmill path: strip trailing slash and any flow folder suffix
// so f/foo, f/foo/, f/foo.flow, and f/foo.flow/ all compare equal.
function normalizeWmPath(p: string): string {
  return stripFolderSuffix(p.replace(/\/$/, ""), FLOW_SUFFIXES);
}

// Anchor on path segments — substring matches like cpath.includes(".flow/")
// also fire on innocent names like "notes_about__flow_design/readme.md".
function isInsideFlowFolder(cpath: string): boolean {
  return cpath.split("/").some(isFlowFolderName);
}

// Return the path prefix up to and including the first flow-folder segment,
// with a trailing slash. Returns undefined if no segment matches.
function findFlowFolderPrefix(cpath: string): string | undefined {
  const segs = cpath.split("/");
  for (let i = 0; i < segs.length; i++) {
    if (isFlowFolderName(segs[i])) {
      return segs.slice(0, i + 1).join("/") + "/";
    }
  }
  return undefined;
}

async function listWorkspacePaths(): Promise<WmPathItem[]> {
  // Walk first, capturing each item's metadata file path. Then read summaries in
  // parallel — one tree pass plus N file reads is faster than a serialized walk.
  const items: (WmPathItem & { _metaPath?: string })[] = [];
  async function walk(dir: string, rel: string) {
    let entries;
    try {
      entries = await readdir(dir, { withFileTypes: true });
    } catch {
      return;
    }
    for (const entry of entries) {
      if (entry.name.startsWith(".") || entry.name === "node_modules") continue;
      const childRel = rel ? `${rel}/${entry.name}` : entry.name;
      const childAbs = path.join(dir, entry.name);
      if (entry.isDirectory()) {
        if (isFlowFolderName(entry.name)) {
          items.push({
            path: stripFolderSuffix(childRel, FLOW_SUFFIXES),
            kind: "flow",
            _metaPath: path.join(childAbs, "flow.yaml"),
          });
          continue;
        }
        if (APP_SUFFIXES.some((s) => entry.name.endsWith(s))) {
          items.push({ path: stripFolderSuffix(childRel, APP_SUFFIXES), kind: "raw_app" });
          continue;
        }
        await walk(childAbs, childRel);
      } else if (entry.isFile()) {
        const matchedExt = exts.find((ext) => entry.name.endsWith(ext));
        if (matchedExt) {
          const noExtAbs = childAbs.slice(0, -matchedExt.length);
          items.push({
            path: childRel.slice(0, -matchedExt.length),
            kind: "script",
            _metaPath: noExtAbs + ".script.yaml",
          });
        }
      }
    }
  }
  await walk(process.cwd(), "");

  await Promise.all(
    items.map(async (item) => {
      if (!item._metaPath) return;
      try {
        const meta: any = await yamlParseFile(item._metaPath);
        if (typeof meta?.summary === "string" && meta.summary.length > 0) {
          item.summary = meta.summary;
        }
      } catch {
        // No metadata file or unparseable — leave summary undefined
      }
    })
  );

  items.sort((a, b) => a.path.localeCompare(b.path));
  return items.map(({ _metaPath, ...item }) => item);
}

export interface DevOpts {
  proxyPort?: number;
  path?: string;
  open?: boolean;
}

export async function dev(opts: GlobalOptions & SyncOptions & DevOpts) {
  // Auto-detect flow folder: if no --path and cwd is a flow folder, resolve path and chdir to workspace root
  if (!opts.path) {
    const cwd = process.cwd();
    const cwdBasename = path.basename(cwd);

    // Need to init nonDottedPaths before checking suffix
    await loadNonDottedPathsSetting();

    if (isFlowFolderName(cwdBasename)) {
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
        opts.path = stripFolderSuffix(relPath, FLOW_SUFFIXES);
        log.info(`Detected flow folder, path: ${opts.path}`);
        process.chdir(workspaceRoot);
      }
    }
  }

  opts = await mergeConfigWithConfigFile(opts);
  // Normalize once so broadcastChanges' equality check survives user input
  // like --path f/foo/ or --path f/foo.flow (and the same set via wmill.yaml).
  if (opts.path) {
    opts.path = normalizeWmPath(opts.path);
  }
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

  const flowMetadataFile = getMetadataFileName("flow", "yaml");
  async function loadPaths(pathsToLoad: string[]) {
    const paths = pathsToLoad.filter((p) =>
      exts.some(
        (ext) => p.endsWith(ext)
          || p.endsWith(".flow/" + flowMetadataFile)
          || p.endsWith("__flow/" + flowMetadataFile)
      )
    );
    if (paths.length == 0) {
      return;
    }
    const nativePath = (await realpath(paths[0])).replace(base + SEP, "");
    const cpath = nativePath.replaceAll("\\", "/");
    // Bypass ignore for paths inside flow folders — ignore() only checks the configured
    // suffix (dotted or non-dotted), but the workspace may contain both kinds
    const insideFlow = isInsideFlowFolder(cpath);
    if (insideFlow || !ignore(nativePath, false)) {
      let typ: string;
      if (insideFlow) {
        // Force flow type for any file inside a flow folder — getTypeStrFromPath
        // only recognises the configured suffix (dotted or non-dotted) and would
        // mis-classify or throw for the other variant
        typ = "flow";
      } else {
        typ = getTypeStrFromPath(cpath);
      }
      log.info("Detected change in " + cpath + " (" + typ + ")");
      if (typ == "flow") {
        // Try extractFolderPath, fallback to segment-anchored extraction for
        // mixed suffix cases (extractFolderPath only checks the configured suffix).
        let localPath = extractFolderPath(cpath, "flow") ?? findFlowFolderPrefix(cpath);
        if (!localPath) return;
        const wmFlowPath = stripFolderSuffix(localPath.replace(/\/$/, ""), FLOW_SUFFIXES);
        const localFlow = (await yamlParseFile(
          localPath + "flow.yaml"
        )) as FlowFile;
        await replaceInlineScripts(
          localFlow.value.modules,
          async (path: string) => await readTextFile(localPath + path),
          log,
          localPath,
          SEP,
          undefined,
        );
        // Snapshot PathScript modules before replacement, then tag after.
        // Helpers walk `flowValue` (modules/failure_module/preprocessor_module),
        // so pass `.value`, not the FlowFile wrapper.
        snapshotPathScripts(localFlow.value);
        const localScriptReader = createPreviewLocalScriptReader({
          exts,
          defaultTs: opts.defaultTs,
          codebases,
        });
        await replaceAllPathScriptsWithLocal(localFlow.value, localScriptReader, log);
        tagReplacedPathScripts(localFlow.value);
        currentLastEdit = {
          type: "flow",
          flow: localFlow,
          uriPath: localPath,
          path: wmFlowPath,
        };
        log.info("Updated " + wmFlowPath);
        broadcastChanges(currentLastEdit);
      } else if (typ == "script") {
        const splitted = cpath.split(".");
        const wmPath = splitted[0];
        const content = await readTextFile(cpath);
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

  // Load a resource by its windmill path (e.g., "u/admin/my_script" or "f/my_flow")
  async function loadWmPath(wmPath: string): Promise<LastEditScript | LastEditFlow | undefined> {
    wmPath = normalizeWmPath(wmPath);
    // Try as flow — check both dotted and non-dotted suffixes
    let flowDir: string | undefined;
    let flowYaml: string | undefined;
    for (const suffix of [".flow", "__flow"]) {
      const candidate = wmPath + suffix + "/";
      try {
        await access(candidate + "flow.yaml");
        flowDir = candidate;
        flowYaml = candidate + "flow.yaml";
        break;
      } catch {}
    }
    try {
      if (!flowDir || !flowYaml) throw new Error("not a flow");
      const localFlow = (await yamlParseFile(flowYaml)) as FlowFile;
      await replaceInlineScripts(
        localFlow.value.modules,
        async (p: string) => await readTextFile(flowDir + p),
        log,
        flowDir,
        SEP,
        undefined,
      );
      snapshotPathScripts(localFlow.value);
      const localScriptReader = createPreviewLocalScriptReader({
        exts,
        defaultTs: opts.defaultTs,
        codebases,
      });
      await replaceAllPathScriptsWithLocal(localFlow.value, localScriptReader, log);
      tagReplacedPathScripts(localFlow.value);
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
        const content = await readTextFile(filePath);
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
  // Mirrors the windmill-vscode extension's `processFlowMessage`
  // (src/extension.ts). Keep them in step — the dev page is the same code in
  // both contexts, and divergence here means the same flow round-trips
  // differently between VS Code and the local dev preview.
  //
  // Deliberate divergence: the orphan-cleanup pass is restricted to
  // INLINE_SCRIPT_EXTS so unrelated files (README.md, fixtures, etc.) are
  // not deleted. The extension's version doesn't filter and would delete
  // them — that's a known issue tracked separately.
  async function handleFlowRoundTrip(data: { flow: any; uriPath: string }) {
    if (!data.uriPath || !data.flow?.value) return;

    let flowDir = data.uriPath;
    if (!flowDir.endsWith("/")) flowDir += "/";
    if (flowDir.includes("://")) {
      flowDir = new URL(flowDir).pathname;
    }

    // Restore PathScripts BEFORE extracting so we don't write a file for the
    // inlined body of a `type: 'script'` reference.
    restorePathScripts(data.flow.value);

    const flowYamlPath = flowDir + "flow.yaml";
    let currentLoadedFlow: any[] | undefined;
    let currentLoadedFailureModule: any | undefined;
    let currentLoadedPreprocessorModule: any | undefined;
    try {
      const currentFlow = (await yamlParseFile(flowYamlPath)) as FlowFile;
      currentLoadedFlow = currentFlow.value?.modules;
      currentLoadedFailureModule = currentFlow.value?.failure_module;
      currentLoadedPreprocessorModule = currentFlow.value?.preprocessor_module;
    } catch {
      // flow.yaml doesn't exist yet or is invalid
    }

    const inlineScriptMapping: Record<string, string> = {};
    extractCurrentMapping(
      currentLoadedFlow,
      inlineScriptMapping,
      currentLoadedFailureModule,
      currentLoadedPreprocessorModule,
    );

    // Share one pathAssigner across all extraction calls so failure /
    // preprocessor modules don't collide on filenames with main modules.
    const extractOptions = { skipInlineScriptSuffix: getNonDottedPaths() };
    const pathAssigner = newPathAssigner(opts.defaultTs ?? "bun", extractOptions);

    const allExtracted = extractInlineScripts(
      data.flow.value.modules ?? [],
      inlineScriptMapping,
      "/",
      opts.defaultTs ?? "bun",
      pathAssigner,
      extractOptions,
    );
    if (data.flow.value.failure_module?.value?.type === "rawscript") {
      allExtracted.push(...extractInlineScripts(
        [data.flow.value.failure_module],
        inlineScriptMapping,
        "/",
        opts.defaultTs ?? "bun",
        pathAssigner,
        extractOptions,
      ));
    }
    if (data.flow.value.preprocessor_module?.value?.type === "rawscript") {
      allExtracted.push(...extractInlineScripts(
        [data.flow.value.preprocessor_module],
        inlineScriptMapping,
        "/",
        opts.defaultTs ?? "bun",
        pathAssigner,
        extractOptions,
      ));
    }

    for (const s of allExtracted) {
      const filePath = flowDir + s.path;
      // `!inline foo.ts` is a YAML directive that points at another file —
      // treat it as a placeholder, not as content to overwrite.
      if (s.content.startsWith("!inline ")) {
        try {
          await stat(filePath);
        } catch {
          await writeFile(filePath, "", "utf-8");
        }
        continue;
      }
      let needsWrite = true;
      try {
        const existing = await readTextFile(filePath);
        if (existing === s.content) needsWrite = false;
      } catch {
        // File doesn't exist
      }
      if (needsWrite) {
        await writeFile(filePath, s.content, "utf-8");
        log.info(`Wrote inline script: ${filePath}`);
      }
    }

    // Only rewrite flow.yaml when the serialized YAML actually differs from
    // what's on disk. Avoids noisy mtime updates that re-trigger the watcher.
    const flowYaml = yamlStringify(data.flow);
    let currentYaml: string | undefined;
    try {
      currentYaml = await readTextFile(flowYamlPath);
    } catch {
      // File doesn't exist
    }
    if (currentYaml?.trimEnd() !== flowYaml.trimEnd()) {
      await writeFile(flowYamlPath, flowYaml, "utf-8");
      log.info(`Wrote flow: ${flowYamlPath}`);
    }

    // Orphan cleanup: extension does this unconditionally and overshoots,
    // deleting README.md / fixtures / .env.local. We restrict to known
    // inline-script extensions.
    const extractedPaths = new Set(allExtracted.map((s) => s.path));
    try {
      const dirFiles = await readdir(flowDir);
      for (const file of dirFiles) {
        if (file === "flow.yaml" || file === "flow.json" || file.startsWith(".")) continue;
        if (!INLINE_SCRIPT_EXTS.has(path.extname(file))) continue;
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

  // Send a message to all connected clients, gated by --path when set so we
  // don't spam clients (or risk yanking their view) with edits to unrelated files.
  function broadcastChanges(lastEdit: LastEditScript | LastEditFlow) {
    if (opts.path && normalizeWmPath(lastEdit.path) !== opts.path) {
      return;
    }
    for (const client of connectedClients.values()) {
      client.send(JSON.stringify(lastEdit));
    }
  }

  function setupDevWs(wss: WebSocketServer) {
    wss.on("connection", (ws: WebSocket) => {
      connectedClients.add(ws);
      console.log("New dev client connected");

      // Push the currently loaded edit so the page renders immediately on
      // page load, without waiting for a file change to trigger a broadcast.
      if (currentLastEdit && ws.readyState === WebSocket.OPEN) {
        try {
          ws.send(JSON.stringify(currentLastEdit));
        } catch (e) {
          console.error("Failed to push initial state to new client:", e);
        }
      }

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
        } else if (data.type === "listPaths") {
          listWorkspacePaths().then((items) => {
            if (ws.readyState === WebSocket.OPEN) {
              ws.send(JSON.stringify({ type: "paths", items }));
            }
          }).catch((err) => {
            log.error(`Failed to list paths: ${err}`);
          });
        }
      });
    });
  }

  function maybeOpenBrowser(url: string) {
    if (opts.open === false) return;
    try {
      open.default(url).catch((error) => {
        console.error(
          `Failed to open browser, please navigate to ${url}, error: ${error}`
        );
      });
      console.log(`Opened browser at ${url}`);
    } catch (error) {
      console.error(
        `Failed to open browser, please navigate to ${url}, ${error}`
      );
    }
  }

  // --- Proxy mode (when --proxy-port is set) ---
  //
  // Runs a localhost HTTP server that:
  //   - serves the dev page from `http://localhost:<proxyPort>/` (forwarded
  //     to the remote workspace), so embedders that need a localhost origin
  //     can render it (e.g. Claude Code's port-detection preview), and
  //   - upgrades local /ws connections back to this same process for the
  //     live-reload channel.
  //
  // The simpler "direct" mode below works for standalone browser tabs and the
  // VS Code extension's iframe — only embedders that demand a localhost origin
  // need this proxy.

  async function startProxyServer(requestedPort: number) {
    // Probe both IPv4 and IPv6 stacks before binding. If the requested port is
    // taken on either, walk upward to the next free one so we don't silently
    // collide with a leftover dev server (see cli/src/utils/port-probe.ts).
    const proxyPort = await resolveBindPort(requestedPort, "--proxy-port", {
      info: (m) => console.log(m),
      warn: (m) => console.warn(m),
    });

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
      proxyServer.listen(proxyPort, BIND_HOST, () => {
        console.log(`Dev proxy listening on http://localhost:${proxyPort}`);
        if (opts.path) {
          console.log(`Watching ${opts.path} — edits will live-reload in the dev page`);
        } else {
          console.log(
            "Open the dev page and pick a flow or script to preview — edits will live-reload"
          );
          console.log("(pass --path <path> to skip the picker)");
        }
        maybeOpenBrowser(`http://localhost:${proxyPort}/`);
        resolve();
      });
    });
  }

  // --- Direct mode (no localhost HTTP proxy) ---
  //
  // The browser loads the dev page from the remote workspace URL and opens a
  // WebSocket directly to this localhost server. Used when:
  //   - the user runs `wmill dev` and opens a regular browser tab, or
  //   - the VS Code extension iframe loads the dev page (its iframe URL omits
  //     `local=true`, so it never opens this WS, but everything else still
  //     functions through the existing remote workspace connection).
  //
  // This is the simplest topology: a bare WebSocket server. The reverse-proxy
  // mode (above) is only needed when something needs to embed the dev UI on a
  // localhost origin (Claude Code's port-detection preview).

  async function startDirectServer() {
    const server = http.createServer((_req, res) => {
      res.writeHead(200);
      res.end();
    });
    const wss = new WebSocketServer({ server });
    setupDevWs(wss);

    // Probe both IPv4 and IPv6 stacks before binding. Same dual-stack
    // collision risk as proxy mode: a leftover wmill dev on [::]:3001 would
    // silently steer localhost:3001 traffic to the wrong process if we only
    // probed one stack (which is what getPort does).
    const port = await resolveBindPort(PORT, "wmill dev", {
      info: (m) => console.log(m),
      warn: (m) => console.warn(m),
    });
    const url =
      `${workspace.remote}dev?workspace=${workspace.workspaceId}&local=true&wm_token=${workspace.token}` +
      (port === PORT ? "" : `&port=${port}`) +
      (opts.path ? `&path=${opts.path}` : "");

    if (opts.open === false) {
      console.log(`Go to ${url}`);
    }
    maybeOpenBrowser(url);

    if (opts.path) {
      console.log(`Watching ${opts.path} — edits will live-reload in the dev page`);
    } else {
      console.log(
        "Open the dev page and pick a flow or script to preview — edits will live-reload"
      );
    }

    server.listen(port, BIND_HOST, () => {
      console.log(`Dev WebSocket listening on ws://localhost:${port}/ws`);
    });
  }

  // --- Start ---

  // If --path is set, load it immediately
  if (opts.path) {
    await loadWmPath(opts.path);
  }

  const startServer = opts.proxyPort
    ? () => startProxyServer(opts.proxyPort!)
    : () => startDirectServer();

  await Promise.all([startServer(), watchChanges()]);
  console.log("Stopped dev mode");
}

const command = new Command()
  .description("Watch local file changes and live-reload the dev page for preview. Does NOT deploy to the remote workspace — use wmill sync push for that.")
  .option(
    "--includes <pattern...:string>",
    "Filter paths given a glob pattern or path"
  )
  .option(
    "--proxy-port <port:number>",
    "Port for a localhost reverse proxy to the remote Windmill server"
  )
  .option(
    "--path <path:string>",
    "Watch a specific windmill path (e.g., u/admin/my_script or f/my_flow)"
  )
  .option(
    "--no-open",
    "Do not open the browser automatically"
  )
  .action(dev as any);

export default command;
