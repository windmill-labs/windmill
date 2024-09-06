// deno-lint-ignore-file no-explicit-any
import * as dntShim from "./_dnt.shims.js";
import { isSuperset, parseFromFile, removeType, } from "./types.js";
import { requireLogin, resolveWorkspace } from "./context.js";
import { colors, Command, log, ResourceService, Table, } from "./deps.js";
export async function pushResourceType(workspace, remotePath, resource, localResource) {
    remotePath = removeType(remotePath, "resource-type");
    try {
        resource = await ResourceService.getResourceType({
            workspace: workspace,
            path: remotePath,
        });
    }
    catch {
        // resource type doesn't exist
    }
    if (resource) {
        if (isSuperset(localResource, resource)) {
            return;
        }
        await ResourceService.updateResourceType({
            workspace: workspace,
            path: remotePath,
            requestBody: {
                ...localResource,
            },
        });
    }
    else {
        log.info(colors.yellow.bold("Creating new resource type..."));
        await ResourceService.createResourceType({
            workspace: workspace,
            requestBody: {
                name: remotePath,
                ...localResource,
            },
        });
    }
}
async function push(opts, filePath, name) {
    const fstat = await dntShim.Deno.stat(filePath);
    if (!fstat.isFile) {
        throw new Error("file path must refer to a file.");
    }
    const workspace = await resolveWorkspace(opts);
    await requireLogin(opts);
    log.info(colors.bold.yellow("Pushing resource..."));
    await pushResourceType(workspace.workspaceId, name, undefined, parseFromFile(filePath));
    log.info(colors.bold.underline.green("Resource pushed"));
}
async function list(opts) {
    const workspace = await resolveWorkspace(opts);
    await requireLogin(opts);
    const res = await ResourceService.listResourceType({
        workspace: workspace.workspaceId,
    });
    new Table()
        .header(["Workspace", "Name"])
        .padding(2)
        .border(true)
        .body(res.map((x) => [x.workspace_id ?? "Global", x.name]))
        .render();
}
const command = new Command()
    .description("resource type related commands")
    .action(list)
    .command("push", "push a local resource spec. This overrides any remote versions.")
    .arguments("<file_path:string> <name:string>")
    .action(push);
export default command;
