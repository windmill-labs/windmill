/**
 * `/ui/...` imports, driven through `createBundle` the way `app dev` and
 * `app lint` find the shared folder: from the app up to its wmill.yaml.
 */

import { afterAll, beforeAll, expect, test } from "bun:test";
import * as fs from "node:fs";
import * as path from "node:path";
import * as os from "node:os";
import { createBundle, findSharedUiDir } from "../src/commands/app/bundle.ts";

let root: string;
let originalCwd: string;

function write(rel: string, content: string) {
  const full = path.join(root, rel);
  fs.mkdirSync(path.dirname(full), { recursive: true });
  fs.writeFileSync(full, content, "utf-8");
}

beforeAll(() => {
  originalCwd = process.cwd();
  root = fs.mkdtempSync(path.join(os.tmpdir(), "raw-app-shared-ui-"));
  write("wmill.yaml", "defaultTs: bun\n");
  write("ui/button.ts", `import { who } from "pkg";\nexport const label = who;\n`);
  write("f/demo.raw_app/package.json", `{"name":"demo"}`);
  write("f/demo.raw_app/index.tsx", `import { label } from "/ui/button";\nconsole.log(label);\n`);
  write("f/demo.raw_app/node_modules/pkg/index.js", `export const who = "from-app";\n`);
  // A copy above ui/ must not win: it would be a second instance of the package.
  write("node_modules/pkg/index.js", `export const who = "from-root";\n`);
  process.chdir(path.join(root, "f/demo.raw_app"));
});

afterAll(() => {
  process.chdir(originalCwd);
  fs.rmSync(root, { recursive: true, force: true });
});

test("bare imports in ui/ resolve from the app's node_modules", async () => {
  const appDir = path.join(root, "f/demo.raw_app");
  const sharedUiDir = findSharedUiDir(appDir);
  expect(sharedUiDir).toBe(path.join(root, "ui"));

  const { js } = await createBundle({
    entryPoint: path.join(appDir, "index.tsx"),
    minify: false,
    sharedUiDir,
  });
  expect(js).toContain("from-app");
  expect(js).not.toContain("from-root");
});
