import { Command } from "@cliffy/command";
import { pullProtectionRules } from "./pull.ts";
import { pushProtectionRules } from "./push.ts";

const command = new Command()
  .description(
    "Manage workspace protection rules between local wmill.yaml and Windmill backend",
  )
  .command("pull")
  .description(
    "Pull protection rules from Windmill backend into local wmill.yaml",
  )
  .option("--default", "Write to top-level protectionRules instead of overrides")
  .option("--replace", "Replace existing protection rules (non-interactive)")
  .option("--override", "Add branch-specific override (non-interactive)")
  .option("--diff", "Show differences without applying changes")
  .option("--json-output", "Output in JSON format")
  .option("--yes", "Skip interactive prompts and use default behavior")
  .option(
    "--promotion <branch:string>",
    "Use promotionOverrides from the specified branch instead of regular overrides",
  )
  .action(pullProtectionRules as any)
  .command("push")
  .description(
    "Push protection rules from local wmill.yaml to Windmill backend (full reconcile: creates, updates, and deletes)",
  )
  .option("--diff", "Show what would be pushed without applying changes")
  .option("--json-output", "Output in JSON format")
  .option("--yes", "Skip interactive prompts and confirmations")
  .option(
    "--promotion <branch:string>",
    "Use promotionOverrides from the specified branch instead of regular overrides",
  )
  .action(pushProtectionRules as any);

export { pullProtectionRules, pushProtectionRules };
export default command;
