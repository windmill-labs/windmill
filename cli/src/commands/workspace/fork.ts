import { GlobalOptions } from "../../types.ts";
import { colors } from "@cliffy/ansi/colors";
import { Input } from "@cliffy/prompt/input";
import * as log from "../../core/log.ts";
import { setClient } from "../../core/client.ts";
import {   allWorkspaces, list, removeWorkspace } from "./workspace.ts";
import * as wmill from "../../../gen/services.gen.ts";
import { getCurrentGitBranch, getOriginalBranchForWorkspaceForks, isGitRepository } from "../../utils/git.ts";
import { WM_FORK_PREFIX } from "../../core/constants.ts";
import { tryResolveBranchWorkspace } from "../../core/context.ts";

// NOTE: This import will work after regenerating the API client
// Run ./gen_wm_client.sh to regenerate after backend changes
// import * as wmill from "../../../gen/services.gen.ts";

async function createWorkspaceFork(
  opts: GlobalOptions & {
    createWorkspaceName: string | undefined;
  },
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
  if (!currentBranch) {
    throw new Error("Could not get git branch name");
  }
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
      workspace: workspace.workspaceId,
      requestBody: {
        id: trueWorkspaceId,
        name: opts.createWorkspaceName ?? trueWorkspaceId,
        color: undefined,
      },
    });

    log.info(colors.green(`✅ ${result}`));

  } catch (error) {
    // If workspace creation fails, we should clean up the git branch
    log.error(
      colors.red(`Failed to create forked workspace: ${(error as Error).message}`),
    );
    throw error;
  }


  const newBranchName = `${WM_FORK_PREFIX}/${clonedBranchName}/${workspaceId}`

  log.info(`Created forked workspace ${trueWorkspaceId}. To start contributing to your fork, create and push edits to the branch \`${newBranchName}\` by using the command:
    
\t`+colors.white(`git checkout -b ${newBranchName}`) + `
    
When doing operations on the forked workspace, it will use the remote setup in gitBranches for the branch it was forked from.`);
}

async function deleteWorkspaceFork(
  opts: GlobalOptions & {
    yes?: boolean;
  },
  name: string,
) {
  const orgWorkspaces = await allWorkspaces(opts.configDir);
  const idxOf = orgWorkspaces.findIndex((x) => x.name === name) ;
  if (idxOf === -1) {
      log.info(
        colors.red.bold(`! Workspace profile ${name} does not exist locally`)
      );
      log.info("available workspace profiles:");
      await list(opts);
    return;
  }

  const workspace = orgWorkspaces[idxOf];

  if (!workspace.workspaceId.startsWith(WM_FORK_PREFIX)) {
      throw new Error(
        `You can only delete forked workspaces where the workspace id starts with \`${WM_FORK_PREFIX}.\` Failed while attempting to delete \`${workspace.workspaceId}\``,
      );
  }

  if (!opts.yes) {
        const { Select } = await import("@cliffy/prompt/select");
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
  await removeWorkspace(name, false, opts);
}

export { createWorkspaceFork, deleteWorkspaceFork };
