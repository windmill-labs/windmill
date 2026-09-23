import { expect, test, describe } from "bun:test";
import { withTestBackend } from "./test_backend.ts";
import { setupWorkspaceProfile, ensureFolder } from "./new_commands_helpers.ts";

describe("trash command", () => {
  test("lists, shows and restores a deleted variable", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);
      await ensureFolder(backend, "test");
      const ws = backend.workspace;
      const path = `f/test/trash_${Date.now()}`;
      const api = (route: string, init: RequestInit = {}) =>
        backend.apiRequest!(`/api/w/${ws}/${route}`, {
          headers: { "Content-Type": "application/json" },
          ...init,
        });

      let resp = await api("variables/create", {
        method: "POST",
        body: JSON.stringify({ path, value: "kept", is_secret: false, description: "" }),
      });
      expect(resp.status).toBeLessThan(300);
      await resp.text();
      resp = await api(`variables/delete/${path}`, { method: "DELETE" });
      expect(resp.status).toBeLessThan(300);
      await resp.text();

      const list = await backend.runCLICommand(
        ["trash", "list", "--json", "--kind", "variable"],
        tempDir
      );
      expect(list.code).toBe(0);
      const item = JSON.parse(list.stdout).find((i: any) => i.item_path === path);
      expect(item).toBeDefined();
      expect(item.item_kind).toBe("variable");

      const get = await backend.runCLICommand(
        ["trash", "get", "--json", String(item.id)],
        tempDir
      );
      expect(get.code).toBe(0);
      expect(JSON.parse(get.stdout).item_data.row.value).toBe("kept");

      // A bogus second id: the first restore must still go through, and the
      // failure must show in the exit code.
      const restore = await backend.runCLICommand(
        ["trash", "restore", String(item.id), "999999999"],
        tempDir
      );
      expect(restore.code).toBe(1);
      expect(restore.stdout).toContain(`variable '${path}' restored`);
      expect(restore.stderr).toContain("999999999");

      resp = await api(`variables/get/${path}`);
      expect(resp.status).toBe(200);
      expect((await resp.json()).value).toBe("kept");

      const after = await backend.runCLICommand(
        ["trash", "list", "--json", "--kind", "variable"],
        tempDir
      );
      expect(after.code).toBe(0);
      expect(JSON.parse(after.stdout).some((i: any) => i.item_path === path)).toBe(false);
    });
  });
});
