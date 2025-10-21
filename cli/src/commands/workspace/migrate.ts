// deno-lint-ignore-file no-explicit-any
import { Command, colors, log } from "../../../deps.ts";
import { GlobalOptions } from "../../types.ts";
import { requireLogin } from "../../core/auth.ts";
import { setClient } from "../../../deps.ts";
import * as wmill from "../../../gen/services.gen.ts";
import { getActiveWorkspace } from "./workspace.ts";

async function migrate(
  opts: GlobalOptions & {
    all?: boolean;
    metadataOnly?: boolean;
    jobsOnly?: boolean;
    noArchiveSource?: boolean;
  },
  sourceWorkspace: string,
  targetWorkspace: string
) {
  await requireLogin(opts);

  const workspace = await getActiveWorkspace(opts);
  if (!workspace) {
    throw new Error("No active workspace. Please run 'wmill workspace add' first.");
  }

  setClient(
    workspace.token,
    workspace.remote.endsWith("/")
      ? workspace.remote.substring(0, workspace.remote.length - 1)
      : workspace.remote
  );

  // Determine migration type from flags
  let migrationType: "all" | "metadata" | "jobs" = "all";
  if (opts.jobsOnly) {
    migrationType = "jobs";
  } else if (opts.metadataOnly) {
    migrationType = "metadata";
  }

  const archiveSource = !opts.noArchiveSource;

  log.info(colors.blue("Starting workspace migration:"));
  log.info(`  Source: ${colors.bold(sourceWorkspace)}`);
  log.info(`  Target: ${colors.bold(targetWorkspace)}`);
  log.info(`  Type: ${colors.bold(migrationType)}`);
  log.info(`  Archive source: ${colors.bold(String(archiveSource))}`);
  log.info("");

  try {
    const result = await wmill.migrateWorkspace({
      requestBody: {
        source_workspace: sourceWorkspace,
        target_workspace: targetWorkspace,
        migration_type: migrationType,
        archive_source: archiveSource
      }
    });

    log.info(colors.green(`✅ ${result}`));

    if (migrationType === "metadata") {
      log.info("");
      log.info(
        colors.yellow(
          "⚠️  Metadata migration complete. Run with --jobs-only to migrate job history."
        )
      );
    }
  } catch (error) {
    log.error(colors.red(`❌ Migration failed: ${error.message}`));
    throw error;
  }
}

const command = new Command()
  .name("migrate")
  .description("Migrate workspace data from source to target workspace")
  .arguments("<source_workspace:string> <target_workspace:string>")
  .option("--all", "Migrate all tables (default)")
  .option("--metadata-only", "Migrate all tables except job tables")
  .option("--jobs-only", "Migrate only job tables (v2_job, v2_job_completed, v2_job_queue)")
  .option("--no-archive-source", "Do not archive source workspace after migration")
  .action(migrate as any);

export default command;
