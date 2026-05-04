import { writeFile, open as fsOpen } from "node:fs/promises";
import { readTextFile } from "../../utils/utils.ts";
import process from "node:process";
import { GlobalOptions } from "../../types.ts";
import {
  getActiveWorkspaceConfigFilePath,
  getWorkspaceConfigFilePath,
} from "../../../windmill-utils-internal/src/config/config.ts";
import { loginInteractive, tryGetLoginInfo } from "../../core/login.ts";
import { colors } from "@cliffy/ansi/colors";
import { Command } from "@cliffy/command";
import { Confirm } from "@cliffy/prompt/confirm";
import { Input } from "@cliffy/prompt/input";
import { Table } from "@cliffy/table";
import * as log from "../../core/log.ts";
import { setClient } from "../../core/client.ts";
import { requireLogin } from "../../core/auth.ts";
import { createWorkspaceFork, deleteWorkspaceFork } from "./fork.ts";
import { mergeWorkspaces } from "./merge.ts";
import { connectSlack, disconnectSlack } from "./slack.ts";

import * as wmill from "../../../gen/services.gen.ts";

export interface Workspace {
  remote: string;
  workspaceId: string;
  name: string;
  token: string;
}

export async function allWorkspaces(
  configDirOverride?: string
): Promise<Workspace[]> {
  try {
    const file = await getWorkspaceConfigFilePath(configDirOverride);
    const txt = await readTextFile(file);
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
    return await readTextFile(file);
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

export async function list(opts: GlobalOptions) {
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
  const workspace = await getWorkspaceByName(workspaceName, opts.configDir);
  log.info(
    colors.green.bold(
      `Switched to workspace ${workspaceName} (${workspace?.workspaceId} on ${workspace?.remote})`
    )
  );
  return;
}

export async function setActiveWorkspace(
  workspaceName: string,
  configDirOverride?: string
) {
  const file = await getActiveWorkspaceConfigFilePath(configDirOverride);
  await writeFile(file, workspaceName, "utf-8");
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
  if (!token && !(process.stdin.isTTY ?? false)) {
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
    if (workspaces.length === 0) {
      log.info("  (none)");
    } else {
      for (const workspace of workspaces) {
        log.info(`- ${workspace.id} (name: ${workspace.name})`);
      }
    }
    process.exit(1);
  }

  const added = await addWorkspace(
    {
      name: workspaceName,
      remote: remote,
      workspaceId: workspaceId,
      token: token,
    },
    opts
  );
  if (!added) {
    return;
  }
  await setActiveWorkspace(workspaceName, opts.configDir);

  log.info(
    colors.green.underline(
      `Added workspace ${workspaceName} for ${workspaceId} on ${remote}!`
    )
  );
}

export async function addWorkspace(workspace: Workspace, opts: any): Promise<boolean> {
  workspace.remote = new URL(workspace.remote).toString(); // add trailing slash in all cases!

  // Check for conflicts before adding
  const existingWorkspaces = await allWorkspaces(opts.configDir);
  const isInteractive =
    (process.stdin.isTTY ?? false) && (process.stdout.isTTY ?? false) && !opts.force;

  // Check 1: Workspace name already exists
  const nameConflict = existingWorkspaces.find(
    (w) => w.name === workspace.name
  );
  if (nameConflict) {
    // If it's the exact same workspace (same remote + workspaceId), just update the token
    if (
      nameConflict.remote === workspace.remote &&
      nameConflict.workspaceId === workspace.workspaceId
    ) {
      log.info(
        colors.yellow(
          `Updating token for existing workspace "${workspace.name}"`
        )
      );
    } else {
      // Different remote or workspaceId - this is a conflict
      log.info(
        colors.red.bold(`❌ Workspace name "${workspace.name}" already exists!`)
      );
      log.info(
        `   Existing: ${nameConflict.workspaceId} on ${nameConflict.remote}`
      );
      log.info(`   New:      ${workspace.workspaceId} on ${workspace.remote}`);

      if (!isInteractive) {
        // In non-interactive mode (tests, scripts), auto-overwrite with force flag
        if (opts.force) {
          log.info(
            colors.yellow("Force flag enabled, overwriting existing workspace.")
          );
        } else {
          throw new Error(
            "Workspace name conflict. Use --force to overwrite or choose a different name."
          );
        }
      } else {
        const overwrite = await Confirm.prompt({
          message: "Do you want to overwrite the existing workspace?",
          default: false,
        });

        if (!overwrite) {
          log.info(colors.yellow("Operation cancelled."));
          return false;
        }
      }
    }
  }

  // Check 2: Same (remote, workspaceId) already exists with a different name
  const backendConflict = existingWorkspaces.find(
    (w) =>
      w.remote === workspace.remote &&
      w.workspaceId === workspace.workspaceId &&
      w.name !== workspace.name
  );
  if (backendConflict) {
    if (opts.force) {
      // Remove the conflicting workspace before adding the new one
      await removeWorkspace(backendConflict.name, true, opts);
    } else {
      throw new Error(
        `Backend constraint violation: (${workspace.remote}, ${workspace.workspaceId}) already exists as "${backendConflict.name}". Use --force to overwrite.`
      );
    }
  }

  // Remove existing workspace with same name (if updating)
  await removeWorkspace(workspace.name, true, opts);

  // Add the new workspace
  const filePath = await getWorkspaceConfigFilePath(opts.configDir);
  const fh = await fsOpen(filePath, "a");
  await fh.write(JSON.stringify(workspace) + "\n");
  await fh.close();
  return true;
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
  await writeFile(
    filePath,
    orgWorkspaces
      .filter((x) => x.name !== name)
      .map((x) => JSON.stringify(x))
      .join("\n") + "\n",
    "utf-8"
  );

  if (!silent) {
    log.info(colors.green.underline(`Succesfully removed workspace ${name}!`));
  }
}

async function remove(_opts: GlobalOptions, name: string) {
  await removeWorkspace(name, false, _opts);
}

async function whoami(_opts: GlobalOptions) {
  const whoamiInfo = await requireLogin(_opts);
  log.info(JSON.stringify(whoamiInfo, null, 2));
  const activeName = await getActiveWorkspaceName(_opts);
  const { getCurrentGitBranch, getOriginalBranchForWorkspaceForks } = await import("../../utils/git.ts");
  const branch = getCurrentGitBranch();
  const originalBranch = branch ? getOriginalBranchForWorkspaceForks(branch) : null;
  if (originalBranch) {
    const { resolveWorkspace } = await import("../../core/context.ts");
    try {
      const ws = await resolveWorkspace(_opts);
      log.info("Active: " + colors.green.bold(ws.workspaceId) + ` (fork of ${activeName || "unknown"})`);
    } catch {
      log.info("Active: " + colors.green.bold(activeName || "none") + " (fork branch)");
    }
  } else {
    log.info("Active: " + colors.green.bold(activeName || "none"));
  }
}

async function listRemote(_opts: GlobalOptions) {
  let remote: string;

  if (_opts.baseUrl && _opts.token && !_opts.workspace) {
    // Allow listing workspaces with just --base-url and --token (no --workspace needed)
    const { setClient } = await import("../../core/client.ts");
    remote = new URL(_opts.baseUrl).toString();
    setClient(_opts.token, remote.replace(/\/$/, ""));
  } else {
    const { resolveWorkspace } = await import("../../core/context.ts");
    const workspace = await resolveWorkspace(_opts);
    await requireLogin(_opts);
    remote = workspace.remote;
  }

  const userWorkspaces = await wmill.listUserWorkspaces();

  const hasForks = userWorkspaces.workspaces.some((x) => x.parent_workspace_id);
  const headers = hasForks
    ? ["id", "name", "username", "fork of", "disabled"]
    : ["id", "name", "username", "disabled"];

  new Table()
    .header(headers)
    .padding(2)
    .border(true)
    .body(
      userWorkspaces.workspaces.map((x) => {
        const row = [
          x.id,
          x.name,
          x.username,
        ];
        if (hasForks) row.push(x.parent_workspace_id ?? "-");
        row.push(x.disabled ? colors.red("true") : "false");
        return row;
      })
    )
    .render();

  log.info(`Remote: ${colors.bold(remote)}`);
  log.info(`Logged in as: ${colors.green.bold(userWorkspaces.email)}`);
}

async function listForks(_opts: GlobalOptions) {
  const { resolveWorkspace } = await import("../../core/context.ts");
  const workspace = await resolveWorkspace(_opts);
  await requireLogin(_opts);

  const userWorkspaces = await wmill.listUserWorkspaces();
  const forks = userWorkspaces.workspaces.filter((w) => w.parent_workspace_id);

  if (forks.length === 0) {
    log.info("No forked workspaces found.");
    return;
  }

  new Table()
    .header(["id", "name", "fork of", "username"])
    .padding(2)
    .border(true)
    .body(
      forks.map((x) => [
        x.id,
        x.name,
        x.parent_workspace_id ?? "",
        x.username,
      ])
    )
    .render();

  log.info(`Remote: ${colors.bold(workspace.remote)}`);
}

export async function getActiveWorkspaceOrFallback(opts: GlobalOptions) {
  let activeWorkspace = await getActiveWorkspace(opts);
  if (!activeWorkspace && opts.baseUrl && opts.workspace) {
    activeWorkspace = {
      name: opts.workspace,
      remote: opts.baseUrl,
      workspaceId: opts.workspace,
      token: "",
    };
  }
  return activeWorkspace;
}
async function bind(
  opts: GlobalOptions & { workspace?: string; branch?: string },
  doBind: boolean
) {
  const { isGitRepository, getCurrentGitBranch } = await import(
    "../../utils/git.ts"
  );
  const { readConfigFile, findWorkspaceByGitBranch, getWorkspaceNames } = await import("../../core/conf.ts");
  const { stringify: yamlStringify } = await import("yaml");
  const config = await readConfigFile();

  if (!config.workspaces) {
    config.workspaces = {} as any;
  }

  const isInteractive = !!process.stdin.isTTY;
  const inGitRepo = isGitRepository();
  const currentBranch = inGitRepo ? getCurrentGitBranch() : null;

  if (doBind) {
    // ── BIND ──────────────────────────────────────────────────

    // Step 1: Pick workspace profile (the source of baseUrl/workspaceId/token)
    const profiles = await allWorkspaces(opts.configDir);
    let selectedProfile: Workspace | undefined;

    if (profiles.length === 0) {
      // No profiles — run wmill workspace add flow
      log.info(colors.yellow("No workspace profiles found. Let's create one."));
      await add(opts as any, undefined, undefined, undefined);
      // Re-read profiles after add
      const updatedProfiles = await allWorkspaces(opts.configDir);
      selectedProfile = updatedProfiles.length > 0
        ? await getActiveWorkspace(opts)
        : undefined;
      if (!selectedProfile) {
        log.error(colors.red("Profile creation failed or was cancelled."));
        return;
      }
    } else if (opts.workspace && profiles.find((p) => p.name === opts.workspace)) {
      // --workspace flag matches a profile name: use it directly
      selectedProfile = profiles.find((p) => p.name === opts.workspace);
    } else if (isInteractive) {
      const { Select } = await import("@cliffy/prompt/select");
      const activeProfile = await getActiveWorkspace(opts);
      const selectedName = await Select.prompt({
        message: "Select workspace profile to bind",
        options: profiles.map((p) => ({
          name: `${p.name} (${p.workspaceId} on ${p.remote})`,
          value: p.name,
        })),
        default: activeProfile?.name,
      });
      selectedProfile = profiles.find((p) => p.name === selectedName);
    } else {
      // Non-interactive: use active profile
      selectedProfile = await getActiveWorkspaceOrFallback(opts);
    }

    if (!selectedProfile) {
      log.error(colors.red("No workspace profile selected. Aborting."));
      return;
    }

    // Step 2: Pick workspace name (the key in wmill.yaml workspaces section)
    let wsName: string;
    if (opts.workspace) {
      wsName = opts.workspace;
    } else if (isInteractive) {
      const { Input } = await import("@cliffy/prompt/input");
      wsName = await Input.prompt({
        message: "Workspace name (key in wmill.yaml)",
        default: selectedProfile.workspaceId,
      });
    } else {
      wsName = selectedProfile.workspaceId;
    }

    // Step 3: Pick git branch (only in git repos)
    let gitBranch: string | undefined;
    if (inGitRepo) {
      if (opts.branch) {
        if (opts.branch !== wsName) {
          gitBranch = opts.branch;
        }
      } else if (isInteractive) {
        const { Input } = await import("@cliffy/prompt/input");
        const branchInput = await Input.prompt({
          message: "Git branch to associate",
          default: currentBranch ?? wsName,
        });
        if (branchInput !== wsName) {
          gitBranch = branchInput;
        }
      } else if (currentBranch && currentBranch !== wsName) {
        gitBranch = currentBranch;
      }
    }

    // Step 4: Write the entry
    const entry = (config.workspaces as any)[wsName] ?? {};
    entry.baseUrl = selectedProfile.remote;
    if (selectedProfile.workspaceId !== wsName) {
      entry.workspaceId = selectedProfile.workspaceId;
    } else {
      delete entry.workspaceId; // clean up if it matches
    }
    if (gitBranch) {
      entry.gitBranch = gitBranch;
    } else {
      delete entry.gitBranch; // clean up if it matches
    }
    (config.workspaces as any)[wsName] = entry;

    log.info(
      colors.green(
        `✓ Bound workspace '${wsName}'` +
          (gitBranch ? ` (gitBranch: ${gitBranch})` : "") +
          ` → ${selectedProfile.workspaceId} on ${selectedProfile.remote}`
      )
    );
  } else {
    // ── UNBIND ────────────────────────────────────────────────
    let wsName: string | undefined;

    if (opts.workspace) {
      wsName = opts.workspace;
    } else if (currentBranch) {
      const match = findWorkspaceByGitBranch(config.workspaces, currentBranch);
      wsName = match?.[0];
    }

    if (!wsName && isInteractive) {
      const names = getWorkspaceNames(config.workspaces);
      if (names.length === 0) {
        log.error(colors.red("No workspaces configured in wmill.yaml."));
        return;
      }
      const { Select } = await import("@cliffy/prompt/select");
      wsName = await Select.prompt({
        message: "Select workspace to unbind",
        options: names,
      });
    }

    if (!wsName || !(config.workspaces as any)[wsName]) {
      log.error(colors.red(
        wsName
          ? `Workspace '${wsName}' not found in wmill.yaml.`
          : "Could not determine workspace. Use --workspace to specify."
      ));
      return;
    }

    const entry = (config.workspaces as any)[wsName];
    delete entry.baseUrl;
    delete entry.workspaceId;
    log.info(colors.green(`✓ Removed binding from workspace '${wsName}'`));
  }

  try {
    await writeFile("wmill.yaml", yamlStringify(config), "utf-8");
  } catch (error) {
    log.error(colors.red(`Failed to save configuration: ${(error as Error).message}`));
    return;
  }

  // After a successful bind, offer to generate resource type namespace
  if (doBind && isInteractive) {
    const { stat: statFile } = await import("node:fs/promises");
    const rtExists = await statFile("rt.d.ts").then(() => true, () => false);
    const { Confirm } = await import("@cliffy/prompt/confirm");
    const generate = await Confirm.prompt({
      message: "Generate rt.d.ts? (TypeScript types for your workspace's resource types, useful for autocompletion)",
      default: !rtExists,
    });
    if (generate) {
      try {
        const { generateRTNamespace } = await import("../resource-type/resource-type.ts");
        await generateRTNamespace(opts);
      } catch (error) {
        log.warn(
          `Could not generate resource type namespace: ${
            error instanceof Error ? error.message : error
          }`
        );
      }
    }
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
  .command("list")
  .description("List local workspace profiles")
  .action(list as any)
  .command("list-remote")
  .description("List workspaces on the remote server that you have access to")
  .action(listRemote as any)
  .command("list-forks")
  .description("List forked workspaces on the remote server")
  .action(listForks as any)
  .command("bind")
  .description("Create or update a workspace entry in wmill.yaml from the active profile")
  .option("--workspace <name:string>", "Workspace name (default: current branch or workspaceId)")
  .option("--branch <branch:string>", "Git branch to associate (default: workspace name)")
  .action((opts) => bind(opts as any, true))
  .command("unbind")
  .description("Remove baseUrl and workspaceId from a workspace entry")
  .option("--workspace <name:string>", "Workspace to unbind")
  .action((opts) => bind(opts as any, false))
  .command("fork")
  .description("Create a forked workspace")
  .arguments("[workspace_name:string] [workspace_id:string]")
  .option(
    "--create-workspace-name <workspace_name:string>",
    "Specify the workspace name. Ignored if --create is not specified or the workspace already exists. Will default to the workspace id."
  )
  .option("--color <color:string>", "Workspace color (hex code, e.g. #ff0000)")
  .option(
    "--datatable-behavior <behavior:string>",
    "How to handle datatables: skip, schema_only, or schema_and_data (default: interactive prompt)"
  )
  .option("-y --yes", "Skip interactive prompts (defaults datatable behavior to 'skip')")
  .action(createWorkspaceFork as any)
  .command("delete-fork")
  .description("Delete a forked workspace and git branch")
  .arguments("<fork_name:string>")
  .option("-y --yes", "Skip confirmation prompt")
  .action(deleteWorkspaceFork as any)
  .command("merge")
  .description("Compare and deploy changes between a fork and its parent workspace")
  .option("--direction <direction:string>", "Deploy direction: to-parent or to-fork")
  .option("--all", "Deploy all changed items including conflicts")
  .option("--skip-conflicts", "Skip items modified in both workspaces")
  .option("--include <items:string>", "Comma-separated kind:path items to include (e.g. script:f/test/main,flow:f/my/flow)")
  .option("--exclude <items:string>", "Comma-separated kind:path items to exclude")
  .option("--preserve-on-behalf-of", "Preserve original on_behalf_of/permissioned_as values")
  .option("-y --yes", "Non-interactive mode (deploy without prompts)")
  .action(mergeWorkspaces as any)
  .command("connect-slack")
  .description(
    "Non-interactively connect Slack to the active workspace using a pre-minted bot token (xoxb-...). Produces the same artifacts as the UI OAuth flow: workspace_settings fields, g/slack group, f/slack_bot folder, and the encrypted bot token variable + resource at f/slack_bot/bot_token."
  )
  .option("--bot-token <bot_token:string>", "Slack bot token (xoxb-...)", { required: true })
  .option("--team-id <team_id:string>", "Slack team id", { required: true })
  .option("--team-name <team_name:string>", "Slack team name", { required: true })
  .action(connectSlack as any)
  .command("disconnect-slack")
  .description(
    "Clear slack_team_id / slack_name on the active workspace (marks the workspace as disconnected). Does NOT remove the bot token variable/resource/folder/group — delete those from the local sync folder and run 'wmill sync push' to tear them down. Does NOT remove the workspace-level OAuth override — set slack_oauth_client_id/_secret to '' in settings.yaml and push."
  )
  .action(disconnectSlack as any);

export default command;
