import type { GlobOptions } from "../_common/glob_to_reg_exp.js";
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
export declare function normalizeGlob(glob: string, { globstar }?: GlobOptions): string;
//# sourceMappingURL=normalize_glob.d.ts.map