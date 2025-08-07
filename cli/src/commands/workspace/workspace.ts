// deno-lint-ignore-file no-explicit-any
import { GlobalOptions } from "../../types.ts";
import { getActiveWorkspaceConfigFilePath, getWorkspaceConfigFilePath } from "../../../windmill-utils-internal/src/config/config.ts";
import { loginInteractive, tryGetLoginInfo } from "../../core/login.ts";
import { colors, Command, Confirm, Input, log, setClient, Table } from "../../../deps.ts";
import { requireLogin } from "../../core/auth.ts";

import * as wmill from "../../../gen/services.gen.ts";

export interface Workspace {
  remote: string;
  workspaceId: string;
  name: string;
  token: string;
}

export async function allWorkspaces(configDirOverride?: string): Promise<Workspace[]> {
  try {
    const file = await getWorkspaceConfigFilePath(configDirOverride);
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
    const file = await getActiveWorkspaceConfigFilePath(opts?.configDir);
    return await Deno.readTextFile(file);
  } catch {
    return undefined;
  }
}

export async function getActiveWorkspace(
  opts: GlobalOptions | undefined = undefined
): Promise<Workspace | undefined> {
  const name = await getActiveWorkspaceName(opts);
  if (!name) {
    return undefined;
  }
  return await getWorkspaceByName(name, opts?.configDir);
}

export async function getWorkspaceByName(
  workspaceName: string,
  configDirOverride?: string
): Promise<Workspace | undefined> {
  const workspaceStream = await allWorkspaces(configDirOverride);
  for await (const workspace of workspaceStream) {
    if (workspace.name === workspaceName) {
      return workspace;
    }
  }
  return undefined;
}

async function list(opts: GlobalOptions) {
  const workspaces = await allWorkspaces(opts.configDir);
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

  const all = await allWorkspaces(opts.configDir);
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

  await setActiveWorkspace(workspaceName, opts.configDir);
  return;
}

export async function setActiveWorkspace(workspaceName: string, configDirOverride?: string) {
  const file = await getActiveWorkspaceConfigFilePath(configDirOverride);
  await Deno.writeTextFile(
    file,
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
  await setActiveWorkspace(workspaceName, opts.configDir);

  log.info(
    colors.green.underline(
      `Added workspace ${workspaceName} for ${workspaceId} on ${remote}!`
    )
  );
}

export async function addWorkspace(workspace: Workspace, opts: any) {
  workspace.remote = new URL(workspace.remote).toString(); // add trailing slash in all cases!

  // Check for conflicts before adding
  const existingWorkspaces = await allWorkspaces(opts.configDir);
  const isInteractive = Deno.stdin.isTerminal() && Deno.stdout.isTerminal() && !opts.force;

  // Check 1: Workspace name already exists
  const nameConflict = existingWorkspaces.find(w => w.name === workspace.name);
  if (nameConflict) {
    // If it's the exact same workspace (same remote + workspaceId), just update the token
    if (nameConflict.remote === workspace.remote && nameConflict.workspaceId === workspace.workspaceId) {
      log.info(colors.yellow(`Updating token for existing workspace "${workspace.name}"`));
    } else {
      // Different remote or workspaceId - this is a conflict
      log.info(colors.red.bold(`❌ Workspace name "${workspace.name}" already exists!`));
      log.info(`   Existing: ${nameConflict.workspaceId} on ${nameConflict.remote}`);
      log.info(`   New:      ${workspace.workspaceId} on ${workspace.remote}`);

      if (!isInteractive) {
        // In non-interactive mode (tests, scripts), auto-overwrite with force flag
        if (opts.force) {
          log.info(colors.yellow("Force flag enabled, overwriting existing workspace."));
        } else {
          throw new Error("Workspace name conflict. Use --force to overwrite or choose a different name.");
        }
      } else {
        const overwrite = await Confirm.prompt({
          message: "Do you want to overwrite the existing workspace?",
          default: false,
        });

        if (!overwrite) {
          log.info(colors.yellow("Operation cancelled."));
          return;
        }
      }
    }
  }

  // Remove existing workspace with same name (if updating)
  await removeWorkspace(workspace.name, true, opts);

  // Add the new workspace
  const filePath = await getWorkspaceConfigFilePath(opts.configDir);
  const file = await Deno.open(filePath, {
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
  const orgWorkspaces = await allWorkspaces(opts.configDir);
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
  if (!silent) {
    log.info(colors.yellow(`Removing existing workspace ${name}`));
  }

  const filePath = await getWorkspaceConfigFilePath(opts.configDir);
  await Deno.writeTextFile(
    filePath,
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

async function bind(
  opts: GlobalOptions & { branch?: string },
  bindWorkspace?: boolean
) {
  const { isGitRepository, getCurrentGitBranch } = await import("../../utils/git.ts");

  if (!isGitRepository()) {
    log.error(colors.red("Not in a Git repository"));
    return;
  }

  const branch = opts.branch || getCurrentGitBranch();
  if (!branch) {
    log.error(colors.red("Could not determine current Git branch"));
    return;
  }

  const { readConfigFile } = await import("../../core/conf.ts");
  const config = await readConfigFile();

  const activeWorkspace = await getActiveWorkspace(opts);
  if (!activeWorkspace && bindWorkspace) {
    log.error(colors.red("No active workspace. Use 'wmill workspace add' or 'wmill workspace switch' first"));
    return;
  }

  // For unbind, check if branch exists
  if (!bindWorkspace && (!config.git_branches || !config.git_branches[branch])) {
    log.error(colors.red(`Branch '${branch}' not found in wmill.yaml git_branches`));
    return;
  }

  // Update the branch configuration with workspace binding
  if (!config.git_branches) {
    config.git_branches = {};
  }
  if (!config.git_branches[branch]) {
    config.git_branches[branch] = { overrides: {} };
  }

  if (bindWorkspace && activeWorkspace) {
    config.git_branches[branch].baseUrl = activeWorkspace.remote;
    config.git_branches[branch].workspaceId = activeWorkspace.workspaceId;

    log.info(colors.green(
      `✓ Bound branch '${branch}' to workspace '${activeWorkspace.name}'\n` +
      `  ${activeWorkspace.workspaceId} on ${activeWorkspace.remote}`
    ));
  } else {
    // Unbind
    delete config.git_branches[branch].baseUrl;
    delete config.git_branches[branch].workspaceId;

    log.info(colors.green(`✓ Removed workspace binding from branch '${branch}'`));
  }

  // Write back the updated config
  const { yamlStringify } = await import("../../../deps.ts");
  try {
    await Deno.writeTextFile("wmill.yaml", yamlStringify(config));
  } catch (error) {
    log.error(colors.red(`Failed to save configuration: ${error.message}`));
    return;
  }
}

const command = new Command()
  .alias("profile")
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
  .action(whoami as any)
  .command("bind")
  .description("Bind the current Git branch to the active workspace")
  .option("--branch <branch:string>", "Specify branch (defaults to current)")
  .action((opts) => bind(opts as any, true))
  .command("unbind")
  .description("Remove workspace binding from the current Git branch")
  .option("--branch <branch:string>", "Specify branch (defaults to current)")
  .action((opts) => bind(opts as any, false));

export default command;
