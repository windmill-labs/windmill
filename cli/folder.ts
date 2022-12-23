import { colors, Command, Folder, FolderService } from "./deps.ts";
import { requireLogin, resolveWorkspace, validatePath } from "./context.ts";
import { GlobalOptions } from "./types.ts";

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

type FolderFile = {
  owners: Array<string> | undefined;
  extra_perms: Record<string, boolean> | undefined;
};

export async function pushFolder(
  workspace: string,
  filePath: string,
  remotePath: string,
) {
  if (remotePath.startsWith("/")) {
    remotePath = remotePath.substring(1);
  }
  if (remotePath.startsWith("f/")) {
    remotePath = remotePath.substring(2);
  }
  const data: FolderFile = JSON.parse(await Deno.readTextFile(filePath));
  let optFolder: Folder | undefined;
  try {
    optFolder = await FolderService.getFolder({ workspace, name: remotePath });
  } catch {
    optFolder = undefined;
  }

  if (optFolder) {
    // for (const [k, v] of Object.entries(optFolder.extra_perms)) {
    //   if (!data.extra_perms || data.extra_perms[k] !== v) {
    //     console.log(colors.red.underline.bold(`Extra Perms missmatch on ${k}`));
    //     return;
    //   }
    // }

    console.log(colors.yellow("Updating existing folder..."));
    await FolderService.updateFolder({
      workspace,
      name: remotePath,
      requestBody: {
        extra_perms: data.extra_perms,
        owners: data.owners,
      },
    });
  } else {
    console.log(colors.yellow("Creating new folder..."));
    await FolderService.createFolder({
      workspace,
      requestBody: {
        name: remotePath,
        extra_perms: data.extra_perms,
        owners: data.owners,
      },
    });

    // HACK: Workaround backend automatically adding current user to folder.
    await pushFolder(workspace, filePath, remotePath);
  }
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
