// deno-lint-ignore-file no-explicit-any
import * as dntShim from "./_dnt.shims.js";
import { requireLogin, resolveWorkspace, validatePath } from "./context.js";
import { isSuperset, parseFromFile, removeType, } from "./types.js";
import { colors, Command, Confirm, log, SEP, Table, VariableService, } from "./deps.js";
async function list(opts) {
    const workspace = await resolveWorkspace(opts);
    await requireLogin(opts);
    const variables = await VariableService.listVariable({
        workspace: workspace.workspaceId,
    });
    new Table()
        .header(["Path", "Is Secret", "Account", "Value"])
        .padding(2)
        .border(true)
        .body(variables.map((x) => [
        x.path,
        x.is_secret ? "true" : "false",
        x.account ?? "-",
        x.value ?? "-",
    ]))
        .render();
}
export async function pushVariable(workspace, remotePath, variable, localVariable, plainSecrets) {
    remotePath = removeType(remotePath, "variable");
    log.debug(`Processing local variable ${remotePath}`);
    try {
        variable = await VariableService.getVariable({
            workspace: workspace,
            path: remotePath.replaceAll(SEP, "/"),
            decryptSecret: plainSecrets,
            includeEncrypted: true,
        });
        log.debug(`Variable ${remotePath} exists on remote`);
    }
    catch {
        log.debug(`Variable ${remotePath} does not exist on remote`);
    }
    if (variable) {
        if (isSuperset(localVariable, variable)) {
            log.debug(`Variable ${remotePath} is up-to-date`);
            return;
        }
        log.debug(`Variable ${remotePath} is not up-to-date, updating`);
        await VariableService.updateVariable({
            workspace,
            path: remotePath.replaceAll(SEP, "/"),
            alreadyEncrypted: !plainSecrets,
            requestBody: {
                ...localVariable,
                is_secret: localVariable.is_secret && !variable.is_secret ? true : undefined,
            },
        });
    }
    else {
        log.info(colors.yellow.bold(`Creating new variable ${remotePath}...`));
        await VariableService.createVariable({
            workspace,
            alreadyEncrypted: !plainSecrets,
            requestBody: {
                path: remotePath.replaceAll(SEP, "/"),
                ...localVariable,
            },
        });
    }
}
async function push(opts, filePath, remotePath) {
    const workspace = await resolveWorkspace(opts);
    await requireLogin(opts);
    if (!validatePath(remotePath)) {
        return;
    }
    const fstat = await dntShim.Deno.stat(filePath);
    if (!fstat.isFile) {
        throw new Error("file path must refer to a file.");
    }
    log.info(colors.bold.yellow("Pushing variable..."));
    await pushVariable(workspace.workspaceId, remotePath, undefined, parseFromFile(filePath), opts.plainSecrets ?? false);
    log.info(colors.bold.underline.green(`Variable ${remotePath} pushed`));
}
async function add(opts, value, remotePath) {
    const workspace = await resolveWorkspace(opts);
    await requireLogin(opts);
    if (!validatePath(remotePath)) {
        return;
    }
    if (await VariableService.existsVariable({
        workspace: workspace.workspaceId,
        path: remotePath,
    })) {
        if (!(await Confirm.prompt({
            message: `Variable already exist, do you want to update its value?`,
            default: true,
        }))) {
            return;
        }
        log.info(colors.bold.yellow("Updating variable..."));
    }
    log.info(colors.bold.yellow("Pushing variable..."));
    await pushVariable(workspace.workspaceId, remotePath + ".variable.yaml", undefined, {
        value,
        is_secret: !opts.public,
        description: "",
    }, opts.plainSecrets ?? false);
    log.info(colors.bold.underline.green(`Variable ${remotePath} pushed`));
}
const command = new Command()
    .description("variable related commands")
    .action(list)
    .command("push", "Push a local variable spec. This overrides any remote versions.")
    .arguments("<file_path:string> <remote_path:string>")
    .option("--plain-secrets", "Push secrets as plain text")
    .action(push)
    .command("add", "Create a new variable on the remote. This will update the variable if it already exists.")
    .arguments("<value:string> <remote_path:string>")
    .option("--public", "Make a public variable")
    .action(add);
export default command;
