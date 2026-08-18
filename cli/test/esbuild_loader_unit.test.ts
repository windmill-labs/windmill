/**
 * Unit tests for esbuild_loader pure logic (no backend, no network).
 */

import { expect, test, describe } from "bun:test";
import { resolveTarEntryPath } from "../src/utils/esbuild_loader.ts";
import { sep, resolve } from "node:path";

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
