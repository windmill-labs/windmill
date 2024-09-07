import * as dntShim from "./_dnt.shims.js";
import {
  Command,
  SEP,
  WebSocketServer,
  express,
  getAvailablePort,
  http,
  log,
  open,
  WebSocket,
} from "./deps.js";
import { GlobalOptions } from "./types.js";
import { ignoreF } from "./sync.js";
import { requireLogin, resolveWorkspace } from "./context.js";
import {
  SyncOptions,
  mergeConfigWithConfigFile,
  readConfigFile,
} from "./conf.js";
import { exts } from "./script.js";
import { inferContentTypeFromFilePath } from "./script_common.js";

const PORT = 3001;
async function dev(opts: GlobalOptions & SyncOptions) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  log.info("Started dev mode");
  const conf = await readConfigFile();
  let currentLastEdit: LastEdit | undefined = undefined;

  const watcher = dntShim.Deno.watchFs(".");
  const base = await dntShim.Deno.realPath(".");
  opts = await mergeConfigWithConfigFile(opts);
  const ignore = await ignoreF(opts);

  async function watchChanges() {
    for await (const event of watcher) {
      log.debug(">>>> event", event);
      // Example event: { kind: "create", paths: [ "/home/alice/deno/foo.txt" ] }
      await loadPaths(event.paths);
    }
  }

  async function loadPaths(pathsToLoad: string[]) {
    const paths = pathsToLoad.filter((path) =>
      exts.some((ext) => path.endsWith(ext))
    );
    if (paths.length == 0) {
      return;
    }
    const cpath = (await dntShim.Deno.realPath(paths[0])).replace(base + SEP, "");
    console.log("Detected change in " + cpath);
    if (!ignore(cpath, false)) {
      const content = await dntShim.Deno.readTextFile(cpath);
      const splitted = cpath.split(".");
      const wmPath = splitted[0];
      const lang = inferContentTypeFromFilePath(cpath, conf.defaultTs);
      currentLastEdit = {
        content,
        path: wmPath,
        language: lang,
      };
      broadcastChanges(currentLastEdit);
      log.info("Updated " + wmPath);
    }
  }
  type LastEdit = {
    content: string;
    path: string;
    language: string;
  };

  const connectedClients: Set<WebSocket> = new Set();

  // Function to send a message to all connected clients
  function broadcastChanges(lastEdit: LastEdit) {
    for (const client of connectedClients.values()) {
      client.send(JSON.stringify(lastEdit));
    }
  }

  async function startApp() {
    const app = express();
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
    const port = await getAvailablePort({ preferredPort: 3001 });
    const url =
      `${workspace.remote}scripts/dev?workspace=${workspace.workspaceId}&local=true` +
      (port === PORT ? "" : `&port=${port}`);

    console.log(`Go to ${url}`);
    try {
      open.openApp(open.apps.browser, { arguments: [url] });
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
