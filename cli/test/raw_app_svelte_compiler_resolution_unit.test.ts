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
