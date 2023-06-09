import getPort from "https://deno.land/x/getport@v2.1.2/mod.ts";
import {
  Application,
  Command,
  Router,
  UserService,
  log,
  path,
} from "./deps.ts";
import { GlobalOptions } from "./types.ts";
import { ignoreF } from "./sync.ts";
import { requireLogin, resolveWorkspace } from "./context.ts";
import { mimelite } from "https://deno.land/x/mimetypes@v1.0.0/mod.ts";

async function dev(opts: GlobalOptions & { filter?: string }) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const username = (await UserService.whoami({ workspace: workspace.name }))
    .username;

  log.info("Started dev mode");
  let currentLastEdit: LastEdit | undefined = undefined;

  const watcher = Deno.watchFs(opts.filter ?? ".");
  const base = await Deno.realPath(".");
  async function watchChanges() {
    const ignore = await ignoreF();

    for await (const event of watcher) {
      log.info(">>>> event", event);
      // Example event: { kind: "create", paths: [ "/home/alice/deno/foo.txt" ] }
      const paths = event.paths.filter(
        (path) =>
          path.endsWith(".go") ||
          path.endsWith(".ts") ||
          path.endsWith(".py") ||
          path.endsWith(".sh")
      );
      if (paths.length == 0) {
        return;
      }
      const cpath = (await Deno.realPath(paths[0])).replace(
        base + path.sep,
        ""
      );
      console.log("Detected change in " + cpath);
      if (!ignore(cpath, false)) {
        const content = await Deno.readTextFile(cpath);
        const splitted = cpath.split(".");
        const wmPath = splitted[0];
        const ext = splitted[splitted.length - 1];
        const lang =
          ext == "py"
            ? "python3"
            : ext == "ts"
            ? "deno"
            : ext == "go"
            ? "go"
            : "bash";
        currentLastEdit = {
          content,
          path: wmPath,
          language: lang,
          workspace: workspace.workspaceId,
          username,
        };
        broadcast_changes(currentLastEdit);
        log.info("Updated " + wmPath);
      }
    }
  }

  type LastEdit = {
    content: string;
    path: string;
    language: string;
    workspace: string;
    username: string;
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
    });

    app.use(router.routes());
    app.use(router.allowedMethods());
    app.use(async (ctx) => {
      const req = ctx.request;
      const url = new URL(req.url);
      let path = url.pathname;
      if (path.startsWith("/api")) {
        const fpath =
          workspace.remote.substring(0, workspace.remote.length - 1) +
          path +
          "?" +
          url.searchParams.toString();
        console.log(fpath);
        console.log("Proxying to " + fpath);
        const proxyRes = await fetch(fpath, {
          headers: {
            Authorization: "Bearer " + workspace.token,
            ...Object.fromEntries(req.headers.entries()),
          },
          method: req.method,
          body: JSON.stringify(await req.body().value),
        });
        ctx.response.body = proxyRes.body;
        ctx.response.status = proxyRes.status;
        ctx.response.headers = proxyRes.headers;
      } else {
        console.log("Serving " + path);
        if (path == "/") {
          path = "devassets/index.html";
        } else {
          path = "devassets" + path;
        }
        try {
          const FILE_URL = new URL(path, import.meta.url).href;
          console.log(FILE_URL);
          const resp = await fetch(FILE_URL);
          ctx.response.body = resp.body;
          ctx.response.headers.set(
            "Content-Type",
            mimelite.getType(path.split(".").pop() ?? "txt") ?? "text/plain"
          );
        } catch (e) {
          console.log(`${path}: ${e}`);
          ctx.response.body = "404";
          ctx.response.status = 404;
        }
      }
    });

    const port = getPort(3000);
    console.log("Started dev server at http://localhost:" + port);
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
