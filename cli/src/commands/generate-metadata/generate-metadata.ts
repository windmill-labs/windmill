import { Command } from "../../../deps.ts";
import { GlobalOptions } from "../../types.ts";
import { SyncOptions } from "../../core/conf.ts";
import { generateMetadata as generateScriptMetadata } from "../script/script.ts";
import { generateLocks as generateFlowLocks } from "../flow/flow.ts";
import { generateLocksCommand as generateAppLocks } from "../app/app_metadata.ts";

async function generateMetadata(
  opts: GlobalOptions & {
    yes?: boolean;
    lockOnly?: boolean;
    schemaOnly?: boolean;
  } & SyncOptions
) {
  await generateScriptMetadata(opts, undefined);
  await generateFlowLocks(opts, undefined);
  await generateAppLocks(opts, undefined);
}

const command = new Command()
  .description("Generate metadata (locks, schemas) for all scripts, flows, and apps")
  .option("--yes", "Skip confirmation prompt")
  .option("--lock-only", "Re-generate only the lock files")
  .option("--schema-only", "Re-generate only script schemas")
  .option(
    "-i --includes <patterns:file[]>",
    "Comma separated patterns to specify which files to include"
  )
  .option(
    "-e --excludes <patterns:file[]>",
    "Comma separated patterns to specify which files to exclude"
  )
  .action(generateMetadata as any);

export default command;
