/**
 * Unit tests for esbuild_loader (no backend, no network).
 */

import { expect, test, describe } from "bun:test";
import { resolveTarEntryPath, withEsbuild } from "../src/utils/esbuild_loader.ts";
import { sep, resolve } from "node:path";

// A parallel `sync push` bundles several apps on the one shared service: the
// first build to finish must not stop it under the others.
test("a finished build does not stop the service under a concurrent one", async () => {
  let releaseSlow!: () => void;
  const slowGate = new Promise<void>((r) => (releaseSlow = r));
  const build = (name: string, gate?: Promise<void>) =>
    withEsbuild((esbuild) =>
      esbuild.build({
        entryPoints: [name],
        bundle: true,
        write: false,
        plugins: [{
          name: "virtual",
          setup(b) {
            b.onResolve({ filter: /.*/ }, (a) => ({ path: a.path, namespace: "v" }));
            b.onLoad({ filter: /.*/, namespace: "v" }, async () => {
              await gate;
              return { contents: `export const n = ${JSON.stringify(name)};`, loader: "js" };
            });
          },
        }],
      })
    );

  const slow = build("slow", slowGate);
  await build("fast");
  releaseSlow();
  const out = await Promise.race([
    slow,
    new Promise<never>((_, rej) => setTimeout(() => rej(new Error("slow build never settled")), 5000)),
  ]);
  expect(out.outputFiles[0].text).toContain('"slow"');
});

describe("resolveTarEntryPath", () => {
  const dest = resolve("/tmp/cache/esbuild-wasm-0.28.0");

  test("strips the leading package/ component and resolves inside dest", () => {
    expect(resolveTarEntryPath(dest, "package/lib/main.js")).toBe(
      dest + sep + "lib" + sep + "main.js"
    );
    expect(resolveTarEntryPath(dest, "package/esbuild.wasm")).toBe(
      dest + sep + "esbuild.wasm"
    );
  });

  test("rejects tar-slip entries that escape the dest dir", () => {
    expect(resolveTarEntryPath(dest, "package/../../etc/passwd")).toBeNull();
    expect(resolveTarEntryPath(dest, "package/../../../outside")).toBeNull();
  });

  test("rejects entries that only share a prefix with dest", () => {
    // ".../esbuild-wasm-0.28.0-evil" must not be treated as inside dest
    expect(resolveTarEntryPath(dest, "evil/../../esbuild-wasm-0.28.0-evil/x"))
      .toBeNull();
  });
});
