// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.

import { assertPath } from "../_common/assert_path.js";
import { isPathSeparator } from "./_util.js";
import { normalize } from "./normalize.js";
import { fromFileUrl } from "./from_file_url.js";

/**
 * Join all given a sequence of `paths`,then normalizes the resulting path.
 *
 * @example Usage
 * ```ts
 * import { join } from "@std/path/windows/join";
 * import { assertEquals } from "@std/assert";
 *
 * const joined = join("C:\\foo", "bar", "baz\\..");
 * assertEquals(joined, "C:\\foo\\bar");
 * ```
 *
 * @param paths The paths to join.
 * @returns The joined path.
 */
export function join(...paths: string[]): string;
/**
 * Join all given a sequence of `paths`, then normalizes the resulting path.
 *
 * @experimental **UNSTABLE**: New API, yet to be vetted.
 *
 * @example Usage
 * ```ts
 * import { join } from "@std/path/windows/join";
 * import { assertEquals } from "@std/assert";
 *
 * assertEquals(join("C:\\foo", "bar", "baz\\.."), "C:\\foo\\bar");
 * assertEquals(join(new URL("file:///C:/foo"), "bar", "baz\\.."), "C:\\foo\\bar");
 * ```
 *
 * @param path The path to join. This can be string or file URL.
 * @param paths The paths to join.
 * @returns The joined path.
 */
export function join(path?: URL | string, ...paths: string[]): string;
export function join(path?: URL | string, ...paths: string[]): string {
  path = path instanceof URL ? fromFileUrl(path) : path;
  paths = path ? [path, ...paths] : paths;
  paths.forEach((path) => assertPath(path));
  paths = paths.filter((path) => path.length > 0);
  if (paths.length === 0) return ".";

  // Make sure that the joined path doesn't start with two slashes, because
  // normalize() will mistake it for an UNC path then.
  //
  // This step is skipped when it is very clear that the user actually
  // intended to point at an UNC path. This is assumed when the first
  // non-empty string arguments starts with exactly two slashes followed by
  // at least one more non-slash character.
  //
  // Note that for normalize() to treat a path as an UNC path it needs to
  // have at least 2 components, so we don't filter for that here.
  // This means that the user can use join to construct UNC paths from
  // a server name and a share name; for example:
  //   path.join('//server', 'share') -> '\\\\server\\share\\'
  let needsReplace = true;
  let slashCount = 0;
  const firstPart = paths[0]!;
  if (isPathSeparator(firstPart.charCodeAt(0))) {
    ++slashCount;
    const firstLen = firstPart.length;
    if (firstLen > 1) {
      if (isPathSeparator(firstPart.charCodeAt(1))) {
        ++slashCount;
        if (firstLen > 2) {
          if (isPathSeparator(firstPart.charCodeAt(2))) ++slashCount;
          else {
            // We matched a UNC path in the first part
            needsReplace = false;
          }
        }
      }
    }
  }
  let joined = paths.join("\\");
  if (needsReplace) {
    // Find any more consecutive slashes we need to replace
    for (; slashCount < joined.length; ++slashCount) {
      if (!isPathSeparator(joined.charCodeAt(slashCount))) break;
    }

    // Replace the slashes if needed
    if (slashCount >= 2) joined = `\\${joined.slice(slashCount)}`;
  }

  return normalize(joined);
}
