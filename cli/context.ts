// deno-lint-ignore-file no-explicit-any
import { colors, log, setClient } from "./deps.ts";
import * as wmill from "./gen/services.gen.ts";
import { GlobalUserInfo } from "./gen/types.gen.ts";

import { loginInteractive, tryGetLoginInfo } from "./login.ts";
import { GlobalOptions } from "./types.ts";
import { getHeaders } from "./utils.ts";
import {
  addWorkspace,
  getActiveWorkspace,
  getWorkspaceByName,
  removeWorkspace,
  Workspace,
} from "./workspace.ts";

export type Context = {
  workspace: string;
  baseUrl: string;
  urlStore: string;
  token: string;
};

async function tryResolveWorkspace(
  opts: GlobalOptions
): Promise<
  { isError: false; value: Workspace } | { isError: true; error: string }
> {
  const cache = (opts as any).__secret_workspace;
  if (cache) return { isError: false, value: cache };

  if (opts.workspace) {
    const e = await getWorkspaceByName(opts.workspace, opts.configDir);
    if (!e) {
      return {
        isError: true,
        error: colors.red.underline("Given workspace does not exist."),
      };
    }
    (opts as any).__secret_workspace = e;
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

export async function resolveWorkspace(
  opts: GlobalOptions
): Promise<Workspace> {
  if (opts.baseUrl) {
    if (opts.workspace && opts.token) {
      const normalizedBaseUrl = new URL(opts.baseUrl).toString(); // add trailing slash if not present
      
      // Try to find existing workspace profile by name, then by workspaceId + remote
      if (opts.workspace) {
        // Try by workspace name first
        let existingWorkspace = await getWorkspaceByName(opts.workspace, opts.configDir);
        
        // If not found by name, try to find by workspaceId + remote match
        if (!existingWorkspace) {
          const { allWorkspaces } = await import("./workspace.ts");
          const workspaces = await allWorkspaces(opts.configDir);
          const matchingWorkspaces = workspaces.filter(
            w => w.workspaceId === opts.workspace && w.remote === normalizedBaseUrl
          );
          
          // Due to uniqueness constraint, there can only be 0 or 1 match
          if (matchingWorkspaces.length === 1) {
            existingWorkspace = matchingWorkspaces[0];
          }
        }
        
        if (existingWorkspace) {
          // Validate that the base URL matches the profile's remote
          if (existingWorkspace.remote !== normalizedBaseUrl) {
            log.info(
              colors.red(
                `Base URL mismatch: --base-url is ${normalizedBaseUrl} but workspace profile "${opts.workspace}" uses ${existingWorkspace.remote}`
              )
            );
            return Deno.exit(-1);
          }
          // Use the existing workspace profile (preserves workspace name)
          return {
            ...existingWorkspace,
            token: opts.token, // Use the provided token
          };
        }
      }
      
      // No existing profile found, create temporary workspace
      return {
        remote: normalizedBaseUrl,
        workspaceId: opts.workspace,
        name: opts.workspace,
        token: opts.token,
      };
    } else {
      log.info(
        colors.red(
          "If you specify a base URL with --base-url, you must also specify a workspace (--workspace) and token (--token)."
        )
      );
      return Deno.exit(-1);
    }
  }
  const res = await tryResolveWorkspace(opts);
  if (res.isError) {
    log.info(colors.red.bold(res.error));
    return Deno.exit(-1);
  } else {
    return res.value;
  }
}

export async function requireLogin(
  opts: GlobalOptions
): Promise<GlobalUserInfo> {
  const workspace = await resolveWorkspace(opts);
  let token = await tryGetLoginInfo(opts);

  if (!token) {
    token = workspace.token;
  }

  setClient(token, workspace.remote.substring(0, workspace.remote.length - 1));

  try {
    return await wmill.globalWhoami();
  } catch (error) {
    // Check for network errors and provide clearer messages
    const errorMsg = error instanceof Error ? error.message : String(error);
    if (errorMsg.includes('fetch') || errorMsg.includes('connection') || errorMsg.includes('ECONNREFUSED') || errorMsg.includes('refused')) {
      throw new Error(`Network error: Could not connect to Windmill server at ${workspace.remote}`);
    }
    
    log.info(
      "! Could not reach API given existing credentials. Attempting to reauth..."
    );
    const newToken = await loginInteractive(workspace.remote);
    if (!newToken) {
      throw new Error("Unauthorized: Could not authenticate with the provided credentials");
    }
    removeWorkspace(workspace.name, false, opts);
    workspace.token = newToken;
    addWorkspace(workspace, opts);

    setClient(
      newToken,
      workspace.remote.substring(0, workspace.remote.length - 1)
    );
    return await wmill.globalWhoami();
  }
}

export async function fetchVersion(baseUrl: string): Promise<string> {
  const requestHeaders = new Headers();

  const extraHeaders = getHeaders();
  if (extraHeaders) {
    for (const [key, value] of Object.entries(extraHeaders)) {
      // @ts-ignore
      requestHeaders.set(key, value);
    }
  }

  const response = await fetch(
    new URL(new URL(baseUrl).origin + "/api/version"),
    { headers: requestHeaders, method: "GET" }
  );
  
  if (!response.ok) {
    // Consume response body even on error to avoid resource leak
    await response.text();
    throw new Error(`Failed to fetch version: ${response.status} ${response.statusText}`);
  }
  
  return await response.text();
}
export async function tryResolveVersion(
  opts: GlobalOptions
): Promise<number | undefined> {
  if ((opts as any).__cache_version) {
    return (opts as any).__cache_version;
  }

  const workspaceRes = await tryResolveWorkspace(opts);
  if (workspaceRes.isError) return undefined;
  const version = await fetchVersion(workspaceRes.value.remote);

  try {
    return Number.parseInt(
      version.split("-", 1)[0].replaceAll(".", "").replace("v", "")
    );
  } catch {
    return undefined;
  }
}

export function validatePath(path: string): boolean {
  if (!(path.startsWith("g") || path.startsWith("u") || path.startsWith("f"))) {
    log.info(
      colors.red(
        "Given remote path looks invalid. Remote paths are typically of the form <u|g|f>/<username|group|folder>/..."
      )
    );
    return false;
  }

  return true;
}
