// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
import { isWindows } from "./_os.js";
import { format as posixFormat } from "./posix/format.js";
import { format as windowsFormat } from "./windows/format.js";
/**
 * Generate a path from `FormatInputPathObject` object.
 * @param pathObject with path
 */
export function format(pathObject) {
    return isWindows ? windowsFormat(pathObject) : posixFormat(pathObject);
}
