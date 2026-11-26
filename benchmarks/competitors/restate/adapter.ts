/**
 * Restate CompetitorAdapter.
 *
 * Uses Restate's pure HTTP APIs:
 * - POST /deployments (admin) to register the app
 * - POST /benchmark/{workflowId}/run (ingress) to start a workflow
 * - GET /restate/workflow/benchmark/{workflowId}/attach to wait for result
 */

import type { CompetitorAdapter } from "../types.ts";
import { composeUp, composeDown, waitForHealth } from "../lib/docker.ts";
import { dirname, fromFileUrl, resolve } from "https://deno.land/std@0.224.0/path/mod.ts";

const SELF_DIR = dirname(fromFileUrl(import.meta.url));
const INGRESS_URL = "http://127.0.0.1:8085";
const ADMIN_URL = "http://127.0.0.1:9075";

async function triggerWorkflow(): Promise<unknown> {
  const workflowId = crypto.randomUUID();

  // Start the workflow via ingress (send mode for immediate return)
  const startResp = await fetch(
    `${INGRESS_URL}/benchmark/${workflowId}/run/send`,
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: "{}",
    },
  );
  if (!startResp.ok) {
    throw new Error(
      `Restate workflow start failed: ${startResp.status} ${await startResp.text()}`,
    );
  }
  await startResp.body?.cancel();

  // Attach and wait for the result
  const attachResp = await fetch(
    `${INGRESS_URL}/restate/workflow/benchmark/${workflowId}/attach`,
    {
      method: "GET",
      headers: { Accept: "application/json" },
    },
  );
  if (!attachResp.ok) {
    throw new Error(
      `Restate workflow attach failed: ${attachResp.status} ${await attachResp.text()}`,
    );
  }
  return await attachResp.json();
}

export const restateAdapter: CompetitorAdapter = {
  name: "restate",
  composeFile: resolve(SELF_DIR, "docker-compose.yml"),

  async setup() {
    await composeUp(this.composeFile);
    await waitForHealth(`${ADMIN_URL}/health`, { maxRetries: 30 });
    // Wait for app server
    await new Promise((r) => setTimeout(r, 3000));
  },

  async deployWorkflow() {
    // Register the app deployment with Restate admin API
    const resp = await fetch(`${ADMIN_URL}/deployments`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ uri: "http://app:9080" }),
    });
    if (!resp.ok) {
      const body = await resp.text();
      // 409 = already registered, that's fine
      if (resp.status !== 409) {
        throw new Error(`Restate deployment registration failed: ${resp.status} ${body}`);
      }
    } else {
      await resp.body?.cancel();
    }
    await new Promise((r) => setTimeout(r, 1000));
  },

  async triggerOne() {
    const start = performance.now();
    const result = await triggerWorkflow();
    const latencyMs = performance.now() - start;
    return { latencyMs, result };
  },

  async triggerBatch(n: number) {
    const start = performance.now();
    const promises = Array.from({ length: n }, () => triggerWorkflow());
    const results = await Promise.all(promises);
    const totalMs = performance.now() - start;
    return { totalMs, results };
  },

  async teardown() {
    await composeDown(this.composeFile);
  },

  async getVersion() {
    try {
      const resp = await fetch(`${ADMIN_URL}/health`);
      return (await resp.text()).trim();
    } catch (_) {
      return "unknown";
    }
  },
};
