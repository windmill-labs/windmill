/**
 * `wmill script push` short-circuits when the local script already matches the
 * remote. The comparison has to hold in both directions: an untouched script
 * deploys nothing, and every field the push body carries (labels and the language
 * inferred from defaultTs included) still counts as a change.
 */

import { expect, test } from "bun:test";
import { writeFile, readFile, mkdir } from "node:fs/promises";
import { withTestBackend } from "./test_backend.ts";

test("Integration: script push skips an unchanged script and deploys a changed one", async () => {
  await withTestBackend(async (backend, tempDir) => {
    const uniqueId = Date.now();
    const scriptPath = `f/test/uptodate_${uniqueId}`;
    const getScript = async () =>
      await (
        await backend.apiRequest!(
          `/api/w/${backend.workspace}/scripts/get/p/${scriptPath}`,
        )
      ).json();
    const push = async () =>
      await backend.runCLICommand(["script", "push", `${scriptPath}.ts`], tempDir);
    const wmillYaml = (defaultTs: string) =>
      `defaultTs: ${defaultTs}\nincludes:\n  - "${scriptPath}**"\nexcludes: []\n`;

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
          summary: "Test up to date",
          description: "",
          language: "bun",
          kind: "script",
          schema: {
            $schema: "https://json-schema.org/draft/2020-12/schema",
            type: "object",
            properties: {},
            required: [],
          },
          labels: ["l1"],
        }),
      },
    );
    expect(createResp.ok).toEqual(true);

    await writeFile(`${tempDir}/wmill.yaml`, wmillYaml("bun"), "utf-8");
    expect((await backend.runCLICommand(["sync", "pull", "--yes"], tempDir)).code).toEqual(0);

    const hashBefore = (await getScript()).hash;
    expect((await push()).stdout).toContain("is up to date");
    expect((await getScript()).hash).toEqual(hashBefore);

    const metadataPath = `${tempDir}/${scriptPath}.script.yaml`;
    await writeFile(
      metadataPath,
      (await readFile(metadataPath, "utf-8")).replace("- l1", "- l2"),
      "utf-8",
    );
    expect((await push()).stdout).not.toContain("is up to date");
    expect((await getScript()).labels).toEqual(["l2"]);

    expect((await backend.runCLICommand(["sync", "pull", "--yes"], tempDir)).code).toEqual(0);
    await writeFile(`${tempDir}/wmill.yaml`, wmillYaml("deno"), "utf-8");
    expect((await push()).stdout).not.toContain("is up to date");
    expect((await getScript()).language).toEqual("deno");
  });
});
