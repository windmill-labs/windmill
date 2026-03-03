import { readFile, writeFile, readdir, mkdir, rm, stat } from "node:fs/promises";
import { appendFile } from "node:fs/promises";
import { colors } from "@cliffy/ansi/colors";
import { Command } from "@cliffy/command";
import { Confirm } from "@cliffy/prompt/confirm";
import { Input } from "@cliffy/prompt/input";
import { Select } from "@cliffy/prompt/select";
import { Table } from "@cliffy/table";
import * as log from "../../core/log.ts";
import * as path from "node:path";
import { stringify as yamlStringify } from "yaml";
import { setClient } from "../../core/client.ts";
import { yamlParseFile } from "../../utils/yaml.ts";
import * as wmill from "../../../gen/services.gen.ts";
import { loginInteractive } from "../../core/login.ts";
import {
  getActiveInstanceFilePath,
  getInstancesConfigFilePath,
} from "../../../windmill-utils-internal/src/config/config.ts";
import { pull, push } from "../sync/sync.ts";
import { showDiff } from "../../types.ts";

import {
  pullInstanceGroups,
  pullInstanceUsers,
  pushInstanceGroups,
  pushInstanceUsers,
} from "../user/user.ts";
import {
  add as workspaceSetup,
  addWorkspace,
  removeWorkspace,
  setActiveWorkspace,
} from "../workspace/workspace.ts";
import {
  pullInstanceConfigs,
  pullInstanceSettings,
  pushInstanceConfigs,
  pushInstanceSettings,
  type SimplifiedSettings,
} from "../../core/settings.ts";
import { deepEqual } from "../../utils/utils.ts";
import { getActiveWorkspace } from "../workspace/workspace.ts";

export interface Instance {
  remote: string;
  name: string;
  token: string;
  prefix: string;
}

export async function allInstances(): Promise<Instance[]> {
  try {
    const file = await getInstancesConfigFilePath();
    const txt = await readFile(file, "utf-8");
    return txt
      .split("\n")
      .map((line) => {
        if (line.length <= 2) {
          return;
        }
        const instance = JSON.parse(line) as Instance;
        return instance;
      })
      .filter(Boolean) as Instance[];
  } catch (_) {
    return [];
  }
}
export async function addInstance(
  opts: {},
  instanceName: string | undefined,
  remote: string | undefined,
  token: string | undefined,
) {
  if (!remote) {
    remote = await Input.prompt({
      message: "Enter the remote url of this instance",
      default: "https://my.windmill.dev/",
    });
    remote = new URL(remote).toString(); // add trailing slash in all cases!
  }

  if (!instanceName) {
    const defaultName = new URL(remote).hostname.split(".")[0];

    instanceName = await Input.prompt({
      message: "Enter a name for this instance",
      default: defaultName,
    });
  }
  const prefix = instanceName.toLowerCase().replace(/[^a-z0-9]/g, "");

  while (!token) {
    token = await loginInteractive(remote);
  }

  await appendInstance({
    name: instanceName,
    remote,
    token,
    prefix,
  });
  log.info(
    colors.green.underline(
      `Added instance ${instanceName} with remote ${remote}!`,
    ),
  );

  await switchI({}, instanceName);
  return {
    name: instanceName,
    remote,
    token,
    prefix,
  };
}

async function appendInstance(instance: Instance) {
  instance.remote = new URL(instance.remote).toString(); // add trailing slash in all cases!
  await removeInstance(instance.name);
  const filePath = await getInstancesConfigFilePath();
  await appendFile(filePath, JSON.stringify(instance) + "\n", "utf-8");
}

async function removeInstance(name: string) {
  const orgWorkspaces = await allInstances();

  await writeFile(
    await getInstancesConfigFilePath(),
    orgWorkspaces
      .filter((x) => x.name !== name)
      .map((x) => JSON.stringify(x))
      .join("\n") + "\n", "utf-8",
  );
}

type CompareObject<T extends string> = {
  [K in T]: string;
};
export function compareInstanceObjects<T extends string>(
  fromObjects: CompareObject<T>[],
  toObjects: CompareObject<T>[],
  idProp: T,
  objectName: string,
) {
  let changes = 0;
  for (const toObject of toObjects) {
    const fromMatch = fromObjects.find((u) => u[idProp] === toObject[idProp]);
    if (!fromMatch) {
      log.info(colors.red(`- instance ${objectName} ${toObject[idProp]}`));
      changes += 1;
    } else if (!deepEqual(toObject, fromMatch)) {
      log.info(colors.yellow(`~ instance ${objectName} ${toObject[idProp]}`));
      showDiff(yamlStringify(toObject), yamlStringify(fromMatch));
      changes += 1;
    }
  }

  for (const fromObject of fromObjects) {
    const toMatch = toObjects.find((u) => u[idProp] === fromObject[idProp]);
    if (!toMatch) {
      log.info(colors.green(`+ instance ${objectName} ${fromObject[idProp]}`));
      changes += 1;
    }
  }

  return changes;
}

export type InstanceSyncOptions = {
  skipUsers?: boolean;
  skipSettings?: boolean;
  skipConfigs?: boolean;
  skipGroups?: boolean;
  includeWorkspaces?: boolean;
  instance?: string;
  baseUrl?: string;
  token?: string;
  folderPerInstance?: boolean;
  dryRun?: boolean;
  yes?: boolean;
  prefix?: string;
  prefixSettings?: boolean;
};

export async function pickInstance(
  opts: InstanceSyncOptions,
  allowNew: boolean,
) {
  const instances = await allInstances();
  if (opts.baseUrl && opts.token && opts.instance) {
    log.info("Using instance defined by --instance, --base-url and --token");

    setClient(
      opts.token,
      opts.baseUrl.endsWith("/") ? opts.baseUrl.slice(0, -1) : opts.baseUrl,
    );

    return {
      name: opts.instance,
      remote: opts.baseUrl,
      token: opts.token,
      prefix: opts.prefix ?? opts.instance,
    };
  }
  if (opts.baseUrl && opts.token) {
    log.info("Using instance fully defined by --base-url and --token");

    setClient(
      opts.token,
      opts.baseUrl.endsWith("/") ? opts.baseUrl.slice(0, -1) : opts.baseUrl,
    );

    return {
      name: "custom",
      remote: opts.baseUrl,
      token: opts.token,
      prefix: opts.prefix ?? "custom",
    };
  }
  if (!allowNew && instances.length < 1) {
    throw new Error("No instance found, please add one first");
  }
  const instanceName = await getActiveInstance(opts);
  let instance: Instance | undefined = instances.find(
    (i) => i.name === instanceName,
  );
  if (!instance) {
    if (instances.length < 1) {
      instance = await addInstance({}, undefined, undefined, undefined);
    } else {
      const choice = (await Select.prompt({
        message: "Select an instance",
        options: [
          ...instances.map((i) => ({
            name: `${i.name} (${i.remote})`,
            value: i.name,
          })),
          { name: "Add new instance", value: "new" },
        ],
      })) as unknown as string;

      if (choice === "new") {
        instance = await addInstance({}, undefined, undefined, undefined);
      } else {
        instance = instances.find((i) => i.name === choice)!;
      }
    }
  } else {
    log.info(`Selected instance: ${instance.name}`);
  }

  setClient(
    instance.token,
    instance.remote.slice(0, instance.remote.length - 1),
  );

  return instance;
}
async function instancePull(opts: InstanceSyncOptions) {
  const instance = await pickInstance(opts, true);
  log.info("Pulling instance-level changes");
  log.info(`remote (${instance.name}) -> local`);

  opts.prefix = opts.prefix ?? instance.prefix;
  let uChanges = 0;
  if (!opts.skipUsers) {
    uChanges = (await pullInstanceUsers(opts, true)) as number;
  }
  let sChanges = 0;
  if (!opts.skipSettings) {
    sChanges = (await pullInstanceSettings(opts, true)) as number;
  }
  let cChanges = 0;
  if (!opts.skipConfigs) {
    cChanges = (await pullInstanceConfigs(opts, true)) as number;
  }
  let gChanges = 0;
  if (!opts.skipGroups) {
    gChanges = (await pullInstanceGroups(opts, true)) as number;
  }

  const totalChanges = uChanges + sChanges + cChanges + gChanges;

  const rootDir = process.cwd();

  if (totalChanges > 0) {
    let confirm = true;
    if (opts.dryRun) {
      log.info(colors.gray(`Dry run complete.`));
      return;
    }
    if (opts.yes !== true) {
      confirm = await Confirm.prompt({
        message:
          `Do you want to pull these ${totalChanges} instance-level changes?`,
        default: true,
      });
    }

    if (confirm) {
      if (uChanges > 0) {
        if (opts.folderPerInstance && opts.prefixSettings) {
          await mkdir(path.join(rootDir, opts.prefix), {
            recursive: true,
          });
        }
      }
      if (!opts.skipUsers && uChanges > 0) {
        await pullInstanceUsers(opts);
      }
      if (!opts.skipSettings && sChanges > 0) {
        await pullInstanceSettings(opts);
      }
      if (!opts.skipConfigs && cChanges > 0) {
        await pullInstanceConfigs(opts);
      }
      if (!opts.skipGroups && gChanges > 0) {
        await pullInstanceGroups(opts);
      }
    }
  } else {
    log.info("No instance-level changes to apply");
  }

  if (opts.includeWorkspaces) {
    log.info("\nPulling all workspaces");
    const localWorkspaces = await getLocalWorkspaces(
      rootDir,
      instance.prefix,
      opts.folderPerInstance,
    );

    const previousActiveWorkspace = await getActiveWorkspace(undefined);
    const remoteWorkspaces = await wmill.listWorkspacesAsSuperAdmin({
      page: 1,
      perPage: 1000,
    });
    for (const remoteWorkspace of remoteWorkspaces) {
      log.info("\nPulling workspace " + remoteWorkspace.id);
      const workspaceName = opts?.folderPerInstance
        ? instance.prefix + "/" + remoteWorkspace.id
        : instance.prefix + "_" + remoteWorkspace.id;
      await mkdir(path.join(rootDir, workspaceName), {
        recursive: true,
      });
      process.chdir(path.join(rootDir, workspaceName));
      await addWorkspace(
        {
          remote: instance.remote,
          name: workspaceName,
          workspaceId: remoteWorkspace.id,
          token: instance.token,
        },
        {
          token: undefined,
          workspace: undefined,
        },
      );

      await pull({
        workspace: workspaceName,
        token: undefined,
        baseUrl: undefined,
        configDir: undefined,
        includeGroups: true,
        includeSchedules: true,
        includeTriggers: true,
        includeSettings: true,
        includeUsers: true,
        includeKey: true,
        yes: opts.yes,
        dryRun: opts.dryRun,
      });
    }

    const localWorkspacesToDelete = localWorkspaces.filter(
      (w) => !remoteWorkspaces.find((r) => r.id === w.id),
    );

    if (localWorkspacesToDelete.length > 0) {
      const confirmDelete = opts.yes ||
        (await Confirm.prompt({
          message:
            "Do you want to delete the local copy of workspaces that don't exist anymore on the instance?\n" +
            localWorkspacesToDelete.map((w) => w).join(", "),
          default: true,
        }));

      if (confirmDelete) {
        for (const workspace of localWorkspacesToDelete) {
          await removeWorkspace(workspace.id, false, {});
          await rm(path.join(rootDir, workspace.dir), {
            recursive: true,
          });
        }
      }
    }

    if (previousActiveWorkspace) {
      await setActiveWorkspace(previousActiveWorkspace?.name);
    }
    log.info(colors.green.underline.bold("All workspaces pulled"));
  }
}

async function instancePush(opts: InstanceSyncOptions) {
  let instances = await allInstances();
  const instance = await pickInstance(opts, true);

  log.info("Pushing instance-level changes");
  log.info!(`remote (${instance.name}) <- local`);

  let uChanges = 0;
  if (!opts.skipUsers) {
    uChanges = (await pushInstanceUsers(opts, true)) as number;
  }
  let sChanges = 0;
  if (!opts.skipSettings) {
    sChanges = (await pushInstanceSettings(opts, true)) as number;
  }
  let cChanges = 0;
  if (!opts.skipConfigs) {
    cChanges = (await pushInstanceConfigs(opts, true)) as number;
  }
  let gChanges = 0;
  if (!opts.skipGroups) {
    gChanges = (await pushInstanceGroups(opts, true)) as number;
  }

  const totalChanges = uChanges + sChanges + cChanges + gChanges;

  if (totalChanges > 0) {
    let confirm = true;
    if (opts.yes !== true) {
      confirm = await Confirm.prompt({
        message:
          `Do you want to apply these ${totalChanges} instance-level changes?`,
        default: true,
      });
    }

    if (confirm) {
      if (!opts.skipUsers && uChanges > 0) {
        await pushInstanceUsers(opts);
      }
      if (!opts.skipSettings && sChanges > 0) {
        await pushInstanceSettings(opts);
      }
      if (!opts.skipConfigs && cChanges > 0) {
        await pushInstanceConfigs(opts);
      }
      if (!opts.skipGroups && gChanges > 0) {
        await pushInstanceGroups(opts);
      }
    }
  } else {
    log.info("No instance-level changes to apply");
  }

  if (opts.includeWorkspaces) {
    instances = await allInstances();
    const rootDir = process.cwd();

    let localPrefix;
    if (opts.prefix) {
      localPrefix = opts.prefix;
    } else {
      localPrefix = (await Select.prompt({
        message: "What is the prefix of the local workspaces you want to sync?",
        options: [
          ...instances.map((i) => ({
            name: `${i.prefix} (${i.name} - ${i.remote})`,
            value: i.prefix,
          })),
        ],
        default: instance.prefix as unknown,
      })) as unknown as string;
    }

    const remoteWorkspaces = await wmill.listWorkspacesAsSuperAdmin({
      page: 1,
      perPage: 1000,
    });

    const previousActiveWorkspace = await getActiveWorkspace(undefined);

    const localWorkspaces = await getLocalWorkspaces(
      rootDir,
      localPrefix,
      opts.folderPerInstance,
    );

    log.info(
      `\nPushing all workspaces: ${
        localWorkspaces.map((x) => x.id).join(", ")
      }`,
    );
    for (const localWorkspace of localWorkspaces) {
      log.info("\nPushing workspace " + localWorkspace.id);
      try {
        process.chdir(path.join(rootDir, localWorkspace.dir));
      } catch (_) {
        throw new Error(
          "Workspace folder not found, are you in the right directory?",
        );
      }

      try {
        const workspaceSettings = (await yamlParseFile(
          path.join(process.cwd(), "settings.yaml"),
        )) as SimplifiedSettings;
        await workspaceSetup(
          {
            token: instance.token,
            workspace: undefined,
            baseUrl: undefined,
            configDir: undefined,
            create: true,
            createWorkspaceName: workspaceSettings.name,
            createUsername: undefined,
          },
          localWorkspace.dir,
          localWorkspace.id,
          instance.remote,
        );
      } catch (e: any) {
        log.error(
          `Settings file not found in workspace ${localWorkspace.id} or error setting up workspace ${e.message}, skipping`,
        );
        continue;
      }
      await push({
        workspace: localWorkspace.dir,
        token: undefined,
        baseUrl: undefined,
        configDir: undefined,
        includeGroups: true,
        includeSchedules: true,
        includeTriggers: true,
        includeSettings: true,
        includeUsers: true,
        includeKey: true,
        yes: opts.yes,
        dryRun: opts.dryRun,
      });
    }

    const workspacesToDelete = remoteWorkspaces.filter(
      (w) => !localWorkspaces.find((l) => l.id === w.id),
    );
    if (workspacesToDelete.length > 0) {
      const confirmDelete = opts.yes ||
        (await Confirm.prompt({
          message:
            "Do you want to delete the following remote workspaces that don't exist locally?\n" +
            workspacesToDelete.map((w) => w.id).join(", "),
          default: true,
        }));

      if (confirmDelete) {
        for (const workspace of workspacesToDelete) {
          await wmill.deleteWorkspace({ workspace: workspace.id });
          log.info(colors.green.underline("Deleted workspace " + workspace.id));
        }
      }
    }
    if (previousActiveWorkspace) {
      await setActiveWorkspace(previousActiveWorkspace?.name);
    }
    log.info(colors.green.underline.bold("All workspaces pushed"));
  }
}

async function getLocalWorkspaces(
  rootDir: string,
  localPrefix: string,
  folderPerInstance?: boolean,
) {
  const localWorkspaces: { dir: string; id: string }[] = [];

  if (!(await stat(localPrefix).catch(() => null))) {
    await mkdir(localPrefix);
  }
  if (folderPerInstance) {
    const prefixEntries = await readdir(rootDir + "/" + localPrefix, { withFileTypes: true });
    for (const dir of prefixEntries) {
      if (dir.isDirectory()) {
        const dirName = dir.name;
        localWorkspaces.push({
          dir: localPrefix + "/" + dirName,
          id: dirName,
        });
      }
    }
  } else {
    const rootEntries = await readdir(rootDir, { withFileTypes: true });
    for (const dir of rootEntries) {
      const dirName = dir.name;
      if (dirName.startsWith(localPrefix + "_")) {
        localWorkspaces.push({
          dir: dirName,
          id: dirName.substring(localPrefix.length + 1),
        });
      }
    }
  }
  log.info(
    "Local workspaces found: " + localWorkspaces.map((x) => x.id).join(", "),
  );
  return localWorkspaces;
}

async function switchI(opts: {}, instanceName: string) {
  const all = await allInstances();
  if (all.findIndex((x) => x.name === instanceName) === -1) {
    log.info(
      colors.red.bold(
        `! This instance ${instanceName} does not exist locally.`,
      ),
    );
    log.info("available instances:");
    for (const w of all) {
      log.info("  - " + w.name);
    }
    return;
  }

  await writeFile(
    await getActiveInstanceFilePath(),
    instanceName, "utf-8",
  );

  log.info(colors.green.underline(`Switched to instance ${instanceName}`));
}

export async function getActiveInstance(opts: {
  instance?: string;
}): Promise<string | undefined> {
  if (opts.instance) {
    return opts.instance;
  }
  try {
    return await readFile(await getActiveInstanceFilePath(), "utf-8");
  } catch {
    return undefined;
  }
}

async function getConfig(opts: InstanceSyncOptions & { outputFile?: string }) {
  await pickInstance(opts, false);
  const config = await wmill.getInstanceConfig();
  const yaml = yamlStringify(config as Record<string, unknown>);
  if (opts.outputFile) {
    await writeFile(opts.outputFile, yaml, "utf-8");
    log.info(colors.green(`Instance config written to ${opts.outputFile}`));
  } else {
    console.log(yaml);
  }
}

async function whoami(opts: {}) {
  await pickInstance({}, false);
  try {
    const whoamiInfo = await wmill.globalWhoami();
    log.info(colors.green.underline(`global whoami infos:`));
    log.info(JSON.stringify(whoamiInfo, null, 2));
  } catch (error) {
    log.error(
      //@ts-ignore
      colors.red(`Failed to retrieve whoami information: ${error.message}`),
    );
  }
}

const command = new Command()
  .description(
    "sync local with a remote instance or the opposite (push or pull)",
  )
  .action(async () => {
    log.info(
      "4 actions available, add, remove, switch, pull and push. Use -h to display help.",
    );
    const activeInstance = await getActiveInstance({});

    new Table()
      .header(["name", "remote", "token"])
      .padding(2)
      .border(true)
      .body(
        (await allInstances()).map((x) => [
          x.name === activeInstance ? colors.underline(x.name) : x.name,
          x.remote,
          x.token.substring(0, 7) + "***",
        ]),
      )
      .render();
    if (activeInstance) {
      log.info(`Selected instance: ${activeInstance}`);
    } else {
      log.info("No active instance selected");
    }
    log.info("Use 'wmill instance add' to add a new instance");
  })
  .command("add")
  .description("Add a new instance")
  .action(addInstance as any)
  .arguments("[instance_name:string] [remote:string] [token:string]")
  .command("remove")
  .description("Remove an instance")
  .complete("instance", async () => (await allInstances()).map((x) => x.name))
  .arguments("<instance:string:instance>")
  .action(async (instance: any) => {
    const instances = await allInstances();

    const choice = (await Select.prompt({
      message: "Select an instance to remove",
      options: instances.map((i) => ({
        name: `${i.name} (${i.remote})`,
        value: i.name,
      })),
    })) as unknown as string;

    await removeInstance(choice);
    log.info(colors.green.underline(`Removed instance ${choice}`));
  })
  .command("switch")
  .complete("instance", async () => (await allInstances()).map((x) => x.name))
  .arguments("<instance:string:instance>")
  .description("Switch the current instance")
  .action(switchI as any)
  .command("pull")
  .description(
    "Pull instance settings, users, configs, instance groups and overwrite local",
  )
  .option("--yes", "Pull without needing confirmation")
  .option("--dry-run", "Perform a dry run without making changes")
  .option("--skip-users", "Skip pulling users")
  .option("--skip-settings", "Skip pulling settings")
  .option("--skip-configs", "Skip pulling configs (worker groups and SMTP)")
  .option("--skip-groups", "Skip pulling instance groups")
  .option("--include-workspaces", "Also pull workspaces")
  .option("--folder-per-instance", "Create a folder per instance")
  .option(
    "--instance <instance:string>",
    "Name of the instance to pull from, override the active instance",
  )
  .option(
    "--prefix <prefix:string>",
    "Prefix of the local workspaces to pull, used to create the folders when using --include-workspaces",
  )
  .option(
    "--prefix-settings",
    "Store instance yamls inside prefixed folders when using --prefix and --folder-per-instance",
  )
  .action(instancePull as any)
  .command("push")
  .description(
    "Push instance settings, users, configs, group and overwrite remote",
  )
  .option("--yes", "Push without needing confirmation")
  .option("--dry-run", "Perform a dry run without making changes")
  .option("--skip-users", "Skip pushing users")
  .option("--skip-settings", "Skip pushing settings")
  .option("--skip-configs", "Skip pushing configs (worker groups and SMTP)")
  .option("--skip-groups", "Skip pushing instance groups")
  .option("--include-workspaces", "Also push workspaces")
  .option("--folder-per-instance", "Create a folder per instance")
  .option(
    "--instance <instance:string>",
    "Name of the instance to push to, override the active instance",
  )
  .option(
    "--prefix <prefix:string>",
    "Prefix of the local workspaces folders to push",
  )
  .option(
    "--prefix-settings",
    "Store instance yamls inside prefixed folders when using --prefix and --folder-per-instance",
  )
  .action(instancePush as any)
  .command("whoami")
  .description("Display information about the currently logged-in user")
  .action(whoami as any)
  .command("get-config")
  .description("Dump the current instance config (global settings + worker configs) as YAML")
  .option("-o, --output-file <file:string>", "Write YAML to a file instead of stdout")
  .option(
    "--instance <instance:string>",
    "Name of the instance, override the active instance",
  )
  .action(getConfig as any);

export default command;
