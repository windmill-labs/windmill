// deno-lint-ignore-file no-explicit-any
import * as dntShim from "./_dnt.shims.js";
import { colors, log, setClient, UserService } from "./deps.js";
import { loginInteractive, tryGetLoginInfo } from "./login.js";
import { getHeaders } from "./utils.js";
import { addWorkspace, getActiveWorkspace, getWorkspaceByName, removeWorkspace, } from "./workspace.js";
async function tryResolveWorkspace(opts) {
    const cache = opts.__secret_workspace;
    if (cache)
        return { isError: false, value: cache };
    if (opts.workspace) {
        const e = await getWorkspaceByName(opts.workspace);
        if (!e) {
            return {
                isError: true,
                error: colors.red.underline("Given workspace does not exist."),
            };
        }
        opts.__secret_workspace = e;
        return { isError: false, value: e };
    }
    const defaultWorkspace = await getActiveWorkspace(opts);
    if (!defaultWorkspace) {
        return {
            isError: true,
            error: colors.red.underline("No workspace given and no default set."),
        };
    }
    return { isError: false, value: defaultWorkspace };
}
export async function resolveWorkspace(opts) {
    const res = await tryResolveWorkspace(opts);
    if (res.isError) {
        log.info(colors.red.bold(res.error));
        return dntShim.Deno.exit(-1);
    }
    else {
        return res.value;
    }
}
export async function requireLogin(opts) {
    const workspace = await resolveWorkspace(opts);
    let token = await tryGetLoginInfo(opts);
    if (!token) {
        token = workspace.token;
    }
    setClient(token, workspace.remote.substring(0, workspace.remote.length - 1));
    try {
        return await UserService.globalWhoami();
    }
    catch {
        log.info("! Could not reach API given existing credentials. Attempting to reauth...");
        const newToken = await loginInteractive(workspace.remote);
        if (!newToken) {
            throw new Error("Could not reauth");
        }
        removeWorkspace(workspace.name, false, opts);
        workspace.token = newToken;
        addWorkspace(workspace, opts);
        setClient(token, workspace.remote.substring(0, workspace.remote.length - 1));
        return await UserService.globalWhoami();
    }
}
export async function fetchVersion(baseUrl) {
    const requestHeaders = new Headers();
    const extraHeaders = getHeaders();
    if (extraHeaders) {
        for (const [key, value] of Object.entries(extraHeaders)) {
            // @ts-ignore
            requestHeaders.set(key, value);
        }
    }
    const response = await fetch(new URL(new URL(baseUrl).origin + "/api/version"), { headers: requestHeaders, method: "GET" });
    return await response.text();
}
export async function tryResolveVersion(opts) {
    if (opts.__cache_version) {
        return opts.__cache_version;
    }
    const workspaceRes = await tryResolveWorkspace(opts);
    if (workspaceRes.isError)
        return undefined;
    const version = await fetchVersion(workspaceRes.value.remote);
    try {
        return Number.parseInt(version.split("-", 1)[0].replaceAll(".", "").replace("v", ""));
    }
    catch {
        return undefined;
    }
}
export function validatePath(path) {
    if (!(path.startsWith("g") || path.startsWith("u") || path.startsWith("f"))) {
        log.info(colors.red("Given remote path looks invalid. Remote paths are typically of the form <u|g|f>/<username|group|folder>/..."));
        return false;
    }
    return true;
}
