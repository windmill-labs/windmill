/**
 * Lockfile Format Pinning Tests
 *
 * Hard-coded golden values for the wire format of `wmill-lock.yaml` and the
 * staleness-detection hash inputs. These tests will FAIL if any of the
 * following changes:
 *
 * - The hash formula in `generateScriptHash` (raw concatenation order, JSON
 *   stringify of workspace deps) — used to compare against the hash stored
 *   in `wmill-lock.yaml` and on the deployed script.
 * - The SHA-256 algorithm or hex encoding in `generateHash`.
 * - The `yaml` library version's serialization output for the canonical
 *   `Lock` shape (sortMapEntries, singleQuote, etc.).
 *
 * Past regressions this catches:
 * - Migration from Deno `@std/yaml` to npm `yaml` (#8041) silently changed
 *   indent/quote choices, producing different bytes for the same logical
 *   `wmill-lock.yaml` content. With these tests in place the byte change
 *   would have been caught at PR time.
 * - The unsorted-keys top-hash issue in #8480 — a one-line `Object.keys().sort()`
 *   addition that silently invalidated every existing flow/app top hash.
 *
 * **If you intentionally change the format**: bump `version` in `metadata.ts`,
 * add a migration path (e.g. `wmill lock upgrade`), AND update the golden
 * values below. Don't just bump the goldens — that defeats the purpose.
 */

import { expect, test, describe } from "bun:test";
import { stringify as yamlStringify } from "yaml";
import { generateHash } from "../src/utils/utils.ts";
import { generateScriptHash } from "../src/utils/metadata.ts";
import { yamlOptions } from "../src/commands/sync/sync.ts";

describe("hash formula pinning", () => {
  test("generateHash: SHA-256 hex of UTF-8 bytes", async () => {
    expect(await generateHash("")).toEqual(
      "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    expect(await generateHash("hello")).toEqual(
      "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
    );
    // Multi-byte UTF-8 must hash bytes (not codepoints) — pin this
    expect(await generateHash("é")).toEqual(
      "4a99557e4033c3539de2eb65472017cad5f9557f7a0625a09f1c3f6e2ba69c4c"
    );
  });

  test("generateScriptHash: order = JSON.stringify(deps) + content + metadata", async () => {
    // Empty deps + empty content + empty metadata
    expect(await generateScriptHash({}, "", "")).toEqual(
      await generateHash("{}")
    );
    // Pin the canonical case used in real lockfiles
    const deps = { "dependencies/package.json": "abc123" };
    const content = "export async function main() { return 1; }\n";
    const metadata = "summary: test\n";
    expect(await generateScriptHash(deps, content, metadata)).toEqual(
      await generateHash(JSON.stringify(deps) + content + metadata)
    );
    // Hard-pin one full byte sequence — if the formula changes (e.g. swap
    // order, switch hash algo) this will diverge.
    expect(await generateScriptHash(deps, content, metadata)).toEqual(
      "69b0f1c596e65ef5e75c74aa2980484f1cae80bcb429132ce3906dd319ec28f1"
    );
  });

  test("generateScriptHash: deps with different key orders produce different hashes", async () => {
    // Documents existing behavior: workspace-deps map is NOT sorted before
    // JSON.stringify, so insertion order matters. If you sort here, you must
    // bump lock version.
    const a = await generateScriptHash({ a: "1", b: "2" }, "", "");
    const b = await generateScriptHash({ b: "2", a: "1" }, "", "");
    expect(a).not.toEqual(b);
  });
});

describe("wmill-lock.yaml byte-format pinning", () => {
  test("yamlOptions: canonical Lock serializes to a stable byte sequence", () => {
    // A representative Lock with both flat and ./-prefixed entries — the
    // shape we produce in the real CLI. If the yaml lib changes
    // indentation, quote style, or sort order, this snapshot will break.
    // Note on ordering: `./`-prefixed keys sort BEFORE non-prefixed because
    // `localeCompare` puts `.` before alphanumerics. If a future change
    // normalizes away `./` on write (i.e. `clearGlobalLock` deletes them
    // before any new entry is added), update this snapshot.
    const lock = {
      version: "v2",
      locks: {
        "f/scripts/utility": "hash1",
        "./f/legacy/dotted": "hash_legacy",
        "f/flows/main.flow+inline/step1.ts": "hash2",
      },
    };
    const expected =
      "version: v2\n" +
      "locks:\n" +
      "  ./f/legacy/dotted: hash_legacy\n" +
      "  f/flows/main.flow+inline/step1.ts: hash2\n" +
      "  f/scripts/utility: hash1\n";
    expect(yamlStringify(lock as Record<string, unknown>, yamlOptions)).toEqual(expected);
  });

  test("yamlOptions: empty Lock", () => {
    const lock = { version: "v2", locks: {} };
    const expected = "version: v2\nlocks: {}\n";
    expect(yamlStringify(lock as Record<string, unknown>, yamlOptions)).toEqual(expected);
  });
});
