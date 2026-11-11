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
    `import "./index.css";\nimport { cls } from "/ui/button";\nconsole.log(cls, "bg-from-app [&>svg]:hidden", "before:content-['x']", '[&>p]:block');\n`,
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
          tw.content + ["bg-from-app", "bg-from-ui", "[&>svg]:hidden", "before:content-['x']", "[&>p]:block"]
            .filter((c) => candidates.includes(c))
            .map((c) => ".hit-" + c.replace(/\\W/g, "_") + " { color: red; }").join("\\n"),
      };
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
  for (const hit of ["bg_from_app", "bg_from_ui", "___svg__hidden", "before_content___x__", "___p__block"]) {
    expect(css).toContain(`.hit-${hit}`);
  }
  expect(css).not.toContain("@theme");
});
