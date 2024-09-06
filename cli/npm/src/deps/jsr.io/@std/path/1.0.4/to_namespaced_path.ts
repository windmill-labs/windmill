// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.

import { isWindows } from "./_os.js";
import { toNamespacedPath as posixToNamespacedPath } from "./posix/to_namespaced_path.js";
import { toNamespacedPath as windowsToNamespacedPath } from "./windows/to_namespaced_path.js";

/**
 * Resolves path to a namespace path.  This is a no-op on
 * non-windows systems.
 *
 * @example Usage
 * ```ts
 * import { toNamespacedPath } from "@std/path/to-namespaced-path";
 * import { assertEquals } from "@std/assert";
 *
 * if (Deno.build.os === "windows") {
 *   assertEquals(toNamespacedPath("C:\\foo\\bar"), "\\\\?\\C:\\foo\\bar");
 * } else {
 *   assertEquals(toNamespacedPath("/foo/bar"), "/foo/bar");
 * }
 * ```
 *
 * @param path Path to resolve to namespace.
 * @returns The resolved namespace path.
 */
export function toNamespacedPath(path: string): string {
  return isWindows
    ? windowsToNamespacedPath(path)
    : posixToNamespacedPath(path);
}
