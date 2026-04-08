import { GlobalOptions } from "../../types.ts";
import { colors } from "@cliffy/ansi/colors";
import { Input } from "@cliffy/prompt/input";
import * as log from "../../core/log.ts";
import { setClient } from "../../core/client.ts";
import { allWorkspaces, list, removeWorkspace } from "./workspace.ts";
import * as wmill from "../../../gen/services.gen.ts";
import { getCurrentGitBranch, getOriginalBranchForWorkspaceForks, isGitRepository } from "../../utils/git.ts";
import { WM_FORK_PREFIX } from "../../core/constants.ts";
import { tryResolveBranchWorkspace } from "../../core/context.ts";

async function createWorkspaceFork(
  opts: GlobalOptions & {
    createWorkspaceName: string | undefined;
    color: string | undefined;
    datatableBehavior: string | undefined;
    yes: boolean | undefined;
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

  // --- Datatable cloning (matches ForkDatatableSection.svelte) ---
  interface ForkedDatatableInfo {
    name: string;
    new_dbname: string;
  }
  const forkedDatatables: ForkedDatatableInfo[] = [];

  let datatables: Awaited<ReturnType<typeof wmill.listDataTables>> = [];
  try {
    datatables = await wmill.listDataTables({
      workspace: workspace.workspaceId,
    });
  } catch (e) {
    log.info(
      colors.yellow(
        `Note: Could not list datatables: ${(e as Error).message}`
      )
    );
  }

  if (datatables && datatables.length > 0) {
    const behavior = opts.datatableBehavior ?? (opts.yes ? "skip" : undefined);

    if (behavior !== "skip") {
      log.info(`\nFound ${datatables.length} datatable(s):`);

      for (const dt of datatables) {
        let dtBehavior: string;

        if (behavior === "schema_only" || behavior === "schema_and_data") {
          dtBehavior = behavior;
        } else {
          // Interactive prompt
          const { Select } = await import("@cliffy/prompt/select");
          dtBehavior = await Select.prompt({
            message: `Datatable "${dt.name}" (${dt.resource_type}):`,
            options: [
              { name: "Keep original (no cloning)", value: "keep_original" },
              { name: "Clone schema only", value: "schema_only" },
              { name: "Clone schema and data", value: "schema_and_data" },
            ],
          });
        }

        if (dtBehavior === "keep_original") {
          continue;
        }

        const newDbName = `${trueWorkspaceId.replace(/-/g, "_")}__${dt.name}`;

        try {
          log.info(
            colors.blue(`  Creating database "${newDbName}" for datatable "${dt.name}"...`)
          );

          await wmill.createPgDatabase({
            workspace: workspace.workspaceId,
            requestBody: {
              source: `datatable://${dt.name}`,
              target_dbname: newDbName,
            },
          });

          log.info(
            colors.blue(
              `  Importing ${dtBehavior === "schema_only" ? "schema" : "schema + data"}...`
            )
          );

          await wmill.importPgDatabase({
            workspace: workspace.workspaceId,
            requestBody: {
              source: `datatable://${dt.name}`,
              target: `datatable://${dt.name}`,
              target_dbname_override: newDbName,
              fork_behavior: dtBehavior as "schema_only" | "schema_and_data",
            },
          });

          log.info(colors.green(`  ✓ Datatable "${dt.name}" cloned.`));
          forkedDatatables.push({ name: dt.name, new_dbname: newDbName });
        } catch (e) {
          log.info(
            colors.yellow(
              `  ✗ Failed to clone datatable "${dt.name}": ${(e as Error).message}`
            )
          );
        }
      }
    }
  }

  // --- Create git branch for fork (matches UI: createWorkspaceForkGitBranch) ---
  const forkColor = opts.color;
  try {
    const gitSyncJobIds = await wmill.createWorkspaceForkGitBranch({
      workspace: workspace.workspaceId,
      requestBody: {
        id: trueWorkspaceId,
        name: opts.createWorkspaceName ?? trueWorkspaceId,
        color: forkColor,
      },
    });
    if (gitSyncJobIds && gitSyncJobIds.length > 0) {
      log.info(
        colors.blue(
          `Git sync branch creation triggered (${gitSyncJobIds.length} job(s)). These will complete asynchronously.`
        )
      );
    }
  } catch (e) {
    log.error(
      colors.red(
        `Failed to create git branch for fork: ${(e as Error).message}`
      )
    );
    throw e;
  }

  // --- Create the fork workspace ---
  try {
    const result = await wmill.createWorkspaceFork({
      workspace: workspace.workspaceId,
      requestBody: {
        id: trueWorkspaceId,
        name: opts.createWorkspaceName ?? trueWorkspaceId,
        color: forkColor,
        forked_datatables: forkedDatatables,
      },
    });

    log.info(colors.green(`✅ ${result}`));
  } catch (error) {
    log.error(
      colors.red(`Failed to create forked workspace: ${(error as Error).message}`),
    );
    throw error;
  }


  const newBranchName = `${WM_FORK_PREFIX}/${clonedBranchName}/${workspaceId}`

  log.info(`Created forked workspace ${trueWorkspaceId}. To start contributing to your fork, create and push edits to the branch \`${newBranchName}\` by using the command:

\t`+colors.white(`git checkout -b ${newBranchName}`) + `

When doing operations on the forked workspace, it will use the remote setup in gitBranches for the branch it was forked from.

To merge changes back to the parent workspace, you can:
  - Use the CLI: ` + colors.white(`git checkout ${newBranchName} && wmill workspace merge`) + `
  - Use the Merge UI from the forked workspace home page
  - Use git: ` + colors.white(`git checkout ${clonedBranchName} && git merge ${newBranchName} && wmill sync push`) + `
  See: https://www.windmill.dev/docs/advanced/workspace_forks`);
}

async function deleteWorkspaceFork(
  opts: GlobalOptions & {
    yes?: boolean;
  },
  name: string,
) {
  let forkWorkspaceId: string;
  let token: string;
  let remote: string;
  let hasLocalProfile = false;

  // Try local profile first (existing behavior)
  const orgWorkspaces = await allWorkspaces(opts.configDir);
  const idxOf = orgWorkspaces.findIndex((x) => x.name === name);

  if (idxOf !== -1) {
    const workspace = orgWorkspaces[idxOf];
    if (!workspace.workspaceId.startsWith(WM_FORK_PREFIX)) {
      throw new Error(
        `You can only delete forked workspaces where the workspace id starts with \`${WM_FORK_PREFIX}.\` Failed while attempting to delete \`${workspace.workspaceId}\``,
      );
    }
    forkWorkspaceId = workspace.workspaceId;
    token = workspace.token;
    remote = workspace.remote;
    hasLocalProfile = true;
  } else {
    // Fallback: resolve parent workspace from branch config and construct fork ID
    const parentWorkspace = await tryResolveBranchWorkspace(opts);
    if (!parentWorkspace) {
      throw new Error(
        "Could not resolve parent workspace. Make sure you are in a git repo with gitBranches configured in wmill.yaml, or create a local workspace profile for the fork.",
      );
    }
    forkWorkspaceId = name.startsWith(`${WM_FORK_PREFIX}-`) ? name : `${WM_FORK_PREFIX}-${name}`;
    token = parentWorkspace.token;
    remote = parentWorkspace.remote;
  }

  if (!opts.yes) {
    const { Select } = await import("@cliffy/prompt/select");
    const choice = await Select.prompt({
      message: `Are you sure you want to delete the forked workspace \`${forkWorkspaceId}\`?`,
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

  setClient(
    token,
    remote.endsWith("/") ? remote.substring(0, remote.length - 1) : remote
  );

  const result = await wmill.deleteWorkspace({
    workspace: forkWorkspaceId
  });
  log.info(
    colors.green(`✅ Forked workspace '${forkWorkspaceId}' deleted successfully!\n${result}`),
  );
  if (hasLocalProfile) {
    await removeWorkspace(name, false, opts);
  }
}

export { createWorkspaceFork, deleteWorkspaceFork };
