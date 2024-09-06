// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
import { isWindows } from "./_os.js";
import { joinGlobs as posixJoinGlobs } from "./posix/join_globs.js";
import { joinGlobs as windowsJoinGlobs } from "./windows/join_globs.js";
/** Like join(), but doesn't collapse "**\/.." when `globstar` is true. */
export function joinGlobs(globs, options = {}) {
    return isWindows
        ? windowsJoinGlobs(globs, options)
        : posixJoinGlobs(globs, options);
}
