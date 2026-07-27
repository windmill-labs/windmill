/**
 * Regression guard for `wmill sync push` reporting success on a script that was
 * never deployed (WIN-2254).
 *
 * A `.script.yaml` change used to be skipped outright on the assumption that the
 * sibling content file carried the deploy. When the content file is not part of
 * the changeset — here because `excludes` keeps it out of the sync — nothing was
 * sent to the remote, yet the push still printed "All N changes pushed" and
 * exited 0.
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
