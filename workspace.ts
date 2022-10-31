import { Command } from "https://deno.land/x/cliffy@v0.25.4/command/command.ts";
import { setClient } from "https://deno.land/x/windmill@v1.41.0/mod.ts";
import { GlobalOptions } from "./types.ts";
import {
  OpenFlowWPath,
  WorkspaceService,
} from "https://deno.land/x/windmill@v1.41.0/windmill-api/index.ts";
import { getToken } from "./login.ts";
import { colors } from "https://deno.land/x/cliffy@v0.25.4/ansi/colors.ts";
import { Table } from "https://deno.land/x/cliffy@v0.25.4/table/mod.ts";
import { getStore } from "./store.ts";

export async function getDefaultId(baseUrl: string): Promise<string | null> {
  const baseStore = await getStore(baseUrl);
  try {
    return await Deno.readTextFile(baseStore + "default_workspace_id");
  } catch {
    return null;
  }
}

type ListOptions = GlobalOptions;
async function list({ baseUrl }: ListOptions) {
  setClient(await getToken(baseUrl), baseUrl);
  const defaultId = await getDefaultId(baseUrl);
  const workspaces = await WorkspaceService.listWorkspaces();
  new Table()
    .header(["id", "name"])
    .body(
      workspaces.map((w) => {
        if (w.id == defaultId)
          return [colors.underline.green(w.id), colors.underline.green(w.name)];
        else return [w.id, w.name];
      })
    )
    .padding(2)
    .align("center")
    .border(true)
    .render();
}

const command = new Command()
  .description("workspace related commands")
  //.command("default", "get the current default workspace")
  //.action(get_default as any)
  //.command("default", "set the current default workspace")
  //.arguments("<workspace_id:string>")
  //.action(set_default as any)
  .command("list", "list available workspaces")
  .action(list as any);

export default command;
