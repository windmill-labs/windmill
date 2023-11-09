import getPort from "https://deno.land/x/getport@v2.1.2/mod.ts";
import { Application, Command, Router, log, open, path } from "./deps.ts";
import { GlobalOptions } from "./types.ts";
import { ignoreF } from "./sync.ts";
import { requireLogin, resolveWorkspace } from "./context.ts";

const PORT = 3001;
async function dev(opts: GlobalOptions & { filter?: string }) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  log.info("Started dev mode");
  let currentLastEdit: LastEdit | undefined = undefined;

  const watcher = Deno.watchFs(opts.filter ?? ".");
  const base = await Deno.realPath(".");
  const ignore = await ignoreF();

  async function watchChanges() {
    for await (const event of watcher) {
      log.debug(">>>> event", event);
      // Example event: { kind: "create", paths: [ "/home/alice/deno/foo.txt" ] }
      await loadPaths(event.paths);
    }
  }

  async function loadPaths(pathsToLoad: string[]) {
    const paths = pathsToLoad.filter(
      (path) =>
        path.endsWith(".go") ||
        path.endsWith(".ts") ||
        path.endsWith(".py") ||
        path.endsWith(".sh") ||
        path.endsWith(".sql") ||
        path.endsWith(".gql") ||
        path.endsWith(".ps1")
    );
    if (paths.length == 0) {
      return;
    }
    const cpath = (await Deno.realPath(paths[0])).replace(base + path.sep, "");
    console.log("Detected change in " + cpath);
    if (!ignore(cpath, false)) {
      const content = await Deno.readTextFile(cpath);
      const splitted = cpath.split(".");
      const wmPath = splitted[0];
      const len = splitted.length;
      const ext = splitted[len - 1];
      const lang =
        ext == "py"
          ? "python3"
          : ext == "ts"
          ? len > 2 && splitted[len - 2] == "fetch"
            ? "nativets"
            : len > 2 && splitted[len - 2] == "bun"
            ? "bun"
            : "deno"
          : ext == "go"
          ? "go"
          : ext == "sh"
          ? "bash"
          : ext == "ps1"
          ? "powershell"
          : ext == "sql"
          ? len > 2 && splitted[len - 2] == "my"
            ? "mysql"
            : len > 2 && splitted[len - 2] == "bq"
            ? "bigquery"
            : len > 2 && splitted[len - 2] == "sf"
            ? "snowflake"
            : len > 2 && splitted[len - 2] == "ms"
            ? "mssql"
            : "postgresql"
          : ext == "gql"
          ? "graphql"
          : "unknown";
      currentLastEdit = {
        content,
        path: wmPath,
        language: lang,
      };
      broadcast_changes(currentLastEdit);
      log.info("Updated " + wmPath);
    }
  }
  type LastEdit = {
    content: string;
    path: string;
    language: string;
  };

  const connectedClients = new Set<WebSocket>();

  const app = new Application();
  const router: Router = new Router();

  // send a message to all connected clients
  function broadcast_changes(lastEdit: LastEdit) {
    for (const client of connectedClients.values()) {
      client.send(JSON.stringify(lastEdit));
    }
  }

  async function startApp() {
    router.get("/ws", async (ctx) => {
      const socket = await ctx.upgrade();
      connectedClients.add(socket);
      log.info(`New client connected`);

      socket.onopen = () => {
        if (currentLastEdit) {
          broadcast_changes(currentLastEdit);
        }
      };

      socket.onclose = () => {
        connectedClients.delete(socket);
      };

      socket.onmessage = (event) => {
        let data: any | undefined = undefined;
        try {
          data = JSON.parse(event.data);
        } catch {
          console.log("Received invalid JSON: " + event.data);
          return;
        }

        if (data.type == "load") {
          loadPaths([data.path] as string[]);
        }
      };
    });

    app.use(router.routes());
    app.use(router.allowedMethods());

    const port = getPort(PORT);
    const url =
      `${workspace.remote}scripts/dev?workspace=${workspace.workspaceId}&local=true` +
      (port == PORT ? "" : "&port=" + port);
    console.log(`Go to ${url}`);
    try {
      await open(url);
      log.info("Opened browser for you");
    } catch {
      console.error(`Failed to open browser, please navigate to ${url}`);
    }
    console.log(
      "Dev server will automatically point to the last script edited locally"
    );
    await app.listen({ port });
  }

  await Promise.all([startApp(), watchChanges()]);
  console.log("Stopped dev mode");
}

const command = new Command()
  .description("Launch a dev server that will spawn a webserver with HMR")
  .option(
    "--filter <filter:string>",
    "Filter paths givena glob pattern or path"
  )
  // deno-lint-ignore no-explicit-any
  .action(dev as any);

export default command;
