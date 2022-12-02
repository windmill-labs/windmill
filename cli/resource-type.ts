import { Command } from "https://deno.land/x/cliffy@v0.25.4/command/command.ts";
import { ResourceService } from "https://deno.land/x/windmill@v1.50.0/mod.ts";
import { GlobalOptions } from "./types.ts";
import { colors } from "https://deno.land/x/cliffy@v0.25.4/ansi/colors.ts";
import { getContext } from "./context.ts";
import { Table } from "https://deno.land/x/cliffy@v0.25.4/table/table.ts";

type ResourceTypeFile = {
  schema?: any;
  description?: string;
};

export async function pushResourceType(
  workspace: string,
  filePath: string,
  name: string,
) {
  const data: ResourceTypeFile = JSON.parse(await Deno.readTextFile(filePath));
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
  const { workspace } = await getContext(opts);

  const fstat = await Deno.stat(filePath);
  if (!fstat.isFile) {
    throw new Error("file path must refer to a file.");
  }

  console.log(colors.bold.yellow("Pushing resource..."));

  await pushResourceType(workspace, filePath, name);
  console.log(colors.bold.underline.green("Resource successfully pushed"));
}

async function list(opts: GlobalOptions) {
  const { workspace } = await getContext(opts);
  const res = await ResourceService.listResourceType({
    workspace,
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
