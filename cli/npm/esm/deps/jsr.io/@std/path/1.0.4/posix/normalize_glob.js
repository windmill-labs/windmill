// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
import { normalize } from "./normalize.js";
import { SEPARATOR_PATTERN } from "./constants.js";
/**
 * Like normalize(), but doesn't collapse "**\/.." when `globstar` is true.
 *
 * @example Usage
 * ```ts
 * import { normalizeGlob } from "@std/path/posix/normalize-glob";
 * import { assertEquals } from "@std/assert";
 *
 * const path = normalizeGlob("foo/bar/../*", { globstar: true });
 * assertEquals(path, "foo/*");
 * ```
 *
 * @param glob The glob to normalize.
 * @param options The options to use.
 * @returns The normalized path.
 */
export function normalizeGlob(glob, { globstar = false } = {}) {
    if (glob.match(/\0/g)) {
        throw new Error(`Glob contains invalid characters: "${glob}"`);
    }
    if (!globstar) {
        return normalize(glob);
    }
    const s = SEPARATOR_PATTERN.source;
    const badParentPattern = new RegExp(`(?<=(${s}|^)\\*\\*${s})\\.\\.(?=${s}|$)`, "g");
    return normalize(glob.replace(badParentPattern, "\0")).replace(/\0/g, "..");
}
