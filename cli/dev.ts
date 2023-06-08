import getPort from "https://deno.land/x/getport@v2.1.2/mod.ts";
import { Application, Command, Router, log, path } from "./deps.ts";
import { GlobalOptions } from "./types.ts";
import { ignoreF } from "./sync.ts";
import { requireLogin, resolveWorkspace } from "./context.ts";

async function dev(opts: GlobalOptions & { filter?: string }) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  log.info("Started dev mode");
  let currentLastEdit: LastEdit | undefined = undefined;

  const watcher = Deno.watchFs(opts.filter ?? ".");
  const base = await Deno.realPath(".");
  async function watchChanges() {
    const ignore = await ignoreF();

    for await (const event of watcher) {
      log.debug(">>>> event", event);
      // Example event: { kind: "create", paths: [ "/home/alice/deno/foo.txt" ] }
      const paths = event.paths.filter(
        (path) =>
          path.endsWith(".go") ||
          path.endsWith(".ts") ||
          path.endsWith(".py") ||
          path.endsWith(".sh")
      );
      const cpath = (await Deno.realPath(paths[0])).replace(
        base + path.sep,
        ""
      );
      console.log("Detected change in " + cpath);
      if (!ignore(cpath, false)) {
        console.log("FOO");
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
        currentLastEdit = { content, path: wmPath, language: lang };
        broadcast_changes(currentLastEdit);
        log.info("Updated " + wmPath);
      }
    }
  }

  type LastEdit = { content: string; path: string; language?: string };

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
    app.use(async (ctx) => {
      const req = ctx.request;
      const path = new URL(req.url).pathname;
      if (path.startsWith("/api")) {
        console.log("Proxying to " + workspace.remote + path);
        const proxyRes = await fetch(
          workspace.remote.substring(0, workspace.remote.length - 1) + path,
          {
            headers: {
              Authorization: "Bearer " + workspace.token,
            },
            method: req.method,
            body: await req.body().value,
          }
        );
        ctx.response.body = proxyRes.body;
        ctx.response.status = proxyRes.status;
      } else {
        await ctx.send({
          root: `${Deno.cwd()}/`,
          index: "public/index.html",
        });
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
