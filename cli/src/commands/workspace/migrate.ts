// deno-lint-ignore-file no-explicit-any
import { Command, colors, log } from "../../../deps.ts";
import { GlobalOptions } from "../../types.ts";
import { requireLogin } from "../../core/auth.ts";
import { setClient } from "../../../deps.ts";
import * as wmill from "../../../gen/services.gen.ts";
import { getActiveWorkspace } from "./workspace.ts";

async function migrate(
  opts: GlobalOptions & {
    token?: string;
    remote?: string;
    sourceWorkspace?: string;
  },
  targetWorkspaceId: string
) {
  let token: string;
  let remote: string;
  let sourceWorkspaceId: string;

  if (opts.token && opts.remote && opts.sourceWorkspace) {
    token = opts.token;
    remote = opts.remote.endsWith("/")
      ? opts.remote.substring(0, opts.remote.length - 1)
      : opts.remote;
    sourceWorkspaceId = opts.sourceWorkspace;
    log.info(
      colors.blue("Running in worker job mode with provided credentials")
    );
  } else {
    await requireLogin(opts);

    const workspace = await getActiveWorkspace(opts);
    if (!workspace) {
      throw new Error(
        "No active workspace. Please run 'wmill workspace add' first."
      );
    }

    token = workspace.token;
    remote = workspace.remote.endsWith("/")
      ? workspace.remote.substring(0, workspace.remote.length - 1)
      : workspace.remote;
    sourceWorkspaceId = workspace.workspaceId;

    log.info(colors.blue("Running in CLI mode with active workspace"));
  }

  setClient(token, remote);

  log.info(colors.blue("Starting workspace migration:"));
  log.info(`  Source: ${colors.bold(sourceWorkspaceId)}`);
  log.info("");

  try {
    log.info(colors.blue("=".repeat(60)));
    log.info(colors.blue("Migrating jobs"));
    log.info(colors.blue("=".repeat(60)));
    log.info("");

    const initialStatus = await wmill.getMigrationStatus({
      sourceWorkspace: sourceWorkspaceId,
    });

    const totalJobs = initialStatus.processed_jobs || 0;
    log.info(`Total jobs to migrate: ${colors.bold(totalJobs.toString())}`);

    if (totalJobs === 0) {
      log.info(colors.yellow("No jobs to migrate"));
      return;
    }

    let totalMigrated = 0;
    const batchSize = 10000;

    while (true) {
      log.info(`Processing batch (size: ${batchSize})...`);

      const batchResult = await wmill.migrateWorkspaceJobs({
        requestBody: {
          source_workspace_id: sourceWorkspaceId,
          target_workspace_id: targetWorkspaceId,
          batch_size: batchSize,
        },
      });

      const migratedInBatch = batchResult.migrated_count || 0;
      totalMigrated += migratedInBatch;

      const progress = Math.round((totalMigrated / totalJobs) * 100);
      log.info(`${colors.green(migratedInBatch.toString())} jobs migrated`);
      log.info(
        `Progress: ${colors.cyan(
          `${totalMigrated}/${totalJobs}`
        )} (${colors.yellow(`${progress}%`)})`
      );

      if (migratedInBatch < batchSize) {
        break;
      }
    }

    const jobsResult = `Successfully migrated ${totalMigrated} jobs`;

    log.info(colors.green(`✅ ${jobsResult}`));
    log.info("");
    log.info(colors.green("=".repeat(60)));
    log.info(colors.green("✅ Complete migration finished successfully!"));
    log.info(colors.green("=".repeat(60)));
  } catch (error) {
    log.error(colors.red(`❌ Migration failed: ${error}`));
    throw error;
  }
}

const command = new Command()
  .name("migrate")
  .description("Migrate workspace data from source to target workspace")
  .arguments("<target_workspace_id:string>")
  .option("-t --token <token:string>", "API token for worker job mode")
  .option("-r --remote <url:string>", "Remote URL for worker job mode")
  .option(
    "-s --source-workspace <workspace:string>",
    "Source workspace ID (defaults to active workspace in CLI mode, required in worker job mode)"
  )
  .action(migrate as any);

export default command;
