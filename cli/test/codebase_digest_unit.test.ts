import { expect, test } from "bun:test";
import { mkdtemp, mkdir, realpath, rm, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import {
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

// Pins the digest's tsconfig coverage to what esbuild applies: in each layout, editing
// the base config esbuild applies must change both the bundle and the digest. The decoy
// is a config esbuild does not apply (its edit leaves the bundle alone), which proves
// the layout resolves where it claims to.
const ON = `{ "compilerOptions": { "useDefineForClassFields": true } }`;
const OFF = `{ "compilerOptions": { "useDefineForClassFields": false } }`;
const tsconfigLayouts: {
  name: string;
  files: Record<string, string>;
  links?: Record<string, string>;
  applied: string;
  decoy?: string;
}[] = [
  {
    name: ".jsonc exports target",
    files: {
      "node_modules/cfg/package.json": JSON.stringify({ exports: { ".": "./base.jsonc" } }),
      "node_modules/cfg/base.jsonc": ON,
      "node_modules/cfg/tsconfig.json": ON,
      "f/tsconfig.json": `{ "extends": "cfg" }`,
    },
    applied: "node_modules/cfg/base.jsonc",
    decoy: "node_modules/cfg/tsconfig.json",
  },
  {
    name: "extensionless subpath",
    files: {
      "node_modules/cfg/package.json": "{}",
      "node_modules/cfg/strict": ON,
      "f/tsconfig.json": `{ "extends": "cfg/strict" }`,
    },
    applied: "node_modules/cfg/strict",
  },
  {
    name: "workspace package symlinked into node_modules",
    files: {
      "packages/cfg/package.json": "{}",
      "packages/cfg/tsconfig.json": ON,
      "f/tsconfig.json": `{ "extends": "cfg" }`,
    },
    links: { "node_modules/cfg": "../packages/cfg" },
    applied: "packages/cfg/tsconfig.json",
  },
  {
    name: "conditional exports",
    files: {
      "node_modules/cfg/package.json": JSON.stringify({ exports: { ".": { import: "./esm.json", require: "./cjs.json" } } }),
      "node_modules/cfg/esm.json": ON,
      "node_modules/cfg/cjs.json": ON,
      "f/tsconfig.json": `{ "extends": "cfg" }`,
    },
    applied: "node_modules/cfg/cjs.json",
    decoy: "node_modules/cfg/esm.json",
  },
  {
    name: "nested unmatched condition",
    files: {
      "node_modules/cfg/package.json": JSON.stringify({ exports: { ".": { node: { import: "./esm.json" }, require: "./req.json" } } }),
      "node_modules/cfg/esm.json": ON,
      "node_modules/cfg/req.json": ON,
      "f/tsconfig.json": `{ "extends": "cfg" }`,
    },
    applied: "node_modules/cfg/req.json",
    decoy: "node_modules/cfg/esm.json",
  },
  {
    name: "explicit null condition",
    files: {
      "node_modules/cfg/package.json": JSON.stringify({ exports: { ".": { node: null, require: "./req.json" } } }),
      "node_modules/cfg/req.json": ON,
      "node_modules/cfg/tsconfig.json": ON,
      "f/tsconfig.json": `{ "extends": "cfg" }`,
    },
    applied: "node_modules/cfg/tsconfig.json",
    decoy: "node_modules/cfg/req.json",
  },
  {
    name: "module condition",
    files: {
      "node_modules/cfg/package.json": JSON.stringify({ exports: { ".": { module: "./module.json", require: "./req.json" } } }),
      "node_modules/cfg/module.json": ON,
      "node_modules/cfg/req.json": ON,
      "f/tsconfig.json": `{ "extends": "cfg" }`,
    },
    applied: "node_modules/cfg/module.json",
    decoy: "node_modules/cfg/req.json",
  },
  {
    name: "invalid target in a fallback array",
    files: {
      "node_modules/cfg/package.json": JSON.stringify({ exports: { ".": ["invalid", "./req.json"] } }),
      "node_modules/cfg/req.json": ON,
      "node_modules/cfg/tsconfig.json": ON,
      "f/tsconfig.json": `{ "extends": "cfg" }`,
    },
    applied: "node_modules/cfg/req.json",
    decoy: "node_modules/cfg/tsconfig.json",
  },
  {
    name: "overlapping wildcard exports",
    files: {
      "node_modules/cfg/package.json": JSON.stringify({ exports: { "./*": "./any/*.json", "./node/*": "./node/*.json" } }),
      "node_modules/cfg/any/node/20.json": ON,
      "node_modules/cfg/node/20.json": ON,
      "f/tsconfig.json": `{ "extends": "cfg/node/20" }`,
    },
    applied: "node_modules/cfg/node/20.json",
    decoy: "node_modules/cfg/any/node/20.json",
  },
  {
    name: "package tsconfig field",
    files: {
      "node_modules/cfg/package.json": JSON.stringify({ tsconfig: "./configs/base.json" }),
      "node_modules/cfg/configs/base.json": ON,
      "node_modules/cfg/tsconfig.json": ON,
      "f/tsconfig.json": `{ "extends": "cfg" }`,
    },
    applied: "node_modules/cfg/configs/base.json",
    decoy: "node_modules/cfg/tsconfig.json",
  },
  {
    name: "BOM-prefixed config, relative extends without .json",
    files: { "base.json": ON, "f/tsconfig.json": `\uFEFF{ "extends": "../base" }` },
    applied: "base.json",
  },
];

for (const layout of tsconfigLayouts) {
  test(`per_script_digest applies the tsconfig esbuild applies: ${layout.name}`, async () => {
    const esbuild = await import("esbuild");
    const root = await realpath(await mkdtemp(join(tmpdir(), "wmill-tsconfig-")));
    const cwd = process.cwd();
    try {
      for (const [p, c] of Object.entries(layout.files)) {
        await mkdir(dirname(join(root, p)), { recursive: true });
        await writeFile(join(root, p), c);
      }
      for (const [p, target] of Object.entries(layout.links ?? {})) {
        await mkdir(dirname(join(root, p)), { recursive: true });
        await symlink(target, join(root, p));
      }
      // useDefineForClassFields decides whether the field shadows the accessor.
      await writeFile(
        join(root, "f/a.ts"),
        "class A { x = 1; constructor() { Object.defineProperty(this, 'x', { get: () => 0 }) } }\nexport const main = () => new A().x\n"
      );
      process.chdir(root);
      const bundle = async () =>
        (await esbuild.build({ entryPoints: ["f/a.ts"], bundle: true, write: false, platform: "node", logLevel: "silent", absWorkingDir: root })).outputFiles[0].text;
      const digest = () =>
        listSyncCodebases({ codebases: [{ relative_path: "f", per_script_digest: true }] } as any)[0].getDigest("f/a.ts");
      const [b0, d0] = [await bundle(), await digest()];
      if (layout.decoy) {
        await writeFile(join(root, layout.decoy), OFF);
        expect(await bundle()).toBe(b0);
      }
      const d1 = await digest();
      await writeFile(join(root, layout.applied), OFF);
      expect(await bundle()).not.toBe(b0);
      expect(await digest()).not.toBe(d1);
    } finally {
      process.chdir(cwd);
      await rm(root, { recursive: true, force: true });
    }
  });
}

test("tsconfigExtends reads extends from JSON with comments", () => {
  expect(tsconfigExtends(`{ "a": "x // not a comment", // c\n "extends": ["./a", "b"], }`)).toEqual(["./a", "b"]);
  expect(tsconfigExtends(`{ /* "extends": "no" */ }`)).toEqual([]);
  expect(tsconfigExtends(`not json`)).toEqual([]);
  expect(tsconfigExtends(`\uFEFF{ "extends": "./base.json" }`)).toEqual(["./base.json"]);
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
