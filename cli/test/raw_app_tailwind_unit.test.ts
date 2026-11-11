import { expect, test } from "bun:test";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { createRequire } from "node:module";
import * as esbuild from "esbuild";
import { createBundle, createSharedUiPlugin, getDevBuildOptions } from "../src/commands/app/bundle.ts";
import { createTailwindPlugin } from "../src/commands/app/tailwind.ts";

test("Tailwind compiles shared themes, utilities and source edits in production and dev", async () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "wmill-tailwind-"));
  const appDir = path.join(root, "app");
  const uiDir = path.join(root, "ui");
  const write = (file: string, text: string) => {
    fs.mkdirSync(path.dirname(path.join(root, file)), { recursive: true });
    fs.writeFileSync(path.join(root, file), text);
  };
  let ctx: esbuild.BuildContext | undefined;
  try {
    write("app/package.json", "{}");
    write("app/index.ts", 'import "./index.css"; console.log("bg-brand badge");');
    write("app/index.css", '@import "/ui/theme.css";');
    write("ui/theme.css", '@import "tailwindcss"; @theme { --color-brand: #123456; } @utility badge { border-radius: 13px; }');
    write("ui/component.tsx", 'export const html = "text-brand";');
    fs.mkdirSync(path.join(appDir, "node_modules"));
    const require = createRequire(import.meta.url);
    fs.symlinkSync(path.dirname(require.resolve("tailwindcss/package.json")), path.join(appDir, "node_modules/tailwindcss"));
    const entryPoint = path.join(appDir, "index.ts");
    const result = await createBundle({ entryPoint, sharedUiDir: uiDir, outDir: path.join(root, "dist"), minify: false });
    expect(result.css).toContain(".bg-brand");
    expect(result.css).toContain(".text-brand");
    expect(result.css).toContain("border-radius: 13px");
    expect(result.css).toContain("--color-brand: #123456");
    expect(result.css).not.toMatch(/@(?:theme|tailwind|utility)\b/);

    let onBuild: (result: esbuild.BuildResult) => void = () => {};
    const nextBuild = () => new Promise<esbuild.BuildResult>((resolve, reject) => {
      const timeout = setTimeout(() => reject(new Error("Timed out waiting for CSS rebuild")), 10000);
      onBuild = (result) => { clearTimeout(timeout); resolve(result); };
    });
    ctx = await esbuild.context({
      ...getDevBuildOptions(entryPoint),
      outfile: path.join(root, "dev/bundle.js"),
      write: false,
      plugins: [
        createTailwindPlugin(appDir, uiDir),
        createSharedUiPlugin(uiDir, appDir),
        { name: "observe-build", setup(build) { build.onEnd((result) => onBuild(result)); } },
      ],
    });
    const initial = nextBuild();
    await ctx.watch();
    await initial;
    const changed = nextBuild();
    write("ui/component.tsx", 'export const html = "text-brand p-7";');
    const rebuilt = await changed;
    expect(rebuilt.outputFiles?.find((f) => f.path.endsWith(".css"))?.text).toContain(".p-7");
    const added = nextBuild();
    write("ui/new-component.tsx", 'export const html = "m-9";');
    expect((await added).outputFiles?.find((f) => f.path.endsWith(".css"))?.text).toContain(".m-9");
    const themeChanged = nextBuild();
    write("ui/theme.css", '@theme { --color-brand: #abcdef; } @utility badge { border-radius: 17px; }');
    const themeOnly = await themeChanged;
    const css = themeOnly.outputFiles?.find((f) => f.path.endsWith(".css"))?.text;
    expect(css).toContain("--color-brand: #abcdef");
    expect(css).toContain(".bg-brand");
    expect(css).toContain("border-radius: 17px");
    expect(css).not.toMatch(/@(?:theme|tailwind|utility)\b/);
  } finally {
    await ctx?.dispose();
    fs.rmSync(root, { recursive: true, force: true });
  }
});
