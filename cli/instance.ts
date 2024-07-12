import {
  Select,
  WorkspaceService,
  path,
  Confirm,
  yamlStringify,
  yamlParse,
  Command,
  setClient,
} from "./deps.ts";
import { DelimiterStream, Input, colors, log } from "./deps.ts";
import { loginInteractive } from "./login.ts";
import { getRootStore } from "./store.ts";
import { push, pull } from "./sync.ts";
import { showDiff } from "./types.ts";

import {
  pushInstanceUsers,
  pullInstanceUsers,
  pullInstanceGroups,
  pushInstanceGroups,
} from "./user.ts";
import {
  add as workspaceSetup,
  addWorkspace,
  allWorkspaces,
  removeWorkspace,
} from "./workspace.ts";
import {
  pushInstanceSettings,
  pullInstanceSettings,
  pullInstanceConfigs,
  pushInstanceConfigs,
  type SimplifiedSettings,
} from "./settings.ts";
import { sleep, deepEqual } from "./utils.ts";
import { GlobalOptions } from "./types.ts";

export interface Instance {
  remote: string;
  name: string;
  token: string;
  prefix: string;
}

function makeInstanceStream(
  readable: ReadableStream<Uint8Array>
): ReadableStream<Instance> {
  return readable
    .pipeThrough(new DelimiterStream(new TextEncoder().encode("\n")))
    .pipeThrough(new TextDecoderStream())
    .pipeThrough(
      new TransformStream({
        transform(line, controller) {
          try {
            if (line.length <= 2) {
              return;
            }
            const instance = JSON.parse(line) as Instance;
            controller.enqueue(instance);
          } catch {
            /* ignore */
          }
        },
      })
    );
}

async function getInstanceStream() {
  const file = await Deno.open((await getRootStore()) + "instances.ndjson", {
    write: false,
    read: true,
  });
  return makeInstanceStream(file.readable);
}

export async function allInstances(): Promise<Instance[]> {
  try {
    const instanceStream = await getInstanceStream();
    const instances: Instance[] = [];
    for await (const instance of instanceStream) {
      instances.push(instance);
    }

    return instances;
  } catch (_) {
    return [];
  }
}
export async function addInstance() {
  let remote = await Input.prompt({
    message: "Enter the remote url of this instance",
    default: "https://app.windmill.dev/",
  });
  remote = new URL(remote).toString(); // add trailing slash in all cases!

  const defaultName = new URL(remote).hostname;

  const name = await Input.prompt({
    message: "Enter a name for this instance",
    default: defaultName,
  });

  const prefix = name.toLowerCase().replace(/[^a-z0-9]/g, "");

  let token: string | undefined = undefined;
  while (!token) {
    token = await loginInteractive(remote);
  }

  await appendInstance({
    name,
    remote,
    token,
    prefix,
  });
  log.info(
    colors.green.underline(`Added instance ${name} with remote ${remote}!`)
  );

  return {
    name,
    remote,
    token,
    prefix,
  };
}

async function appendInstance(instance: Instance) {
  instance.remote = new URL(instance.remote).toString(); // add trailing slash in all cases!
  await removeInstance(instance.name);
  const file = await Deno.open((await getRootStore()) + "instances.ndjson", {
    append: true,
    write: true,
    read: true,
    create: true,
  });
  await file.write(new TextEncoder().encode(JSON.stringify(instance) + "\n"));

  file.close();
}

async function removeInstance(name: string) {
  const orgWorkspaces = await allInstances();

  await Deno.writeTextFile(
    (await getRootStore()) + "instances.ndjson",
    orgWorkspaces
      .filter((x) => x.name !== name)
      .map((x) => JSON.stringify(x))
      .join("\n") + "\n"
  );
}

type CompareObject<T extends string> = {
  [K in T]: string;
};
export function compareInstanceObjects<T extends string>(
  fromObjects: CompareObject<T>[],
  toObjects: CompareObject<T>[],
  idProp: T,
  objectName: string
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

type InstanceSyncOptions = {
  skipUsers?: boolean;
  skipSettings?: boolean;
  skipConfigs?: boolean;
  skipGroups?: boolean;
  includeWorkspaces?: boolean;
  baseUrl?: string;
};

async function instancePull(opts: GlobalOptions & InstanceSyncOptions) {
  const instances = await allInstances();
  let instance: Instance;
  if (instances.length < 1) {
    instance = await addInstance();
  } else {
    const choice = (await Select.prompt({
      message: "Select an instance to pull from",
      options: [
        ...instances.map((i) => ({
          name: `${i.name} (${i.remote})`,
          value: i.name,
        })),
        { name: "Add new instance", value: "new" },
      ],
    })) as unknown as string;

    if (choice === "new") {
      instance = await addInstance();
    } else {
      instance = instances.find((i) => i.name === choice)!;
    }
  }

  setClient(
    instance.token,
    instance.remote.slice(0, instance.remote.length - 1)
  );

  log.info("Pulling instance-level changes");
  log.info(`remote (${instance.name}) -> local`);

  let uChanges = 0;
  if (!opts.skipUsers) {
    uChanges = (await pullInstanceUsers(true)) as number;
  }
  let sChanges = 0;
  if (!opts.skipSettings) {
    sChanges = (await pullInstanceSettings(true)) as number;
  }
  let cChanges = 0;
  if (!opts.skipConfigs) {
    cChanges = (await pullInstanceConfigs(true)) as number;
  }
  let gChanges = 0;
  if (!opts.skipGroups) {
    gChanges = (await pullInstanceGroups(true)) as number;
  }

  const totalChanges = uChanges + sChanges + cChanges + gChanges;

  if (totalChanges > 0) {
    const confirm = await Confirm.prompt({
      message: `Do you want to apply these ${totalChanges} instance-level changes?`,
      default: true,
    });

    if (confirm) {
      if (!opts.skipUsers && uChanges > 0) {
        await pullInstanceUsers();
      }
      if (!opts.skipSettings && sChanges > 0) {
        await pullInstanceSettings();
      }
      if (!opts.skipConfigs && cChanges > 0) {
        await pullInstanceConfigs();
      }
      if (!opts.skipGroups && gChanges > 0) {
        await pullInstanceGroups();
      }
    }
  } else {
    log.info("No instance-level changes to apply");
  }

  sleep(1000);

  if (opts.includeWorkspaces) {
    log.info("\nPulling all workspaces");
    const remoteWorkspaces = await WorkspaceService.listWorkspacesAsSuperAdmin({
      page: 1,
      perPage: 1000,
    });
    let localWorkspaces = await allWorkspaces();
    localWorkspaces = localWorkspaces.filter((w) =>
      w.name.startsWith(instance.prefix + "_")
    );
    const rootDir = Deno.cwd();
    for (const remoteWorkspace of remoteWorkspaces) {
      log.info("\nPulling workspace " + remoteWorkspace.id);
      sleep(1000);
      const workspaceName = instance.prefix + "_" + remoteWorkspace.id;
      await Deno.mkdir(path.join(rootDir, workspaceName), {
        recursive: true,
      });
      await Deno.chdir(path.join(rootDir, workspaceName));
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
        }
      );

      await pull({
        workspace: workspaceName,
        token: undefined,
        includeGroups: true,
        includeSchedules: true,
        includeSettings: true,
        includeUsers: true,
        includeKey: true,
      });
    }

    const localWorkspacesToDelete = localWorkspaces.filter(
      (w) => !remoteWorkspaces.find((r) => r.id === w.workspaceId)
    );

    if (localWorkspacesToDelete.length > 0) {
      const confirmDelete = await Confirm.prompt({
        message:
          "Do you want to delete the local copy of workspaces that don't exist anymore on the instance?\n" +
          localWorkspacesToDelete.map((w) => w.workspaceId).join(", "),
        default: true,
      });

      if (confirmDelete) {
        for (const workspace of localWorkspacesToDelete) {
          await removeWorkspace(workspace.name, false, {});
          await Deno.remove(path.join(rootDir, workspace.name), {
            recursive: true,
          });
        }
      }
    }

    log.info(colors.green.underline.bold("All workspaces pulled"));
  }
}

async function instancePush(opts: GlobalOptions & InstanceSyncOptions) {
  let instances = await allInstances();
  let instance: Instance;
  if (instances.length < 1) {
    instance = await addInstance();
  } else {
    const choice = (await Select.prompt({
      message: "Select an instance to push to",
      options: [
        ...instances.map((i) => ({
          name: `${i.name} (${i.remote})`,
          value: i.name,
        })),
        { name: "Add new instance", value: "new" },
      ],
    })) as unknown as string;

    if (choice === "new") {
      instance = await addInstance();
    } else {
      instance = instances.find((i) => i.name === choice)!;
    }
  }

  setClient(
    instance.token,
    instance.remote.slice(0, instance.remote.length - 1)
  );

  log.info("Pushing instance-level changes");
  log.info!(`remote (${instance.name}) <- local`);

  let uChanges = 0;
  if (!opts.skipUsers) {
    uChanges = (await pushInstanceUsers(true)) as number;
  }
  let sChanges = 0;
  if (!opts.skipSettings) {
    sChanges = (await pushInstanceSettings(true, opts.baseUrl)) as number;
  }
  let cChanges = 0;
  if (!opts.skipConfigs) {
    cChanges = (await pushInstanceConfigs(true)) as number;
  }
  let gChanges = 0;
  if (!opts.skipGroups) {
    gChanges = (await pushInstanceGroups(true)) as number;
  }

  const totalChanges = uChanges + sChanges + cChanges + gChanges;

  if (totalChanges > 0) {
    const confirm = await Confirm.prompt({
      message: `Do you want to apply these ${totalChanges} instance-level changes?`,
      default: true,
    });

    if (confirm) {
      if (!opts.skipUsers && uChanges > 0) {
        await pushInstanceUsers();
      }
      if (!opts.skipSettings && sChanges > 0) {
        await pushInstanceSettings(false, opts.baseUrl);
      }
      if (!opts.skipConfigs && cChanges > 0) {
        await pushInstanceConfigs();
      }
      if (!opts.skipGroups && gChanges > 0) {
        await pushInstanceGroups();
      }
    }
  } else {
    log.info("No instance-level changes to apply");
  }

  sleep(1000);

  if (opts.includeWorkspaces) {
    instances = await allInstances();
    const localPrefix = (await Select.prompt({
      message: "What is the prefix of the local workspaces you want to sync?",
      options: [
        ...instances.map((i) => ({
          name: `${i.prefix} (${i.name} - ${i.remote})`,
          value: i.prefix,
        })),
      ],
      default: instance.prefix as unknown,
    })) as unknown as string;

    const remoteWorkspaces = await WorkspaceService.listWorkspacesAsSuperAdmin({
      page: 1,
      perPage: 1000,
    });
    let localWorkspaces = await allWorkspaces();
    localWorkspaces = localWorkspaces.filter((w) =>
      w.name.startsWith(localPrefix + "_")
    );

    log.info("\nPushing all workspaces");
    const rootDir = Deno.cwd();
    for (const localWorkspace of localWorkspaces) {
      log.info("\nPushing workspace " + localWorkspace.workspaceId);
      sleep(1000);
      try {
        await Deno.chdir(path.join(rootDir, localWorkspace.name));
      } catch (_) {
        throw new Error(
          "Workspace folder not found, are you in the right directory?"
        );
      }

      try {
        const workspaceSettings = yamlParse(
          await Deno.readTextFile("settings.yaml")
        ) as SimplifiedSettings;
        await workspaceSetup(
          {
            token: instance.token,
            workspace: undefined,
            create: true,
            createWorkspaceName: workspaceSettings.name,
            createUsername: undefined,
          },
          localWorkspace.name,
          localWorkspace.workspaceId,
          instance.remote
        );
      } catch (_) {
        log.error(
          "Settings file not found in workspace local folder, skipping"
        );
        continue;
      }
      await push({
        workspace: localWorkspace.name,
        token: undefined,
        includeGroups: true,
        includeSchedules: true,
        includeSettings: true,
        includeUsers: true,
        includeKey: true,
      });
    }

    const workspacesToDelete = remoteWorkspaces.filter(
      (w) => !localWorkspaces.find((l) => l.workspaceId === w.id)
    );
    if (workspacesToDelete.length > 0) {
      const confirmDelete = await Confirm.prompt({
        message:
          "Do you want to delete the following remote workspaces that don't exist locally?\n" +
          workspacesToDelete.map((w) => w.id).join(", "),
        default: true,
      });

      if (confirmDelete) {
        for (const workspace of workspacesToDelete) {
          await WorkspaceService.deleteWorkspace({ workspace: workspace.id });
          log.info(colors.green.underline("Deleted workspace " + workspace.id));
        }
      }
    }
    log.info(colors.green.underline.bold("All workspaces pushed"));
  }
}

const command = new Command()
  .description(
    "sync local with a remote instance or the opposite (push or pull)"
  )
  .action(() =>
    log.info("2 actions available, pull and push. Use -h to display help.")
  )
  .command("pull")
  .description(
    "Pull instance settings, users, configs, instance groups and overwrite local"
  )
  .option("--skip-users", "Skip pulling users")
  .option("--skip-settings", "Skip pulling settings")
  .option("--skip-configs", "Skip pulling configs (worker groups and SMTP)")
  .option("--skip-groups", "Skip pulling instance groups")
  .option("--include-workspaces", "Also pull workspaces")
  .action(instancePull as any)
  .command("push")
  .description(
    "Push instance settings, users, configs, group and overwrite remote"
  )
  .option("--skip-users", "Skip pushing users")
  .option("--skip-settings", "Skip pushing settings")
  .option("--skip-configs", "Skip pushing configs (worker groups and SMTP)")
  .option("--skip-groups", "Skip pushing instance groups")
  .option("--include-workspaces", "Also push workspaces")
  .option(
    "--base-url",
    "Base url to be passed to the instance settings instead of the local one"
  )
  .action(instancePush as any);

export default command;
