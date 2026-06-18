/**
 * Integration tests for `wmill audit` and `wmill token` commands.
 */

import { expect, test, describe } from "bun:test";
import { withTestBackend } from "./test_backend.ts";
import { setupWorkspaceProfile, createRemoteScript } from "./new_commands_helpers.ts";

// =============================================================================
// audit commands
// =============================================================================

describe("audit command", () => {
  test("audit list returns valid JSON", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      await createRemoteScript(backend, `f/test/audit_test_${Date.now()}`);

      const result = await backend.runCLICommand(
        ["audit", "list", "--json"],
        tempDir
      );

      expect(result.code).toEqual(0);
      const parsed = JSON.parse(result.stdout);
      expect(Array.isArray(parsed)).toBe(true);
    });
  });

  test("audit list with filters", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      await createRemoteScript(backend, `f/test/audit_filter_${Date.now()}`);

      const result = await backend.runCLICommand(
        ["audit", "list", "--json", "--operation", "scripts", "--limit", "5"],
        tempDir
      );

      expect(result.code).toEqual(0);
      const parsed = JSON.parse(result.stdout);
      expect(Array.isArray(parsed)).toBe(true);
      if (parsed.length > 0) {
        expect(parsed[0].operation).toMatch(/^scripts/);
      }
    });
  });

  test("audit list shows table or empty message", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      await createRemoteScript(backend, `f/test/audit_table_${Date.now()}`);

      const result = await backend.runCLICommand(["audit", "list"], tempDir);

      expect(result.code).toEqual(0);
      const output = result.stdout;
      const hasTable = output.includes("ID") && output.includes("Operation");
      const hasEmpty = output.includes("No audit logs found");
      expect(hasTable || hasEmpty).toBe(true);
    });
  });

  test("audit get returns specific entry", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      await createRemoteScript(backend, `f/test/audit_get_${Date.now()}`);

      const listResult = await backend.runCLICommand(
        ["audit", "list", "--json", "--limit", "1"],
        tempDir
      );
      expect(listResult.code).toEqual(0);
      const logs = JSON.parse(listResult.stdout);
      if (logs.length === 0) return;

      const auditId = String(logs[0].id);
      const getResult = await backend.runCLICommand(
        ["audit", "get", auditId, "--json"],
        tempDir
      );

      expect(getResult.code).toEqual(0);
      const parsed = JSON.parse(getResult.stdout);
      expect(parsed.id).toBe(logs[0].id);
    });
  });

  test("audit --help shows all subcommands", async () => {
    await withTestBackend(async (backend, tempDir) => {
      const result = await backend.runCLICommand(["audit", "--help"], tempDir);
      const output = result.stdout + result.stderr;
      expect(output).toContain("list");
      expect(output).toContain("get");
    });
  });
});

// =============================================================================
// token commands
// =============================================================================

describe("token command", () => {
  test("token list returns valid JSON", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const result = await backend.runCLICommand(
        ["token", "list", "--json"],
        tempDir
      );

      expect(result.code).toEqual(0);
      const parsed = JSON.parse(result.stdout);
      expect(Array.isArray(parsed)).toBe(true);
    });
  });

  test("token list shows table output", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const result = await backend.runCLICommand(["token", "list"], tempDir);

      expect(result.code).toEqual(0);
      expect(result.stdout).toContain("Prefix");
      expect(result.stdout).toContain("Label");
    });
  });

  test("token create + delete lifecycle", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      // Create
      const createResult = await backend.runCLICommand(
        ["token", "create", "--label", "cli-test-token"],
        tempDir
      );
      expect(createResult.code).toEqual(0);
      const newToken = createResult.stdout.trim();
      expect(newToken.length).toBeGreaterThan(10);

      // List and find it
      const listResult = await backend.runCLICommand(
        ["token", "list", "--json"],
        tempDir
      );
      expect(listResult.code).toEqual(0);
      const tokens = JSON.parse(listResult.stdout);
      const found = tokens.find((t: any) => t.label === "cli-test-token");
      expect(found).toBeDefined();

      // Delete
      const deleteResult = await backend.runCLICommand(
        ["token", "delete", found.token_prefix],
        tempDir
      );
      expect(deleteResult.code).toEqual(0);
      expect(deleteResult.stdout).toContain("deleted");
    });
  });

  test("default action (wmill token) lists tokens", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const result = await backend.runCLICommand(["token", "--json"], tempDir);

      expect(result.code).toEqual(0);
      const parsed = JSON.parse(result.stdout);
      expect(Array.isArray(parsed)).toBe(true);
    });
  });

  test("token --help shows all subcommands", async () => {
    await withTestBackend(async (backend, tempDir) => {
      const result = await backend.runCLICommand(["token", "--help"], tempDir);
      const output = result.stdout + result.stderr;
      expect(output).toContain("list");
      expect(output).toContain("create");
      expect(output).toContain("delete");
    });
  });
});
