import { expect, test } from "bun:test";
import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  listSyncCodebases,
  uncoveredBundleInputs,
} from "../src/utils/codebase.ts";

test("extra_digest_paths edits change the codebase digest", async () => {
  const root = await mkdtemp(join(tmpdir(), "wmill-codebase-"));
  await mkdir(join(root, "f"));
  await mkdir(join(root, "packages/utils"), { recursive: true });
  await writeFile(join(root, "f/main.ts"), "export * from '../packages/utils/a'");
  await writeFile(join(root, "packages/utils/a.ts"), "export const a = 1");

  const digest = () =>
    listSyncCodebases({
      codebases: [
        {
          relative_path: join(root, "f"),
          extra_digest_paths: [join(root, "packages/utils")],
        },
      ],
    } as any)[0].getDigest();

  try {
    const before = await digest();
    await writeFile(join(root, "packages/utils/a.ts"), "export const a = 2");
    expect(await digest()).not.toBe(before);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("uncoveredBundleInputs reports only files outside the digested roots", () => {
  const codebase = { relative_path: "f", extra_digest_paths: ["packages/a"] };
  expect(
    uncoveredBundleInputs(codebase, "g/entry.ts", [
      "g/entry.ts",
      "f/main.ts",
      "packages/a/x.ts",
      "packages/ab/y.ts",
      "node_modules/pg/index.js",
      "<define:process.env>",
    ])
  ).toEqual(["packages/ab/y.ts"]);
});
