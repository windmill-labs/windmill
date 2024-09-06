// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
import { isWindows } from "./_os.js";
import { normalizeGlob as posixNormalizeGlob } from "./posix/normalize_glob.js";
import { normalizeGlob as windowsNormalizeGlob, } from "./windows/normalize_glob.js";
/**
 * Normalizes a glob string.
 *
 * Behaves like
 * {@linkcode https://jsr.io/@std/path/doc/~/normalize | normalize()}, but
 * doesn't collapse "**\/.." when `globstar` is true.
 *
 * @example Usage
 * ```ts
 * import { normalizeGlob } from "@std/path/normalize-glob";
 * import { assertEquals } from "@std/assert";
 *
 * if (Deno.build.os === "windows") {
 *   assertEquals(normalizeGlob("foo\\bar\\..\\baz"), "foo\\baz");
 *   assertEquals(normalizeGlob("foo\\**\\..\\bar\\..\\baz", { globstar: true }), "foo\\**\\..\\baz");
 * } else {
 *   assertEquals(normalizeGlob("foo/bar/../baz"), "foo/baz");
 *   assertEquals(normalizeGlob("foo/**\/../bar/../baz", { globstar: true }), "foo/**\/../baz");
 * }
 * ```
 *
 * @param glob Glob string to normalize.
 * @param options Glob options.
 * @returns The normalized glob string.
 */
export function normalizeGlob(glob, options = {}) {
    return isWindows
        ? windowsNormalizeGlob(glob, options)
        : posixNormalizeGlob(glob, options);
}
