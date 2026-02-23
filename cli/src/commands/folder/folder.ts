import { stat, writeFile, mkdir } from "node:fs/promises";
import { stringify as yamlStringify } from "yaml";

import { colors } from "@cliffy/ansi/colors";
import { Command } from "@cliffy/command";
import { Table } from "@cliffy/table";
import * as log from "../../core/log.ts";
import { sep as SEP } from "node:path";
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

async function list(opts: GlobalOptions & { json?: boolean }) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const folders = await wmill.listFolders({
    workspace: workspace.workspaceId,
  });

  if (opts.json) {
    console.log(JSON.stringify(folders));
  } else {
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
}

async function newFolder(opts: GlobalOptions, name: string) {
  const dirPath = `f${SEP}${name}`;
  const filePath = `${dirPath}${SEP}folder.meta.yaml`;
  try {
    await stat(filePath);
    throw new Error("File already exists: " + filePath);
  } catch (e: any) {
    if (e.message?.startsWith("File already exists")) throw e;
  }
  const template: Omit<FolderFile, "display_name"> = {
    owners: [],
    extra_perms: {},
  };
  await mkdir(dirPath, { recursive: true });
  await writeFile(filePath, yamlStringify(template as Record<string, any>), {
    flag: "wx",
    encoding: "utf-8",
  });
  log.info(colors.green(`Created ${filePath}`));
}

async function get(opts: GlobalOptions & { json?: boolean }, name: string) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);
  const f = await wmill.getFolder({
    workspace: workspace.workspaceId,
    name,
  });
  if (opts.json) {
    console.log(JSON.stringify(f));
  } else {
    console.log(colors.bold("Name:") + " " + f.name);
    console.log(colors.bold("Summary:") + " " + (f.summary ?? ""));
    console.log(colors.bold("Owners:") + " " + (f.owners?.join(", ") ?? "-"));
    console.log(colors.bold("Extra Perms:") + " " + JSON.stringify(f.extra_perms ?? {}));
  }
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
  .option("--json", "Output as JSON (for piping to jq)")
  .action(list as any)
  .command("list", "list all folders")
  .option("--json", "Output as JSON (for piping to jq)")
  .action(list as any)
  .command("get", "get a folder's details")
  .arguments("<name:string>")
  .option("--json", "Output as JSON (for piping to jq)")
  .action(get as any)
  .command("new", "create a new folder locally")
  .arguments("<name:string>")
  .action(newFolder as any)
  .command(
    "push",
    "push a local folder spec. This overrides any remote versions."
  )
  .arguments("<file_path:string> <remote_path:string>")
  .action(push as any);

export default command;
