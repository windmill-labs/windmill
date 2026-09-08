import { expect, test } from "bun:test";
import { readFile, writeFile } from "node:fs/promises";
import { withTestBackend } from "./test_backend.ts";
import { addWorkspace } from "../workspace.ts";
import { setClient } from "../src/core/client.ts";

// Regression guard for WIN-2201: with syncBehavior v1 the pull strips
// `on_behalf_of_email` and keeps only a boolean marker. That marker must be
// emitted ONLY when an owner exists — writing `has_on_behalf_of: false` for the
// (overwhelmingly common) ownerless script produced a spurious diff on every pull.
test("pull omits has_on_behalf_of for ownerless scripts", async () => {
  await withTestBackend(async (backend, tempDir) => {
    await addWorkspace(
      {
        remote: backend.baseUrl,
        workspaceId: backend.workspace,
        name: "obo_marker_test",
        token: backend.token,
      },
      { force: true, configDir: backend.testConfigDir },
    );
    setClient(backend.token, backend.baseUrl);

    await writeFile(
      `${tempDir}/wmill.yaml`,
      `defaultTs: bun\nsyncBehavior: v1\nincludes:\n  - "**"\nexcludes: []`,
      "utf-8",
    );

    const scriptPath = "f/test/no_owner";
    const createResp = await backend.apiRequest!(
      `/api/w/${backend.workspace}/scripts/create`,
      {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          path: scriptPath,
          content: `export async function main() { return 1; }`,
          language: "bun",
          summary: "no owner",
          description: "",
        }),
      },
    );
    expect(createResp.status).toBeLessThan(300);

    const pullResult = await backend.runCLICommand(
      ["sync", "pull", "--yes"],
      tempDir,
      "obo_marker_test",
    );
    expect(pullResult.code).toEqual(0);

    const meta = await readFile(`${tempDir}/${scriptPath}.script.yaml`, "utf-8");
    expect(meta).not.toContain("has_on_behalf_of");
    expect(meta).not.toContain("on_behalf_of_email");
  });
});
