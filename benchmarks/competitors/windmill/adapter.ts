/**
 * Windmill CompetitorAdapter.
 *
 * Uses Windmill REST API to deploy and trigger WAC 3-step sequential workflows.
 * Follows the same patterns as benchmarks/lib.ts.
 */

import type { CompetitorAdapter } from "../types.ts";
import { composeUp, composeDown, waitForHealth } from "../lib/docker.ts";
import { dirname, fromFileUrl, resolve } from "https://deno.land/std@0.224.0/path/mod.ts";

const SELF_DIR = dirname(fromFileUrl(import.meta.url));

const HOST = "http://127.0.0.1:8010";
const WORKSPACE = "admins";
const EMAIL = "admin@windmill.dev";
const PASSWORD = "changeme";

// Use step() (inline execution, no child jobs) — equivalent to what
// Restate ctx.run(), Inngest step.run(), and Kestra Return tasks do.
// task() would create separate child jobs which none of the competitors do.
const WAC_SCRIPT_CONTENT = [
  'import { step, workflow } from "windmill-client";',
  "export const main = workflow(async () => {",
  '  const a = await step("a", () => 1);',
  '  const b = await step("b", () => 2);',
  '  const c = await step("c", () => 3);',
  "  return { a, b, c };",
  "});",
].join("\n");

const SCRIPT_PATH = "f/benchmarks/wac_competitor_seq_3";

async function getToken(): Promise<string> {
  const resp = await fetch(`${HOST}/api/auth/login`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ email: EMAIL, password: PASSWORD }),
  });
  if (!resp.ok) throw new Error(`Login failed: ${resp.status} ${await resp.text()}`);
  return (await resp.text()).replace(/"/g, "");
}

let token = "";

function headers(): Record<string, string> {
  return {
    Authorization: `Bearer ${token}`,
    "Content-Type": "application/json",
  };
}

export const windmillAdapter: CompetitorAdapter = {
  name: "windmill",
  composeFile: resolve(SELF_DIR, "docker-compose.bench.yml"),

  async setup() {
    await composeUp(this.composeFile);
    await waitForHealth(`${HOST}/api/version`, { maxRetries: 40 });
    token = await getToken();
  },

  async deployWorkflow() {
    // Delete if exists
    try {
      await fetch(
        `${HOST}/api/w/${WORKSPACE}/scripts/delete/p/${SCRIPT_PATH}`,
        { method: "POST", headers: headers() },
      );
    } catch (_) {
      // ignore
    }

    // Create script
    const resp = await fetch(`${HOST}/api/w/${WORKSPACE}/scripts/create`, {
      method: "POST",
      headers: headers(),
      body: JSON.stringify({
        path: SCRIPT_PATH,
        content: WAC_SCRIPT_CONTENT,
        summary: "WAC competitor benchmark (3 sequential steps)",
        description: "",
        language: "bun",
        schema: {
          $schema: "https://json-schema.org/draft/2020-12/schema",
          properties: {},
          required: [],
          type: "object",
        },
      }),
    });
    if (!resp.ok) throw new Error(`Deploy failed: ${resp.status} ${await resp.text()}`);
    const hash = await resp.text();

    // Wait for deployment (lock file generation)
    for (let i = 0; i < 30; i++) {
      try {
        const status = await fetch(
          `${HOST}/api/w/${WORKSPACE}/scripts/deployment_status/h/${hash.replace(/"/g, "")}`,
          { headers: headers() },
        );
        const data = await status.json();
        if (data.lock !== null && data.lock !== undefined) return;
      } catch (_) {
        // retry
      }
      await new Promise((r) => setTimeout(r, 500));
    }
    throw new Error("Script did not deploy in time");
  },

  async triggerOne() {
    const start = performance.now();
    const resp = await fetch(
      `${HOST}/api/w/${WORKSPACE}/jobs/run_wait_result/p/${SCRIPT_PATH}`,
      {
        method: "POST",
        headers: headers(),
        body: "{}",
      },
    );
    const latencyMs = performance.now() - start;
    if (!resp.ok) throw new Error(`Trigger failed: ${resp.status} ${await resp.text()}`);
    const result = await resp.json();
    return { latencyMs, result };
  },

  async triggerBatch(n: number) {
    const start = performance.now();
    const promises = Array.from({ length: n }, () =>
      fetch(`${HOST}/api/w/${WORKSPACE}/jobs/run_wait_result/p/${SCRIPT_PATH}`, {
        method: "POST",
        headers: headers(),
        body: "{}",
      }).then((r) => r.json()),
    );
    const results = await Promise.all(promises);
    const totalMs = performance.now() - start;
    return { totalMs, results };
  },

  async teardown() {
    await composeDown(this.composeFile);
  },

  async getVersion() {
    const resp = await fetch(`${HOST}/api/version`);
    return (await resp.text()).trim();
  },
};
