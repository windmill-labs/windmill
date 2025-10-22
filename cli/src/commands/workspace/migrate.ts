// deno-lint-ignore-file no-explicit-any
import { Command, colors, log } from "../../../deps.ts";
import { GlobalOptions } from "../../types.ts";
import { requireLogin } from "../../core/auth.ts";
import { setClient } from "../../../deps.ts";
import * as wmill from "../../../gen/services.gen.ts";
import {
  getActiveWorkspace,
  setActiveWorkspace,
  removeWorkspace,
  addWorkspace,
} from "./workspace.ts";
import { getWorkspaceConfigFilePath } from "../../../windmill-utils-internal/src/config/config.ts";

async function migrate(
  opts: GlobalOptions & {
    all?: boolean;
    metadataOnly?: boolean;
    jobsOnly?: boolean;
    noDisableSource?: boolean;
    targetName?: string;
    noSwitchWorkspace?: boolean;
    token?: string;
    remote?: string;
    sourceWorkspaceId?: string;
  },
  targetWorkspaceId: string
) {
  let token: string;
  let remote: string;
  let sourceWorkspaceId: string;
  let isCliMode: boolean;

  if (opts.token && opts.remote && opts.sourceWorkspaceId) {
    token = opts.token;
    remote = opts.remote.endsWith("/")
      ? opts.remote.substring(0, opts.remote.length - 1)
      : opts.remote;
    sourceWorkspaceId = opts.sourceWorkspaceId;
    isCliMode = false;
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

    isCliMode = true;
    token = workspace.token;
    remote = workspace.remote.endsWith("/")
      ? workspace.remote.substring(0, workspace.remote.length - 1)
      : workspace.remote;
    sourceWorkspaceId = workspace.workspaceId;

    log.info(colors.blue("Running in CLI mode with active workspace"));
  }

  setClient(token, remote);

  let migrationType: "all" | "metadata" | "jobs" = "all";
  if (opts.metadataOnly) {
    migrationType = "metadata";
  } else if (opts.jobsOnly) {
    migrationType = "jobs";
  }

  const disableSource = !opts.noDisableSource;
  const targetName = opts.targetName || targetWorkspaceId;
  const shouldSwitchWorkspace = !opts.noSwitchWorkspace;

  log.info(colors.blue("Starting workspace migration:"));
  log.info(`  Source: ${colors.bold(sourceWorkspaceId)}`);
  log.info(`  Target: ${colors.bold(targetWorkspaceId)} (${targetName})`);
  log.info(`  Type: ${colors.bold(migrationType)}`);
  log.info(`  Disable source: ${colors.bold(String(disableSource))}`);
  if (shouldSwitchWorkspace) {
    log.info(`  Switch to target workspace: ${colors.bold("yes")}`);
  }
  log.info("");

  try {
    if (isCliMode && migrationType === "all") {
      log.info(colors.blue("=".repeat(60)));
      log.info(colors.blue("STEP 1: Migrating Metadata"));
      log.info(colors.blue("=".repeat(60)));
      log.info("");

      const metadataResult = await wmill.migrateWorkspace({
        requestBody: {
          source_workspace_id: sourceWorkspaceId,
          target_workspace_id: targetWorkspaceId,
          target_workspace_name: targetName,
          migration_type: "metadata",
          disable_workspace: disableSource,
        },
      });

      log.info(colors.green(`✅ ${metadataResult}`));
      log.info("");

      log.info(colors.blue("=".repeat(60)));
      log.info(colors.blue("STEP 2: Migrating Job History (v2_job_completed)"));
      log.info(colors.blue("=".repeat(60)));
      log.info("");

      const jobsResult = await wmill.migrateWorkspace({
        requestBody: {
          source_workspace_id: sourceWorkspaceId,
          target_workspace_id: targetWorkspaceId,
          target_workspace_name: targetName,
          migration_type: "jobs",
          disable_workspace: false,
        },
      });

      log.info(colors.green(`✅ ${jobsResult}`));
      log.info("");
      log.info(colors.green("=".repeat(60)));
      log.info(colors.green("✅ Complete migration finished successfully!"));
      log.info(colors.green("=".repeat(60)));
    } else {
      const result = await wmill.migrateWorkspace({
        requestBody: {
          source_workspace_id: sourceWorkspaceId,
          target_workspace_id: targetWorkspaceId,
          target_workspace_name: targetName,
          migration_type: migrationType,
          disable_workspace: disableSource,
        },
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
    }

    if (isCliMode && shouldSwitchWorkspace && migrationType !== "jobs") {
      const workspace = await getActiveWorkspace(opts);
      if (workspace) {
        await removeWorkspace(workspace.name, true, opts);

        log.info("");
        log.info(colors.blue("Switching to target workspace..."));
        workspace.name = targetName;
        workspace.workspaceId = targetWorkspaceId;
        const filePath = await getWorkspaceConfigFilePath(opts.configDir);
        const file = await Deno.open(filePath, {
          append: true,
          write: true,
          read: true,
          create: true,
        });
        await file.write(
          new TextEncoder().encode(JSON.stringify(workspace) + "\n")
        );

        await setActiveWorkspace(targetName, opts.configDir);

        log.info(
          colors.green(`✅ Switched active workspace to ${targetWorkspaceId}`)
        );
        log.info(
          colors.green(
            `✅ Removed old workspace configuration for ${sourceWorkspaceId}`
          )
        );
      }
    }
  } catch (error) {
    log.error(colors.red(`❌ Migration failed: ${error}`));
    throw error;
  }
}

const command = new Command()
  .name("migrate")
  .description("Migrate workspace data from source to target workspace")
  .arguments("<target_workspace_id:string>")
  .option("--all", "Migrate all tables (default)")
  .option("--metadata-only", "Migrate all tables except v2_job_completed")
  .option(
    "--jobs-only",
    "Migrate only v2_job_completed table (workspace must already exist)"
  )
  .option(
    "--no-disable-source",
    "Do not disable source workspace after migration"
  )
  .option(
    "--target-name <name:string>",
    "Name for the target workspace (defaults to target workspace ID)"
  )
  .option(
    "--no-switch-workspace",
    "Do not switch active workspace to target after migration (by default, switches and removes old workspace)"
  )
  .option("--token <token:string>", "API token for worker job mode")
  .option("--remote <url:string>", "Remote URL for worker job mode")
  .option(
    "--source-workspace <workspace:string>",
    "Source workspace ID (defaults to active workspace in CLI mode, required in worker job mode)"
  )
  .action(migrate as any);

export default command;
