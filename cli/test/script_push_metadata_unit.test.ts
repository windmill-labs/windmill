import { describe, expect, test } from "bun:test";
import { mkdtemp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { stringify as yamlStringify } from "yaml";

import { computePushMetadataHash } from "../src/commands/script/script.ts";
import { generateScriptHash, checkifMetadataUptodate } from "../src/utils/metadata.ts";

async function withTempDir(fn: (tempDir: string) => Promise<void>): Promise<void> {
  const tempDir = await mkdtemp(path.join(os.tmpdir(), "wmill_push_meta_"));
  const originalCwd = process.cwd();
  try {
    process.chdir(tempDir);
    await fn(tempDir);
  } finally {
    process.chdir(originalCwd);
    await rm(tempDir, { recursive: true, force: true });
  }
}

describe("script push metadata hash", () => {
  test("matches generate-metadata lock hash for a script with inline metadata", async () => {
    await withTempDir(async (tempDir) => {
      const scriptPath = path.join(tempDir, "f/test/example.ts");
      const metadataPath = path.join(tempDir, "f/test/example.script.yaml");
      await mkdir(path.dirname(scriptPath), { recursive: true });

      await writeFile(
        scriptPath,
        'export async function main() { return "metadata"; }\n'
      );

      const metadataContent = yamlStringify({
        summary: "test",
        description: "",
        lock: "",
        kind: "script",
        schema: {
          $schema: "https://json-schema.org/draft/2020-12/schema",
          type: "object",
          properties: {},
          required: [],
        },
      });
      await writeFile(metadataPath, metadataContent);

      const scriptContent = await readFile(scriptPath, "utf-8");
      const expectedHash = await generateScriptHash({}, scriptContent, metadataContent);
      const pushHash = await computePushMetadataHash("f/test/example.ts", scriptContent);

      expect(pushHash).toEqual(expectedHash);
      expect(
        await checkifMetadataUptodate("f/test/example", pushHash, {
          version: "v2",
          locks: { "f/test/example": pushHash },
        })
      ).toBeTrue();
    });
  });
});
