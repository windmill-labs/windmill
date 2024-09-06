// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
import { isWindows } from "./_os.js";
import { join as posixJoin } from "./posix/join.js";
import { join as windowsJoin } from "./windows/join.js";
/**
 * Join all given a sequence of `paths`,then normalizes the resulting path.
 * @param paths to be joined and normalized
 */
export function join(...paths) {
    return isWindows ? windowsJoin(...paths) : posixJoin(...paths);
}
