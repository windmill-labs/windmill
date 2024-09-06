// deno-lint-ignore-file no-explicit-any
import * as dntShim from "./_dnt.shims.js";
import { requireLogin, resolveWorkspace, validatePath } from "./context.js";
import { AppService, colors, Command, log, SEP, Table, yamlParse, } from "./deps.js";
import { isSuperset } from "./types.js";
import { readInlinePathSync } from "./utils.js";
const alreadySynced = [];
export async function pushApp(workspace, remotePath, localPath, message) {
    if (alreadySynced.includes(localPath)) {
        return;
    }
    alreadySynced.push(localPath);
    remotePath.replaceAll(SEP, "/");
    let app = undefined;
    // deleting old app if it exists in raw mode
    try {
        app = await AppService.getAppByPath({
            workspace,
            path: remotePath,
        });
    }
    catch {
        //ignore
    }
    if (!localPath.endsWith(SEP)) {
        localPath += SEP;
    }
    const localAppRaw = await dntShim.Deno.readTextFile(localPath + "app.yaml");
    const localApp = yamlParse(localAppRaw);
    function replaceInlineScripts(rec) {
        if (!rec) {
            return;
        }
        if (typeof rec == "object") {
            return Object.entries(rec).flatMap(([k, v]) => {
                if (k == "inlineScript" && typeof v == "object") {
                    const o = v;
                    if (o["content"] && o["content"].startsWith("!inline")) {
                        const basePath = localPath + o["content"].split(" ")[1];
                        o["content"] = readInlinePathSync(basePath);
                    }
                    if (o["lock"] && o["lock"].startsWith("!inline")) {
                        const basePath = localPath + o["lock"].split(" ")[1];
                        o["lock"] = readInlinePathSync(basePath);
                    }
                }
                else {
                    replaceInlineScripts(v);
                }
            });
        }
        return [];
    }
    replaceInlineScripts(localApp.value);
    if (app) {
        if (isSuperset(localApp, app)) {
            log.info(colors.green(`App ${remotePath} is up to date`));
            return;
        }
        log.info(colors.bold.yellow(`Updating app ${remotePath}...`));
        await AppService.updateApp({
            workspace,
            path: remotePath,
            requestBody: {
                deployment_message: message,
                ...localApp,
            },
        });
    }
    else {
        log.info(colors.yellow.bold("Creating new app..."));
        await AppService.createApp({
            workspace,
            requestBody: {
                path: remotePath,
                deployment_message: message,
                ...localApp,
            },
        });
    }
}
async function list(opts) {
    const workspace = await resolveWorkspace(opts);
    await requireLogin(opts);
    let page = 0;
    const perPage = 10;
    const total = [];
    while (true) {
        const res = await AppService.listApps({
            workspace: workspace.workspaceId,
            page,
            perPage,
            includeDraftOnly: opts.includeDraftOnly ?? false,
        });
        page += 1;
        total.push(...res);
        if (res.length < perPage) {
            break;
        }
    }
    new Table()
        .header(["path", "summary"])
        .padding(2)
        .border(true)
        .body(total.map((x) => [x.path, x.summary]))
        .render();
}
async function push(opts, filePath, remotePath) {
    if (!validatePath(remotePath)) {
        return;
    }
    const workspace = await resolveWorkspace(opts);
    await requireLogin(opts);
    await pushApp(workspace.workspaceId, remotePath, filePath);
    log.info(colors.bold.underline.green("Flow pushed"));
}
const command = new Command()
    .description("app related commands")
    .action(list)
    .command("push", "push a local app ")
    .arguments("<file_path:string> <remote_path:string>")
    .action(push);
export default command;
