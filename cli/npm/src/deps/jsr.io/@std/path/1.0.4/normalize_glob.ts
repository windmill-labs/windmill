// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.

import type { GlobOptions } from "./_common/glob_to_reg_exp.js";
import { isWindows } from "./_os.js";
import { normalizeGlob as posixNormalizeGlob } from "./posix/normalize_glob.js";
import {
  normalizeGlob as windowsNormalizeGlob,
} from "./windows/normalize_glob.js";

export type { GlobOptions };

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
export function normalizeGlob(
  glob: string,
  options: GlobOptions = {},
): string {
  return isWindows
    ? windowsNormalizeGlob(glob, options)
    : posixNormalizeGlob(glob, options);
}
