// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.

import type { GlobOptions } from "../_common/glob_to_reg_exp.js";
import { normalize } from "./normalize.js";
import { SEPARATOR_PATTERN } from "./constants.js";

export type { GlobOptions };

/**
 * Like normalize(), but doesn't collapse "**\/.." when `globstar` is true.
 *
 * @example Usage
 * ```ts
 * import { normalizeGlob } from "@std/path/windows/normalize-glob";
 * import { assertEquals } from "@std/assert";
 *
 * const normalized = normalizeGlob("**\\foo\\..\\bar", { globstar: true });
 * assertEquals(normalized, "**\\bar");
 * ```
 *
 * @param glob The glob pattern to normalize.
 * @param options The options for glob pattern.
 * @returns The normalized glob pattern.
 */
export function normalizeGlob(
  glob: string,
  { globstar = false }: GlobOptions = {},
): string {
  if (glob.match(/\0/g)) {
    throw new Error(`Glob contains invalid characters: "${glob}"`);
  }
  if (!globstar) {
    return normalize(glob);
  }
  const s = SEPARATOR_PATTERN.source;
  const badParentPattern = new RegExp(
    `(?<=(${s}|^)\\*\\*${s})\\.\\.(?=${s}|$)`,
    "g",
  );
  return normalize(glob.replace(badParentPattern, "\0")).replace(/\0/g, "..");
}
