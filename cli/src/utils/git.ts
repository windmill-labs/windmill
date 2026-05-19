import * as log from "../core/log.ts";
import { execSync, spawnSync } from "node:child_process";
import { WM_FORK_PREFIX } from "../core/constants.ts";

// Fork *workspace id* prefix ("wm-fork-"). WM_FORK_PREFIX is the *branch*
// prefix ("wm-fork") used inside the wm-fork/<branch>/<id> branch name.
const FORK_WORKSPACE_PREFIX = `${WM_FORK_PREFIX}-`;

export function getCurrentGitBranch(): string | null {
  try {
    const result = execSync("git rev-parse --abbrev-ref HEAD", {
      encoding: "utf8",
      stdio: "pipe"
    });
    const branch = result.trim();
    return branch || null;
  } catch (error) {
    log.debug(`Failed to get Git branch: ${error}`);
    return null;
  }
}

export function getOriginalBranchForWorkspaceForks(branchName: string | null): string | null {
  if (!branchName || !branchName.startsWith(WM_FORK_PREFIX)) {
    return null
  }

  const start = branchName.indexOf("/") + 1;
  const end = branchName.lastIndexOf("/");

  if (start < 0 || end < 0 || end - start <= 0) {
    return null
  }

  return branchName.slice(start, end)
}

export function getWorkspaceIdForWorkspaceForkFromBranchName(branchName: string): string | null {
  if (!branchName.startsWith(WM_FORK_PREFIX)) {
    return null
  }

  const start = branchName.lastIndexOf("/") + 1;

  if (start < 0) {
    return null
  }

  return `${WM_FORK_PREFIX}-${branchName.slice(start)}`
}
export function isGitRepository(): boolean {
  try {
    execSync("git rev-parse --git-dir", {
      encoding: "utf8",
      stdio: "pipe"
    });
    return true;
  } catch (error) {
    log.debug(`Failed to check Git repository: ${error}`);
    return false;
  }
}

// ===========================================================================
// Git-sync deployment-callback branch/commit/push.
//
// The git-sync hub script ("sync-script-to-git-repo-windmill") historically
// owned this logic. When `use_individual_branch` is set, a deploy must land on
// a dedicated `wm_deploy/<workspace>/<...>` branch (or a `wm-fork/<branch>/<id>`
// branch for forks) instead of the repo's protected base branch — otherwise
// the push to a protected `main` is rejected (GH006). A hub-script rewrite once
// dropped this and every deploy pushed straight to `main`; keeping the logic
// here (in-repo, behind `wmill sync pull`) makes it deterministically testable.
// ===========================================================================

export interface GitSyncDeployItem {
  path_type: string;
  path?: string | null;
  parent_path?: string | null;
  commit_msg?: string;
}

// Mirrors the hub script's get_fork_branch_name: "wm-fork-<id>" becomes
// "wm-fork/<originalBranch>/<id>".
export function forkBranchName(
  workspaceId: string,
  originalBranch: string,
): string {
  return workspaceId.replace(
    FORK_WORKSPACE_PREFIX,
    `${WM_FORK_PREFIX}/${originalBranch}/`,
  );
}

// Pure branch-name resolution mirroring the hub script's git_checkout_branch.
// Returns null when the deploy should stay on the cloned/base branch (i.e.
// workspace-wide mode, or user/group objects which never get their own branch).
export function computeGitSyncDeployBranch(params: {
  workspaceId: string;
  items: GitSyncDeployItem[];
  useIndividualBranch: boolean;
  groupByFolder: boolean;
  clonedBranchName: string;
}): string | null {
  const {
    workspaceId,
    items,
    useIndividualBranch,
    groupByFolder,
    clonedBranchName,
  } = params;

  if (workspaceId.startsWith(FORK_WORKSPACE_PREFIX)) {
    return forkBranchName(workspaceId, clonedBranchName);
  }

  if (items.length === 0) return null;
  const first = items[0];

  // `use_individual_branch` disables debouncing, so items is length 1 here.
  if (
    !useIndividualBranch ||
    first.path_type === "user" ||
    first.path_type === "group"
  ) {
    return null;
  }

  const ref = first.path ?? first.parent_path;
  if (!ref) return null;

  return groupByFolder
    ? `wm_deploy/${workspaceId}/${ref.split("/").slice(0, 2).join("__")}`
    : `wm_deploy/${workspaceId}/${first.path_type}/${ref.replaceAll("/", "__")}`;
}

// Mirrors the hub script's composeCommitHeader: summarise the deployed object
// types, e.g. "[WM]: Deployed 2 scripts, 1 flow and 3 other objects".
export function composeGitSyncCommitHeader(
  items: GitSyncDeployItem[],
): string {
  const typeCounts = new Map<string, number>();
  for (const item of items) {
    typeCounts.set(item.path_type, (typeCounts.get(item.path_type) ?? 0) + 1);
  }
  const sorted = Array.from(typeCounts.entries()).sort((a, b) => b[1] - a[1]);

  const parts: string[] = [];
  let othersCount = 0;
  for (let i = 0; i < sorted.length; i++) {
    const [pathType, count] = sorted[i];
    if (i < 3) {
      const label = count > 1 ? `${pathType}s` : pathType;
      if (i === 2 && sorted.length === 3) {
        parts.push(`and ${count} ${label}`);
      } else {
        parts.push(`${count} ${label}`);
      }
    } else {
      othersCount += count;
    }
  }

  let header = `[WM]: Deployed ${parts.join(", ")}`;
  if (othersCount > 0) {
    header += ` and ${othersCount} other object${othersCount > 1 ? "s" : ""}`;
  }
  return header;
}

function git(
  args: string[],
  opts?: { allowFail?: boolean },
): { status: number; stdout: string; stderr: string } {
  const r = spawnSync("git", args, { encoding: "utf8", stdio: "pipe" });
  const status = r.status ?? 1;
  if (r.error) {
    if (opts?.allowFail) return { status, stdout: "", stderr: String(r.error) };
    throw r.error;
  }
  if (status !== 0 && !opts?.allowFail) {
    throw new Error(
      `git ${args.join(" ")} failed (exit ${status}): ${r.stderr ?? ""}`,
    );
  }
  return { status, stdout: r.stdout ?? "", stderr: r.stderr ?? "" };
}

// Checkout (or create) the dedicated deploy branch, mirroring the hub script:
// try `git checkout <branch>`, on failure create it with -b and enable
// push.autoSetupRemote so the subsequent bare `git push` targets it.
export function checkoutGitSyncDeployBranch(branch: string): void {
  const existing = git(["checkout", branch], { allowFail: true });
  if (existing.status === 0) {
    log.info(`Switched to existing branch ${branch}`);
    return;
  }
  git(["checkout", "-b", branch]);
  git(["config", "--add", "--bool", "push.autoSetupRemote", "true"]);
  log.info(`Created and switched to branch ${branch}`);
}

// Stage, commit and push the deploy, mirroring the hub script's git_push:
// stage `wmill-lock.yaml` plus each item's path tree, no-op when nothing is
// staged, and push the *current* branch with a rebase-retry fallback.
export function gitSyncDeployPush(params: {
  items: GitSyncDeployItem[];
  authorName: string;
  authorEmail: string;
  onlyCreateBranch?: boolean;
}): { pushed: boolean } {
  const { items, authorName, authorEmail, onlyCreateBranch } = params;

  git(["config", "user.email", authorEmail]);
  git(["config", "user.name", authorName]);

  // only_create_branch: publish the (possibly empty) branch ref and stop,
  // mirroring the hub script's early `git push --porcelain`.
  if (onlyCreateBranch) {
    git(["push", "--porcelain"], { allowFail: true });
    return { pushed: true };
  }

  // Staged separately (not `git add wmill-lock.yaml <path>**` in one call):
  // `git add a b` aborts entirely if `a` matches nothing, which would drop the
  // object files whenever wmill-lock.yaml is absent.
  git(["add", "wmill-lock.yaml"], { allowFail: true });
  const commitMsgs: string[] = [];
  for (const { path, parent_path, commit_msg } of items) {
    if (path) {
      git(["add", `${path}**`], { allowFail: true });
    }
    if (parent_path) {
      git(["add", `${parent_path}**`], { allowFail: true });
    }
    if (commit_msg) commitMsgs.push(commit_msg);
  }

  // `git diff --cached --quiet` exits 1 iff there is something staged.
  const staged = git(["diff", "--cached", "--quiet"], { allowFail: true });
  if (staged.status === 0) {
    log.info("No changes detected, nothing to commit.");
    return { pushed: false };
  }

  const [header, description] =
    commitMsgs.length === 1
      ? [commitMsgs[0], ""]
      : [composeGitSyncCommitHeader(items), commitMsgs.join("\n")];

  git([
    "commit",
    "--author",
    `${authorName} <${authorEmail}>`,
    "-m",
    header && header.length > 0 ? header : "no commit msg",
    "-m",
    description,
  ]);

  const push = git(["push", "--porcelain"], { allowFail: true });
  if (push.status !== 0) {
    log.info(`Push failed, rebasing and retrying: ${push.stderr}`);
    git(["pull", "--rebase"]);
    git(["push", "--porcelain"]);
  }
  return { pushed: true };
}
