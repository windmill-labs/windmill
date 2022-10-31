import { colors } from "https://deno.land/x/cliffy@v0.25.4/ansi/colors.ts";
import {
  setClient,
  UserService,
} from "https://deno.land/x/windmill@v1.41.0/mod.ts";
import { getToken } from "./login.ts";
import { getStore } from "./store.ts";
import { GlobalOptions } from "./types.ts";
import { getDefaultWorkspaceId } from "./workspace.ts";

export type Context = {
  workspace: string;
  baseUrl: string;
  urlStore: string;
};

export async function getContext({
  baseUrl,
  workspace,
  token,
  email,
  password,
}: GlobalOptions): Promise<Context> {
  if (email && password) {
    token =
      token ?? (await UserService.login({ requestBody: { email, password } }));
  }
  token = token ?? (await getToken(baseUrl));
  setClient(token, baseUrl);
  const workspaceId = workspace ?? (await getDefaultWorkspaceId(baseUrl));
  if (!workspaceId) {
    console.log(colors.red("No default workspace set and no override given."));
    Deno.exit(-2);
  }
  const urlStore = await getStore(baseUrl);
  return {
    workspace: workspaceId,
    baseUrl: baseUrl,
    urlStore: urlStore,
  };
}
