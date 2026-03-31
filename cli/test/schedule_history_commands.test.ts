/**
 * Integration tests for:
 * - `wmill schedule enable/disable`
 * - `wmill script history`
 * - `wmill flow history/show-version`
 */

import { expect, test, describe } from "bun:test";
import { withTestBackend } from "./test_backend.ts";
import {
  setupWorkspaceProfile,
  createRemoteScript,
  createRemoteFlow,
  createRemoteSchedule,
} from "./new_commands_helpers.ts";

// =============================================================================
// schedule enable/disable commands
// =============================================================================

describe("schedule enable/disable", () => {
  test("schedule enable and disable toggle schedule state", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const uniqueId = Date.now();
      const scriptPath = `f/test/sched_script_${uniqueId}`;
      const schedulePath = `f/test/sched_${uniqueId}`;

      await createRemoteScript(backend, scriptPath);
      await createRemoteSchedule(backend, schedulePath, scriptPath);

      // Enable
      const enableResult = await backend.runCLICommand(
        ["schedule", "enable", schedulePath],
        tempDir
      );
      expect(enableResult.code).toEqual(0);
      expect(enableResult.stdout).toContain("enabled");

      // Verify enabled via get
      const getResult1 = await backend.runCLICommand(
        ["schedule", "get", schedulePath, "--json"],
        tempDir
      );
      expect(getResult1.code).toEqual(0);
      const schedule1 = JSON.parse(getResult1.stdout);
      expect(schedule1.enabled).toBe(true);

      // Disable
      const disableResult = await backend.runCLICommand(
        ["schedule", "disable", schedulePath],
        tempDir
      );
      expect(disableResult.code).toEqual(0);
      expect(disableResult.stdout).toContain("disabled");

      // Verify disabled via get
      const getResult2 = await backend.runCLICommand(
        ["schedule", "get", schedulePath, "--json"],
        tempDir
      );
      expect(getResult2.code).toEqual(0);
      const schedule2 = JSON.parse(getResult2.stdout);
      expect(schedule2.enabled).toBe(false);
    });
  });

  test("schedule enable/disable shows in help", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const result = await backend.runCLICommand(
        ["schedule", "--help"],
        tempDir
      );

      expect(result.code).toEqual(0);
      const output = result.stdout + result.stderr;
      expect(output).toContain("enable");
      expect(output).toContain("disable");
    });
  });
});

// =============================================================================
// script history command
// =============================================================================

describe("script history", () => {
  test("script history returns version list", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const uniqueId = Date.now();
      const scriptPath = `f/test/history_script_${uniqueId}`;
      await createRemoteScript(backend, scriptPath);

      const result = await backend.runCLICommand(
        ["script", "history", scriptPath, "--json"],
        tempDir
      );

      expect(result.code).toEqual(0);
      const parsed = JSON.parse(result.stdout);
      expect(Array.isArray(parsed)).toBe(true);
      expect(parsed.length).toBeGreaterThanOrEqual(1);
      expect(parsed[0]).toHaveProperty("script_hash");
    });
  });

  test("script history shows table output", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const uniqueId = Date.now();
      const scriptPath = `f/test/history_table_${uniqueId}`;
      await createRemoteScript(backend, scriptPath);

      const result = await backend.runCLICommand(
        ["script", "history", scriptPath],
        tempDir
      );

      expect(result.code).toEqual(0);
      expect(result.stdout).toContain("Hash");
      expect(result.stdout).toContain("Deployment Message");
    });
  });
});

// =============================================================================
// flow history and show-version commands
// =============================================================================

describe("flow history", () => {
  test("flow history returns version list", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const uniqueId = Date.now();
      const flowPath = `f/test/history_flow_${uniqueId}`;
      await createRemoteFlow(backend, flowPath);

      const result = await backend.runCLICommand(
        ["flow", "history", flowPath, "--json"],
        tempDir
      );

      expect(result.code).toEqual(0);
      const parsed = JSON.parse(result.stdout);
      expect(Array.isArray(parsed)).toBe(true);
      expect(parsed.length).toBeGreaterThanOrEqual(1);
      expect(parsed[0]).toHaveProperty("id");
      expect(parsed[0]).toHaveProperty("created_at");
    });
  });

  test("flow history shows table output", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const uniqueId = Date.now();
      const flowPath = `f/test/history_table_flow_${uniqueId}`;
      await createRemoteFlow(backend, flowPath);

      const result = await backend.runCLICommand(
        ["flow", "history", flowPath],
        tempDir
      );

      expect(result.code).toEqual(0);
      expect(result.stdout).toContain("Version");
      expect(result.stdout).toContain("Created At");
    });
  });

  test("flow show-version returns specific version", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const uniqueId = Date.now();
      const flowPath = `f/test/show_ver_flow_${uniqueId}`;
      await createRemoteFlow(backend, flowPath);

      const histResult = await backend.runCLICommand(
        ["flow", "history", flowPath, "--json"],
        tempDir
      );
      expect(histResult.code).toEqual(0);
      const versions = JSON.parse(histResult.stdout);
      expect(versions.length).toBeGreaterThanOrEqual(1);

      const versionId = String(versions[0].id);

      const showResult = await backend.runCLICommand(
        ["flow", "show-version", flowPath, versionId, "--json"],
        tempDir
      );

      expect(showResult.code).toEqual(0);
      const flow = JSON.parse(showResult.stdout);
      expect(flow.path).toBe(flowPath);
      expect(flow.value).toBeDefined();
    });
  });
});
