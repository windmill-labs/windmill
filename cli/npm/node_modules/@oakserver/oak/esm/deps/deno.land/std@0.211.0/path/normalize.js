// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
import { isWindows } from "./_os.js";
import { normalize as posixNormalize } from "./posix/normalize.js";
import { normalize as windowsNormalize } from "./windows/normalize.js";
/**
 * Normalize the `path`, resolving `'..'` and `'.'` segments.
 * Note that resolving these segments does not necessarily mean that all will be eliminated.
 * A `'..'` at the top-level will be preserved, and an empty path is canonically `'.'`.
 * @param path to be normalized
 */
export function normalize(path) {
    return isWindows ? windowsNormalize(path) : posixNormalize(path);
}
