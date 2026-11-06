import { expect, test } from "bun:test";
import { mkdtemp, mkdir, rm, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  exportsTarget,
  listSyncCodebases,
  tsconfigExtends,
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

async function perScriptFixture() {
  const root = await mkdtemp(join(tmpdir(), "wmill-codebase-"));
  await mkdir(join(root, "f/app/lib"), { recursive: true });
  await mkdir(join(root, "packages/utils"), { recursive: true });
  const files: Record<string, string> = {
    "f/app/a.ts": "import { greet } from './lib/greet'; export function main() { return greet() }",
    "f/app/a.spec.ts": "// a test, imported by nothing",
    "f/app/b.ts": "import { Kind } from './lib/kind'; export function main() { return Kind.A }",
    "f/app/c.ts": "import { u } from '../../packages/utils/u'; export function main() { return u }",
    "f/app/lib/greet.ts": "export const greet = () => 'hi'",
    "f/app/lib/kind.ts": "export enum Kind { A = 1 }",
    "packages/utils/u.ts": "export const u = 1",
  };
  for (const [p, c] of Object.entries(files)) await writeFile(join(root, p), c);
  const scripts = ["a", "b", "c"].map((s) => join(root, `f/app/${s}.ts`));
  const codebase = () =>
    listSyncCodebases({
      codebases: [{ relative_path: join(root, "f/app"), per_script_digest: true }],
    } as any)[0];
  const digests = async () => {
    const c = codebase();
    await c.primeDigests(scripts);
    return Promise.all(scripts.map((s) => c.getDigest(s)));
  };
  return { root, scripts, codebase, digests };
}

test("per_script_digest changes only the digests of scripts that bundle the edited file", async () => {
  const { root, digests } = await perScriptFixture();
  try {
    const [a, b, c] = await digests();
    expect(new Set([a, b, c]).size).toBe(3);

    await writeFile(join(root, "f/app/a.spec.ts"), "// edited");
    expect(await digests()).toEqual([a, b, c]);

    await writeFile(join(root, "f/app/lib/greet.ts"), "export const greet = () => 'hello'");
    const [a2, b2, c2] = await digests();
    expect(a2).not.toBe(a);
    expect([b2, c2]).toEqual([b, c]);

    // The enum's value is inlined into b's bundle, which then no longer lists kind.ts.
    await writeFile(join(root, "f/app/lib/kind.ts"), "export enum Kind { A = 2 }");
    const [, b3] = await digests();
    expect(b3).not.toBe(b);

    // Imports outside relative_path are covered without extra_digest_paths.
    await writeFile(join(root, "packages/utils/u.ts"), "export const u = 2");
    const [, , c4] = await digests();
    expect(c4).not.toBe(c);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("per_script_digest is the same for a script digested alone or in a batch", async () => {
  const { root, scripts, codebase, digests } = await perScriptFixture();
  try {
    const batched = await digests();
    const alone = await Promise.all(scripts.map((s) => codebase().getDigest(s)));
    expect(alone).toEqual(batched);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("per_script_digest follows the imports of a script reached through a symlink", async () => {
  const { root, scripts, codebase } = await perScriptFixture();
  const link = root + "-link";
  try {
    await symlink(root, link);
    const viaLink = join(link, "f/app/a.ts");
    const before = await codebase().getDigest(viaLink);
    expect(before).toBe(await codebase().getDigest(scripts[0]));
    await writeFile(join(root, "f/app/lib/greet.ts"), "export const greet = () => 'hello'");
    expect(await codebase().getDigest(viaLink)).not.toBe(before);
  } finally {
    await rm(link, { force: true });
    await rm(root, { recursive: true, force: true });
  }
});

test("per_script_digest covers the nearest tsconfig and what it extends, not shadowed ones", async () => {
  const { root, digests } = await perScriptFixture();
  try {
    await writeFile(join(root, "tsconfig.json"), "{}");
    await writeFile(join(root, "tsconfig.base.json"), `{ "compilerOptions": {} }`);
    await writeFile(
      join(root, "f/app/tsconfig.json"),
      `{\n  // shared settings\n  "extends": "../../tsconfig.base.json", /* jsonc */\n}`
    );
    const before = await digests();

    // f/app's own tsconfig shadows the root one; only c, which also imports
    // packages/utils, has a file the root tsconfig applies to.
    await writeFile(join(root, "tsconfig.json"), `{ "compilerOptions": { "strict": true } }`);
    const shadowed = await digests();
    expect(shadowed.slice(0, 2)).toEqual(before.slice(0, 2));
    expect(shadowed[2]).not.toBe(before[2]);

    await writeFile(join(root, "tsconfig.base.json"), `{ "compilerOptions": { "jsx": "react" } }`);
    const extended = await digests();
    expect(extended[0]).not.toBe(shadowed[0]);
    expect(extended[1]).not.toBe(shadowed[1]);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("per_script_digest covers build fields of package.json, not its dependencies", async () => {
  const { root, digests } = await perScriptFixture();
  try {
    await writeFile(join(root, "package.json"), `{ "name": "x", "dependencies": {} }`);
    const before = await digests();
    await writeFile(join(root, "package.json"), `{ "name": "x", "dependencies": { "pg": "8" } }`);
    expect(await digests()).toEqual(before);
    await writeFile(join(root, "package.json"), `{ "name": "x", "sideEffects": false }`);
    const after = await digests();
    expect(after.every((d, i) => d != before[i])).toBe(true);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("per_script_digest follows a tsconfig extends resolved through a package", async () => {
  const { root, digests } = await perScriptFixture();
  const pkg = join(root, "node_modules/@acme/config");
  try {
    await mkdir(join(pkg, "configs"), { recursive: true });
    await writeFile(join(pkg, "package.json"), `{ "name": "@acme/config", "tsconfig": "./configs/base.json" }`);
    await writeFile(join(pkg, "configs/base.json"), "{}");
    await writeFile(join(root, "f/app/tsconfig.json"), `{ "extends": "@acme/config" }`);
    const before = await digests();
    await writeFile(join(pkg, "configs/base.json"), `{ "compilerOptions": { "useDefineForClassFields": false } }`);
    expect((await digests())[0]).not.toBe(before[0]);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("exportsTarget resolves subpaths, patterns and conditions", () => {
  expect(exportsTarget("./base.json", ".")).toBe("./base.json");
  // esbuild resolves `extends` with the require conditions, never `import` or `types`.
  expect(exportsTarget({ ".": { import: "./esm.json", require: "./cjs.json" } }, ".")).toBe("./cjs.json");
  expect(exportsTarget({ ".": { types: "./t.json", default: "./d.json" } }, ".")).toBe("./d.json");
  expect(exportsTarget({ "./*": "./configs/*.json" }, "./strict")).toBe("./configs/strict.json");
  // The pattern with the longest prefix wins, whatever the declaration order.
  expect(exportsTarget({ "./*": "./any/*.json", "./node/*": "./node/*.json" }, "./node/20")).toBe("./node/20.json");
  expect(exportsTarget({ "./a": "./a.json" }, "./b")).toBeUndefined();
});

test("tsconfigExtends reads extends from JSON with comments", () => {
  expect(tsconfigExtends(`{ "a": "x // not a comment", // c\n "extends": ["./a", "b"], }`)).toEqual(["./a", "b"]);
  expect(tsconfigExtends(`{ /* "extends": "no" */ }`)).toEqual([]);
  expect(tsconfigExtends(`not json`)).toEqual([]);
  expect(tsconfigExtends(`﻿{ "extends": "./base.json" }`)).toEqual(["./base.json"]);
});

test("per_script_digest fails only the script that does not bundle", async () => {
  const { root, scripts, codebase } = await perScriptFixture();
  try {
    const before = await Promise.all(scripts.map((s) => codebase().getDigest(s)));
    await writeFile(join(root, "f/app/b.ts"), "import { x } from './lib/missing'; export const main = () => x");
    const c = codebase();
    await c.primeDigests(scripts);
    expect(await c.getDigest(scripts[0])).toBe(before[0]);
    await expect(c.getDigest(scripts[1])).rejects.toThrow("f/app/b.ts");
    expect(await c.getDigest(scripts[2])).toBe(before[2]);
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
