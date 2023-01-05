// deno-lint-ignore-file no-explicit-any
import { GlobalOptions } from "./types.ts";
import { requireLogin, resolveWorkspace } from "./context.ts";
import { colors, Command, ResourceService, Table } from "./deps.ts";
import { Any, decoverto, model, property } from "./decoverto.ts";

@model()
export class ResourceTypeFile {
  @property(Any)
  schema?: any;
  @property(() => String)
  description?: string;
}

export async function pushResourceType(
  workspace: string,
  filePath: string,
  name: string,
) {
  const data: ResourceTypeFile = decoverto.type(ResourceTypeFile).rawToInstance(
    await Deno.readTextFile(filePath),
  );
  await pushResourceTypeDef(workspace, name, data);
}

export async function pushResourceTypeDef(
  workspace: string,
  name: string,
  data: ResourceTypeFile,
) {
  if (
    await ResourceService.existsResourceType({
      workspace: workspace,
      path: name,
    })
  ) {
    console.log(colors.yellow("Updating existing resource type..."));
    if (
      (await ResourceService.listResourceType({ workspace })).findIndex((x) =>
        x.name === name
      ) === -1
    ) {
      console.log(
        "Resource type " + name +
          " is already taken for the current workspace, but cannot be updated. Is this a conflict with starter?",
      );
      return;
    }
    await ResourceService.updateResourceType({
      workspace: workspace,
      path: name,
      requestBody: {
        description: data.description,
        schema: data.schema,
      },
    });
  } else {
    console.log(colors.yellow("Creating new resource type..."));
    await ResourceService.createResourceType({
      workspace: workspace,
      requestBody: {
        name: name,
        description: data.description,
        schema: data.schema,
        workspace_id: workspace,
      },
    });
  }
}

type PushOptions = GlobalOptions;
async function push(opts: PushOptions, filePath: string, name: string) {
  const fstat = await Deno.stat(filePath);
  if (!fstat.isFile) {
    throw new Error("file path must refer to a file.");
  }
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  console.log(colors.bold.yellow("Pushing resource..."));

  await pushResourceType(workspace.workspaceId, filePath, name);
  console.log(colors.bold.underline.green("Resource successfully pushed"));
}

async function list(opts: GlobalOptions) {
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
  .action(list as any)
  .command(
    "push",
    "push a local resource spec. This overrides any remote versions.",
  )
  .arguments("<file_path:string> <name:string>")
  .action(push as any);

export default command;
