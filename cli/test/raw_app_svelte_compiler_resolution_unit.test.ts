/**
 * A raw app brings its own Svelte *runtime* via package.json, so it has to be
 * compiled with its own *compiler* too — the two share internals that change
 * between versions. Svelte 5.52.0 moved delegated event handlers off
 * `element.__click` onto a Symbol-keyed map, so compiling with a version on the
 * other side of that line produces an app that builds, renders, and has every
 * `onclick` silently dead.
 *
 * `import("svelte/compiler")` resolves against the CLI, whose svelte floats
 * independently of the app's. These cover the app-local resolution that avoids
 * that drift.
 */

import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import * as fs from "node:fs";
import * as path from "node:path";
import * as os from "node:os";
import { loadSvelteCompiler } from "../src/commands/app/bundle.ts";

let tempDir: string;

/** A stand-in svelte install, identifiable by a version the real one never has. */
function installStubSvelte(dir: string, version: string) {
  const pkgDir = path.join(dir, "node_modules", "svelte");
  fs.mkdirSync(pkgDir, { recursive: true });
  fs.writeFileSync(
    path.join(pkgDir, "package.json"),
    JSON.stringify({
      name: "svelte",
      version,
      type: "module",
      exports: { "./compiler": "./compiler.js" },
    }),
    "utf-8",
  );
  fs.writeFileSync(
    path.join(pkgDir, "compiler.js"),
    `export const VERSION = ${JSON.stringify(version)};\n` +
      `export function compile() { return { js: { code: "", map: null }, warnings: [] }; }\n` +
      `export function compileModule() { return { js: { code: "", map: null }, warnings: [] }; }\n`,
    "utf-8",
  );
}

/**
 * A stand-in shaped like the real svelte: `./compiler` maps `require` at a UMD
 * bundle and `default` at the ESM sources, and only the ESM half survives an
 * `import()` with its named exports intact.
 */
function installDualStubSvelte(dir: string) {
  const pkgDir = path.join(dir, "node_modules", "svelte");
  fs.mkdirSync(pkgDir, { recursive: true });
  fs.writeFileSync(
    path.join(pkgDir, "package.json"),
    JSON.stringify({
      name: "svelte",
      version: "0.0.0-dual",
      type: "module",
      exports: {
        "./package.json": "./package.json",
        "./compiler": {
          types: "./types/index.d.ts",
          require: "./compiler/index.cjs",
          default: "./src/compiler/index.js",
        },
      },
    }),
    "utf-8",
  );
  fs.mkdirSync(path.join(pkgDir, "src", "compiler"), { recursive: true });
  fs.writeFileSync(
    path.join(pkgDir, "src", "compiler", "index.js"),
    `export const VERSION = "0.0.0-esm";\n` +
      `export function compile() { return { js: { code: "", map: null }, warnings: [] }; }\n`,
    "utf-8",
  );
  fs.mkdirSync(path.join(pkgDir, "compiler"), { recursive: true });
  fs.writeFileSync(
    path.join(pkgDir, "compiler", "index.cjs"),
    // The UMD wrapper the published bundle uses: no static `exports.x = ...` for
    // a lexer to find, so an `import()` of this file sees only `default`.
    `!function(e,t){"object"==typeof exports&&"undefined"!=typeof module?t(exports):e(t)}` +
      `(0,function(e){e.VERSION="0.0.0-cjs";` +
      `e.compile=function(){return{js:{code:"",map:null},warnings:[]}}});\n`,
    "utf-8",
  );
}

beforeEach(() => {
  tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "raw-app-svelte-compiler-"));
  fs.writeFileSync(
    path.join(tempDir, "package.json"),
    JSON.stringify({ name: "app", private: true, dependencies: { svelte: "*" } }),
    "utf-8",
  );
});

afterEach(() => {
  fs.rmSync(tempDir, { recursive: true, force: true });
});

describe("loadSvelteCompiler", () => {
  test("uses the app's own compiler rather than the CLI's", async () => {
    installStubSvelte(tempDir, "0.0.0-app-local");

    const compiler = await loadSvelteCompiler(tempDir);

    expect(compiler.VERSION).toBe("0.0.0-app-local");
  });

  test("takes the ESM entry, not the `require` one, off a dual exports map", async () => {
    installDualStubSvelte(tempDir);

    const compiler = await loadSvelteCompiler(tempDir);

    expect(compiler.VERSION).toBe("0.0.0-esm");
    expect(typeof compiler.compile).toBe("function");
  });

  test("resolves against the app even when given a relative dir", async () => {
    installStubSvelte(tempDir, "0.0.0-relative");
    const cwd = process.cwd();
    try {
      process.chdir(path.dirname(tempDir));
      const compiler = await loadSvelteCompiler(path.basename(tempDir));
      expect(compiler.VERSION).toBe("0.0.0-relative");
    } finally {
      process.chdir(cwd);
    }
  });

  test("falls back to the CLI's compiler when the app has none", async () => {
    const compiler = await loadSvelteCompiler(tempDir);

    // The real thing, not a stub — and new enough to emit the Symbol-map form
    // of delegated handlers that a modern runtime reads.
    expect(typeof compiler.compile).toBe("function");
    expect(compiler.VERSION).not.toContain("app-local");
    const [major, minor] = compiler.VERSION.split(".").map(Number);
    expect(major > 5 || (major === 5 && minor >= 52)).toBe(true);
  });
});
