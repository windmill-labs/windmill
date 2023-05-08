// deno-lint-ignore-file no-explicit-any
import { colors, Command, Folder, FolderService } from "./deps.ts";
import { requireLogin, resolveWorkspace, validatePath } from "./context.ts";
import { GlobalOptions, isSuperset, parseFromFile } from "./types.ts";

export interface FolderFile {
  owners: Array<string> | undefined;
  extra_perms: Map<string, boolean> | undefined;
  display_name: string | undefined;
}

export async function pushFolder(
  workspace: string,
  name: string,
  folder: Folder | FolderFile | undefined,
  localFolder: FolderFile,
  raw: boolean
): Promise<void> {
  if (name.startsWith("/")) {
    name = name.substring(1);
  }
  if (name.startsWith("f/")) {
    name = name.substring(2);
  }
  name = name.split("/")[0];
  if (raw) {
    // deleting old app if it exists in raw mode
    try {
      folder = await FolderService.getFolder({ workspace, name });
    } catch {
      //ignore
    }
  }

  if (folder) {
    if (isSuperset(localFolder, folder)) {
      return;
    }
    try {
      await FolderService.updateFolder({
        workspace: workspace,
        name: name,
        requestBody: {
          ...localFolder,
        },
      });
    } catch (e) {
      console.error(colors.red.bold(e.body));
      throw e;
    }
  } else {
    console.log(colors.bold.yellow("Creating new folder: " + name));
    await FolderService.createFolder({
      workspace: workspace,
      requestBody: {
        name: name,
        ...localFolder,
      },
    });
  }
}

async function push(opts: GlobalOptions, filePath: string, remotePath: string) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  if (!validatePath(remotePath)) {
    return;
  }

  const fstat = await Deno.stat(filePath);
  if (!fstat.isFile) {
    throw new Error("file path must refer to a file.");
  }

  console.log(colors.bold.yellow("Pushing folder..."));

  await pushFolder(
    workspace.workspaceId,
    remotePath,
    undefined,
    parseFromFile(filePath),
    false
  );
  console.log(colors.bold.underline.green("Folder pushed"));
}

const command = new Command()
  .description("resource related commands")
  .command(
    "push",
    "push a local folder spec. This overrides any remote versions."
  )
  .arguments("<file_path:string> <remote_path:string>")
  .action(push as any);

export default command;
