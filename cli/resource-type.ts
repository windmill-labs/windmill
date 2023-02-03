// deno-lint-ignore-file no-explicit-any
import {
  Difference,
  GlobalOptions,
  PushDiffs,
  Resource as ResourceI,
  setValueByPath,
} from "./types.ts";
import { requireLogin, resolveWorkspace } from "./context.ts";
import {
  colors,
  Command,
  EditResourceType,
  microdiff,
  ResourceService,
  ResourceType,
  Table,
} from "./deps.ts";
import { Any, decoverto, model, property } from "./decoverto.ts";

@model()
export class ResourceTypeFile implements ResourceI, PushDiffs {
  @property(Any)
  schema?: any;
  @property(() => String)
  description?: string;

  async push(workspace: string, remotePath: string): Promise<void> {
    let existing: ResourceType | undefined;
    try {
      existing = await ResourceService.getResourceType({
        workspace,
        path: remotePath,
      });
    } catch {
      existing = undefined;
    }
    this.pushDiffs(
      workspace,
      remotePath,
      microdiff(existing ?? {}, this, { cyclesFix: false }),
    );
  }

  async pushDiffs(
    workspace: string,
    remotePath: string,
    diffs: Difference[],
  ): Promise<void> {
    if (
      await ResourceService.existsResourceType({
        workspace: workspace,
        path: remotePath,
      })
    ) {
      if (
        (await ResourceService.listResourceType({ workspace })).findIndex((x) =>
          x.name === remotePath
        ) === -1
      ) {
        console.log(
          "Resource type " + remotePath +
            " is already taken for the current workspace, but cannot be updated. Is this a conflict with starter?",
        );
        return;
      }
      console.log(
        colors.yellow(
          `Applying ${diffs.length} diffs to existing resource type...`,
        ),
      );
      const changeset: EditResourceType = {};
      for (const diff of diffs) {
        if (
          diff.type !== "REMOVE" &&
          (
            diff.path.length !== 1 ||
            !["schema", "description"].includes(diff.path[0] as string)
          )
        ) {
          throw new Error("Invalid resource type diff with path " + diff.path);
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
        console.log(colors.yellow("! Skipping empty changeset"));
        return;
      }

      await ResourceService.updateResourceType({
        workspace: workspace,
        path: remotePath,
        requestBody: changeset,
      });
    } else {
      console.log(colors.yellow("Creating new resource type..."));
      await ResourceService.createResourceType({
        workspace: workspace,
        requestBody: {
          name: remotePath,
          description: this.description,
          schema: this.schema,
          workspace_id: workspace,
        },
      });
    }
  }
}

export async function pushResourceType(
  workspace: string,
  filePath: string,
  name: string,
) {
  const data: ResourceTypeFile = decoverto.type(ResourceTypeFile).rawToInstance(
    await Deno.readTextFile(filePath),
  );
  await data.push(workspace, name);
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
