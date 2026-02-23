import { GlobalOptions } from "../../types.ts";
import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace } from "../../core/context.ts";
import { Command } from "@cliffy/command";
import { colors } from "@cliffy/ansi/colors";
import { Confirm } from "@cliffy/prompt/confirm";
import * as log from "../../core/log.ts";
import { mergeConfigWithConfigFile } from "../../core/conf.ts";
import * as fs from "node:fs/promises";
import * as wmill from "../../../gen/services.gen.ts";

async function pullJobs(
  opts: GlobalOptions & {
    completedOutput?: string;
    queuedOutput?: string;
    skipWorkerCheck?: boolean;
  },
  workspace?: string
) {
  opts = await mergeConfigWithConfigFile(opts);
  const ws = await resolveWorkspace({ ...opts, workspace });
  await requireLogin(opts);

  log.info("Pulling jobs from workspace " + ws.workspaceId);

  // Check for active workers and warn user
  if (!opts.skipWorkerCheck) {
    try {
      const workers = await wmill.listWorkers({ pingSince: 60 });

      if (workers.length > 0) {
        log.info(
          colors.yellow(
            `\nWarning: Found ${workers.length} active worker(s) on the instance.`
          )
        );
        log.info(
          "It's recommended to scale down all workers before exporting jobs to ensure that no new jobs are being created during export."
        );

        const proceed = await Confirm.prompt({
          message: "Do you want to continue with the export anyway?",
          default: false,
        });

        if (!proceed) {
          log.info(
            "Export cancelled. Please scale down workers and try again."
          );
          log.info("You can skip this check with --skip-worker-check flag.");
          return;
        }
      }
    } catch (e) {
      log.debug(`Could not check for active workers: ${e}`);
    }
  }

  // Pull completed jobs
  let completedJobs: any[] = [];
  let page = 1;
  const perPage = 1000;

  while (true) {
    const batch = await wmill.exportCompletedJobs({
      workspace: ws.workspaceId,
      page,
      perPage,
    });

    if (batch.length === 0) break;
    completedJobs = completedJobs.concat(batch);

    if (batch.length < perPage) break;
    page++;
  }

  const completedPath = opts.completedOutput || "completed_jobs.json";
  await fs.writeFile(completedPath, JSON.stringify(completedJobs, null, 2));
  log.info(
    colors.green(
      `Successfully pulled ${completedJobs.length} completed jobs to ${completedPath}`
    )
  );

  // Pull queued jobs
  let queuedJobs: any[] = [];
  page = 1;

  while (true) {
    const batch = await wmill.exportQueuedJobs({
      workspace: ws.workspaceId,
      page,
      perPage,
    });

    if (batch.length === 0) break;
    queuedJobs = queuedJobs.concat(batch);

    if (batch.length < perPage) break;
    page++;
  }
  const queuedPath = opts.queuedOutput || "queued_jobs.json";
  await fs.writeFile(queuedPath, JSON.stringify(queuedJobs, null, 2));
  log.info(
    colors.green(
      `Successfully pulled ${queuedJobs.length} queued jobs to ${queuedPath}`
    )
  );

  // Ask to delete all jobs (queued and completed)
  const allJobs = [...queuedJobs, ...completedJobs];
  if (allJobs.length > 0) {
    const confirmed = await Confirm.prompt({
      message: `Do you want to delete the ${allJobs.length} pulled jobs (queued + completed) from the workspace? If you don't, you won't be able to import them again on the same instance because of the unique constraint on the job ID.`,
      default: false,
    });

    if (confirmed) {
      const jobIds = allJobs.map((job: any) => job.id);

      await wmill.deleteJobs({
        workspace: ws.workspaceId,
        requestBody: jobIds,
      });

      log.info(
        colors.green(`Deleted ${jobIds.length} jobs (queued + completed)`)
      );
    } else {
      log.info("Skipping deletion of jobs");
    }
  }
}

async function pushJobs(
  opts: GlobalOptions & {
    completedFile?: string;
    queuedFile?: string;
    skipWorkerCheck?: boolean;
  },
  workspace?: string
) {
  opts = await mergeConfigWithConfigFile(opts);
  const ws = await resolveWorkspace({ ...opts, workspace });
  await requireLogin(opts);

  log.info(`Pushing jobs to workspace ${ws.workspaceId}`);

  // Check for active workers before importing
  if (!opts.skipWorkerCheck) {
    try {
      const workers = await wmill.listWorkers({ pingSince: 60 });

      if (workers.length > 0) {
        log.info(
          colors.yellow(
            `\nWarning: Found ${workers.length} active worker(s) on the instance.`
          )
        );
        log.info(
          "It's recommended to scale down all workers before importing jobs to ensure:"
        );
        log.info(
          "  - No imported jobs are processed immediately during import"
        );
        log.info(
          "  - You have time to review or adjust the imported jobs before they start running"
        );
        log.info("");

        const proceed = await Confirm.prompt({
          message: "Do you want to continue with the import anyway?",
          default: false,
        });

        if (!proceed) {
          log.info(
            "Import cancelled. Please scale down workers and try again."
          );
          log.info("You can skip this check with --skip-worker-check flag.");
          return;
        }
      }
    } catch (e) {
      log.debug(`Could not check for active workers: ${e}`);
    }
  }

  // Push completed jobs
  const completedPath = opts.completedFile || "completed_jobs.json";
  try {
    const completedContent = await fs.readFile(completedPath, "utf-8");
    const completedJobs = JSON.parse(completedContent);

    if (!Array.isArray(completedJobs)) {
      throw new Error("Completed jobs file must contain an array of jobs");
    }

    const completedResult = await wmill.importCompletedJobs({
      workspace: ws.workspaceId,
      requestBody: completedJobs,
    });

    log.info(colors.green(`Completed jobs: ${completedResult}`));
  } catch (e: any) {
    if (e.code === "ENOENT") {
      log.info(
        colors.yellow(
          `No completed jobs file found at ${completedPath}, skipping`
        )
      );
    } else {
      throw new Error(`Failed to push completed jobs: ${e}`);
    }
  }

  // Push queued jobs
  const queuedPath = opts.queuedFile || "queued_jobs.json";
  try {
    const queuedContent = await fs.readFile(queuedPath, "utf-8");
    const queuedJobs = JSON.parse(queuedContent);

    if (!Array.isArray(queuedJobs)) {
      throw new Error("Queued jobs file must contain an array of jobs");
    }

    const queuedResult = await wmill.importQueuedJobs({
      workspace: ws.workspaceId,
      requestBody: queuedJobs,
    });

    log.info(colors.green(`Queued jobs: ${queuedResult}`));
  } catch (e: any) {
    if (e.code === "ENOENT") {
      log.info(
        colors.yellow(`No queued jobs file found at ${queuedPath}, skipping`)
      );
    } else {
      throw new Error(`Failed to push queued jobs: ${e}`);
    }
  }
}

const pull = new Command()
  .description("Pull completed and queued jobs from workspace")
  .option(
    "-c, --completed-output <file:string>",
    "Completed jobs output file (default: completed_jobs.json)"
  )
  .option(
    "-q, --queued-output <file:string>",
    "Queued jobs output file (default: queued_jobs.json)"
  )
  .option(
    "--skip-worker-check",
    "Skip checking for active workers before export"
  )
  .arguments("[workspace:string]")
  .action(pullJobs as any);

const push = new Command()
  .description("Push completed and queued jobs to workspace")
  .option(
    "-c, --completed-file <file:string>",
    "Completed jobs input file (default: completed_jobs.json)"
  )
  .option(
    "-q, --queued-file <file:string>",
    "Queued jobs input file (default: queued_jobs.json)"
  )
  .option(
    "--skip-worker-check",
    "Skip checking for active workers before import"
  )
  .arguments("[workspace:string]")
  .action(pushJobs as any);

const command = new Command()
  .description("Manage jobs (import/export)")
  .command("pull", pull)
  .command("push", push);

export default command;
