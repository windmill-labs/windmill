import { GlobalOptions } from "../../types.ts";
import { colors } from "@cliffy/ansi/colors";
import { Input } from "@cliffy/prompt/input";
import * as log from "../../core/log.ts";
import { setClient } from "../../core/client.ts";
import { allWorkspaces, list, removeWorkspace } from "./workspace.ts";
import * as wmill from "../../../gen/services.gen.ts";
import {
  getCurrentGitBranch,
  getOriginalBranchForWorkspaceForks,
  gitBranchExists,
  isGitRepository,
  renameCurrentGitBranch,
} from "../../utils/git.ts";
import process from "node:process";
import { WM_FORK_PREFIX } from "../../core/constants.ts";
import { tryResolveBranchWorkspace } from "../../core/context.ts";
import {
  findWorkspaceByGitBranch,
  getEffectiveGitBranch,
  getWorkspaceNames,
  readConfigFile,
} from "../../core/conf.ts";

async function createWorkspaceFork(
  opts: GlobalOptions & {
    createWorkspaceName: string | undefined;
    color: string | undefined;
    datatableBehavior: string | undefined;
    fromBranch: string | undefined;
    yes: boolean | undefined;
  },
  workspaceName: string | undefined,
  workspaceId: string | undefined = undefined,
) {
  if (!isGitRepository()) {
    throw new Error("You can only create forks within a git repo. Forks are tracked with git and synced to your instance with the git sync workflow.");
  }

  const currentBranch = getCurrentGitBranch()
  if (!currentBranch) {
    throw new Error("Could not get git branch name");
  }

  const config = await readConfigFile({ warnIfMissing: false });
  const originalBranchIfForked = getOriginalBranchForWorkspaceForks(currentBranch);

  // A "base branch" is one we must not rename onto a fork branch: mapped to a
  // workspace in wmill.yaml, or a conventional default (main/master).
  const isBaseBranch = (branch: string): boolean =>
    branch === "main" ||
    branch === "master" ||
    findWorkspaceByGitBranch(config.workspaces, branch) !== undefined;

  // Decide the base branch the fork links to, and whether to rename the
  // current working branch onto the fork branch. Auto-detected from where you
  // are; `--from-branch` is the explicit/non-interactive override.
  let clonedBranchName: string;
  let renameCurrent: boolean;

  if (opts.fromBranch) {
    // Explicit override: base on <fromBranch>, rename the current branch.
    if (opts.fromBranch === currentBranch) {
      throw new Error(
        `--from-branch is for converting a *different* working branch into the fork branch, but you are already on \`${currentBranch}\`. ` +
        `Omit --from-branch to create a fresh fork branch with \`git checkout -b\`.`,
      );
    }
    if (isBaseBranch(currentBranch)) {
      throw new Error(
        `Refusing to rename your current branch \`${currentBranch}\` — it looks like a base branch (mapped to a workspace in wmill.yaml, or main/master). ` +
        `Check out the disposable working branch you want to convert first.`,
      );
    }
    if (getOriginalBranchForWorkspaceForks(currentBranch)) {
      // Current branch is itself a fork branch (wm-fork/<base>/<old-id>).
      // Renaming it onto the new fork branch would detach the existing fork.
      throw new Error(
        `Refusing to rename your current branch \`${currentBranch}\` — it is already a fork branch. ` +
        `To fork a fork, omit --from-branch: \`wmill workspace fork\` bases the new fork on this fork's original branch and creates a fresh fork branch without renaming.`,
      );
    }
    if (!findWorkspaceByGitBranch(config.workspaces, opts.fromBranch)) {
      throw new Error(
        `Could not find a workspace mapped to branch \`${opts.fromBranch}\` in wmill.yaml's workspaces section. ` +
        `Pass the base branch your fork should be based on (e.g. the branch bound to the parent workspace).`,
      );
    }
    clonedBranchName = opts.fromBranch;
    renameCurrent = true;
  } else if (originalBranchIfForked) {
    // Fork of a fork: link to the original branch; user checks out a new branch.
    log.info(`You are creating a fork of a fork. The branch will be linked to the original branch this was forked from, i.e. \`${originalBranchIfForked}\`, for all settings and overrides.`);
    clonedBranchName = originalBranchIfForked;
    renameCurrent = false;
  } else if (isBaseBranch(currentBranch)) {
    // On a base branch: base the fork on it; user checks out a fresh fork branch.
    clonedBranchName = currentBranch;
    renameCurrent = false;
  } else {
    // On a non-base working branch: offer to base the fork on it and rename it.
    clonedBranchName = await resolveWorkingBranchBase(config, opts, currentBranch);
    renameCurrent = true;
  }

  // Resolve the parent workspace. When the base differs from the current
  // branch (the rename workflows), resolve via the base branch's workspace;
  // otherwise use plain branch resolution.
  let workspace;
  if (clonedBranchName === currentBranch) {
    workspace = await tryResolveBranchWorkspace(opts);
  } else {
    const baseMatch = findWorkspaceByGitBranch(config.workspaces, clonedBranchName);
    workspace = baseMatch
      ? await tryResolveBranchWorkspace(opts, baseMatch[0])
      : await tryResolveBranchWorkspace(opts);
  }

  if (!workspace) {
    throw new Error("Could not resolve workspace from branch name. Make sure you are in a git repo to use workspace forks");
  }

  log.info(`You are forking workspace (${workspace.workspaceId})`)

  if (opts.workspace) {
    log.info(
      colors.red.bold(
        "! Workspace needs to be specified as positional argument, not as option."
      )
    );
    return;
  }

  // When we're converting the current branch into the fork branch, default
  // the fork's name/id to that branch — almost always what you want, and it
  // keeps the fork branch named after the work you already have
  // (wm-fork/<base>/<branch>). Interactive: pre-fill the prompt (press enter
  // to accept). Non-interactive (`--yes`): use it automatically.
  const branchDefaultId = renameCurrent ? branchToForkId(currentBranch) : undefined;
  const interactive = process.stdin.isTTY && opts.yes !== true;

  if (workspaceName === undefined) {
    if (branchDefaultId && !interactive) {
      workspaceName = branchDefaultId;
      log.info(`Naming the fork after the current branch: \`${workspaceName}\``);
    } else {
      workspaceName = await Input.prompt({
        message: "Name this forked workspace:",
        default: branchDefaultId,
      });
    }
  }

  if (!workspaceId) {
    // The id (unlike the display name) must be a valid slug — derive it from
    // the name rather than using the free-form name verbatim.
    const idDefault = branchToForkId(workspaceName);
    if (branchDefaultId && !interactive) {
      workspaceId = idDefault;
    } else {
      workspaceId = await Input.prompt({
        message: `Enter the ID of this forked workspace, it will then be prefixed by ${WM_FORK_PREFIX}. It will also determine the branch name`,
        default: idDefault,
        suggestions: [idDefault],
      });
    }
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
  // Fail fast on an invalid id (e.g. an explicit positional id, or a long
  // branch name under --yes) before existsWorkspace, datatable cloning, and
  // branch creation — a late backend rejection would leave cloned databases
  // behind.
  validateForkWorkspaceId(trueWorkspaceId);
  // The workspace.name column is character varying(50); reject an over-long name
  // up front (matching the name actually sent to the backend below) instead of
  // failing late after databases are cloned and the git branch is created.
  const effectiveName = opts.createWorkspaceName ?? workspaceName ?? trueWorkspaceId;
  if (effectiveName.length > 50) {
    throw new Error(
      `Fork workspace name is too long (${effectiveName.length} chars; max 50). Choose a shorter name.`
    );
  }
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
    throw new Error(
      `This forked workspace '${workspaceId}' (${workspaceName}) already exists, possibly archived (archiving keeps the id reserved). ` +
      `Permanently delete it with \`wmill workspace delete-fork ${workspaceId}\` to reuse the id, or choose a different id`,
    );
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
        name: opts.createWorkspaceName ?? workspaceName ?? trueWorkspaceId,
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
        name: opts.createWorkspaceName ?? workspaceName ?? trueWorkspaceId,
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

  // Rename workflow: turn the current working branch into the fork branch in
  // place so its commits become the fork's. (Consent was already established —
  // by `--from-branch`, or the interactive prompt for a non-base branch.)
  // Otherwise: leave the user on their branch and have them check out a fresh
  // fork branch.
  let onForkBranch = false;
  if (renameCurrent) {
    if (currentBranch === newBranchName) {
      onForkBranch = true;
      log.info(colors.green(`Your current branch is already \`${newBranchName}\`.`));
    } else if (gitBranchExists(newBranchName)) {
      log.warn(
        `Branch \`${newBranchName}\` already exists locally, so the current branch \`${currentBranch}\` was not renamed. ` +
        `Check out the fork branch yourself (e.g. \`git checkout ${newBranchName}\`).`,
      );
    } else {
      renameCurrentGitBranch(newBranchName);
      onForkBranch = true;
      log.info(
        colors.green(
          `Renamed \`${currentBranch}\` → \`${newBranchName}\`. Your existing commits are now on the fork branch.`,
        ),
      );
    }
  }

  const checkoutHint = onForkBranch
    ? `Created forked workspace ${trueWorkspaceId}. You are on the fork branch \`${newBranchName}\` — push it to sync your fork.`
    : `Created forked workspace ${trueWorkspaceId}. To start contributing to your fork, create and push edits to the branch \`${newBranchName}\` by using the command:

\t` + colors.white(`git checkout -b ${newBranchName}`);

  log.info(`${checkoutHint}

When doing operations on the forked workspace, it will use the remote setup in the workspaces section for the branch it was forked from.

To merge changes back to the parent workspace, you can:
  - Use the CLI: ` + colors.white(`git checkout ${newBranchName} && wmill workspace merge`) + `
  - Use the Merge UI from the forked workspace home page
  - Use git: ` + colors.white(`git checkout ${clonedBranchName} && git merge ${newBranchName} && wmill sync push`) + `
  See: https://www.windmill.dev/docs/advanced/workspace_forks`);
}

/**
 * When `wmill workspace fork` is run from a non-base working branch, confirm
 * the user wants to turn it into a fork branch, and resolve which base branch
 * the fork should be linked to. Throws in non-interactive mode (where the user
 * must pass `--from-branch <base>` instead).
 */
async function resolveWorkingBranchBase(
  config: Awaited<ReturnType<typeof readConfigFile>>,
  opts: { yes?: boolean },
  currentBranch: string,
): Promise<string> {
  const interactive = process.stdin.isTTY && opts.yes !== true;
  if (!interactive) {
    throw new Error(
      `You are on working branch \`${currentBranch}\`, which is not a base branch. ` +
      `Pass --from-branch <base> to base the fork on a base branch and rename this branch onto the fork branch, ` +
      `or check out a base branch and run \`wmill workspace fork\` to create a fresh fork branch.`,
    );
  }

  const { Select } = await import("@cliffy/prompt/select");
  const proceed = await Select.prompt({
    message: `You're on working branch \`${currentBranch}\`, not a base branch. Base a fork on it and rename it onto the fork branch?`,
    options: [
      { name: "Yes, base the fork on this branch and rename it", value: "yes" },
      { name: "No, cancel", value: "no" },
    ],
  });
  if (proceed !== "yes") {
    throw new Error("Fork cancelled. Check out a base branch to create a fresh fork branch instead.");
  }

  const baseBranches = listConfiguredBaseBranches(config);
  if (baseBranches.length === 0) {
    throw new Error(
      `No base branches are configured in wmill.yaml's workspaces section, so the fork can't be linked to a parent. ` +
      `Add the parent workspace to wmill.yaml, or pass --from-branch <base>.`,
    );
  }
  if (baseBranches.length === 1) {
    log.info(`Basing the fork on \`${baseBranches[0]}\`.`);
    return baseBranches[0];
  }
  return await Select.prompt({
    message: "Which base branch is this fork based on (the parent)?",
    options: baseBranches.map((b) => ({ name: b, value: b })),
  });
}

// The backend caps a fork workspace id (`wm-fork-<slug>`) at 50 chars total
// (validate_fork_workspace_id in windmill-common), so the slug is at most
// 50 - "wm-fork-".length (8) = 42.
const MAX_FORK_ID_SLUG = 42;

/**
 * Derive a workspace-id-safe slug from a git branch name. Branch names can
 * contain `/` and other characters that aren't valid in a workspace id and
 * would break the `wm-fork/<base>/<id>` branch-name parsing, so collapse any
 * invalid run to a single dash, trim, and cap to the backend length limit.
 */
function branchToForkId(branch: string): string {
  const slug = branch
    .replace(/[^a-zA-Z0-9_-]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, MAX_FORK_ID_SLUG)
    .replace(/-+$/g, ""); // re-trim if the cut landed on a dash
  return slug || "fork";
}

/**
 * Mirror the backend `validate_fork_workspace_id` so an invalid id fails fast
 * — before `existsWorkspace`, datatable cloning (which creates real per-fork
 * Postgres databases), and branch creation, none of which get cleaned up on a
 * late backend rejection. `id` is the full `wm-fork-<slug>` workspace id.
 */
function validateForkWorkspaceId(id: string): void {
  const reject = (reason: string): never => {
    throw new Error(
      `Fork workspace id \`${id}\` is invalid: ${reason}. Choose a shorter or simpler name/id.`,
    );
  };
  if (id.length > 50) {
    reject(`too long (${id.length} chars; max 50 including the \`${WM_FORK_PREFIX}-\` prefix)`);
  }
  if (id.endsWith(".")) reject("cannot end with '.'");
  if (id.endsWith(".lock")) reject("cannot end with '.lock'");
  if (id.includes("..")) reject("cannot contain '..'");
  if (id.includes("@{")) reject("cannot contain '@{'");
  if (id.includes("//")) reject("cannot contain '//'");
  for (const ch of id) {
    if (":~^?*[\\ ".includes(ch)) reject(`contains forbidden character '${ch}'`);
    const code = ch.charCodeAt(0);
    if (code < 0x20 || code === 0x7f) reject("contains a control character");
  }
  for (const component of id.split("/")) {
    if (component.startsWith(".")) reject("a path component cannot start with '.'");
    if (component.endsWith(".lock")) reject("a path component cannot end with '.lock'");
  }
}

/**
 * Base branches configured in wmill.yaml, mirroring how `findWorkspaceByGitBranch`
 * keys them (effective git branch = `gitBranch ?? workspaceName`, reserved keys
 * excluded) so the chosen base resolves to a workspace afterwards.
 */
function listConfiguredBaseBranches(
  config: Awaited<ReturnType<typeof readConfigFile>>,
): string[] {
  const workspaces = config.workspaces;
  const branches = new Set<string>();
  for (const name of getWorkspaceNames(workspaces)) {
    branches.add(getEffectiveGitBranch(name, workspaces![name]));
  }
  return [...branches];
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
        "Could not resolve parent workspace. Make sure you are in a git repo with 'workspaces' configured in wmill.yaml, or create a local workspace profile for the fork.",
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

export {
  branchToForkId,
  createWorkspaceFork,
  deleteWorkspaceFork,
  validateForkWorkspaceId,
};
