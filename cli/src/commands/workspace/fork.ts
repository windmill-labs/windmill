// deno-lint-ignore-file no-explicit-any
import { GlobalOptions } from "../../types.ts";
import { colors, Command, Input, log, setClient } from "../../../deps.ts";
import { requireLogin } from "../../core/auth.ts";
import { add, addWorkspace, allWorkspaces, getActiveWorkspace, list, removeWorkspace } from "./workspace.ts";
import { loginInteractive, tryGetLoginInfo } from "../../core/login.ts";
import * as wmill from "../../../gen/services.gen.ts";
import { getCurrentGitBranch, getOriginalBranchForWorkspaceForks, isGitRepository } from "../../utils/git.ts";
import { WM_FORK_PREFIX } from "../../main.ts";
import { tryResolveBranchWorkspace } from "../../core/context.ts";

// NOTE: This import will work after regenerating the API client
// Run ./gen_wm_client.sh to regenerate after backend changes
// import * as wmill from "../../../gen/services.gen.ts";

async function runGitCommand(
  args: string[],
): Promise<{ success: boolean; output: string }> {
  try {
    const command = new Deno.Command("git", {
      args,
      stdout: "piped",
      stderr: "piped",
    });

    const { code, stdout, stderr } = await command.output();
    const output = new TextDecoder().decode(code === 0 ? stdout : stderr);

    return {
      success: code === 0,
      output: output.trim(),
    };
  } catch (error) {
    return {
      success: false,
      output: `Failed to execute git command: ${error.message}`,
    };
  }
}

async function createWorkspaceFork(
  opts: GlobalOptions,
  workspaceName: string | undefined,
  workspaceId: string | undefined = undefined,
) {
  if (!isGitRepository()) {
    throw new Error("You can only create forks within a git repo. Forks are tracked with git and synced to your instance with the git sync workflow.");
  }

  const workspace = await tryResolveBranchWorkspace(opts);

  if (!workspace) {
    throw new Error("Could not resolve workspace from branch name. Make sure you are in a git repo to use workspace forks");
  }

  log.info(`You are forking workspace (${workspace.workspaceId})`)

  const currentBranch = getCurrentGitBranch()
  const originalBranchIfForked = getOriginalBranchForWorkspaceForks(currentBranch);

  let clonedBranchName: string | null;
  if (originalBranchIfForked) {
    log.info(`You are creating a fork of a fork. The branch will be linked to the original branch this was forked from, i.e. \`${originalBranchIfForked}\`, for all settings and overrides.`);
    clonedBranchName = originalBranchIfForked;
  } else {
    clonedBranchName = currentBranch;
  }

  if (!clonedBranchName) {
    throw new Error("Failed to get current branch name, aborting operation");
  }

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
      workspaceName = await Input.prompt("Name this forked workspace:");
    }
  }

  if (!workspaceId) {
    workspaceId = await Input.prompt({
      message: `Enter the ID of this forked workspace, it will then be prefixed by ${WM_FORK_PREFIX}. It will also determine the branch name`,
      default: workspaceName,
      suggestions: [workspaceName],
    });
  }

  const token = workspace.token;

  if (!token) {
    throw new Error("Not logged in. Please run 'wmill workspace add' first.");
  }

  const remote = workspace.remote
  setClient(
    token,
    remote.endsWith("/") ? remote.substring(0, remote.length - 1) : remote
  );

  log.info(colors.blue(`Creating forked workspace: ${workspaceName}...`));

  const trueWorkspaceId = `${WM_FORK_PREFIX}-${workspaceId}`;
  let alreadyExists = false;
  try {
    alreadyExists = await wmill.existsWorkspace({
      requestBody: { id: trueWorkspaceId },
    });
  } catch (e) {
    log.info(
      colors.red.bold("! Credentials or instance is invalid. Aborting.")
    );
    throw e;
  }

  if (alreadyExists) {
    throw new Error(`This forked workspace '${workspaceId}' (${workspaceName}) already exists. Choose a different id`);
  }

  try {
    // TODO: Update to createWorkspaceFork after regenerating client from new OpenAPI spec
    const result = await wmill.createWorkspaceFork({
      requestBody: {
        id: trueWorkspaceId,
        name: workspaceName,
        username: undefined, // Let the server handle username
        color: undefined,
        parent_workspace_id: workspace.workspaceId,
      },
    });

    log.info(colors.green(`✅ ${result}`));

  } catch (error) {
    // If workspace creation fails, we should clean up the git branch
    log.error(
      colors.red(`Failed to create forked workspace: ${error.message}`),
    );
    throw error;
  }

  await addWorkspace(
    {
      name: workspaceName,
      remote: remote,
      workspaceId: trueWorkspaceId,
      token: token,
    },
    opts
  );

  const newBranchName = `${WM_FORK_PREFIX}/${clonedBranchName}/${workspaceId}`

  log.info(`Created forked workspace ${trueWorkspaceId}. To start contributing to your fork, create and push edits to the branch \`${newBranchName}\` by using the command:\n\n\t`+colors.white(`git checkout -b ${newBranchName}`) + `\n\nThe changes will then be reflected in your fork if you've setup the git sync workflows correctly.`);
}

async function deleteWorkspaceFork(
  opts: GlobalOptions & {
    yes?: boolean;
  },
  silent: boolean,
  name: string,
) {

  const orgWorkspaces = await allWorkspaces(opts.configDir);
  const idxOf = orgWorkspaces.findIndex((x) => x.name === name) ;
  if (idxOf === -1) {
    if (!silent) {
      log.info(
        colors.red.bold(`! Workspace profile ${name} does not exist locally`)
      );
      log.info("available workspace profiles:");
      await list(opts);
    }
    return;
  }

  const workspace = orgWorkspaces[idxOf];

  if (!workspace.workspaceId.startsWith(WM_FORK_PREFIX)) {
      throw new Error(
        `You can only delete forked workspaces where the workspace id starts with \`${WM_FORK_PREFIX}.\` Failed while attempting to delete \`${workspace.workspaceId}\``,
      );
  }

  if (!opts.yes) {
        const { Select } = await import("../../../deps.ts");
        const choice = await Select.prompt({
          message: `Are you sure you want to delete the forked workspace with id: \`${workspace.workspaceId}\`? This action will delete the workspace `,
          options: [
            { name: "Yes", value: "confirm" },
            { name: "No", value: "cancel" },
          ],
        });

        if (choice === "cancel") {
          log.info("Operation cancelled");
          return;
        }
  }

  const remote = workspace.remote
  setClient(
    workspace.token,
    remote.endsWith("/") ? remote.substring(0, remote.length - 1) : remote
  );

  const result = await wmill.deleteWorkspace({
    workspace: workspace.workspaceId
  });
  log.info(
    colors.green(`✅ Forked workspace '${workspace.workspaceId}' deleted successfully!\n${result}`),
  );
  await removeWorkspace(name, silent, opts);
}

const forkCommand = new Command()
  .description("Create a forked workspace and git branch")
  .arguments("[workspace_id:string]")
  .option(
    "--create-workspace-name <workspace_name:string>",
    "Specify the workspace name. Ignored if --create is not specified or the workspace already exists. Will default to the workspace id."
  )
  .action(async (opts: GlobalOptions, workspace_id: string) => {
    await requireLogin(opts);
    await createWorkspaceFork(opts, workspace_id, undefined);
  });

const deleteForkCommand = new Command()
  .description("Delete a forked workspace and git branch")
  .arguments("<fork_name:string>")
  .action(async (opts: GlobalOptions, name: string, silent: boolean) => {
    await deleteWorkspaceFork(opts, silent, name);
  });

export { forkCommand, deleteForkCommand };
