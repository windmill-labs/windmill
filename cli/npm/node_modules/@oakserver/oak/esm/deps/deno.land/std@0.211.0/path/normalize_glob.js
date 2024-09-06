// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
import { isWindows } from "./_os.js";
import { normalizeGlob as posixNormalizeGlob } from "./posix/normalize_glob.js";
import { normalizeGlob as windowsNormalizeGlob, } from "./windows/normalize_glob.js";
/** Like normalize(), but doesn't collapse "**\/.." when `globstar` is true. */
export function normalizeGlob(glob, options = {}) {
    return isWindows
        ? windowsNormalizeGlob(glob, options)
        : posixNormalizeGlob(glob, options);
}
