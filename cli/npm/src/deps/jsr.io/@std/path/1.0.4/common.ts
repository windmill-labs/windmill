// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.

import { common as _common } from "./_common/common.js";
import { SEPARATOR } from "./constants.js";

/**
 * Determines the common path from a set of paths for the given OS.
 *
 * @param paths Paths to search for common path.
 * @returns The common path.
 *
 * @example Usage
 * ```ts
 * import { common } from "@std/path/common";
 * import { assertEquals } from "@std/assert";
 *
 * if (Deno.build.os === "windows") {
 *   const path = common([
 *     "C:\\deno\\std\\path\\mod.ts",
 *     "C:\\deno\\std\\fs\\mod.ts"
 *   ]);
 *   assertEquals(path, "C:\\deno\\std\\");
 * } else {
 *   const path = common([
 *     "./deno/std/path/mod.ts",
 *     "./deno/std/fs/mod.ts"
 *   ]);
 *   assertEquals(path, "./deno/std/");
 * }
 * ```
 */
export function common(paths: string[]): string {
  return _common(paths, SEPARATOR);
}
