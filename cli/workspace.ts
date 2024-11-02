// deno-lint-ignore-file no-explicit-any
import { GlobalOptions } from "./types.ts";
import { getRootStore } from "./store.ts";
import { loginInteractive, tryGetLoginInfo } from "./login.ts";
import { colors, Command, Input, log, setClient, Table } from "./deps.ts";

import * as wmill from "./gen/services.gen.ts";
import { requireLogin } from "./context.ts";

export interface Workspace {
  remote: string;
  workspaceId: string;
  name: string;
  token: string;
}

export async function allWorkspaces(): Promise<Workspace[]> {
  try {
    const file = (await getRootStore()) + "remotes.ndjson";
    const txt = await Deno.readTextFile(file);
    return txt
      .split("\n")
      .map((line) => {
        if (line.length <= 2) {
          return;
        }
        const instance = JSON.parse(line) as Workspace;
        return instance;
      })
      .filter(Boolean) as Workspace[];
  } catch (_) {
    return [];
  }
}

async function getActiveWorkspaceName(
  opts: GlobalOptions | undefined
): Promise<string | undefined> {
  if (opts?.workspace) {
    return opts?.workspace;
  }
  try {
    return await Deno.readTextFile((await getRootStore()) + "/activeWorkspace");
  } catch {
    return undefined;
  }
}

export async function getActiveWorkspace(
  opts: GlobalOptions | undefined
): Promise<Workspace | undefined> {
  const name = await getActiveWorkspaceName(opts);
  if (!name) {
    return undefined;
  }
  return await getWorkspaceByName(name);
}

export async function getWorkspaceByName(
  workspaceName: string
): Promise<Workspace | undefined> {
  const workspaceStream = await allWorkspaces();
  for await (const workspace of workspaceStream) {
    if (workspace.name === workspaceName) {
      return workspace;
    }
  }
  return undefined;
}

async function list(opts: GlobalOptions) {
  const workspaces = await allWorkspaces();
  const activeName = await getActiveWorkspaceName(opts);

  new Table()
    .header(["name", "remote", "workspace id"])
    .padding(2)
    .border(true)
    .body(
      workspaces.map((x) => {
        const a = [x.name, x.remote, x.workspaceId];
        if (x.name === activeName) {
          return a.map((x) => colors.underline(x));
        } else {
          return a;
        }
      })
    )
    .render();

  log.info("Active: " + colors.green.bold(activeName || "none"));
}

async function switchC(opts: GlobalOptions, workspaceName: string) {
  if (opts.workspace) {
    log.info(
      colors.red.bold(
        "! Workspace needs to be specified as positional argument, not as option."
      )
    );
    return;
  }

  const all = await allWorkspaces();
  if (all.findIndex((x) => x.name === workspaceName) === -1) {
    log.info(
      colors.red.bold(
        `! This workspace profile ${workspaceName} does not exist locally.`
      )
    );
    log.info("available workspace profiles:");
    for (const w of all) {
      log.info("  - " + w.name);
    }
    return;
  }

  await setActiveWorkspace(workspaceName);
  return;
}

export async function setActiveWorkspace(workspaceName: string) {
  await Deno.writeTextFile(
    (await getRootStore()) + "/activeWorkspace",
    workspaceName
  );
}

export async function add(
  opts: GlobalOptions & {
    create: boolean;
    createWorkspaceName: string | undefined;
    createUsername: string | undefined;
  },
  workspaceName: string | undefined,
  workspaceId: string | undefined,
  remote: string | undefined
) {
  if (opts.workspace) {
    log.info(
      colors.red.bold(
        "! Workspace needs to be specified as positional argument, not as option."
      )
    );
    return;
  }

  while (workspaceName === undefined) {
    if (!workspaceName) {
      workspaceName = await Input.prompt("Name this workspace:");
    }
  }

  if (!workspaceId) {
    workspaceId = await Input.prompt({
      message: "Enter the ID of this workspace",
      default: workspaceName,
      suggestions: [workspaceName],
    });
  }

  if (!remote) {
    // first check whether workspaceId is actually a URL
    try {
      const url = new URL(workspaceId);
      workspaceId = workspaceName;
      remote = url.toString();
    } catch {
      // not a url
      remote = new URL(
        await Input.prompt({
          message: "Enter the Remote URL",
          suggestions: ["https://app.windmill.dev/"],
          default: "https://app.windmill.dev/",
        })
      ).toString();
    }
  }
  remote = new URL(remote).toString(); // add trailing slash in all cases!

  let token = await tryGetLoginInfo(opts);
  if (!token && Deno.stdin.isTerminal && !Deno.stdin.isTerminal()) {
    log.info("Not a TTY, can't login interactively. Pass the token in --token");
    return;
  }
  while (!token) {
    token = await loginInteractive(remote);
  }

  setClient(
    token,
    remote.endsWith("/") ? remote.substring(0, remote.length - 1) : remote
  );
  let alreadyExists = false;
  try {
    alreadyExists = await wmill.existsWorkspace({
      requestBody: { id: workspaceId },
    });
  } catch (e) {
    log.info(
      colors.red.bold("! Credentials or instance is invalid. Aborting.")
    );
    throw e;
  }
  if (opts.create) {
    if (!alreadyExists) {
      log.info(
        colors.yellow(
          `Workspace at id ${workspaceId} on ${remote} does not exist. Creating...`
        )
      );
      const automateUsernameCreation: boolean =
        ((await wmill.getGlobal({
          key: "automate_username_creation",
        })) as any) ?? false;
      await wmill.createWorkspace({
        requestBody: {
          id: workspaceId,
          name: opts.createWorkspaceName ?? workspaceName,
          username: automateUsernameCreation ? undefined : opts.createUsername,
        },
      });
    }
  } else if (!alreadyExists) {
    log.info(
      colors.red.bold(
        `! Workspace at id ${workspaceId} on ${remote} does not exist. Re-run with --create to create it. Aborting.`
      )
    );
    log.info(
      "On that instance and with those credentials, the workspaces that you can access are:"
    );
    const workspaces = await wmill.listWorkspaces();
    for (const workspace of workspaces) {
      log.info(`- ${workspace.id} (name: ${workspace.name})`);
    }
    Deno.exit(1);
  }

  await addWorkspace(
    {
      name: workspaceName,
      remote: remote,
      workspaceId: workspaceId,
      token: token,
    },
    opts
  );
  await setActiveWorkspace(workspaceName);

  log.info(
    colors.green.underline(
      `Added workspace ${workspaceName} for ${workspaceId} on ${remote}!`
    )
  );
}

export async function addWorkspace(workspace: Workspace, opts: any) {
  workspace.remote = new URL(workspace.remote).toString(); // add trailing slash in all cases!
  await removeWorkspace(workspace.name, true, opts);
  const file = await Deno.open((await getRootStore()) + "remotes.ndjson", {
    append: true,
    write: true,
    read: true,
    create: true,
  });
  await file.write(new TextEncoder().encode(JSON.stringify(workspace) + "\n"));

  file.close();
}

export async function removeWorkspace(
  name: string,
  silent: boolean,
  opts: any
) {
  const orgWorkspaces = await allWorkspaces();
  if (orgWorkspaces.findIndex((x) => x.name === name) === -1) {
    if (!silent) {
      log.info(
        colors.red.bold(`! Workspace profile ${name} does not exist locally`)
      );
      log.info("available workspace profiles:");
      await list(opts);
    }
    return;
  }
  if (silent) {
    log.info(colors.yellow(`Replacing existing workspace ${name}`));
  }

  await Deno.writeTextFile(
    (await getRootStore()) + "remotes.ndjson",
    orgWorkspaces
      .filter((x) => x.name !== name)
      .map((x) => JSON.stringify(x))
      .join("\n") + "\n"
  );

  if (!silent) {
    log.info(colors.green.underline(`Succesfully removed workspace ${name}!`));
  }
}

async function remove(_opts: GlobalOptions, name: string) {
  await removeWorkspace(name, false, _opts);
}

async function whoami(_opts: GlobalOptions) {
  await requireLogin(_opts);
  log.info(await wmill.globalWhoami());
  const activeName = await getActiveWorkspaceName(_opts);
  log.info("Active: " + colors.green.bold(activeName || "none"));
}

const command = new Command()
  .description("workspace related commands")
  .action(list as any)
  .command("switch")
  .complete("workspace", async () => (await allWorkspaces()).map((x) => x.name))
  .description("Switch to another workspace")
  .arguments("<workspace_name:string:workspace>")
  .action(switchC as any)
  .command("add")
  .description("Add a workspace")
  .arguments("[workspace_name:string] [workspace_id:string] [remote:string]")
  .option("-c --create", "Create the workspace if it does not exist")
  .option(
    "--create-workspace-name <workspace_name:string>",
    "Specify the workspace name. Ignored if --create is not specified or the workspace already exists. Will default to the workspace id."
  )
  .option(
    "--create-username <username:string>",
    "Specify your own username in the newly created workspace. Ignored if --create is not specified, the workspace already exists or automatic username creation is enabled on the instance.",
    {
      default: "admin",
    }
  )
  .action(add as any)
  .command("remove")
  .description("Remove a workspace")
  .arguments("<workspace_name:string>")
  .action(remove as any)
  .command("whoami")
  .description("Show the currently active user")
  .action(whoami as any);

export default command;
