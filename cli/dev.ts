import {
  Command,
  SEP,
  WebSocketServer,
  express,
  getPort,
  http,
  log,
  open,
  WebSocket,
  yamlParseFile,
} from "./deps.ts";
import { getTypeStrFromPath, GlobalOptions } from "./types.ts";
import { ignoreF } from "./sync.ts";
import { requireLogin } from "./auth.ts";
import { resolveWorkspace } from "./context.ts";
import {
  SyncOptions,
  mergeConfigWithConfigFile,
  readConfigFile,
} from "./conf.ts";
import { exts, findGlobalDeps, removeExtensionToPath } from "./script.ts";
import { inferContentTypeFromFilePath } from "./script_common.ts";
import { OpenFlow } from "./gen/types.gen.ts";
import { FlowFile } from "./flow.ts";
import { replaceInlineScripts } from "npm:centdix-utils";
import { parseMetadataFile } from "./metadata.ts";

const PORT = 3001;
async function dev(opts: GlobalOptions & SyncOptions) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  log.info("Started dev mode");
  const conf = await readConfigFile();
  let currentLastEdit: LastEditScript | LastEditFlow | undefined = undefined;

  const watcher = Deno.watchFs(".");
  const base = await Deno.realPath(".");
  opts = await mergeConfigWithConfigFile(opts);
  const ignore = await ignoreF(opts);

  const changesTimeouts: Record<string, number> = {};
  async function watchChanges() {
    for await (const event of watcher) {
      // console.log(">>>> event", event);
      const key = event.paths.join(",");
      if (changesTimeouts[key]) {
        clearTimeout(changesTimeouts[key]);
      }
      // @ts-ignore
      changesTimeouts[key] = setTimeout(async () => {
        delete changesTimeouts[key];
        await loadPaths(event.paths);
      }, 100);
    }
  }

  const DOT_FLOW_SEP = ".flow" + SEP;
  async function loadPaths(pathsToLoad: string[]) {
    const paths = pathsToLoad.filter((path) =>
      exts.some(
        (ext) => path.endsWith(ext) || path.endsWith(DOT_FLOW_SEP + "flow.yaml")
      )
    );
    if (paths.length == 0) {
      return;
    }
    const cpath = (await Deno.realPath(paths[0])).replace(base + SEP, "");
    if (!ignore(cpath, false)) {
      const typ = getTypeStrFromPath(cpath);
      log.info("Detected change in " + cpath + " (" + typ + ")");
      if (typ == "flow") {
        const localPath = cpath.split(DOT_FLOW_SEP)[0] + DOT_FLOW_SEP;
        const localFlow = (await yamlParseFile(
          localPath + "flow.yaml"
        )) as FlowFile;
        replaceInlineScripts(localFlow.value.modules, Deno.readTextFileSync, localPath);
        currentLastEdit = {
          type: "flow",
          flow: localFlow,
          uriPath: localPath,
        };
        log.info("Updated " + localPath);
        broadcastChanges(currentLastEdit);
      } else if (typ == "script") {
        const content = await Deno.readTextFile(cpath);
        const splitted = cpath.split(".");
        const wmPath = splitted[0];
        const lang = inferContentTypeFromFilePath(cpath, conf.defaultTs);
        const globalDeps = await findGlobalDeps();
        const typed =
          (await parseMetadataFile(
            removeExtensionToPath(cpath),
            undefined,
            globalDeps,
            []
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
    const app = express.default();
    const server = http.createServer(app);
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
          data = JSON.parse(message);
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
  // deno-lint-ignore no-explicit-any
  .action(dev as any);

export default command;
