/**
 * Kestra CompetitorAdapter.
 *
 * Kestra 1.3+ has mandatory basic auth that requires UI-based initial setup.
 * To work around this, we:
 * - Deploy flows via direct SQL insertion into the Kestra Postgres database
 * - Trigger executions via the webhook trigger which can be configured with a key
 * - Poll execution status via SQL queries
 *
 * For execution triggering, we use the internal queue mechanism:
 * insert execution records directly and let the Kestra worker pick them up.
 *
 * Alternative simpler approach: use `docker exec` to run flows via the kestra CLI.
 */

import type { CompetitorAdapter } from "../types.ts";
import { composeUp, composeDown, waitForHealth } from "../lib/docker.ts";
import { dirname, fromFileUrl, resolve } from "https://deno.land/std@0.224.0/path/mod.ts";

const SELF_DIR = dirname(fromFileUrl(import.meta.url));
const KESTRA_URL = "http://127.0.0.1:8081";
const KESTRA_PG = "postgresql://kestra:kestra@127.0.0.1:5434/kestra";

async function dockerExec(container: string, ...args: string[]): Promise<string> {
  const cmd = new Deno.Command("docker", {
    args: ["exec", container, ...args],
    stdout: "piped",
    stderr: "piped",
  });
  const output = await cmd.output();
  const stdout = new TextDecoder().decode(output.stdout);
  if (output.code !== 0) {
    const stderr = new TextDecoder().decode(output.stderr);
    throw new Error(`docker exec failed: ${stderr}`);
  }
  return stdout.trim();
}

async function psql(query: string): Promise<string> {
  return await dockerExec(
    "kestra-postgres-1",
    "psql", "-U", "kestra", "-d", "kestra", "-t", "-A", "-c", query,
  );
}

async function deployFlowViaSQL(): Promise<void> {
  const flowYaml = await Deno.readTextFile(resolve(SELF_DIR, "workflow.yml"));
  const now = new Date().toISOString();
  const flowJson = JSON.stringify({
    id: "benchmark-3step",
    namespace: "benchmark",
    tenantId: "main",
    revision: 1,
    deleted: false,
    disabled: false,
    updated: now,
    source: flowYaml,
    tasks: [
      { id: "step_a", type: "io.kestra.plugin.core.debug.Return", format: "1" },
      { id: "step_b", type: "io.kestra.plugin.core.debug.Return", format: "2" },
      { id: "step_c", type: "io.kestra.plugin.core.debug.Return", format: "3" },
    ],
    triggers: [
      { id: "benchmark_webhook", type: "io.kestra.plugin.core.trigger.Webhook", key: "benchmark-key" },
    ],
  });
  const escapedJson = flowJson.replace(/'/g, "''");
  const escapedYaml = flowYaml.replace(/'/g, "''");

  // Delete existing flow if any
  await psql(`DELETE FROM flows WHERE id = 'benchmark-3step' AND namespace = 'benchmark';`).catch(() => {});

  // Insert the flow with the correct key format: tenantId_namespace_id_revision
  await psql(
    `INSERT INTO flows (key, value, source_code) VALUES ('main_benchmark_benchmark-3step_1', '${escapedJson}'::jsonb, '${escapedYaml}');`,
  );

  // Also insert into the queues table to notify Kestra of the new flow
  const queueMsg = JSON.stringify({
    type: "io.kestra.core.models.flows.Flow",
    key: "main_benchmark_benchmark-3step_1",
  });
  const escapedQueue = queueMsg.replace(/'/g, "''");
  await psql(
    `INSERT INTO queues (type, key, value, consumers, updated) VALUES ('io.kestra.core.models.flows.Flow', 'main_benchmark_benchmark-3step_1', '${escapedQueue}'::jsonb, '{}', NOW());`,
  ).catch(() => {});
}

async function triggerViaWebhook(): Promise<{ executionId: string }> {
  const resp = await fetch(
    `${KESTRA_URL}/api/v1/executions/webhook/benchmark/benchmark-3step/benchmark-key`,
    { method: "POST" },
  );
  if (!resp.ok) {
    throw new Error(`Kestra webhook trigger failed: ${resp.status} ${await resp.text()}`);
  }
  const body = await resp.json();
  return { executionId: body.id };
}

async function waitForExecution(executionId: string, timeoutMs = 30000): Promise<unknown> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      const result = await psql(
        `SELECT value->>'state' FROM executions WHERE key = '${executionId}';`,
      );
      const lines = result.split("\n").filter(Boolean);
      if (lines.length > 0) {
        try {
          const state = JSON.parse(lines[0]);
          if (state.current === "SUCCESS") return state;
          if (state.current === "FAILED" || state.current === "KILLED") {
            throw new Error(`Kestra execution failed: ${lines[0]}`);
          }
        } catch (e) {
          if (e instanceof Error && e.message.startsWith("Kestra execution failed")) throw e;
          // Not valid JSON state yet, maybe "SUCCESS" as raw string
          if (lines[0].includes("SUCCESS")) return { current: "SUCCESS" };
          if (lines[0].includes("FAILED")) throw new Error(`Kestra execution failed: ${lines[0]}`);
        }
      }
    } catch (e) {
      if (e instanceof Error && e.message.startsWith("Kestra execution failed")) throw e;
    }
    await new Promise((r) => setTimeout(r, 100));
  }
  throw new Error(`Kestra execution ${executionId} did not complete within ${timeoutMs}ms`);
}

async function triggerAndWait(): Promise<unknown> {
  const { executionId } = await triggerViaWebhook();
  return await waitForExecution(executionId);
}

export const kestraAdapter: CompetitorAdapter = {
  name: "kestra",
  composeFile: resolve(SELF_DIR, "docker-compose.yml"),

  async setup() {
    await composeUp(this.composeFile);
    // Wait for Kestra to be ready (configs endpoint is public)
    await waitForHealth(`${KESTRA_URL}/api/v1/configs`, {
      maxRetries: 40,
      intervalMs: 3000,
    });
    // Extra wait for Kestra to finish internal initialization
    await new Promise((r) => setTimeout(r, 5000));
  },

  async deployWorkflow() {
    await deployFlowViaSQL();
    // Wait for Kestra to pick up the flow from the database
    await new Promise((r) => setTimeout(r, 3000));
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
      const resp = await fetch(`${KESTRA_URL}/api/v1/configs`);
      const body = await resp.json();
      return body.version ?? "unknown";
    } catch (_) {
      return "unknown";
    }
  },
};
