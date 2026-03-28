import { GlobalOptions } from "../../types.ts";
import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace } from "../../core/context.ts";
import { Command } from "@cliffy/command";
import { Table } from "@cliffy/table";
import { colors } from "@cliffy/ansi/colors";
import * as log from "../../core/log.ts";
import { mergeConfigWithConfigFile } from "../../core/conf.ts";
import * as wmill from "../../../gen/services.gen.ts";
import { formatTimestamp } from "../../utils/utils.ts";

function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  const seconds = Math.floor(ms / 1000);
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  const remainingSeconds = seconds % 60;
  if (minutes < 60) return `${minutes}m${remainingSeconds}s`;
  const hours = Math.floor(minutes / 60);
  const remainingMinutes = minutes % 60;
  return `${hours}h${remainingMinutes}m`;
}

function getJobStatus(job: any): string {
  if (job.type === "QueuedJob") {
    if (job.canceled) return colors.red("canceled");
    if (job.running) return colors.blue("running");
    return colors.yellow("queued");
  }
  // CompletedJob
  if (job.canceled) return colors.red("canceled");
  if (job.success) return colors.green("success");
  return colors.red("failure");
}

function getJobStatusPlain(job: any): string {
  if (job.type === "QueuedJob") {
    if (job.canceled) return "canceled";
    if (job.running) return "running";
    return "queued";
  }
  if (job.canceled) return "canceled";
  if (job.success) return "success";
  return "failure";
}

async function list(
  opts: GlobalOptions & {
    json?: boolean;
    scriptPath?: string;
    createdBy?: string;
    running?: boolean;
    success?: boolean;
    failed?: boolean;
    limit?: number;
    jobKinds?: string;
    label?: string;
    all?: boolean;
  }
) {
  if (opts.json) log.setSilent(true);
  opts = await mergeConfigWithConfigFile(opts);
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  // --failed is a convenience alias for --success false
  let successFilter = opts.success;
  if (opts.failed) successFilter = false;

  const jobs = await wmill.listJobs({
    workspace: workspace.workspaceId,
    scriptPathExact: opts.scriptPath,
    createdBy: opts.createdBy,
    running: opts.running,
    success: successFilter,
    perPage: Math.min(opts.limit ?? 30, 100),
    jobKinds: opts.jobKinds ?? "script,flow,singlestepflow",
    label: opts.label,
    hasNullParent: opts.all ? undefined : true,
  });

  if (opts.json) {
    console.log(JSON.stringify(jobs));
  } else {
    if (jobs.length === 0) {
      log.info("No jobs found.");
      return;
    }
    new Table()
      .header(["ID", "Status", "Script/Flow", "Created By", "Duration", "Created At"])
      .padding(2)
      .border(true)
      .body(
        jobs.map((j: any) => [
          j.id.substring(0, 8),
          getJobStatus(j),
          j.script_path ?? j.raw_code?.substring(0, 30) ?? "-",
          j.created_by ?? j.email ?? "-",
          j.duration_ms != null ? formatDuration(j.duration_ms) : (j.running ? "running" : "-"),
          j.created_at ? formatTimestamp(j.created_at) : "-",
        ])
      )
      .render();
    log.info(`\nShowing ${jobs.length} job(s). Use --limit to show more.`);
  }
}

async function get(
  opts: GlobalOptions & { json?: boolean },
  id: string
) {
  if (opts.json) log.setSilent(true);
  opts = await mergeConfigWithConfigFile(opts);
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const job = await wmill.getJob({
    workspace: workspace.workspaceId,
    id,
  });

  if (opts.json) {
    console.log(JSON.stringify(job));
  } else {
    const j = job as any;
    console.log(colors.bold("ID:") + " " + j.id);
    console.log(colors.bold("Status:") + " " + getJobStatusPlain(j));
    console.log(colors.bold("Kind:") + " " + j.job_kind);
    console.log(colors.bold("Script Path:") + " " + (j.script_path ?? "-"));
    console.log(colors.bold("Created By:") + " " + (j.created_by ?? "-"));
    console.log(colors.bold("Created At:") + " " + (j.created_at ? formatTimestamp(j.created_at) : "-"));
    if (j.started_at) {
      console.log(colors.bold("Started At:") + " " + formatTimestamp(j.started_at));
    }
    if (j.duration_ms != null) {
      console.log(colors.bold("Duration:") + " " + formatDuration(j.duration_ms));
    }
    if (j.schedule_path) {
      console.log(colors.bold("Schedule:") + " " + j.schedule_path);
    }
    if (j.result !== undefined) {
      console.log(colors.bold("Result:"));
      console.log(JSON.stringify(j.result, null, 2));
    }
  }
}

async function result(
  opts: GlobalOptions,
  id: string
) {
  log.setSilent(true);
  opts = await mergeConfigWithConfigFile(opts);
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const jobResult = await wmill.getCompletedJobResult({
    workspace: workspace.workspaceId,
    id,
  });

  console.log(JSON.stringify(jobResult));
}

async function logs(
  opts: GlobalOptions,
  id: string
) {
  opts = await mergeConfigWithConfigFile(opts);
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const jobLogs = await wmill.getJobLogs({
    workspace: workspace.workspaceId,
    id,
  });

  console.log(jobLogs);
}

async function cancel(
  opts: GlobalOptions & { reason?: string },
  id: string
) {
  opts = await mergeConfigWithConfigFile(opts);
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  await wmill.cancelQueuedJob({
    workspace: workspace.workspaceId,
    id,
    requestBody: {
      reason: opts.reason ?? "Canceled via CLI",
    },
  });

  log.info(colors.green(`Job ${id} canceled.`));
}

// Shared list options to avoid repetition between default action and list subcommand
const listOptions = (cmd: Command) =>
  cmd
    .option("--json", "Output as JSON (for piping to jq)")
    .option("--script-path <scriptPath:string>", "Filter by exact script/flow path")
    .option("--created-by <createdBy:string>", "Filter by creator username")
    .option("--running", "Show only running jobs")
    .option("--failed", "Show only failed jobs")
    .option("--success <success:boolean>", "Filter by success status (true/false)")
    .option("--limit <limit:number>", "Number of jobs to return (default 30, max 100)")
    .option("--job-kinds <jobKinds:string>", "Filter by job kinds (default: script,flow,singlestepflow)")
    .option("--label <label:string>", "Filter by job label")
    .option("--all", "Include sub-jobs (flow steps). By default only top-level jobs are shown");

const command = listOptions(new Command()
  .description("Manage jobs (list, inspect, cancel)"))
  .action(list as any)
  .command("list", listOptions(new Command().description("List recent jobs")))
  .action(list as any)
  .command("get", "Get job details and result")
  .arguments("<id:string>")
  .option("--json", "Output as JSON (for piping to jq)")
  .action(get as any)
  .command("result", "Get the result of a completed job (machine-friendly)")
  .arguments("<id:string>")
  .action(result as any)
  .command("logs", "Get job logs")
  .arguments("<id:string>")
  .action(logs as any)
  .command("cancel", "Cancel a running or queued job")
  .arguments("<id:string>")
  .option("--reason <reason:string>", "Reason for cancellation")
  .action(cancel as any);

export default command;
