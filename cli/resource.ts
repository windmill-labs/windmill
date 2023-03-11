import {
  Difference,
  GlobalOptions,
  PushDiffs,
  Resource as Resource2,
  setValueByPath,
} from "./types.ts";
import { requireLogin, resolveWorkspace, validatePath } from "./context.ts";
import {
  colors,
  Command,
  EditResource,
  microdiff,
  Resource,
  ResourceService,
  Table,
} from "./deps.ts";
import { Any, decoverto, model, property } from "./decoverto.ts";

@model()
export class ResourceFile implements Resource2, PushDiffs {
  @property(Any)
  value?: any;
  @property(() => String)
  description?: string;
  @property(() => String)
  resource_type: string;
  @property(() => Boolean)
  is_oauth?: boolean; // deprecated

  constructor(resource_type: string) {
    this.resource_type = resource_type;
  }
  async pushDiffs(
    workspace: string,
    remotePath: string,
    diffs: Difference[],
  ): Promise<void> {
    if (
      await ResourceService.existsResource({
        workspace: workspace,
        path: remotePath,
      })
    ) {
      console.log(
        colors.yellow.bold(`Applying ${diffs.length} diffs to existing resource... ${remotePath}`),
      );

      const changeset: EditResource = {
        path: remotePath, // TODO: Remove this in backend
      };
      for (const diff of diffs) {
        if (diff.path[0] === "is_oauth") {
          console.log(
            colors.yellow(
              "! is_oauth has been removed in newer versions. Ignoring.",
            ),
          );
          continue;
        }
        if (
          diff.type !== "REMOVE" &&
          (
            diff.path[0] !== "value" && (
              diff.path.length !== 1 ||
              diff.path[0] !== "description"
            ) && diff.path[0] !== "resource_type"
          )
        ) {
          console.log(colors.red("Invalid variable diff with path " + diff.path));
          throw new Error("Invalid folder diff with path " + diff.path);
        }
        if (diff.type === "CREATE" || diff.type === "CHANGE") {
          setValueByPath(changeset, diff.path, diff.value);
        } else if (diff.type === "REMOVE") {
          setValueByPath(changeset, diff.path, null);
        }
      }

      const hasChanges = Object.values(changeset).some((v) =>
        v !== null && typeof v !== "undefined"
      );
      if (!hasChanges) {
        return;
      }

      await ResourceService.updateResource({
        workspace: workspace,
        path: remotePath,
        requestBody: changeset,
      });
    } else {
      if (typeof this.is_oauth !== "undefined") {
        console.log(
          colors.yellow(
            "! is_oauth has been removed in newer versions. Ignoring.",
          ),
        );
      }

      console.log(colors.yellow.bold("Creating new resource..."));
      await ResourceService.createResource({
        workspace: workspace,
        requestBody: {
          path: remotePath,
          resource_type: this.resource_type,
          value: this.value,
          description: this.description,
        },
      });
    }
  }
  async push(workspace: string, remotePath: string): Promise<void> {
    await this.pushDiffs(
      workspace,
      remotePath,
      microdiff({}, this, { cyclesFix: false }),
    );
  }
}

export async function pushResource(
  workspace: string,
  filePath: string,
  remotePath: string,
) {
  const data = decoverto.type(ResourceFile).rawToInstance(
    await Deno.readTextFile(filePath),
  );
  await data.push(workspace, remotePath);
}

type PushOptions = GlobalOptions;
async function push(opts: PushOptions, filePath: string, remotePath: string) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  if (!await validatePath(opts, remotePath)) {
    return;
  }

  const fstat = await Deno.stat(filePath);
  if (!fstat.isFile) {
    throw new Error("file path must refer to a file.");
  }

  console.log(colors.bold.yellow("Pushing resource..."));

  await pushResource(workspace.workspaceId, filePath, remotePath);
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
    "push a local resource spec. This overrides any remote versions.",
  )
  .arguments("<file_path:string> <remote_path:string>")
  .action(push as any);

export default command;
