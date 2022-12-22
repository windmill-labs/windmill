// deno-lint-ignore-file no-explicit-any
import { colors, setClient } from "./deps.ts";
import { tryGetLoginInfo } from "./login.ts";
import { GlobalOptions } from "./types.ts";
import {
  getActiveWorkspace,
  getWorkspaceByName,
  Workspace,
} from "./workspace.ts";

export type Context = {
  workspace: string;
  baseUrl: string;
  urlStore: string;
  token: string;
};

async function tryResolveWorkspace(
  opts: GlobalOptions,
): Promise<
  { isError: false; value: Workspace } | { isError: true; error: string }
> {
  const cache = (opts as any).__secret_workspace;
  if (cache) return cache;

  if (opts.workspace) {
    const e = await getWorkspaceByName(opts.workspace);
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
  opts: GlobalOptions,
): Promise<Workspace> {
  const res = await tryResolveWorkspace(opts);
  if (res.isError) {
    console.log(res.error);
    return Deno.exit(-1);
  } else {
    return res.value;
  }
}

export async function requireLogin(opts: GlobalOptions) {
  const workspace = await resolveWorkspace(opts);

  let token = await tryGetLoginInfo(opts);
  if (!token) {
    token = workspace.token;
  }

  setClient(token, workspace.remote.substring(0, workspace.remote.length - 1));
}

export async function tryResolveVersion(
  opts: GlobalOptions,
): Promise<number | undefined> {
  if ((opts as any).__cache_version) {
    return (opts as any).__cache_version;
  }

  const workspaceRes = await tryResolveWorkspace(opts);
  if (workspaceRes.isError) return undefined;

  const response = await fetch(
    new URL(new URL(workspaceRes.value.remote).origin + "/api/version"),
  );
  const version = await response.text();
  try {
    return Number.parseInt(
      version.split("-", 1)[0].replaceAll(".", "").replace("v", ""),
    );
  } catch {
    return undefined;
  }
}

export async function validatePath(
  opts: GlobalOptions,
  path: string,
): Promise<boolean> {
  const backendVersion = await tryResolveVersion(opts);
  if (path.startsWith("f")) {
    if (!backendVersion || backendVersion >= 1550) {
      return true;
    }
    console.log(
      `Attempting to use folders, but the current remote does not have support. Remote version is ${backendVersion} but folders are supported from 1560.`,
    );
    return false;
  }

  if (
    !(path.startsWith("g") ||
      path.startsWith("u"))
  ) {
    console.log(
      colors.red(
        "Given remote path looks invalid. Remote paths are typicall of the form <u|g|f>/<username|group|folder>/...",
      ),
    );
    return false;
  }

  return true;
}
