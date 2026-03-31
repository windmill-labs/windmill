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
  createRemoteMultiStepFlow,
  createRemoteFailingFlow,
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

  test("job logs for flow job aggregates step logs", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const uniqueId = Date.now();
      const flowPath = `f/test/flow_logs_${uniqueId}`;
      await createRemoteMultiStepFlow(backend, flowPath);
      const jobId = await runRemoteFlow(backend, flowPath);
      await waitForJob(backend, jobId);

      const result = await backend.runCLICommand(
        ["job", "logs", jobId],
        tempDir
      );

      expect(result.code).toEqual(0);
      // Should show labeled step headers instead of "no direct logs"
      expect(result.stdout).toContain("======");
      expect(result.stdout).toContain("a");
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
      expect(output).toContain("--parent");
      expect(output).toContain("--is-flow-step");
    });
  });

  test("job get for flow shows hierarchical step tree", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const uniqueId = Date.now();
      const flowPath = `f/test/flow_get_${uniqueId}`;
      await createRemoteMultiStepFlow(backend, flowPath);
      const jobId = await runRemoteFlow(backend, flowPath);
      await waitForJob(backend, jobId);

      const result = await backend.runCLICommand(
        ["job", "get", jobId],
        tempDir
      );

      expect(result.code).toEqual(0);
      expect(result.stdout).toContain("Steps:");
      // Should show step IDs from the flow definition
      expect(result.stdout).toContain("a");
      expect(result.stdout).toContain("b");
      // Should show status icons (✓ for success)
      expect(result.stdout).toContain("✓");
    });
  });

  test("job get --json for flow includes flow_status", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const uniqueId = Date.now();
      const flowPath = `f/test/flow_json_${uniqueId}`;
      await createRemoteMultiStepFlow(backend, flowPath);
      const jobId = await runRemoteFlow(backend, flowPath);
      await waitForJob(backend, jobId);

      const result = await backend.runCLICommand(
        ["job", "get", jobId, "--json"],
        tempDir
      );

      expect(result.code).toEqual(0);
      const parsed = JSON.parse(result.stdout);
      expect(parsed.flow_status).toBeDefined();
      expect(parsed.flow_status.modules).toBeDefined();
      expect(parsed.flow_status.modules.length).toBe(2);
    });
  });

  test("job list --parent shows sub-jobs of a flow", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const uniqueId = Date.now();
      const flowPath = `f/test/flow_parent_${uniqueId}`;
      await createRemoteMultiStepFlow(backend, flowPath);
      const jobId = await runRemoteFlow(backend, flowPath);
      await waitForJob(backend, jobId);

      const result = await backend.runCLICommand(
        ["job", "list", "--json", "--parent", jobId],
        tempDir
      );

      expect(result.code).toEqual(0);
      const parsed = JSON.parse(result.stdout);
      expect(Array.isArray(parsed)).toBe(true);
      // A 2-step flow should have at least 2 sub-jobs
      expect(parsed.length).toBeGreaterThanOrEqual(2);
      // All sub-jobs should reference the parent flow
      expect(parsed.every((j: any) => j.parent_job === jobId)).toBe(true);
    });
  });

  test("job list --all includes sub-jobs", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const uniqueId = Date.now();
      const flowPath = `f/test/flow_all_${uniqueId}`;
      await createRemoteMultiStepFlow(backend, flowPath);
      const jobId = await runRemoteFlow(backend, flowPath);
      await waitForJob(backend, jobId);

      const result = await backend.runCLICommand(
        ["job", "list", "--json", "--all"],
        tempDir
      );

      expect(result.code).toEqual(0);
      const parsed = JSON.parse(result.stdout);
      // Should contain both the parent flow and its sub-jobs
      const parentJob = parsed.find((j: any) => j.id === jobId);
      const subJobs = parsed.filter((j: any) => j.parent_job === jobId);
      expect(parentJob).toBeDefined();
      expect(subJobs.length).toBeGreaterThanOrEqual(2);
    });
  });

  test("job get for failed flow shows failure status", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const uniqueId = Date.now();
      const flowPath = `f/test/flow_fail_${uniqueId}`;
      await createRemoteFailingFlow(backend, flowPath);
      const jobId = await runRemoteFlow(backend, flowPath);
      await waitForJob(backend, jobId);

      const result = await backend.runCLICommand(
        ["job", "get", jobId],
        tempDir
      );

      expect(result.code).toEqual(0);
      expect(result.stdout).toContain("failure");
      expect(result.stdout).toContain("Steps:");
      // Step a should succeed, step b should fail
      expect(result.stdout).toContain("✓");
      expect(result.stdout).toContain("✗");
    });
  });
});
