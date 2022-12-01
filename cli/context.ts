// deno-lint-ignore-file no-explicit-any
import { colors } from "https://deno.land/x/cliffy@v0.25.4/ansi/colors.ts";
import { setClient } from "https://deno.land/x/windmill@v1.50.0/mod.ts";
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

export async function resolveWorkspace(
  opts: GlobalOptions
): Promise<Workspace> {
  const cache = (opts as any).__secret_workspace;
  if (cache) return cache;

  if (opts.workspace) {
    const e = await getWorkspaceByName(opts.workspace);
    if (!e) {
      console.log(colors.red.underline("Given workspace does not exist."));
      return Deno.exit(-1);
    }
    (opts as any).__secret_workspace = e;
    return e;
  }

  const defaultWorkspace = await getActiveWorkspace(opts);
  if (!defaultWorkspace) {
    console.log(colors.red.underline("No workspace given and no default set."));
    return Deno.exit(-3);
  }

  return defaultWorkspace;
}

export async function requireLogin(opts: GlobalOptions) {
  const workspace = await resolveWorkspace(opts);

  const token = await tryGetLoginInfo(opts);

  if (token) {
    setClient(token, workspace.remote);
  } else {
    console.log(colors.red.underline("You need to be logged in to do this."));
    Deno.exit(-2);
  }
}
