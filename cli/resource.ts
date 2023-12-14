// deno-lint-ignore-file no-explicit-any
import {
  GlobalOptions,
  isSuperset,
  parseFromFile,
  removeType,
} from "./types.ts";
import { requireLogin, resolveWorkspace, validatePath } from "./context.ts";
import {
  colors,
  Command,
  log,
  Resource,
  ResourceService,
  Table,
} from "./deps.ts";

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
  try {
    resource = await ResourceService.getResource({
      workspace: workspace,
      path: remotePath.replaceAll("\\", "/"),
    });
  } catch {
    // flow doesn't exist
  }

  if (resource) {
    if (isSuperset(localResource, resource)) {
      return;
    }

    await ResourceService.updateResource({
      workspace: workspace,
      path: remotePath.replaceAll("\\", "/"),
      requestBody: { ...localResource },
    });
  } else {
    if (localResource.is_oauth) {
      log.info(
        colors.yellow(
          "! is_oauth has been removed in newer versions. Ignoring."
        )
      );
    }

    log.info(colors.yellow.bold("Creating new resource..."));
    await ResourceService.createResource({
      workspace: workspace,
      requestBody: {
        path: remotePath.replaceAll("\\", "/"),
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

  log.info(colors.bold.yellow("Pushing resource..."));

  await pushResource(
    workspace.workspaceId,
    remotePath,
    undefined,
    parseFromFile(filePath)
  );
  log.info(colors.bold.underline.green(`Resource ${remotePath} pushed`));
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
