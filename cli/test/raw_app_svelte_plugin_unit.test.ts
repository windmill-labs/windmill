/**
 * The svelte esbuild plugin, driven through `createBundle`.
 */

import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import * as fs from "node:fs";
import * as path from "node:path";
import * as os from "node:os";
import { createBundle } from "../src/commands/app/bundle.ts";

let tempDir: string;
let originalCwd: string;

// Runes used as bare calls, ignoring the `$state(...)` mentions inside Svelte's
// own warning message template literals.
function bareRuneCalls(js: string): string[] {
  return [...js.matchAll(/(^|[^.\w$`])\$(state|derived|effect|props)\s*\(/g)].map(
    (m) => m[0]
  );
}

function writeApp(files: Record<string, string>) {
  for (const [name, content] of Object.entries(files)) {
    fs.writeFileSync(path.join(tempDir, name), content, "utf-8");
  }
}

async function bundle(entry: string): Promise<string> {
  const { js } = await createBundle({
    entryPoint: path.join(tempDir, entry),
    minify: false,
    sourcemap: false,
  });
  return js;
}

beforeAll(() => {
  originalCwd = process.cwd();
  tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "raw-app-svelte-module-"));
  fs.writeFileSync(
    path.join(tempDir, "package.json"),
    JSON.stringify({ name: "app", private: true, dependencies: { svelte: "*" } }),
    "utf-8"
  );
  // Reuse the CLI's own svelte install instead of paying for an npm install;
  // `ensureNodeModules` only checks that the directory is there.
  fs.symlinkSync(
    path.join(originalCwd, "node_modules"),
    path.join(tempDir, "node_modules")
  );
  process.chdir(tempDir);
});

afterAll(() => {
  process.chdir(originalCwd);
  fs.rmSync(tempDir, { recursive: true, force: true });
});

/**
 * `lib.svelte.ts` / `lib.svelte.js` modules are plain modules that may use
 * runes. They need `svelte.compileModule`; without it esbuild happily bundles
 * `$state(...)` as an ordinary call and the app dies at runtime with
 * "ReferenceError: $state is not defined".
 */
describe("svelte plugin: .svelte.ts modules", () => {
  test("compiles runes in a TypeScript rune module and the bundle runs", async () => {
    writeApp({
      "lib.svelte.ts": `export class Cycle<T> {
  #options: T[] = [];
  #index = $state(0);
  current = $derived(this.#options[this.#index]);

  constructor(options: T[]) {
    this.#options = options;
  }

  next() {
    this.#index = (this.#index + 1) % this.#options.length;
  }
}
`,
      "ts_entry.ts": `import { Cycle } from './lib.svelte';

const cycle = new Cycle(['a', 'b', 'c']);
cycle.next();
(globalThis as any).__cycleResult = cycle.current;
`,
    });

    const js = await bundle("ts_entry.ts");

    expect(bareRuneCalls(js)).toEqual([]);
    // Runtime is the real check: unfixed, this throws
    // "ReferenceError: $state is not defined".
    new Function(js)();
    expect((globalThis as any).__cycleResult).toBe("b");
  });

  test("compiles runes in a JavaScript rune module", async () => {
    writeApp({
      "counter.svelte.js": `export const counter = $state({ n: 0 });

export function bump() {
  counter.n += 1;
}
`,
      "js_entry.ts": `import { counter, bump } from './counter.svelte.js';

bump();
bump();
(globalThis as any).__counterResult = counter.n;
`,
    });

    const js = await bundle("js_entry.ts");

    expect(bareRuneCalls(js)).toEqual([]);
    new Function(js)();
    expect((globalThis as any).__counterResult).toBe(2);
  });
});

/**
 * Svelte's default `css: "external"` hands a component's <style> back on a
 * field the plugin never emits, so the markup keeps its `svelte-<hash>` class
 * while the rule matching it disappears — no build error, just an app that
 * renders unstyled from the CLI and styled in the editor.
 */
describe("svelte plugin: component styles", () => {
  test("a <style> block reaches the bundle under the class its markup carries", async () => {
    writeApp({
      "Styled.svelte": `<main>
  <h1>Hello</h1>
</main>

<style>
  h1 {
    font-size: 1.5rem;
  }
</style>
`,
      "styles_entry.ts": `import Styled from './Styled.svelte';
export default Styled;
`,
    });

    const js = await bundle("styles_entry.ts");

    const scopeClass = js.match(/<h1 class="(svelte-[a-z0-9]+)"/)?.[1];
    expect(scopeClass).toBeDefined();
    expect(js).toContain(`h1.${scopeClass}`);
    expect(js).toContain("font-size");
  });
});
