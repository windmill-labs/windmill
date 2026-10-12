/**
 * Prefect CompetitorAdapter.
 *
 * Uses Prefect 3.x REST API:
 * - GET /api/deployments/name/{flow}/{deployment} to find deployment ID
 * - POST /api/deployments/{id}/create_flow_run to trigger
 * - GET /api/flow_runs/{id} to poll status
 *
 * The worker runs flow.py with .serve() which both registers and executes.
 */

import type { CompetitorAdapter } from "../types.ts";
import { composeUp, composeDown, waitForHealth } from "../lib/docker.ts";
import { dirname, fromFileUrl, resolve } from "https://deno.land/std@0.224.0/path/mod.ts";

const SELF_DIR = dirname(fromFileUrl(import.meta.url));
const PREFECT_URL = "http://127.0.0.1:4200/api";

let deploymentId = "";

async function getDeploymentId(): Promise<string> {
  const resp = await fetch(
    `${PREFECT_URL}/deployments/name/benchmark-3step/benchmark-deployment`,
  );
  if (!resp.ok) throw new Error(`Get deployment failed: ${resp.status} ${await resp.text()}`);
  const body = await resp.json();
  return body.id;
}

async function triggerFlowRun(): Promise<string> {
  const resp = await fetch(
    `${PREFECT_URL}/deployments/${deploymentId}/create_flow_run`,
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: "{}",
    },
  );
  if (!resp.ok) throw new Error(`Trigger failed: ${resp.status} ${await resp.text()}`);
  const body = await resp.json();
  return body.id;
}

async function waitForFlowRun(runId: string, timeoutMs = 60000): Promise<unknown> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const resp = await fetch(`${PREFECT_URL}/flow_runs/${runId}`);
    if (resp.ok) {
      const body = await resp.json();
      const state = body.state_type;
      if (state === "COMPLETED") return body.state?.result;
      if (state === "FAILED" || state === "CRASHED" || state === "CANCELLED") {
        throw new Error(`Prefect flow run failed: ${state}`);
      }
    } else {
      await resp.body?.cancel();
    }
    await new Promise((r) => setTimeout(r, 100));
  }
  throw new Error(`Prefect flow run ${runId} did not complete within ${timeoutMs}ms`);
}

async function triggerAndWait(): Promise<unknown> {
  const runId = await triggerFlowRun();
  return await waitForFlowRun(runId);
}

export const prefectAdapter: CompetitorAdapter = {
  name: "prefect",
  composeFile: resolve(SELF_DIR, "docker-compose.yml"),

  async setup() {
    await composeUp(this.composeFile);
    await waitForHealth(`${PREFECT_URL}/health`, { maxRetries: 40, intervalMs: 3000 });
    // Wait for worker to register the deployment
    await new Promise((r) => setTimeout(r, 10000));
  },

  async deployWorkflow() {
    // The worker's .serve() auto-registers. Poll until the deployment appears.
    for (let i = 0; i < 30; i++) {
      try {
        deploymentId = await getDeploymentId();
        if (deploymentId) return;
      } catch (_) {
        // not registered yet
      }
      await new Promise((r) => setTimeout(r, 2000));
    }
    throw new Error("Prefect deployment did not register in time");
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
      const resp = await fetch(`${PREFECT_URL}/admin/version`);
      return (await resp.text()).replace(/"/g, "");
    } catch (_) {
      return "unknown";
    }
  },
};
