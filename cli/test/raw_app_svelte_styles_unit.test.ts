/**
 * A component's `<style>` block has to reach the bundle. Svelte's default
 * (`css: "external"`) hands it back on a separate `css` field that an esbuild
 * onLoad cannot emit, so the markup keeps its `svelte-<hash>` class and the
 * rule silently disappears — the app renders unstyled with no build error.
 */

import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import * as fs from "node:fs";
import * as path from "node:path";
import * as os from "node:os";
import { createBundle } from "../src/commands/app/bundle.ts";

let tempDir: string;
let originalCwd: string;

beforeAll(() => {
  originalCwd = process.cwd();
  tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "raw-app-svelte-styles-"));
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

describe("svelte plugin: component styles", () => {
  test("a <style> block reaches the bundle under the class its markup carries", async () => {
    fs.writeFileSync(
      path.join(tempDir, "Styled.svelte"),
      `<main>
  <h1>Hello</h1>
</main>

<style>
  h1 {
    font-size: 1.5rem;
  }
</style>
`,
      "utf-8"
    );
    fs.writeFileSync(
      path.join(tempDir, "styles_entry.ts"),
      `import Styled from './Styled.svelte';
export default Styled;
`,
      "utf-8"
    );

    const { js } = await createBundle({
      entryPoint: path.join(tempDir, "styles_entry.ts"),
      minify: false,
      sourcemap: false,
    });

    // The scoping class esbuild put on the <h1>, which the rule has to match.
    const scopeClass = js.match(/<h1 class="(svelte-[a-z0-9]+)"/)?.[1];
    expect(scopeClass).toBeDefined();
    expect(js).toContain(`h1.${scopeClass}`);
    expect(js).toContain("font-size");
  });
});
