/**
 * Unit tests for per-script codebase digests (no backend).
 */

import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { listSyncCodebases } from "../src/utils/codebase.ts";
import type { SyncOptions } from "../src/core/conf.ts";

let root: string;
let previousCwd: string;

async function write(file: string, content: string) {
  await mkdir(join(root, file, ".."), { recursive: true });
  await writeFile(join(root, file), content);
}

async function digests(scripts: string[]) {
  const [codebase] = listSyncCodebases({
    codebases: [{ relative_path: "src" }],
  } as SyncOptions);
  await codebase.primeDigests(scripts);
  return Promise.all(scripts.map((s) => codebase.getDigest(s)));
}

beforeEach(async () => {
  previousCwd = process.cwd();
  root = await mkdtemp(join(tmpdir(), "wmill-codebase-digest-"));
  await write("packages/common/shared.ts", "export const greet = () => 'hi';\n");
  await write(
    "packages/common/kind.ts",
    "export enum Kind { A = 'a', B = 'b' }\n"
  );
  await write("service/src/placeholder.ts", "export {};\n");
  await write(
    "service/f/uses_shared.ts",
    "import { greet } from '../../packages/common/shared.ts';\nexport function main() { return greet(); }\n"
  );
  await write(
    "service/f/uses_enum.ts",
    "import { Kind } from '../../packages/common/kind.ts';\nexport function main() { return Kind.A; }\n"
  );
  process.chdir(join(root, "service"));
});

afterEach(async () => {
  process.chdir(previousCwd);
  await rm(root, { recursive: true, force: true });
});

describe("codebase digest", () => {
  const scripts = ["f/uses_shared.ts", "f/uses_enum.ts"];

  test("changes only for scripts that bundle an edited file outside relative_path", async () => {
    const before = await digests(scripts);
    await write("packages/common/shared.ts", "export const greet = () => 'hello';\n");
    const after = await digests(scripts);

    expect(after[0]).not.toBe(before[0]);
    expect(after[1]).toBe(before[1]);
  });

  test("is the same whether a script is digested alone or in a batch", async () => {
    await write(
      "service/f/nested/deeper/index.ts",
      "export function main() { return 1; }\n"
    );
    const nested = "f/nested/deeper/index.ts";
    const batched = await digests([...scripts, nested]);

    const [codebase] = listSyncCodebases({
      codebases: [{ relative_path: "src" }],
    } as SyncOptions);
    expect(await codebase.getDigest(nested)).toBe(batched[2]);
  });

  test("a script that fails to bundle does not change the others' digests", async () => {
    await write(
      "service/f/broken.ts",
      "import { missing } from './does_not_exist.ts';\nexport function main() { return missing; }\n"
    );
    const [alone] = await digests([scripts[0]]);
    const [inFailingBatch] = await digests([scripts[0], "f/broken.ts"]);

    expect(inFailingBatch).toBe(alone);
  });

  test("changes when an inlined enum is edited", async () => {
    const before = await digests(scripts);
    await write(
      "packages/common/kind.ts",
      "export enum Kind { A = 'z', B = 'b' }\n"
    );
    const after = await digests(scripts);

    expect(after[1]).not.toBe(before[1]);
    expect(after[0]).toBe(before[0]);
  });
});
