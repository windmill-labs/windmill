/**
 * Tailwind CSS v4 in `createBundle`: compiled only when the app declares
 * `tailwindcss`, from the app's own install, scanning the shared `ui/` folder.
 */

import { afterAll, beforeAll, expect, test } from "bun:test";
import * as fs from "node:fs";
import * as path from "node:path";
import * as os from "node:os";
import * as esbuild from "esbuild";
import { createBundle, createFrameworkPlugins, getDevBuildOptions } from "../src/commands/app/bundle.ts";

let root: string;
const originalCwd = process.cwd();
const cliNodeModules = path.resolve(import.meta.dir, "../node_modules");

function app(name: string, dependencies: Record<string, string>) {
  const dir = path.join(root, name);
  const write = (rel: string, content: string) => {
    fs.mkdirSync(path.dirname(path.join(dir, rel)), { recursive: true });
    fs.writeFileSync(path.join(dir, rel), content);
  };
  write("package.json", JSON.stringify({ dependencies }));
  write("index.css", `@import "tailwindcss";\n@import "/ui/theme.css";\n@source "../lib";\n`);
  write(
    "index.ts",
    `import "./index.css";\nimport { card } from "/ui/card";\nconsole.log(card, "flex data-[state=open]:p-3");\n` +
      "console.log(`<div className=\"after:content-['hello']\"></div><div class=\"bg-[url('https://x.test/hero.png')]\"></div>`);\n" +
      "const open = true;\nconsole.log({ hidden: open, grid: !open }, {italic:open});\n",
  );
  // The app's own install, as `bun install` would leave it.
  fs.symlinkSync(cliNodeModules, path.join(dir, "node_modules"));
  return dir;
}

beforeAll(() => {
  root = fs.mkdtempSync(path.join(os.tmpdir(), "raw-app-tailwind-"));
  fs.mkdirSync(path.join(root, "ui"));
  fs.writeFileSync(
    path.join(root, "ui/theme.css"),
    `@theme { --color-brand: #123456; }\n@utility glow { box-shadow: 0 0 2px red; }\n`,
  );
  fs.writeFileSync(path.join(root, "ui/card.ts"), `export const card = "bg-brand glow";\n`);
  // A component library built to dist/, scanned through an explicit @source.
  fs.mkdirSync(path.join(root, "lib/dist"), { recursive: true });
  fs.writeFileSync(path.join(root, "lib/dist/button.js"), `export const button = "tracking-widest";\n`);
});

afterAll(() => {
  process.chdir(originalCwd);
  fs.rmSync(root, { recursive: true, force: true });
});

async function bundle(dir: string) {
  process.chdir(dir);
  return createBundle({
    entryPoint: path.join(dir, "index.ts"),
    minify: false,
    sharedUiDir: path.join(root, "ui"),
  });
}

test("an app declaring tailwindcss gets the utilities it and ui/ use", async () => {
  const { css } = await bundle(app("with", { tailwindcss: "^4.1.0" }));
  expect(css).not.toContain("@tailwind");
  for (const selector of [
    ".hidden",
    ".grid",
    ".italic",
    ".tracking-widest",
    ".bg-brand",
    "#123456",
    ".glow",
    ".flex",
    "data-\\[state\\=open\\]\\:p-3",
    "after\\:content-\\[\\'hello\\'\\]",
    "https://x.test/hero.png",
  ]) {
    expect(css).toContain(selector);
  }
});

test("an app without the dependency builds as before", async () => {
  const { css } = await bundle(app("without", {}));
  expect(css).toContain("@tailwind utilities");
  expect(css).not.toContain(".bg-brand");
});

// `app dev` recompiles in one process: an edit to a file the `@config`
// imports must not come back from the module cache.
test("app dev rebuilds with the edited dependency of an @config", async () => {
  const dir = path.join(root, "config");
  fs.mkdirSync(dir);
  fs.symlinkSync(cliNodeModules, path.join(dir, "node_modules"));
  fs.writeFileSync(path.join(dir, "package.json"), JSON.stringify({ dependencies: { tailwindcss: "^4.1.0" } }));
  fs.writeFileSync(path.join(dir, "index.css"), `@import "tailwindcss";\n@config "./tw.config.cjs";\n`);
  fs.writeFileSync(path.join(dir, "index.ts"), `import "./index.css";\nconsole.log("text-brand");\n`);
  fs.writeFileSync(
    path.join(dir, "tw.config.cjs"),
    `module.exports = { theme: { extend: { colors: { brand: require("./tokens.cjs").brand } } } };\n`,
  );
  const tokens = (color: string) =>
    fs.writeFileSync(path.join(dir, "tokens.cjs"), `module.exports = { brand: "${color}" };\n`);
  tokens("#123456");
  process.chdir(dir);

  let built: (css: string) => void = () => {};
  const nextBuild = () => new Promise<string>((resolve) => (built = resolve));
  const ctx = await esbuild.context({
    ...getDevBuildOptions(path.join(dir, "index.ts")),
    outfile: path.join(dir, "dist/bundle.js"),
    logLevel: "silent",
    plugins: [
      ...(await createFrameworkPlugins(dir)),
      {
        name: "built",
        setup(build: any) {
          build.onEnd(() => built(fs.readFileSync(path.join(dir, "dist/bundle.css"), "utf-8")));
        },
      },
    ],
  });
  try {
    let next = nextBuild();
    await ctx.watch();
    expect(await next).toContain("#123456");
    next = nextBuild();
    tokens("#abcdef");
    const css = await next;
    expect(css).toContain("#abcdef");
    expect(css).not.toContain("#123456");
  } finally {
    await ctx.dispose();
  }
});
