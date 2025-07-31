import { log } from "./deps.ts";
import { execSync } from "node:child_process";

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
