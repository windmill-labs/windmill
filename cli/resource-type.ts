// deno-lint-ignore-file no-explicit-any
import {
  GlobalOptions,
  isSuperset,
  parseFromFile,
  removeType,
} from "./types.ts";
import { requireLogin } from "./auth.ts";
import { resolveWorkspace } from "./context.ts";
import { colors, Command, log, Table } from "./deps.ts";
import * as wmill from "./gen/services.gen.ts";
import { ResourceType } from "./gen/types.gen.ts";

export interface ResourceTypeFile {
  schema?: any;
  description?: string;
}

export async function pushResourceType(
  workspace: string,
  remotePath: string,
  resource: ResourceTypeFile | ResourceType | undefined,
  localResource: ResourceTypeFile
): Promise<void> {
  remotePath = removeType(remotePath, "resource-type");
  try {
    resource = await wmill.getResourceType({
      workspace: workspace,
      path: remotePath,
    });
  } catch {
    // resource type doesn't exist
  }

  if (resource) {
    if (isSuperset(localResource, resource)) {
      return;
    }

    await wmill.updateResourceType({
      workspace: workspace,
      path: remotePath,
      requestBody: {
        ...localResource,
      },
    });
  } else {
    log.info(colors.yellow.bold("Creating new resource type..."));
    await wmill.createResourceType({
      workspace: workspace,
      requestBody: {
        name: remotePath,
        ...localResource,
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

  log.info(colors.bold.yellow("Pushing resource..."));

  await pushResourceType(
    workspace.workspaceId,
    name,
    undefined,
    parseFromFile(filePath)
  );
  log.info(colors.bold.underline.green("Resource pushed"));
}

async function list(opts: GlobalOptions & { schema?: boolean }) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);
  const res = await wmill.listResourceType({
    workspace: workspace.workspaceId,
  });

  if (opts.schema) {
    new Table()
      .header(["Workspace", "Name", "Schema"])
      .padding(2)
      .border(true)
      .body(res.map((x) => [x.workspace_id ?? "Global", x.name, JSON.stringify(x.schema, null, 2)]))
      .render();
  } else {
    new Table()
      .header(["Workspace", "Name"])
      .padding(2)
      .border(true)
      .body(res.map((x) => [x.workspace_id ?? "Global", x.name]))
      .render();
  }
}

const command = new Command()
  .description("resource type related commands")
  .action(() =>
    log.info("2 actions available, list and push.")
  )
  .command("list", "list all resource types")
  .option("--schema", "Show schema in the output")
  .action(list as any)
  .command(
    "push",
    "push a local resource spec. This overrides any remote versions."
  )
  .arguments("<file_path:string> <name:string>")
  .action(push as any);

export default command;
