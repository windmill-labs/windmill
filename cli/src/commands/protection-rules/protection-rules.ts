import { Command } from "@cliffy/command";
import { pullProtectionRules } from "./pull.ts";
import { pushProtectionRules } from "./push.ts";

const command = new Command()
  .description(
    "Sync workspace protection rules between protection-rules.yaml and Windmill. The file is keyed by workspace name; keys must match wmill.yaml 'workspaces'.",
  )
  .command("pull")
  .description(
    "Pull protection rules from Windmill into protection-rules.yaml for a workspace",
  )
  .arguments("[workspace:string]")
  .option("--all", "Pull every workspace defined in wmill.yaml")
  .option("--dry-run", "Show what would change without writing the file")
  .option("--json-output", "Output in JSON format")
  .action(pullProtectionRules as any)
  .command("push")
  .description(
    "Push protection rules from protection-rules.yaml to Windmill for a workspace (full reconcile: creates, updates, and deletes)",
  )
  .arguments("[workspace:string]")
  .option("--all", "Push every workspace defined in protection-rules.yaml")
  .option("--dry-run", "Show what would change without applying")
  .option("--json-output", "Output in JSON format")
  .option("--yes", "Skip the confirmation prompt (including deletions)")
  .action(pushProtectionRules as any);

export { pullProtectionRules, pushProtectionRules };
export default command;
