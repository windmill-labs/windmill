/**
 * Shared helpers for new CLI command tests.
 */

import { expect } from "bun:test";
import { type TestBackend } from "./test_backend.ts";
import { addWorkspace } from "../workspace.ts";

export async function setupWorkspaceProfile(backend: TestBackend): Promise<void> {
  await addWorkspace(
    {
      remote: backend.baseUrl,
      workspaceId: backend.workspace,
      name: "localhost_test",
      token: backend.token!,
    },
    { force: true, configDir: backend.testConfigDir }
  );
}

export async function ensureFolder(backend: TestBackend, name: string): Promise<void> {
  const resp = await backend.apiRequest!(
    `/api/w/${backend.workspace}/folders/create`,
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ name }),
    }
  );
  await resp.text();
}

export async function createRemoteScript(
  backend: TestBackend,
  scriptPath: string,
  content: string = 'export async function main() { return "hello"; }'
): Promise<void> {
  const parts = scriptPath.split("/");
  if (parts[0] === "f" && parts.length > 2) {
    await ensureFolder(backend, parts[1]);
  }

  const resp = await backend.apiRequest!(
    `/api/w/${backend.workspace}/scripts/create`,
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        path: scriptPath,
        content,
        language: "bun",
        summary: "Test script",
        schema: {
          $schema: "https://json-schema.org/draft/2020-12/schema",
          type: "object",
          properties: {},
          required: [],
        },
      }),
    }
  );
  expect(resp.status).toBeLessThan(300);
  await resp.text();
}

export async function runRemoteScript(
  backend: TestBackend,
  scriptPath: string,
  retries: number = 10
): Promise<string> {
  for (let i = 0; i < retries; i++) {
    const resp = await backend.apiRequest!(
      `/api/w/${backend.workspace}/jobs/run/p/${scriptPath}`,
      {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({}),
      }
    );
    if (resp.status < 300) {
      return (await resp.text()).replace(/"/g, "");
    }
    await resp.text();
    if (i < retries - 1) {
      await new Promise((r) => setTimeout(r, 1000));
    }
  }
  throw new Error(`Failed to run script ${scriptPath} after ${retries} retries`);
}

export async function waitForJob(
  backend: TestBackend,
  jobId: string,
  timeoutMs: number = 15000
): Promise<void> {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    const resp = await backend.apiRequest!(
      `/api/w/${backend.workspace}/jobs_u/completed/get/${jobId}`
    );
    if (resp.ok) return;
    await new Promise((r) => setTimeout(r, 500));
  }
  throw new Error(`Job ${jobId} did not complete within ${timeoutMs}ms`);
}

export async function createRemoteFlow(
  backend: TestBackend,
  flowPath: string
): Promise<void> {
  const parts = flowPath.split("/");
  if (parts[0] === "f" && parts.length > 2) {
    await ensureFolder(backend, parts[1]);
  }

  const resp = await backend.apiRequest!(
    `/api/w/${backend.workspace}/flows/create`,
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        path: flowPath,
        summary: "Test flow",
        description: "A test flow",
        value: {
          modules: [
            {
              id: "a",
              value: {
                type: "rawscript",
                content: 'export async function main() { return "flow done"; }',
                language: "bun",
                input_transforms: {},
              },
            },
          ],
        },
        schema: {
          $schema: "https://json-schema.org/draft/2020-12/schema",
          type: "object",
          properties: {},
          required: [],
        },
      }),
    }
  );
  expect(resp.status).toBeLessThan(300);
  await resp.text();
}

export async function createRemoteSchedule(
  backend: TestBackend,
  schedulePath: string,
  scriptPath: string
): Promise<void> {
  const parts = schedulePath.split("/");
  if (parts[0] === "f" && parts.length > 2) {
    await ensureFolder(backend, parts[1]);
  }

  const resp = await backend.apiRequest!(
    `/api/w/${backend.workspace}/schedules/create`,
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        path: schedulePath,
        schedule: "0 0 */6 * * *",
        timezone: "Etc/UTC",
        script_path: scriptPath,
        is_flow: false,
        args: {},
        enabled: false,
      }),
    }
  );
  expect(resp.status).toBeLessThan(300);
  await resp.text();
}
