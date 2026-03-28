/**
 * Integration tests for `wmill job` commands:
 * list, get, result, logs, cancel
 */

import { expect, test, describe } from "bun:test";
import { withTestBackend } from "./test_backend.ts";
import {
  setupWorkspaceProfile,
  createRemoteScript,
  createRemoteFlow,
  runRemoteScript,
  runRemoteFlow,
  waitForJob,
} from "./new_commands_helpers.ts";

describe("job command", () => {
  test("job list returns valid JSON", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const uniqueId = Date.now();
      const scriptPath = `f/test/job_test_${uniqueId}`;
      await createRemoteScript(backend, scriptPath);
      const jobId = await runRemoteScript(backend, scriptPath);
      await waitForJob(backend, jobId);

      const result = await backend.runCLICommand(
        ["job", "list", "--json"],
        tempDir
      );

      expect(result.code).toEqual(0);
      const parsed = JSON.parse(result.stdout);
      expect(Array.isArray(parsed)).toBe(true);
      expect(parsed.some((j: any) => j.id === jobId)).toBe(true);
    });
  });

  test("job list shows table output", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const uniqueId = Date.now();
      const scriptPath = `f/test/job_table_${uniqueId}`;
      await createRemoteScript(backend, scriptPath);
      const jobId = await runRemoteScript(backend, scriptPath);
      await waitForJob(backend, jobId);

      const result = await backend.runCLICommand(
        ["job", "list"],
        tempDir
      );

      expect(result.code).toEqual(0);
      expect(result.stdout).toContain("ID");
      expect(result.stdout).toContain("Status");
      expect(result.stdout).toContain(jobId);
    });
  });

  test("job list --script-path filters correctly", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const uniqueId = Date.now();
      const scriptPath = `f/test/filter_test_${uniqueId}`;
      await createRemoteScript(backend, scriptPath);
      const jobId = await runRemoteScript(backend, scriptPath);
      await waitForJob(backend, jobId);

      const result = await backend.runCLICommand(
        ["job", "list", "--json", "--script-path", scriptPath],
        tempDir
      );

      expect(result.code).toEqual(0);
      const parsed = JSON.parse(result.stdout);
      expect(parsed.every((j: any) => j.script_path === scriptPath)).toBe(true);
    });
  });

  test("job get returns job details", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const uniqueId = Date.now();
      const scriptPath = `f/test/job_get_${uniqueId}`;
      await createRemoteScript(backend, scriptPath);
      const jobId = await runRemoteScript(backend, scriptPath);
      await waitForJob(backend, jobId);

      const result = await backend.runCLICommand(
        ["job", "get", jobId],
        tempDir
      );

      expect(result.code).toEqual(0);
      expect(result.stdout).toContain("ID:");
      expect(result.stdout).toContain(jobId);
      expect(result.stdout).toContain("Status:");
      expect(result.stdout).toContain("success");
    });
  });

  test("job get --json returns valid JSON", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const uniqueId = Date.now();
      const scriptPath = `f/test/job_get_json_${uniqueId}`;
      await createRemoteScript(backend, scriptPath);
      const jobId = await runRemoteScript(backend, scriptPath);
      await waitForJob(backend, jobId);

      const result = await backend.runCLICommand(
        ["job", "get", jobId, "--json"],
        tempDir
      );

      expect(result.code).toEqual(0);
      const parsed = JSON.parse(result.stdout);
      expect(parsed.id).toBe(jobId);
      expect(parsed.success).toBe(true);
    });
  });

  test("job result returns job result as JSON", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const uniqueId = Date.now();
      const scriptPath = `f/test/job_result_${uniqueId}`;
      await createRemoteScript(backend, scriptPath);
      const jobId = await runRemoteScript(backend, scriptPath);
      await waitForJob(backend, jobId);

      const result = await backend.runCLICommand(
        ["job", "result", jobId],
        tempDir
      );

      expect(result.code).toEqual(0);
      // Result may be in stdout or combined output
      const output = result.stdout.trim();
      expect(output.length).toBeGreaterThan(0);
      const parsed = JSON.parse(output);
      expect(parsed).toBe("hello");
    });
  });

  test("job logs returns job logs", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const uniqueId = Date.now();
      const scriptPath = `f/test/job_logs_${uniqueId}`;
      await createRemoteScript(backend, scriptPath);
      const jobId = await runRemoteScript(backend, scriptPath);
      await waitForJob(backend, jobId);

      const result = await backend.runCLICommand(
        ["job", "logs", jobId],
        tempDir
      );

      expect(result.code).toEqual(0);
      expect(result.stdout.length).toBeGreaterThan(0);
    });
  });

  test("job logs for flow job shows helpful message", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const uniqueId = Date.now();
      const flowPath = `f/test/flow_logs_${uniqueId}`;
      await createRemoteFlow(backend, flowPath);
      const jobId = await runRemoteFlow(backend, flowPath);
      await waitForJob(backend, jobId);

      const result = await backend.runCLICommand(
        ["job", "logs", jobId],
        tempDir
      );

      expect(result.code).toEqual(0);
      expect(result.stdout).toContain("Flow jobs don't have direct logs");
    });
  });

  test("default action (wmill job) lists jobs", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const result = await backend.runCLICommand(
        ["job", "--json"],
        tempDir
      );

      expect(result.code).toEqual(0);
      const parsed = JSON.parse(result.stdout);
      expect(Array.isArray(parsed)).toBe(true);
    });
  });

  test("job --help shows all subcommands", async () => {
    await withTestBackend(async (backend, tempDir) => {
      const result = await backend.runCLICommand(["job", "--help"], tempDir);
      const output = result.stdout + result.stderr;
      expect(output).toContain("list");
      expect(output).toContain("get");
      expect(output).toContain("result");
      expect(output).toContain("logs");
      expect(output).toContain("cancel");
      expect(output).toContain("--failed");
      expect(output).toContain("--running");
    });
  });
});
