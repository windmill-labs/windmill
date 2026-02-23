import * as log from "../core/log.ts";
import { execSync } from "node:child_process";
import { WM_FORK_PREFIX } from "../core/constants.ts";

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
