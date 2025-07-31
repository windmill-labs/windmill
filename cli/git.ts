import { log } from "./deps.ts";

export function getCurrentGitBranch(): string | null {
  try {
    const command = new Deno.Command("git", {
      args: ["rev-parse", "--abbrev-ref", "HEAD"],
      stdout: "piped",
      stderr: "piped",
    });
    const { code, stdout } = command.outputSync();
    if (code !== 0) return null;
    const branch = new TextDecoder().decode(stdout).trim();
    return branch || null;
  } catch (error) {
    log.debug(`Failed to get Git branch: ${error}`);
    return null;
  }
}

export function isGitRepository(): boolean {
  try {
    const command = new Deno.Command("git", {
      args: ["rev-parse", "--git-dir"],
      stdout: "piped",
      stderr: "piped",
    });
    const { code } = command.outputSync();
    return code === 0;
  } catch (error) {
    log.debug(`Failed to check Git repository: ${error}`);
    return false;
  }
}