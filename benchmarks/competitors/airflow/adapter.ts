/**
 * Airflow CompetitorAdapter.
 *
 * Uses Airflow 2.x REST API:
 * - GET /api/v1/dags/{dag_id} to check DAG is loaded
 * - PATCH /api/v1/dags/{dag_id} to unpause
 * - POST /api/v1/dags/{dag_id}/dagRuns to trigger
 * - GET /api/v1/dags/{dag_id}/dagRuns/{run_id} to poll status
 *
 * All requests require Basic Auth (admin:admin).
 */

import type { CompetitorAdapter } from "../types.ts";
import { composeUp, composeDown, waitForHealth } from "../lib/docker.ts";
import { dirname, fromFileUrl, resolve } from "https://deno.land/std@0.224.0/path/mod.ts";

const SELF_DIR = dirname(fromFileUrl(import.meta.url));
const AIRFLOW_URL = "http://127.0.0.1:8090/api/v1";
const AUTH_HEADER = "Basic " + btoa("admin:admin");
const DAG_ID = "benchmark_3step";

function headers(): Record<string, string> {
  return {
    Authorization: AUTH_HEADER,
    "Content-Type": "application/json",
  };
}

async function waitForDag(timeoutMs = 120000): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      const resp = await fetch(`${AIRFLOW_URL}/dags/${DAG_ID}`, {
        headers: headers(),
      });
      if (resp.ok) {
        await resp.body?.cancel();
        return;
      }
      await resp.body?.cancel();
    } catch (_) {
      // not ready
    }
    await new Promise((r) => setTimeout(r, 2000));
  }
  throw new Error("Airflow DAG did not appear in time");
}

async function unpauseDag(): Promise<void> {
  const resp = await fetch(`${AIRFLOW_URL}/dags/${DAG_ID}`, {
    method: "PATCH",
    headers: headers(),
    body: JSON.stringify({ is_paused: false }),
  });
  if (!resp.ok) throw new Error(`Unpause failed: ${resp.status} ${await resp.text()}`);
  await resp.body?.cancel();
}

async function triggerDagRun(): Promise<string> {
  const resp = await fetch(`${AIRFLOW_URL}/dags/${DAG_ID}/dagRuns`, {
    method: "POST",
    headers: headers(),
    body: JSON.stringify({ conf: {} }),
  });
  if (!resp.ok) throw new Error(`Trigger failed: ${resp.status} ${await resp.text()}`);
  const body = await resp.json();
  return body.dag_run_id;
}

async function waitForDagRun(runId: string, timeoutMs = 120000): Promise<unknown> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const resp = await fetch(
      `${AIRFLOW_URL}/dags/${DAG_ID}/dagRuns/${encodeURIComponent(runId)}`,
      { headers: headers() },
    );
    if (resp.ok) {
      const body = await resp.json();
      if (body.state === "success") return body;
      if (body.state === "failed") throw new Error(`Airflow DAG run failed: ${JSON.stringify(body)}`);
    } else {
      await resp.body?.cancel();
    }
    await new Promise((r) => setTimeout(r, 200));
  }
  throw new Error(`Airflow DAG run ${runId} did not complete within ${timeoutMs}ms`);
}

async function triggerAndWait(): Promise<unknown> {
  const runId = await triggerDagRun();
  return await waitForDagRun(runId);
}

export const airflowAdapter: CompetitorAdapter = {
  name: "airflow",
  composeFile: resolve(SELF_DIR, "docker-compose.yml"),

  async setup() {
    await composeUp(this.composeFile);
    // Airflow takes a while to initialize (DB migration + scheduler startup)
    await waitForHealth(`${AIRFLOW_URL}/health`, {
      maxRetries: 60,
      intervalMs: 3000,
    });
  },

  async deployWorkflow() {
    // DAG file is volume-mounted. Wait for scheduler to parse it.
    await waitForDag();
    await unpauseDag();
    await new Promise((r) => setTimeout(r, 2000));
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
      const resp = await fetch(`${AIRFLOW_URL}/version`, { headers: headers() });
      const body = await resp.json();
      return body.version ?? "unknown";
    } catch (_) {
      return "unknown";
    }
  },
};
