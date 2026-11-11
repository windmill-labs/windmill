/**
 * Tailwind v4 stylesheets go through the app's own `tailwindcss` compile() rather
 * than reaching bundle.css as raw `@theme` / `@tailwind utilities`. The package is
 * a stub so the test needs no install; it checks what the plugin hands compile().
 */

import { afterAll, beforeAll, expect, test } from "bun:test";
import * as fs from "node:fs";
import * as path from "node:path";
import * as os from "node:os";
import { createBundle } from "../src/commands/app/bundle.ts";

let root: string;
let originalCwd: string;

function write(rel: string, content: string) {
  const full = path.join(root, rel);
  fs.mkdirSync(path.dirname(full), { recursive: true });
  fs.writeFileSync(full, content, "utf-8");
}

beforeAll(() => {
  originalCwd = process.cwd();
  root = fs.mkdtempSync(path.join(os.tmpdir(), "raw-app-tailwind-"));
  write("ui/theme.css", `@import "tailwindcss";\n@theme { --color-brand: #123456; }\n`);
  write("ui/button.tsx", `export const cls = "bg-from-ui";\n`);
  write("f/demo.raw_app/package.json", `{"name":"demo"}`);
  write("f/demo.raw_app/index.css", `@import "/ui/theme.css";\n`);
  write(
    "f/demo.raw_app/index.tsx",
    `import "./index.css";\nimport { cls } from "/ui/button";\nconsole.log(cls, "bg-from-app [&>svg]:hidden", "before:content-['x']", '[&>p]:block data-[s=o]:flex');\n`,
  );
  const pkg = "f/demo.raw_app/node_modules/tailwindcss";
  write(
    `${pkg}/package.json`,
    JSON.stringify({
      name: "tailwindcss",
      exports: { ".": { style: "./index.css", import: "./lib.mjs", require: "./lib.mjs" } },
    }),
  );
  write(`${pkg}/index.css`, `.tailwind-entry { color: blue; }\n`);
  write(
    `${pkg}/lib.mjs`,
    `export async function compile(css, { base, loadStylesheet }) {
      const ui = await loadStylesheet("/ui/theme.css", base);
      const tw = await loadStylesheet("tailwindcss", ui.base);
      return {
        build: (candidates) =>
          tw.content + ["bg-from-app", "bg-from-ui", "[&>svg]:hidden", "before:content-['x']", "[&>p]:block", "data-[s=o]:flex"]
            .filter((c) => candidates.includes(c))
            .map((c) => ".hit-" + c.replace(/\\W/g, "_") + " { color: red; }").join("\\n"),
      };
    }\n`,
  );
  // An app that also has Tailwind's own toolchain, which must take precedence.
  write("f/official.raw_app/package.json", `{"name":"official"}`);
  write("f/official.raw_app/index.css", `@import "tailwindcss";\n`);
  write("f/official.raw_app/index.tsx", `import "./index.css";\n`);
  const nm = "f/official.raw_app/node_modules/@tailwindcss";
  write(
    `${nm}/node/package.json`,
    JSON.stringify({ name: "@tailwindcss/node", exports: { ".": "./index.mjs", "./require-cache": "./rc.mjs" } }),
  );
  write(
    `${nm}/node/index.mjs`,
    `export async function compile() {
      return { root: null, sources: [], build: (c) => c.map((x) => "." + x + " { color: red; }").join("\\n") };
    }\n`,
  );
  write(`${nm}/node/rc.mjs`, `export function clearRequireCache() {}\n`);
  write(`${nm}/oxide/package.json`, JSON.stringify({ name: "@tailwindcss/oxide", main: "./index.mjs" }));
  write(
    `${nm}/oxide/index.mjs`,
    `export class Scanner {
      constructor({ sources }) { this.sources = sources; }
      get files() { return []; }
      scan() {
        return this.sources.filter((s) => !s.negated)
          .map((s) => (s.base.endsWith("ui") ? "scanned-ui" : "scanned-app"));
      }
    }\n`,
  );
  process.chdir(path.join(root, "f/demo.raw_app"));
});

afterAll(() => {
  process.chdir(originalCwd);
  fs.rmSync(root, { recursive: true, force: true });
});

test("tailwind stylesheets compile with the app's tailwindcss over app and ui/ sources", async () => {
  const appDir = path.join(root, "f/demo.raw_app");
  const { css } = await createBundle({
    entryPoint: path.join(appDir, "index.tsx"),
    minify: false,
    sharedUiDir: path.join(root, "ui"),
  });
  expect(css).toContain("tailwind-entry");
  for (const hit of ["bg_from_app", "bg_from_ui", "___svg__hidden", "before_content___x__", "___p__block", "data__s_o__flex"]) {
    expect(css).toContain(`.hit-${hit}`);
  }
  expect(css).not.toContain("@theme");
});

test("an app with @tailwindcss/node and oxide is compiled by them over app and ui/", async () => {
  const { css } = await createBundle({
    entryPoint: path.join(root, "f/official.raw_app/index.tsx"),
    minify: false,
    sharedUiDir: path.join(root, "ui"),
  });
  expect(css).toContain(".scanned-app");
  expect(css).toContain(".scanned-ui");
});
