import { Command } from "@cliffy/command";
import * as log from "../../core/log.ts";
import { sep as SEP } from "node:path";
import { yamlParseFile } from "../../utils/yaml.ts";
import { WebSocket, WebSocketServer } from "ws";

import * as getPort from "get-port";
import * as http from "node:http";
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
  readConfigFile,
} from "../../core/conf.ts";
import { exts, removeExtensionToPath } from "../script/script.ts";
import { inferContentTypeFromFilePath } from "../../utils/script_common.ts";
import { OpenFlow } from "../../../gen/types.gen.ts";
import { FlowFile } from "../flow/flow.ts";
import { replaceInlineScripts } from "../../../windmill-utils-internal/src/inline-scripts/replacer.ts";
import { parseMetadataFile } from "../../utils/metadata.ts";
import {
  getFolderSuffixWithSep,
  getMetadataFileName,
  extractFolderPath,
} from "../../utils/resource_folders.ts";

const PORT = 3001;
async function dev(opts: GlobalOptions & SyncOptions) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  log.info("Started dev mode");
  const conf = await readConfigFile();
  let currentLastEdit: LastEditScript | LastEditFlow | undefined = undefined;

  const fsWatcher = watch(".", { recursive: true });
  const base = await realpath(".");
  opts = await mergeConfigWithConfigFile(opts);
  const ignore = await ignoreF(opts);

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
          await loadPaths([filePath]);
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
        currentLastEdit = {
          type: "flow",
          flow: localFlow,
          uriPath: localPath,
        };
        log.info("Updated " + localPath);
        broadcastChanges(currentLastEdit);
      } else if (typ == "script") {
        const content = await readFile(cpath, "utf-8");
        const splitted = cpath.split(".");
        const wmPath = splitted[0];
        const lang = inferContentTypeFromFilePath(cpath, conf.defaultTs);
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
  };

  const connectedClients: Set<WebSocket> = new Set();

  // Function to send a message to all connected clients
  function broadcastChanges(lastEdit: LastEditScript | LastEditFlow) {
    for (const client of connectedClients.values()) {
      client.send(JSON.stringify(lastEdit));
    }
  }

  async function startApp() {
    const server = http.createServer((_req, res) => {
      res.writeHead(200);
      res.end();
    });
    const wss = new WebSocketServer({ server });

    // WebSocket server event listeners
    wss.on("connection", (ws: WebSocket) => {
      connectedClients.add(ws);
      console.log("New client connected");

      ws.on("open", () => {
        if (currentLastEdit) {
          broadcastChanges(currentLastEdit);
        }
      });

      ws.on("close", () => {
        connectedClients.delete(ws);
        console.log("Client disconnected");
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
        }
      });
    });

    // Start the server
    const port = await getPort.default({ port: 3001 });
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

  await Promise.all([startApp(), watchChanges()]);
  console.log("Stopped dev mode");
}

const command = new Command()
  .description("Launch a dev server that will spawn a webserver with HMR")
  .option(
    "--includes <pattern...:string>",
    "Filter paths givena glob pattern or path"
  )
  .action(dev as any);

export default command;
