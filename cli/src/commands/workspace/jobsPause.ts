import { colors } from "@cliffy/ansi/colors";
import { GlobalOptions } from "../../types.ts";
import * as log from "../../core/log.ts";
import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace } from "../../core/context.ts";
import { getHeaders } from "../../utils/utils.ts";
import * as wmill from "../../../gen/services.gen.ts";

export const DEFAULT_PAUSE_DURATION = "30m";

const UNIT_SECONDS: Record<string, number> = { s: 1, m: 60, h: 3600, d: 86400 };

/** `90s`, `30m`, `2h`, `1d` or a bare number of seconds. */
export function parseDuration(value: string): number {
  const m = /^\s*(\d+)\s*([smhd]?)\s*$/.exec(value);
  if (!m || Number(m[1]) <= 0) {
    throw new Error(
      `Invalid duration "${value}": expected a positive number followed by s, m, h or d (e.g. 30m)`,
    );
  }
  return Number(m[1]) * UNIT_SECONDS[m[2] || "s"];
}

export async function pauseWorkspaceJobs(
  workspaceId: string,
  durationS: number,
  reason?: string,
) {
  const pause = await wmill.pauseJobs({
    workspace: workspaceId,
    requestBody: { duration_s: durationS, reason, source: "cli" },
  });
  log.info(
    colors.yellow(
      `New jobs of ${workspaceId} are paused until ${pause?.paused_until}` +
        (pause?.held_count ? ` (${pause.held_count} queued jobs held)` : ""),
    ),
  );
  return pause;
}

export async function resumeWorkspaceJobs(workspaceId: string) {
  const released = await wmill.resumeJobs({ workspace: workspaceId });
  log.info(
    colors.green(
      `Jobs of ${workspaceId} resumed, ${released} held job${released === 1 ? "" : "s"} released`,
    ),
  );
  return released;
}

const DEPENDENCY_JOB_KINDS = "dependencies,flowdependencies,appdependencies";

/**
 * Waits until no dependency job created since `sinceIso` is still queued or running, or until
 * `deadline`. Returns whether they all finished.
 */
export async function waitForDependencyJobs(
  workspaceId: string,
  sinceIso: string,
  deadline: Date,
): Promise<boolean> {
  // Raw fetch: the generated client has no `created_after` filter on the queue list.
  const { OpenAPI } = await import("../../../gen/index.ts");
  const url =
    `${OpenAPI.BASE}/w/${workspaceId}/jobs/queue/list?job_kinds=${DEPENDENCY_JOB_KINDS}` +
    `&created_after=${encodeURIComponent(sinceIso)}&per_page=100`;
  let announced = false;
  while (true) {
    const resp = await fetch(url, {
      headers: { ...getHeaders(), Authorization: `Bearer ${OpenAPI.TOKEN}` },
    });
    if (!resp.ok) throw new Error(`Listing dependency jobs failed: ${resp.status}`);
    const queued = (await resp.json()) as unknown[];
    if (queued.length === 0) return true;
    if (Date.now() >= deadline.getTime()) return false;
    if (!announced) {
      log.info(
        colors.gray(
          `Waiting for ${queued.length} dependency job(s) to finish before resuming jobs ...`,
        ),
      );
      announced = true;
    }
    await new Promise((r) => setTimeout(r, 2000));
  }
}

export async function pauseJobsCommand(
  opts: GlobalOptions & { for?: string; reason?: string },
) {
  await requireLogin(opts);
  const workspace = await resolveWorkspace(opts);
  await pauseWorkspaceJobs(
    workspace.workspaceId,
    parseDuration(opts.for ?? DEFAULT_PAUSE_DURATION),
    opts.reason,
  );
}

export async function resumeJobsCommand(opts: GlobalOptions) {
  await requireLogin(opts);
  const workspace = await resolveWorkspace(opts);
  await resumeWorkspaceJobs(workspace.workspaceId);
}

export async function jobsPausedCommand(opts: GlobalOptions & { json?: boolean }) {
  await requireLogin(opts);
  const workspace = await resolveWorkspace(opts);
  const pause = await wmill.getJobsPaused({ workspace: workspace.workspaceId });
  if (opts.json) {
    console.log(JSON.stringify(pause, null, 2));
  } else if (!pause) {
    log.info("New jobs are not paused");
  } else {
    log.info(
      `New jobs are paused until ${pause.paused_until} by ${pause.paused_by}, ${pause.held_count} held` +
        (pause.reason ? ` (${pause.reason})` : ""),
    );
  }
}
