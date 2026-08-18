/**
 * Runtime settings that live only in the script metadata file (the retention
 * delay, the debouncing bounds, the cache s3-path flag) must survive a sync
 * pull/push cycle. A field missing from the create_script body the CLI builds
 * lands as NULL on the deployed version; one missing from its up-to-date
 * comparison makes a change to it alone report as up to date and never deploy.
 */

import { expect, test } from "bun:test";
import { writeFile, readFile, mkdir } from "node:fs/promises";
import { withTestBackend } from "./test_backend.ts";

const SETTINGS = {
  delete_after_secs: 900,
  cache_ignore_s3_path: true,
  debounce_key: "dk",
  debounce_delay_s: 5,
  debounce_args_to_accumulate: ["a"],
  max_total_debouncing_time: 60,
  max_total_debounces_amount: 5,
};

test("Integration: script runtime settings survive a sync pull/push cycle", async () => {
  await withTestBackend(async (backend, tempDir) => {
    const uniqueId = Date.now();
    const scriptPath = `f/test/settings_${uniqueId}`;
    const getScript = async () => {
      const resp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/scripts/get/p/${scriptPath}`,
      );
      expect(resp.ok).toEqual(true);
      return await resp.json();
    };

    await mkdir(`${tempDir}/f/test`, { recursive: true });
    await backend.apiRequest!(`/api/w/${backend.workspace}/folders/create`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ name: "test" }),
    });

    const createResp = await backend.apiRequest!(
      `/api/w/${backend.workspace}/scripts/create`,
      {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          path: scriptPath,
          content: `export async function main() {\n  return "Hello world";\n}`,
          summary: "Test runtime settings",
          description: "",
          language: "bun",
          kind: "script",
          schema: {
            $schema: "https://json-schema.org/draft/2020-12/schema",
            type: "object",
            properties: {},
            required: [],
          },
          // An unset priority never compares equal to the remote's null in the
          // CLI's up-to-date check, which would keep phase 2 from ever reaching it.
          priority: 1,
          ...SETTINGS,
        }),
      },
    );
    expect(createResp.ok).toEqual(true);

    await writeFile(
      `${tempDir}/wmill.yaml`,
      `defaultTs: bun\nincludes:\n  - "${scriptPath}**"\nexcludes: []\n`,
      "utf-8",
    );

    const pullResult = await backend.runCLICommand(["sync", "pull", "--yes"], tempDir);
    expect(pullResult.code).toEqual(0);

    const metadataPath = `${tempDir}/${scriptPath}.script.yaml`;
    const pulledMetadata = await readFile(metadataPath, "utf-8");
    for (const key of Object.keys(SETTINGS)) {
      expect(pulledMetadata).toContain(key);
    }

    // A content-only edit must carry the settings through to the new version.
    const scriptFilePath = `${tempDir}/${scriptPath}.ts`;
    const originalContent = await readFile(scriptFilePath, "utf-8");
    await writeFile(
      scriptFilePath,
      originalContent.replace("Hello world", "Hello world modified"),
      "utf-8",
    );
    expect((await backend.runCLICommand(["sync", "push", "--yes"], tempDir)).code).toEqual(0);

    const afterContentPush = await getScript();
    expect(afterContentPush.content).toContain("Hello world modified");
    for (const [key, value] of Object.entries(SETTINGS)) {
      expect(afterContentPush[key]).toEqual(value);
    }

    // A settings-only edit must reach the remote rather than be skipped as up to
    // date. 0 is "delete immediately after completion", not "unset".
    await writeFile(
      metadataPath,
      (await readFile(metadataPath, "utf-8")).replace(
        `delete_after_secs: ${SETTINGS.delete_after_secs}`,
        "delete_after_secs: 0",
      ),
      "utf-8",
    );
    expect((await backend.runCLICommand(["sync", "push", "--yes"], tempDir)).code).toEqual(0);

    expect((await getScript()).delete_after_secs).toEqual(0);
  });
});
