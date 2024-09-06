import * as dntShim from "./_dnt.shims.js";
import { Application, Command, Router, SEP, getAvailablePort, log, open, } from "./deps.js";
import { ignoreF } from "./sync.js";
import { requireLogin, resolveWorkspace } from "./context.js";
import { mergeConfigWithConfigFile, readConfigFile, } from "./conf.js";
import { exts } from "./script.js";
import { inferContentTypeFromFilePath } from "./script_common.js";
const PORT = 3001;
async function dev(opts) {
    const workspace = await resolveWorkspace(opts);
    await requireLogin(opts);
    log.info("Started dev mode");
    const conf = await readConfigFile();
    let currentLastEdit = undefined;
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
    async function loadPaths(pathsToLoad) {
        const paths = pathsToLoad.filter((path) => exts.some((ext) => path.endsWith(ext)));
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
            broadcast_changes(currentLastEdit);
            log.info("Updated " + wmPath);
        }
    }
    const connectedClients = new Set();
    const app = new Application();
    const router = new Router();
    // send a message to all connected clients
    function broadcast_changes(lastEdit) {
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
                let data = undefined;
                try {
                    data = JSON.parse(event.data);
                }
                catch {
                    console.log("Received invalid JSON: " + event.data);
                    return;
                }
                if (data.type == "load") {
                    loadPaths([data.path]);
                }
            };
        });
        app.use(router.routes());
        app.use(router.allowedMethods());
        const port = getAvailablePort({ preferredPort: 3001 });
        const url = `${workspace.remote}scripts/dev?workspace=${workspace.workspaceId}&local=true` +
            (port == PORT ? "" : "&port=" + port);
        console.log(`Go to ${url}`);
        try {
            await open.openApp(open.apps.browser, { arguments: [url] });
            log.info("Opened browser for you");
        }
        catch {
            console.error(`Failed to open browser, please navigate to ${url}`);
        }
        console.log("Dev server will automatically point to the last script edited locally");
        await app.listen({ port });
    }
    await Promise.all([startApp(), watchChanges()]);
    console.log("Stopped dev mode");
}
const command = new Command()
    .description("Launch a dev server that will spawn a webserver with HMR")
    .option("--includes <pattern...:string>", "Filter paths givena glob pattern or path")
    // deno-lint-ignore no-explicit-any
    .action(dev);
export default command;
