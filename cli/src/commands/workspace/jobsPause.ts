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

// Servers and workers cache whether a workspace is paused for up to 10s; until then they can still
// start a new job. Deploying only after that keeps such a job on the pre-deploy state.
const PAUSE_PROPAGATION_MS = 11_000;

export async function waitForPauseToPropagate() {
  log.info(colors.gray("Waiting for the pause to reach every server ..."));
  await new Promise((r) => setTimeout(r, PAUSE_PROPAGATION_MS));
}

/**
 * With `pauseId`, resumes only if no one paused again since (another deploy's pause stays) and
 * says whether it resumed.
 */
export async function resumeWorkspaceJobs(
  workspaceId: string,
  pauseId?: string,
): Promise<boolean> {
  let released: number;
  try {
    released = await wmill.resumeJobs({
      workspace: workspaceId,
      requestBody: pauseId ? { pause_id: pauseId } : undefined,
    });
  } catch (e: any) {
    if (pauseId && e?.status === 409) {
      log.warn(`Jobs not resumed: ${e.body ?? e.message}`);
      return false;
    }
    throw e;
  }
  log.info(
    colors.green(
      `Jobs of ${workspaceId} resumed, ${released} held job${released === 1 ? "" : "s"} released`,
    ),
  );
  return true;
}

const DEPENDENCY_JOB_KINDS = "dependencies,flowdependencies,appdependencies";

/**
 * Waits until no dependency job created since `sinceIso` is still queued or running, or until
 * `deadline`. `finished` is false when some are still running at the deadline; `failed` counts
 * those that failed.
 */
export async function waitForDependencyJobs(
  workspaceId: string,
  sinceIso: string,
  deadline: Date,
): Promise<{ finished: boolean; failed: number }> {
  // Raw fetch: the generated client has no `created_after` filter on the job lists.
  const { OpenAPI } = await import("../../../gen/index.ts");
  const filter =
    `job_kinds=${DEPENDENCY_JOB_KINDS}&created_after=${encodeURIComponent(sinceIso)}&per_page=100`;
  const list = async (route: string) => {
    const resp = await fetch(`${OpenAPI.BASE}/w/${workspaceId}/jobs/${route}`, {
      headers: { ...getHeaders(), Authorization: `Bearer ${OpenAPI.TOKEN}` },
    });
    if (!resp.ok) throw new Error(`Listing dependency jobs failed: ${resp.status}`);
    return (await resp.json()) as unknown[];
  };
  let announced = false;
  while (true) {
    const queued = await list(`queue/list?${filter}`);
    const finished = queued.length === 0;
    if (finished || Date.now() >= deadline.getTime()) {
      const failed = await list(`completed/list?${filter}&success=false`);
      return { finished, failed: failed.length };
    }
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
