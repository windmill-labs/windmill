// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
import { isWindows } from "./_os.js";
import { isAbsolute as posixIsAbsolute } from "./posix/is_absolute.js";
import { isAbsolute as windowsIsAbsolute } from "./windows/is_absolute.js";
/**
 * Verifies whether provided path is absolute
 * @param path to be verified as absolute
 */
export function isAbsolute(path) {
    return isWindows ? windowsIsAbsolute(path) : posixIsAbsolute(path);
}
