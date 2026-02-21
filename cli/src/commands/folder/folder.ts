import { stat } from "node:fs/promises";

import { colors } from "@cliffy/ansi/colors";
import { Command } from "@cliffy/command";
import { Table } from "@cliffy/table";
import * as log from "@std/log";
import { SEPARATOR as SEP } from "@std/path";
import * as wmill from "../../../gen/services.gen.ts";

import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace, validatePath } from "../../core/context.ts";
import { GlobalOptions, isSuperset, parseFromFile } from "../../types.ts";
import { Folder } from "../../../gen/types.gen.ts";

export interface FolderFile {
  owners: Array<string> | undefined;
  extra_perms: { [record: string]: boolean } | undefined;
  display_name: string | undefined;
}

async function list(opts: GlobalOptions) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const folders = await wmill.listFolders({
    workspace: workspace.workspaceId,
  });

  new Table()
    .header(["Name", "Owners", "Extra Perms"])
    .padding(2)
    .border(true)
    .body(
      folders.map((x) => [
        x.name,
        x.owners?.join(",") ?? "-",
        JSON.stringify(x.extra_perms ?? {}),
      ])
    )
    .render();
}

export async function pushFolder(
  workspace: string,
  name: string,
  folder: Folder | FolderFile | undefined,
  localFolder: FolderFile
): Promise<void> {
  if (name.startsWith(SEP)) {
    name = name.substring(1);
  }
  if (name.startsWith("f" + SEP)) {
    name = name.substring(2);
  }
  name = name.split(SEP)[0];
  log.debug(`Processing local folder ${name}`);

  // deleting old app if it exists in raw mode
  try {
    folder = await wmill.getFolder({ workspace, name });
    log.debug(`Folder ${name} exists on remote`);
  } catch {
    log.debug(`Folder ${name} does not exist on remote`);
    //ignore
  }

  if (folder) {
    if (isSuperset(localFolder, folder)) {
      log.debug(`Folder ${name} is up to date`);
      return;
    }
    log.debug(`Folder ${name} is not up-to-date, updating...`);
    try {
      await wmill.updateFolder({
        workspace: workspace,
        name: name,
        requestBody: {
          ...localFolder,
        },
      });
    } catch (e) {
      //@ts-ignore
      console.error(e.body);
      throw e;
    }
  } else {
    console.log(colors.bold.yellow("Creating new folder: " + name));
    try {
      await wmill.createFolder({
        workspace: workspace,
        requestBody: {
          name: name,
          ...localFolder,
        },
      });
    } catch (e) {
      //@ts-ignore
      throw Error(`Failed to create folder ${name}: ${e.body ?? e.message}`);
    }
  }
}

async function push(opts: GlobalOptions, filePath: string, remotePath: string) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  if (!validatePath(remotePath)) {
    return;
  }

  const fstat = await stat(filePath);
  if (!fstat.isFile()) {
    throw new Error("file path must refer to a file.");
  }

  console.log(colors.bold.yellow("Pushing folder..."));

  await pushFolder(
    workspace.workspaceId,
    remotePath,
    undefined,
    parseFromFile(filePath)
  );
  console.log(colors.bold.underline.green("Folder pushed"));
}

const command = new Command()
  .description("folder related commands")
  .action(list as any)
  .command(
    "push",
    "push a local folder spec. This overrides any remote versions."
  )
  .arguments("<file_path:string> <remote_path:string>")
  .action(push as any);

export default command;
