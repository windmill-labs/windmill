// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.

import { isWindows } from "./_os.js";
import { isAbsolute as posixIsAbsolute } from "./posix/is_absolute.js";
import { isAbsolute as windowsIsAbsolute } from "./windows/is_absolute.js";

/**
 * Verifies whether provided path is absolute.
 *
 * @example Usage
 * ```ts
 * import { isAbsolute } from "@std/path/is-absolute";
 * import { assert, assertFalse } from "@std/assert";
 *
 * if (Deno.build.os === "windows") {
 *   assert(isAbsolute("C:\\home\\foo"));
 *   assertFalse(isAbsolute("home\\foo"));
 * } else {
 *   assert(isAbsolute("/home/foo"));
 *   assertFalse(isAbsolute("home/foo"));
 * }
 * ```
 *
 * @param path Path to be verified as absolute.
 * @returns `true` if path is absolute, `false` otherwise
 */
export function isAbsolute(path: string): boolean {
  return isWindows ? windowsIsAbsolute(path) : posixIsAbsolute(path);
}
