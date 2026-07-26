/**
 * Temporal CompetitorAdapter.
 *
 * Since the Temporal TypeScript SDK requires Node.js native bindings (gRPC),
 * we can't import it directly in Deno. Instead we spawn a Node.js subprocess
 * that connects to Temporal, starts a workflow, waits for the result, and
 * prints JSON to stdout.
 *
 * The worker itself runs inside Docker (Dockerfile.worker).
 */

import type { CompetitorAdapter } from "../types.ts";
import { composeUp, composeDown } from "../lib/docker.ts";
import { dirname, fromFileUrl, resolve } from "https://deno.land/std@0.224.0/path/mod.ts";

const SELF_DIR = dirname(fromFileUrl(import.meta.url));
const TEMPORAL_ADDRESS = "127.0.0.1:7233";

async function nodeEval(script: string): Promise<string> {
  const cmd = new Deno.Command("node", {
    args: ["-e", script],
    cwd: resolve(SELF_DIR),
    stdout: "piped",
    stderr: "piped",
    env: {
      ...Deno.env.toObject(),
      TEMPORAL_ADDRESS,
    },
  });
  const output = await cmd.output();
  if (output.code !== 0) {
    const stderr = new TextDecoder().decode(output.stderr);
    throw new Error(`Temporal node call failed:\n${stderr}`);
  }
  return new TextDecoder().decode(output.stdout).trim();
}

async function triggerWorkflow(): Promise<unknown> {
  const script = `
    const { Client, Connection } = require("@temporalio/client");
    (async () => {
      const conn = await Connection.connect({ address: "${TEMPORAL_ADDRESS}" });
      const client = new Client({ connection: conn });
      const handle = await client.workflow.start("benchmarkWorkflow", {
        taskQueue: "benchmark",
        workflowId: "bench-" + require("crypto").randomUUID(),
      });
      const result = await handle.result();
      console.log(JSON.stringify(result));
      process.exit(0);
    })();
  `;
  const out = await nodeEval(script);
  return JSON.parse(out);
}

export const temporalAdapter: CompetitorAdapter = {
  name: "temporal",
  composeFile: resolve(SELF_DIR, "docker-compose.yml"),

  async setup() {
    // Install npm deps locally first (for the client subprocess)
    console.log("[temporal] Installing npm deps...");
    const install = new Deno.Command("npm", {
      args: ["install"],
      cwd: resolve(SELF_DIR),
      stdout: "piped",
      stderr: "piped",
    });
    const { code } = await install.output();
    if (code !== 0) throw new Error("npm install failed in temporal/");

    await composeUp(this.composeFile);
    // Wait for Temporal server to be ready (auto-setup takes time for DB schema)
    console.log("[temporal] Waiting for Temporal server to be ready...");
    for (let i = 0; i < 60; i++) {
      try {
        const script = `
          const { Connection } = require("@temporalio/client");
          (async () => {
            const conn = await Connection.connect({ address: "${TEMPORAL_ADDRESS}" });
            console.log("ok");
            process.exit(0);
          })();
        `;
        const result = await nodeEval(script);
        if (result === "ok") break;
      } catch (_) {
        // not ready yet
      }
      await new Promise((r) => setTimeout(r, 3000));
    }
    // Give the worker time to register after server is up
    await new Promise((r) => setTimeout(r, 5000));
  },

  async deployWorkflow() {
    // The worker auto-registers the workflow on startup.
    // Verify connectivity.
    const script = `
      const { Connection } = require("@temporalio/client");
      (async () => {
        const conn = await Connection.connect({ address: "${TEMPORAL_ADDRESS}" });
        console.log("connected");
        process.exit(0);
      })();
    `;
    await nodeEval(script);
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
      const script = `
        const { Connection } = require("@temporalio/client");
        (async () => {
          const conn = await Connection.connect({ address: "${TEMPORAL_ADDRESS}" });
          console.log("temporal");
          process.exit(0);
        })();
      `;
      return await nodeEval(script);
    } catch (_) {
      return "unknown";
    }
  },
};
