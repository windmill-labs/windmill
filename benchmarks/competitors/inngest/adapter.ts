/**
 * Inngest CompetitorAdapter.
 *
 * Uses Inngest Dev Server HTTP APIs:
 * - POST /e/{eventKey} to send events (trigger functions)
 * - GET /v1/events/{eventId}/runs to poll for function completion
 */

import type { CompetitorAdapter } from "../types.ts";
import { composeUp, composeDown, waitForHealth } from "../lib/docker.ts";
import { dirname, fromFileUrl, resolve } from "https://deno.land/std@0.224.0/path/mod.ts";

const SELF_DIR = dirname(fromFileUrl(import.meta.url));
const INNGEST_URL = "http://127.0.0.1:8288";
const EVENT_KEY = "bench-event-key";

async function sendEvent(): Promise<string> {
  const resp = await fetch(`${INNGEST_URL}/e/${EVENT_KEY}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      name: "benchmark/run",
      data: {},
    }),
  });
  if (!resp.ok) throw new Error(`Event send failed: ${resp.status} ${await resp.text()}`);
  const body = await resp.json();
  // Dev server returns { ids: [eventId], status: 200 }
  return body.ids?.[0] ?? body.internal_id ?? "";
}

async function waitForRun(
  eventId: string,
  timeoutMs = 30000,
): Promise<unknown> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      const resp = await fetch(
        `${INNGEST_URL}/v1/events/${eventId}/runs?ts=${Date.now()}`,
        { headers: { "Cache-Control": "no-cache" } },
      );
      if (resp.ok) {
        const body = await resp.json();
        const runs = body.data ?? body;
        if (Array.isArray(runs) && runs.length > 0) {
          const run = runs[0];
          if (run.status === "Completed" || run.status === "completed") {
            return run.output;
          }
          if (run.status === "Failed" || run.status === "failed") {
            throw new Error(`Inngest run failed: ${JSON.stringify(run)}`);
          }
        }
      } else {
        await resp.body?.cancel();
      }
    } catch (e) {
      if (e instanceof Error && e.message.startsWith("Inngest run failed")) throw e;
      // connection error, retry
    }
    await new Promise((r) => setTimeout(r, 50));
  }
  throw new Error(`Inngest run did not complete within ${timeoutMs}ms`);
}

async function triggerAndWait(): Promise<unknown> {
  const eventId = await sendEvent();
  return await waitForRun(eventId);
}

export const inngestAdapter: CompetitorAdapter = {
  name: "inngest",
  composeFile: resolve(SELF_DIR, "docker-compose.yml"),

  async setup() {
    await composeUp(this.composeFile);
    await waitForHealth(`${INNGEST_URL}/health`, { maxRetries: 30 });
    await waitForHealth("http://127.0.0.1:3010/health", { maxRetries: 30 });
    // Give the dev server time to discover and sync the app functions
    await new Promise((r) => setTimeout(r, 3000));
  },

  async deployWorkflow() {
    // Trigger a sync by hitting the app's inngest endpoint from the dev server
    try {
      await fetch(`${INNGEST_URL}/v0/gql`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          query: `mutation { syncNewApp(appURL: "http://app:3000/api/inngest") { app { id } } }`,
        }),
      });
    } catch (_) {
      // Best effort
    }

    // Wait until the function is actually registered by sending a test event
    // and checking that a run appears
    for (let i = 0; i < 20; i++) {
      try {
        const testEventId = await sendEvent();
        await new Promise((r) => setTimeout(r, 2000));
        const resp = await fetch(`${INNGEST_URL}/v1/events/${testEventId}/runs`);
        if (resp.ok) {
          const body = await resp.json();
          const runs = body.data ?? body;
          if (Array.isArray(runs) && runs.length > 0) {
            // Function is registered and running/completed. Wait for completion.
            await waitForRun(testEventId, 15000).catch(() => {});
            return;
          }
        }
      } catch (_) {
        // not ready yet
      }
      await new Promise((r) => setTimeout(r, 2000));
    }
    throw new Error("Inngest function did not register after retries");
  },

  async triggerOne() {
    const start = performance.now();
    const result = await triggerAndWait();
    const latencyMs = performance.now() - start;
    return { latencyMs, result };
  },

  async triggerBatch(n: number) {
    const start = performance.now();
    const promises = Array.from({ length: n }, () => triggerAndWait());
    const results = await Promise.all(promises);
    const totalMs = performance.now() - start;
    return { totalMs, results };
  },

  async teardown() {
    await composeDown(this.composeFile);
  },

  async getVersion() {
    try {
      const resp = await fetch(`${INNGEST_URL}/health`);
      return (await resp.text()).trim();
    } catch (_) {
      return "unknown";
    }
  },
};
