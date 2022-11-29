import { parse } from "https://deno.land/std@0.141.0/datetime/mod.ts";
import { colors } from "https://deno.land/x/cliffy@v0.25.4/ansi/colors.ts";
import {
  setClient,
  UserService,
} from "https://deno.land/x/windmill@v1.50.0/mod.ts";
import { getToken } from "./login.ts";
import { getDefaultRemote, getRemote } from "./remote.ts";
import { getStore } from "./store.ts";
import { GlobalOptions } from "./types.ts";
import { getDefaultWorkspaceId } from "./workspace.ts";

export type Context = {
  workspace: string;
  baseUrl: string;
  urlStore: string;
  token: string;
};

export async function getContext({
  baseUrl,
  remote,
  workspace,
  token,
  email,
  password,
}: GlobalOptions): Promise<Context> {
  if (remote) {
    baseUrl = baseUrl ?? (await getRemote(remote))?.baseUrl;
  }
  baseUrl = baseUrl ?? (await getDefaultRemote())?.baseUrl;
  baseUrl = baseUrl ?? "https://app.windmill.dev";
  const parsedUrl = new URL(baseUrl);
  let urlWorkspace: string | undefined = parsedUrl.username;
  if (urlWorkspace == "") urlWorkspace = undefined;
  parsedUrl.username = "";
  baseUrl = parsedUrl.toString();
  baseUrl = baseUrl.substring(0, baseUrl.length - 1);
  if (email && password) {
    setClient("no-token", baseUrl);
    token =
      token ?? (await UserService.login({ requestBody: { email, password } }));
  }
  token = token ?? (await getToken(baseUrl));
  setClient(token, baseUrl);
  const urlStore = await getStore(baseUrl);
  const workspaceId =
    workspace ?? urlWorkspace ?? (await getDefaultWorkspaceId(urlStore));
  if (!workspaceId) {
    console.log(colors.red("No default workspace set and no override given."));
    Deno.exit(-2);
  }
  return {
    workspace: workspaceId,
    baseUrl: baseUrl,
    urlStore: urlStore,
    token: token,
  };
}
