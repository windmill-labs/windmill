import { Command } from "@cliffy/command";
import { pullGitSyncSettings } from "./pull.ts";
import { pushGitSyncSettings } from "./push.ts";

const command = new Command()
  .description(
    "Manage git-sync settings between local wmill.yaml and Windmill backend",
  )
  .command("pull")
  .description(
    "Pull git-sync settings from Windmill backend to local wmill.yaml",
  )
  .option(
    "--repository <repo:string>",
    "Specify repository path (e.g., u/user/repo)",
  )
  .option(
    "--default",
    "Write settings to top-level defaults instead of overrides",
  )
  .option("--replace", "Replace existing settings (non-interactive mode)")
  .option(
    "--override",
    "Add branch-specific override (non-interactive mode)",
  )
  .option("--diff", "Show differences without applying changes")
  .option("--json-output", "Output in JSON format")
  .option(
    "--with-backend-settings <json:string>",
    "Use provided JSON settings instead of querying backend (for testing)",
  )
  .option("--yes", "Skip interactive prompts and use default behavior")
  .option(
    "--promotion <branch:string>",
    "Use promotionOverrides from the specified branch instead of regular overrides"
  )
  .action(pullGitSyncSettings as any)
  .command("push")
  .description(
    "Push git-sync settings from local wmill.yaml to Windmill backend",
  )
  .option(
    "--repository <repo:string>",
    "Specify repository path (e.g., u/user/repo)",
  )
  .option("--diff", "Show what would be pushed without applying changes")
  .option("--json-output", "Output in JSON format")
  .option(
    "--with-backend-settings <json:string>",
    "Use provided JSON settings instead of querying backend (for testing)",
  )
  .option("--yes", "Skip interactive prompts and use default behavior")
  .option(
    "--promotion <branch:string>",
    "Use promotionOverrides from the specified branch instead of regular overrides"
  )
  .action(pushGitSyncSettings as any);

export { pullGitSyncSettings, pushGitSyncSettings };
export default command;
