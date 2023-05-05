// deno-lint-ignore-file no-explicit-any
import {
  GlobalOptions,
  isSuperset,
  parseFromFile,
  removeType,
} from "./types.ts";
import { requireLogin, resolveWorkspace, validatePath } from "./context.ts";
import { colors, Command, Resource, ResourceService, Table } from "./deps.ts";

export interface ResourceFile {
  value: any;
  description?: string;
  resource_type: string;
  is_oauth?: boolean; // deprecated
}

export async function pushResource(
  workspace: string,
  remotePath: string,
  resource: ResourceFile | Resource | undefined,
  localResource: ResourceFile
): Promise<void> {
  remotePath = removeType(remotePath, "resource");

  if (resource) {
    if (isSuperset(localResource, resource)) {
      return;
    }

    await ResourceService.updateResource({
      workspace: workspace,
      path: remotePath,
      requestBody: { ...localResource },
    });
  } else {
    if (localResource.is_oauth) {
      console.log(
        colors.yellow(
          "! is_oauth has been removed in newer versions. Ignoring."
        )
      );
    }

    console.log(colors.yellow.bold("Creating new resource..."));
    await ResourceService.createResource({
      workspace: workspace,
      requestBody: {
        path: remotePath,
        ...localResource,
      },
    });
  }
}

type PushOptions = GlobalOptions;
async function push(opts: PushOptions, filePath: string, remotePath: string) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  if (!validatePath(remotePath)) {
    return;
  }

  const fstat = await Deno.stat(filePath);
  if (!fstat.isFile) {
    throw new Error("file path must refer to a file.");
  }

  console.log(colors.bold.yellow("Pushing resource..."));
  let resource: Resource | undefined = undefined;
  try {
    resource = await ResourceService.getResource({
      workspace: workspace.workspaceId,
      path: remotePath,
    });
  } catch {
    // flow doesn't exist
  }

  await pushResource(
    workspace.workspaceId,
    remotePath,
    resource,
    parseFromFile(filePath)
  );
  console.log(colors.bold.underline.green(`Resource ${remotePath} pushed`));
}

async function list(opts: GlobalOptions) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);
  let page = 0;
  const perPage = 10;
  const total: Resource[] = [];
  while (true) {
    const res = await ResourceService.listResource({
      workspace: workspace.workspaceId,
      page,
      perPage,
    });
    total.push(...res);
    page += 1;
    if (res.length < perPage) {
      break;
    }
  }

  new Table()
    .header(["Path", "Resource Type"])
    .padding(2)
    .border(true)
    .body(total.map((x) => [x.path, x.resource_type]))
    .render();
}

const command = new Command()
  .description("resource related commands")
  .action(list as any)
  .command(
    "push",
    "push a local resource spec. This overrides any remote versions."
  )
  .arguments("<file_path:string> <remote_path:string>")
  .action(push as any);

export default command;
