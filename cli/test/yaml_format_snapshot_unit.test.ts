/**
 * YAML Format Snapshot Tests
 *
 * Asserts that representative `flow.yaml` files in
 * `test/fixtures/yaml-snapshots/` round-trip through the CLI's parse +
 * stringify pipeline without byte changes.
 *
 * What this catches:
 * - yaml library upgrades that alter indent / quote / map-flow style
 *   (e.g. the Deno `@std/yaml` → npm `yaml` migration in #8041 silently
 *   changed bytes for the same logical content).
 * - Edits to `yamlOptions` (sortMapEntries via `prioritizeName`,
 *   `singleQuote`, `aliasDuplicateObjects`) that re-order fields or
 *   re-quote scalars.
 * - Custom-tag handler regressions for `!inline` / `!inline_fileset` —
 *   the parser resolves them to `"!inline X"` strings, the serializer
 *   then has to single-quote them because of the leading `!`.
 *
 * What this does NOT catch:
 * - Logic regressions in the backend-Flow-JSON → on-disk-folder
 *   conversion (extracting inline scripts, splitting into files). That
 *   would need synthetic backend fixtures and is intentionally out of
 *   scope.
 *
 * **If a fixture's expected output changes**: re-run
 * `bun run test/fixtures/yaml-snapshots/regenerate.ts` and review the
 * diff. If intentional, commit. If not, investigate the upstream change.
 */

import { expect, test, describe } from "bun:test";
import { readdirSync, readFileSync } from "node:fs";
import * as path from "node:path";
import { stringify as yamlStringify } from "yaml";
import { yamlParseContent } from "../src/utils/yaml.ts";
import { yamlOptions } from "../src/commands/sync/sync.ts";

const FIXTURE_DIR = path.join(import.meta.dir, "fixtures", "yaml-snapshots");

describe("yaml format snapshot — flow.yaml fixtures", () => {
  const fixtures = readdirSync(FIXTURE_DIR).filter((f) => f.endsWith(".yaml"));

  // Sanity guard so a typo in the fixture dir path doesn't silently
  // produce zero tests and a green CI.
  test("fixture directory is non-empty", () => {
    expect(fixtures.length).toBeGreaterThan(0);
  });

  for (const filename of fixtures) {
    test(`${filename}: round-trip is byte-stable`, () => {
      const filePath = path.join(FIXTURE_DIR, filename);
      // Normalize CRLF→LF: on Windows, git checkout may rewrite line endings
      // even though .gitattributes pins these fixtures to LF. yamlStringify
      // always emits LF, so we compare in LF form.
      const original = readFileSync(filePath, "utf-8").replace(/\r\n/g, "\n");
      const parsed = yamlParseContent(filename, original);
      const reSerialized = yamlStringify(parsed, yamlOptions);
      expect(reSerialized).toEqual(original);
    });
  }
});
