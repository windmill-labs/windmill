// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
import { isWindows } from "./_os.js";
import { dirname as posixDirname } from "./posix/dirname.js";
import { dirname as windowsDirname } from "./windows/dirname.js";
/**
 * Return the directory path of a `path`.
 * @param path - path to extract the directory from.
 */
export function dirname(path) {
    return isWindows ? windowsDirname(path) : posixDirname(path);
}
