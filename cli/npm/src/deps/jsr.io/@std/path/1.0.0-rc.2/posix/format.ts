// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.

import { _format, assertArg } from "../_common/format.js";
import type { FormatInputPathObject } from "../_interface.js";

/**
 * Generate a path from `FormatInputPathObject` object.
 *
 * @example Usage
 * ```ts
 * import { format } from "@std/path/posix/format";
 * import { assertEquals } from "@std/assert/assert-equals";
 *
 * const path = format({
 *   root: "/",
 *   dir: "/path/dir",
 *   base: "file.txt",
 *   ext: ".txt",
 *   name: "file"
 * });
 * assertEquals(path, "/path/dir/file.txt");
 * ```
 *
 * @param pathObject The path object to format.
 * @returns The formatted path.
 */
export function format(pathObject: FormatInputPathObject): string {
  assertArg(pathObject);
  return _format("/", pathObject);
}
