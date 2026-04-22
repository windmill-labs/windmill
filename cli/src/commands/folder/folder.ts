import { stat, readdir, writeFile, mkdir } from "node:fs/promises";
import { stringify as yamlStringify } from "yaml";

import { colors } from "@cliffy/ansi/colors";
import { Command } from "@cliffy/command";
import { Table } from "@cliffy/table";
import * as log from "../../core/log.ts";
import { sep as SEP } from "node:path";
import { Confirm } from "@cliffy/prompt/confirm";
import * as wmill from "../../../gen/services.gen.ts";

import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace } from "../../core/context.ts";
import { GlobalOptions, isSuperset, parseFromFile } from "../../types.ts";
import { Folder } from "../../../gen/types.gen.ts";

export interface FolderFile {
  summary: string | undefined;
  display_name: string | undefined;
  owners: Array<string> | undefined;
  extra_perms: { [record: string]: boolean } | undefined;
  default_permissioned_as?: Array<{ path_glob: string; permissioned_as: string }>;
}

async function list(opts: GlobalOptions & { json?: boolean }) {
  if (opts.json) log.setSilent(true);
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

async function newFolder(opts: GlobalOptions & { summary?: string }, name: string) {
  const dirPath = `f${SEP}${name}`;
  const filePath = `${dirPath}${SEP}folder.meta.yaml`;
  try {
    await stat(filePath);
    throw new Error("File already exists: " + filePath);
  } catch (e: any) {
    if (e.message?.startsWith("File already exists")) throw e;
  }
  const template: FolderFile = {
    summary: opts.summary ?? "",
    display_name: name,
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

async function push(opts: GlobalOptions, name: string) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const metaPath = `f${SEP}${name}${SEP}folder.meta.yaml`;
  try {
    await stat(metaPath);
  } catch {
    throw new Error(`Could not find ${metaPath}. Does the folder exist locally?`);
  }

  console.log(colors.bold.yellow("Pushing folder..."));

  await pushFolder(
    workspace.workspaceId,
    name,
    undefined,
    parseFromFile(metaPath)
  );
  console.log(colors.bold.underline.green("Folder pushed"));
}

async function addMissing(opts: GlobalOptions & { yes?: boolean }) {
  const fDir = `f`;
  try {
    await stat(fDir);
  } catch {
    log.info("No 'f/' directory found. Nothing to do.");
    return;
  }
  const entries = await readdir(fDir, { withFileTypes: true });
  const missing: string[] = [];
  for (const entry of entries) {
    if (!entry.isDirectory()) continue;
    const metaPath = `${fDir}${SEP}${entry.name}${SEP}folder.meta.yaml`;
    try {
      await stat(metaPath);
    } catch {
      missing.push(entry.name);
    }
  }
  if (missing.length === 0) {
    log.info("All folders already have a folder.meta.yaml. Nothing to do.");
    return;
  }
  log.info(`Missing folder.meta.yaml for:`);
  for (const name of missing) {
    log.info(`  - ${name}`);
  }
  if (
    !opts.yes &&
    !(await Confirm.prompt({
      message: `Create ${missing.length} folder.meta.yaml file(s)?`,
      default: true,
    }))
  ) {
    return;
  }
  for (const name of missing) {
    await newFolder(opts, name);
  }
  log.info(
    `\nCreated ${missing.length} folder.meta.yaml file(s). You can now run 'wmill sync push' to push them.`,
  );
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
  .option("--summary <summary:string>", "folder summary")
  .action(newFolder as any)
  .command(
    "push",
    "push a local folder to the remote by name. This overrides any remote versions."
  )
  .arguments("<name:string>")
  .action(push as any)
  .command(
    "add-missing",
    "create default folder.meta.yaml for all subdirectories of f/ that are missing one"
  )
  .option("-y, --yes", "skip confirmation prompt")
  .action(addMissing as any)
  .command(
    "show-rules",
    "Show default_permissioned_as rules for a folder. Use --test-path to see which rule matches a given item path."
  )
  .arguments("<name:string>")
  .option("--test-path <path:string>", "Test which rule matches this item path (e.g. f/prod/jobs/my_script)")
  .option("--json", "Output as JSON")
  .action((async (opts: any, folderName: string) => {
    const workspace = await resolveWorkspace(opts);
    await requireLogin(opts);

    const folder = await wmill.getFolder({
      workspace: workspace.workspaceId,
      name: folderName,
    });

    const rules = (folder as any).default_permissioned_as ?? [];

    if (opts.json && !opts.testPath) {
      console.log(JSON.stringify(rules, null, 2));
      return;
    }

    if (rules.length === 0) {
      log.info(`Folder '${folderName}' has no default_permissioned_as rules.`);
      return;
    }

    if (!opts.testPath) {
      log.info(colors.bold(`Rules for folder '${folderName}' (first match wins):\n`));
      new Table()
        .header(["#", "path_glob", "permissioned_as"])
        .padding(2)
        .border(true)
        .body(rules.map((r: any, i: number) => [String(i + 1), r.path_glob, r.permissioned_as]))
        .render();
      return;
    }

    // Test a path against the rules
    const testPath = opts.testPath as string;
    const prefix = `f/${folderName}/`;
    if (!testPath.startsWith(prefix)) {
      log.error(`Path '${testPath}' is not under folder '${folderName}' (expected prefix '${prefix}')`);
      return;
    }
    const relative = testPath.slice(prefix.length);

    const { minimatch } = await import("minimatch");
    for (let i = 0; i < rules.length; i++) {
      const rule = rules[i];
      if (minimatch(relative, rule.path_glob)) {
        if (opts.json) {
          console.log(JSON.stringify({ matched: true, rule_index: i, rule, relative_path: relative }));
        } else {
          log.info(colors.green(
            `✓ Rule #${i + 1} matches: path_glob='${rule.path_glob}' → permissioned_as='${rule.permissioned_as}'`
          ));
          log.info(colors.gray(`  (relative path tested: '${relative}')`));
        }
        return;
      }
    }

    if (opts.json) {
      console.log(JSON.stringify({ matched: false, relative_path: relative }));
    } else {
      log.info(colors.yellow(`No rule matches path '${testPath}' (relative: '${relative}')`));
    }
  }) as any);

export default command;
