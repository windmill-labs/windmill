import { expect, test } from "bun:test";
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

// =============================================================================
// PARSER PACKAGE LOCKSTEP TEST
//
// The npm publish (build-npm.ts) derives its externals + published
// dependencies from package.json's windmill-parser-wasm-* entries. A parser
// package that src/ loads but package.json does not declare therefore ships a
// CLI where loadParser() throws and callers silently degrade — e.g. the local
// pipeline graph (`wmill pipeline show|dev --local`) loses every inferred
// write edge. This test scans src/ for parser-package string literals and
// requires each one to be a declared dependency.
// =============================================================================

const cliRoot = join(import.meta.dir, "..");

function collectSourceFiles(dir: string, out: string[] = []): string[] {
  for (const entry of readdirSync(dir)) {
    const p = join(dir, entry);
    if (statSync(p).isDirectory()) collectSourceFiles(p, out);
    else if (p.endsWith(".ts")) out.push(p);
  }
  return out;
}

test("every windmill-parser-wasm-* package referenced in src/ is a declared dependency", () => {
  const deps: Record<string, string> =
    JSON.parse(readFileSync(join(cliRoot, "package.json"), "utf-8"))
      .dependencies ?? {};

  const referenced = new Set<string>();
  for (const file of collectSourceFiles(join(cliRoot, "src"))) {
    for (const m of readFileSync(file, "utf-8").matchAll(
      /"(windmill-parser-wasm-[a-z-]+)"/g,
    )) {
      referenced.add(m[1]);
    }
  }

  // Sanity: the scan must see the known call sites, else the regex went stale.
  expect(referenced.has("windmill-parser-wasm-ts")).toBe(true);
  expect(referenced.has("windmill-parser-wasm-asset")).toBe(true);

  const missing = [...referenced].filter((p) => !(p in deps)).sort();
  expect(missing).toEqual([]);
});
