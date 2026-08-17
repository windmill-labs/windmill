import * as wmill from "../../gen/services.gen.ts";
import * as log from "../core/log.ts";
import { colors } from "@cliffy/ansi/colors";

const DEFAULT_FAST_POLL_INTERVAL_MS = 100;
const DEFAULT_FAST_POLL_DURATION_MS = 2000;
const DEFAULT_SLOW_POLL_INTERVAL_MS = 2000;
const QUEUE_LOG_INTERVAL_MS = 5000;
const HEARTBEAT_INTERVAL_MS = 60000;
const MAX_CONSECUTIVE_POLL_ERRORS = 10;

export type JobCompletion = { result: unknown; success: boolean };

export async function logQueueStatus(
  workspace: string,
  jobId: string,
  label: string = "Job ",
): Promise<boolean> {
  try {
    const job: any = await wmill.getJob({ workspace, id: jobId });
    if (!job) return false;

    if (job.running === true) {
      log.info(
        colors.gray(`${label}${jobId}: running, waiting for completion...`),
      );
      return true;
    }

    if (typeof job.running !== "boolean") {
      return false;
    }

    const scheduledFor = job.scheduled_for as string | undefined;
    if (!scheduledFor) {
      log.info(colors.gray(`${label}${jobId}: queued, waiting for executor...`));
      return true;
    }

    const scheduledForMs = new Date(scheduledFor).getTime();
    if (!Number.isFinite(scheduledForMs)) {
      log.info(colors.gray(`${label}${jobId}: queued, waiting for executor...`));
      return true;
    }

    try {
      const pos = await wmill.getQueuePosition({
        workspace,
        scheduledFor: scheduledForMs,
      });
      const position = (pos as any)?.position;
      if (position != null) {
        log.info(
          colors.gray(
            `${label}${jobId}: queued, waiting for executor (position ${position} in queue)`,
          ),
        );
      } else {
        log.info(
          colors.gray(`${label}${jobId}: queued, waiting for executor...`),
        );
      }
      return true;
    } catch {
      log.info(colors.gray(`${label}${jobId}: queued, waiting for executor...`));
      return true;
    }
  } catch {
    return false;
  }
}

export async function pollJobWithQueueLogging(
  workspace: string,
  jobId: string,
  options?: {
    fastPollIntervalMs?: number;
    fastPollDurationMs?: number;
    slowPollIntervalMs?: number;
    label?: string;
  },
): Promise<JobCompletion> {
  const fastPollIntervalMs =
    options?.fastPollIntervalMs ?? DEFAULT_FAST_POLL_INTERVAL_MS;
  const fastPollDurationMs =
    options?.fastPollDurationMs ?? DEFAULT_FAST_POLL_DURATION_MS;
  const slowPollIntervalMs =
    options?.slowPollIntervalMs ?? DEFAULT_SLOW_POLL_INTERVAL_MS;
  const label = options?.label ? `[${options.label}] ` : "Job ";
  const startedAt = Date.now();
  let lastQueueLogAt = Date.now();
  let lastHeartbeatAt = Date.now();
  let consecutiveErrors = 0;

  while (true) {
    try {
      const maybe = await wmill.getCompletedJobResultMaybe({
        workspace,
        id: jobId,
        getStarted: false,
      });

      consecutiveErrors = 0;

      if (maybe.completed) {
        return { result: maybe.result, success: maybe.success ?? false };
      }
    } catch (err: any) {
      consecutiveErrors++;
      log.warn(
        colors.yellow(
          `${label}${jobId}: error checking job status (${consecutiveErrors}/${MAX_CONSECUTIVE_POLL_ERRORS}): ${err?.message ?? err}`,
        ),
      );
      lastHeartbeatAt = Date.now();
      if (consecutiveErrors >= MAX_CONSECUTIVE_POLL_ERRORS) {
        throw new Error(
          `Giving up polling job ${jobId} after ${MAX_CONSECUTIVE_POLL_ERRORS} consecutive errors. Last error: ${err?.message ?? err}`,
        );
      }
    }

    if (Date.now() - lastQueueLogAt >= QUEUE_LOG_INTERVAL_MS) {
      lastQueueLogAt = Date.now();
      const logged = await logQueueStatus(workspace, jobId, label);
      if (logged) lastHeartbeatAt = Date.now();
    }

    if (Date.now() - lastHeartbeatAt >= HEARTBEAT_INTERVAL_MS) {
      lastHeartbeatAt = Date.now();
      log.info(
        colors.gray(
          `${label}${jobId}: still polling, queue status unavailable...`,
        ),
      );
    }

    const delayMs =
      Date.now() - startedAt < fastPollDurationMs
        ? fastPollIntervalMs
        : slowPollIntervalMs;
    await new Promise((resolve) => setTimeout(resolve, delayMs));
  }
}
