import * as wmill from "../../gen/services.gen.ts";
import * as log from "./log.ts";

type CiTestedKind = "script" | "flow" | "resource";

// A test run between two deploys of a batch resolves its imports against a
// half-deployed workspace, so while a batch is open every deploy asks the
// server to hold its CI tests back, and the batch queues them once at the end.
// Nothing else queues them: a push that dies before `closeCiTestBatch` leaves
// its batch untested.
let batch: Map<string, { kind: CiTestedKind; path: string }> | undefined;

export function openCiTestBatch() {
  batch = new Map();
}

/** The deploy call's `skipCiTests` argument: set only while a batch is open. */
export function ciTestBatchSkip(): { skipCiTests?: boolean } {
  return batch ? { skipCiTests: true } : {};
}

/**
 * Record a deploy made with `ciTestBatchSkip`, once it has succeeded: the batch
 * endpoint refuses an item the caller may not deploy, and with it the batch.
 */
export function ciTestBatchDeployed(kind: CiTestedKind, path: string) {
  batch?.set(`${kind}:${path}`, { kind, path });
}

const POLL_MS = 2000;
const MAX_WAIT_MS = 10 * 60 * 1000;

/**
 * Queue the CI tests of everything deployed since `openCiTestBatch`, once the
 * server reports none of it is still being locked. After `MAX_WAIT_MS` they are
 * queued regardless, so a stuck lock job cannot leave the batch untested.
 */
export async function closeCiTestBatch(workspace: string): Promise<string[]> {
  const items = [...(batch?.values() ?? [])];
  batch = undefined;
  if (items.length === 0) return [];
  const deadline = Date.now() + MAX_WAIT_MS;
  let announced = false;
  try {
    while (true) {
      const ignorePending = Date.now() >= deadline;
      const res = await wmill.triggerCiTestsBatch({
        workspace,
        requestBody: { items, ignore_pending: ignorePending },
      });
      if (res.pending.length === 0 || ignorePending) {
        if (res.job_ids.length > 0) {
          log.info(`Queued ${res.job_ids.length} CI test job(s)`);
        }
        return res.job_ids;
      }
      if (!announced) {
        const shown = res.pending.slice(0, 5).join(", ");
        const more =
          res.pending.length > 5 ? ` and ${res.pending.length - 5} more` : "";
        log.info(
          `Waiting for lock generation of ${shown}${more} before running CI tests...`,
        );
        announced = true;
      }
      await new Promise((r) => setTimeout(r, POLL_MS));
    }
  } catch (e: any) {
    // A server that predates this endpoint already ran the tests on each deploy.
    if (e?.status === 404) return [];
    log.warn(`Failed to queue CI tests: ${e?.body ?? e?.message ?? e}`);
    return [];
  }
}
