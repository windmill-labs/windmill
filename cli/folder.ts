import { colors, Command, Folder, FolderService, microdiff } from "./deps.ts";
import { requireLogin, resolveWorkspace, validatePath } from "./context.ts";
import {
  Difference,
  GlobalOptions,
  PushDiffs,
  Resource,
  setValueByPath,
} from "./types.ts";
import {
  array,
  decoverto,
  map,
  MapShape,
  model,
  property,
} from "./decoverto.ts";

@model()
export class FolderFile implements Resource, PushDiffs {
  @property(array(() => String))
  owners: Array<string> | undefined;
  @property(map(() => String, () => Boolean, { shape: MapShape.Object }))
  extra_perms: Map<string, boolean> | undefined;

  async push(workspace: string, remotePath: string): Promise<void> {
    if (remotePath.startsWith("/")) {
      remotePath = remotePath.substring(1);
    }
    if (remotePath.startsWith("f/")) {
      remotePath = remotePath.substring(2);
    }

    let existing: Folder | undefined;
    try {
      existing = await FolderService.getFolder({ workspace, name: remotePath });
    } catch {
      existing = undefined;
    }
    await this.pushDiffs(
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
    console.log(diffs);
    if (remotePath.startsWith("/")) {
      remotePath = remotePath.substring(1);
    }
    if (remotePath.startsWith("f/")) {
      remotePath = remotePath.substring(2);
    }

    // TODO: Support this in backend
    let exists: boolean;
    try {
      exists = !!await FolderService.getFolder({ workspace, name: remotePath });
    } catch {
      exists = false;
    }
    if (exists) {
      console.log(
        colors.bold.yellow(
          `Applying ${diffs.length} diffs to existing folder...`,
        ),
      );

      const changeset: {
        owners?: string[] | undefined;
        extra_perms?: any;
      } = {};
      for (const diff of diffs) {
        if (
          diff.type !== "REMOVE" &&
          (
            diff.path.length !== 1 ||
            !(diff.path[0] in ["owners", "extra_perms"])
          )
        ) {
          throw new Error("Invalid folder diff with path " + diff.path);
        }
        if (diff.type === "CREATE" || diff.type === "CHANGE") {
          setValueByPath(changeset, diff.path, diff.value);
        } else if (diff.type === "REMOVE") {
          setValueByPath(changeset, diff.path, null);
        }
      }
      await FolderService.updateFolder({
        workspace: workspace,
        name: remotePath,
        requestBody: changeset,
      });
    } else {
      console.log(colors.bold.yellow("Creating new folder..."));
      await FolderService.createFolder({
        workspace: workspace,
        requestBody: {
          name: remotePath,
          extra_perms: this.extra_perms,
          owners: this.owners,
        },
      });
    }
  }
}

async function push(opts: GlobalOptions, filePath: string, remotePath: string) {
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

  await pushFolder(workspace.workspaceId, filePath, remotePath);
  console.log(colors.bold.underline.green("Resource successfully pushed"));
}

export async function pushFolder(
  workspace: string,
  filePath: string,
  remotePath: string,
) {
  const data = decoverto.type(FolderFile).rawToInstance(
    await Deno.readTextFile(filePath),
  );
  data.push(workspace, remotePath);
}

const command = new Command()
  .description("resource related commands")
  .command(
    "push",
    "push a local folder spec. This overrides any remote versions.",
  )
  .arguments("<file_path:string> <remote_path:string>")
  .action(push as any);

export default command;
