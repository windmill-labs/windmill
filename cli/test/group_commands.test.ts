/**
 * Integration tests for `wmill group` commands:
 * list, get, create, delete, add-user, remove-user
 */

import { expect, test, describe } from "bun:test";
import { withTestBackend } from "./test_backend.ts";
import { setupWorkspaceProfile } from "./new_commands_helpers.ts";

describe("group command", () => {
  test("group list returns valid JSON", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const result = await backend.runCLICommand(
        ["group", "list", "--json"],
        tempDir
      );

      expect(result.code).toEqual(0);
      const parsed = JSON.parse(result.stdout);
      expect(Array.isArray(parsed)).toBe(true);
      expect(parsed.some((g: any) => g.name === "all")).toBe(true);
    });
  });

  test("group list shows table output", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const result = await backend.runCLICommand(
        ["group", "list"],
        tempDir
      );

      expect(result.code).toEqual(0);
      expect(result.stdout).toContain("Name");
      expect(result.stdout).toContain("Summary");
      expect(result.stdout).toContain("Members");
    });
  });

  test("group create + get + delete lifecycle", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const groupName = `cli_test_${Date.now()}`;

      // Create
      const createResult = await backend.runCLICommand(
        ["group", "create", groupName, "--summary", "CLI test group"],
        tempDir
      );
      expect(createResult.code).toEqual(0);
      expect(createResult.stdout).toContain("created");

      // Get
      const getResult = await backend.runCLICommand(
        ["group", "get", groupName],
        tempDir
      );
      expect(getResult.code).toEqual(0);
      expect(getResult.stdout).toContain(groupName);
      expect(getResult.stdout).toContain("CLI test group");

      // Get --json
      const getJsonResult = await backend.runCLICommand(
        ["group", "get", groupName, "--json"],
        tempDir
      );
      expect(getJsonResult.code).toEqual(0);
      const parsed = JSON.parse(getJsonResult.stdout);
      expect(parsed.name).toBe(groupName);

      // Delete
      const deleteResult = await backend.runCLICommand(
        ["group", "delete", groupName],
        tempDir
      );
      expect(deleteResult.code).toEqual(0);
      expect(deleteResult.stdout).toContain("deleted");
    });
  });

  test("group add-user and remove-user", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const groupName = `cli_member_test_${Date.now()}`;

      await backend.runCLICommand(["group", "create", groupName], tempDir);

      // Add user
      const addResult = await backend.runCLICommand(
        ["group", "add-user", groupName, "admin@windmill.dev"],
        tempDir
      );
      expect(addResult.code).toEqual(0);
      expect(addResult.stdout).toContain("added");

      // Remove user
      const removeResult = await backend.runCLICommand(
        ["group", "remove-user", groupName, "admin@windmill.dev"],
        tempDir
      );
      expect(removeResult.code).toEqual(0);
      expect(removeResult.stdout).toContain("removed");

      // Cleanup
      await backend.runCLICommand(["group", "delete", groupName], tempDir);
    });
  });

  test("default action (wmill group) lists groups", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const result = await backend.runCLICommand(["group", "--json"], tempDir);

      expect(result.code).toEqual(0);
      const parsed = JSON.parse(result.stdout);
      expect(Array.isArray(parsed)).toBe(true);
    });
  });

  test("group --help shows all subcommands", async () => {
    await withTestBackend(async (backend, tempDir) => {
      const result = await backend.runCLICommand(["group", "--help"], tempDir);
      const output = result.stdout + result.stderr;
      expect(output).toContain("list");
      expect(output).toContain("get");
      expect(output).toContain("create");
      expect(output).toContain("delete");
      expect(output).toContain("add-user");
      expect(output).toContain("remove-user");
    });
  });
});
