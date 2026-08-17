/**
 * Integration test for `wmill app dev`'s schema-fetch path.
 *
 * Verifies that path-based runnables (type: script) get a real schema in the
 * generated wmill.d.ts after fetchPathRunnableSchemas() pulls it from the
 * Windmill API. Without this fix the runnable would always render as
 * `args: {}` because the local YAML doesn't carry the schema.
 */

import { expect, test } from "bun:test";
import { withTestBackend } from "./test_backend.ts";
import { addWorkspace } from "../workspace.ts";
import * as fs from "node:fs";
import * as path from "node:path";
import { mkdir, writeFile } from "node:fs/promises";
import { setClient } from "../src/core/client.ts";
import { APP_BACKEND_FOLDER } from "../src/commands/app/app_metadata.ts";
import { loadRunnablesFromBackend } from "../src/commands/app/raw_apps.ts";
import {
  buildWmillTs,
  fetchPathRunnableSchemas,
} from "../src/commands/app/dev.ts";

const TYPED_SCRIPT_CONTENT =
  `export async function main(userId: string, includeProfile: boolean) {\n` +
  `  return { userId, includeProfile };\n` +
  `}\n`;

test("fetchPathRunnableSchemas pulls schema for a path-based script and feeds wmill.d.ts", async () => {
  await withTestBackend(async (backend, tempDir) => {
    // Bind CLI workspace + arm the OpenAPI client
    await addWorkspace(
      {
        remote: backend.baseUrl,
        workspaceId: backend.workspace,
        name: "app_dev_schema_fetch",
        token: backend.token,
      },
      { force: true, configDir: backend.testConfigDir },
    );
    setClient(backend.token, backend.baseUrl);

    // Create the folder + script that the raw app will reference by path
    await backend
      .apiRequest!(`/api/w/${backend.workspace}/folders/create`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name: "test" }),
      })
      .then((r) => r.text());

    const scriptPath = "f/test/fetch_user";
    const createResp = await backend.apiRequest!(
      `/api/w/${backend.workspace}/scripts/create`,
      {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          path: scriptPath,
          content: TYPED_SCRIPT_CONTENT,
          language: "bun",
          summary: "Fetch user",
          description: "Test script with typed args",
          schema: {
            $schema: "https://json-schema.org/draft/2020-12/schema",
            type: "object",
            properties: {
              userId: { type: "string", description: "user id" },
              includeProfile: { type: "boolean" },
            },
            required: ["userId", "includeProfile"],
          },
        }),
      },
    );
    expect(createResp.status).toBeLessThan(300);

    // Lay out a raw app with a backend folder that references the path script
    const appDir = path.join(tempDir, "f", "test", "schema_fetch_app.raw_app");
    const backendDir = path.join(appDir, APP_BACKEND_FOLDER);
    await mkdir(backendDir, { recursive: true });
    await writeFile(
      path.join(backendDir, "fetchUser.yaml"),
      `type: script\npath: ${scriptPath}\nfields: {}\nname: fetchUser\n`,
      "utf-8",
    );

    // Drive the dev pipeline directly: load -> enrich from API -> render
    const runnables = await loadRunnablesFromBackend(backendDir);
    expect(runnables.fetchUser?.type).toBe("path");
    expect(runnables.fetchUser?.runType).toBe("script");
    expect(runnables.fetchUser?.schema).toBeUndefined();

    const pathSchemas = await fetchPathRunnableSchemas(
      backend.workspace,
      runnables,
    );
    expect(pathSchemas.fetchUser).toBeDefined();
    expect(pathSchemas.fetchUser.properties.userId).toBeDefined();
    expect(pathSchemas.fetchUser.properties.includeProfile).toBeDefined();

    const ts = buildWmillTs(runnables, {}, pathSchemas);

    // Without the fix the line would be `fetchUser: (args: {}) => ...`
    expect(ts).toContain("userId");
    expect(ts).toContain("includeProfile");
    expect(ts).toMatch(/fetchUser:\s*\(args:\s*\{[^}]*userId/);
    expect(ts).not.toMatch(/fetchUser:\s*\(args:\s*\{\s*\}\)/);

    // Sanity: the generated d.ts is syntactically what the dev server writes
    const outPath = path.join(tempDir, "wmill.d.ts.out");
    fs.writeFileSync(outPath, ts);
    expect(fs.statSync(outPath).size).toBeGreaterThan(0);
  });
});
