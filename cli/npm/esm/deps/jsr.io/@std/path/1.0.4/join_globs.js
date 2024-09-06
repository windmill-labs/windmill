// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
import { isWindows } from "./_os.js";
import { joinGlobs as posixJoinGlobs } from "./posix/join_globs.js";
import { joinGlobs as windowsJoinGlobs } from "./windows/join_globs.js";
/**
 * Joins a sequence of globs, then normalizes the resulting glob.
 *
 * Behaves like {@linkcode https://jsr.io/@std/path/doc/~/join | join()}, but
 * doesn't collapse `**\/..` when `globstar` is true.
 *
 * @example Usage
 * ```ts
 * import { joinGlobs } from "@std/path/join-globs";
 * import { assertEquals } from "@std/assert";
 *
 * if (Deno.build.os === "windows") {
 *   assertEquals(joinGlobs(["foo", "bar", "..", "baz"]), "foo\\baz");
 *   assertEquals(joinGlobs(["foo", "**", "bar", "..", "baz"], { globstar: true }), "foo\\**\\baz");
 * } else {
 *   assertEquals(joinGlobs(["foo", "bar", "..", "baz"]), "foo/baz");
 *   assertEquals(joinGlobs(["foo", "**", "bar", "..", "baz"], { globstar: true }), "foo/**\/baz");
 * }
 * ```
 *
 * @param globs Globs to be joined and normalized.
 * @param options Glob options.
 * @returns The joined and normalized glob string.
 */
export function joinGlobs(globs, options = {}) {
    return isWindows
        ? windowsJoinGlobs(globs, options)
        : posixJoinGlobs(globs, options);
}
