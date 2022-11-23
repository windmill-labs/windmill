import { Command } from "https://deno.land/x/cliffy@v0.25.4/command/command.ts";
import { GlobalOptions } from "./types.ts";
import { WorkspaceService } from "https://deno.land/x/windmill@v1.50.0/windmill-api/index.ts";
import { colors } from "https://deno.land/x/cliffy@v0.25.4/ansi/colors.ts";
import { Table } from "https://deno.land/x/cliffy@v0.25.4/table/mod.ts";
import { getContext } from "./context.ts";

export async function getDefaultWorkspaceId(
  urlStore: string
): Promise<string | null> {
  try {
    return await Deno.readTextFile(urlStore + "default_workspace_id");
  } catch {
    return null;
  }
}

type ListOptions = GlobalOptions;
async function list(opts: ListOptions) {
  const { workspace: defaultId } = await getContext(opts);
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

type GetDefaultOptions = GlobalOptions;
async function getDefault(opts: GetDefaultOptions) {
  const { urlStore } = await getContext(opts);
  const id = await getDefaultWorkspaceId(urlStore);
  if (!id) {
    console.log(
      colors.red(
        "No default workspace set. Run windmill workspace set-default <workspace_id> to set."
      )
    );
    return;
  }
  const info = (await WorkspaceService.listWorkspaces()).find(
    (x) => x.id == id
  );
  if (!info) {
    console.log(
      colors.underline.red(
        "Default workspace is set, but cannot be found on remote. Maybe it has been deleted?"
      )
    );
    return;
  }
  console.log(colors.green("Id: " + info.id + " Name: " + info.name));
}

type SetDefaultOptions = GlobalOptions;
async function setDefault(opts: SetDefaultOptions, workspace_id: string) {
  if (opts.workspace) {
    console.log(
      colors.underline.bold.red(
        "!! --workspace option set, but this command expects the workspace to be passed as a positional argument. !!"
      )
    );
    return;
  }

  const { urlStore } = await getContext(opts);
  const info = (await WorkspaceService.listWorkspaces()).find(
    (x) => x.id == workspace_id
  );
  if (!info) {
    console.log(
      colors.underline.red(
        "Given workspace id " +
          workspace_id +
          " cannot be found on remote. Maybe it has been deleted?"
      )
    );
    return;
  }
  await Deno.writeTextFile(urlStore + "default_workspace_id", workspace_id);
}

const command = new Command()
  .description("workspace related commands")
  .action(list as any)
  .command("get-default", "get the current default workspace")
  .action(getDefault as any)
  .command("set-default", "set the current default workspace")
  .arguments("<workspace_id:string>")
  .action(setDefault as any);

export default command;
