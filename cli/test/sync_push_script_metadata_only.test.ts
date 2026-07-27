/**
 * A script change that `sync push` counts must reach the remote. Its content
 * file is not guaranteed to be in the changeset — here `excludes` keeps it out
 * of the sync, leaving the `.script.yaml` as the only change — so asserting the
 * deployed content, not just the exit code, is what pins the guarantee.
 */

import { expect, test } from "bun:test";
import { writeFile } from "node:fs/promises";
import { withTestBackend } from "./test_backend.ts";
import { createLocalScript } from "./test_fixtures.ts";

test(
  "sync push deploys a script whose metadata is the only changed file",
  { timeout: 120000 },
  async () => {
    await withTestBackend(async (backend, tempDir) => {
      await writeFile(
        `${tempDir}/wmill.yaml`,
        `defaultTs: bun\nincludes: ["f/**"]\nexcludes: ["**/*.py"]\n`,
      );
      await createLocalScript(
        tempDir,
        "f/test",
        "metadata_only",
        "python3",
        'def main():\n    return "deployed"\n',
      );

      const result = await backend.runCLICommand(
        ["sync", "push", "--yes"],
        tempDir,
      );
      expect(result.code).toBe(0);

      const res = await backend.apiRequest!(
        `/api/w/${backend.workspace}/scripts/get/p/f/test/metadata_only`,
      );
      expect(res.status).toBe(200);
      expect((await res.json()).content).toContain("deployed");
    });
  },
);
