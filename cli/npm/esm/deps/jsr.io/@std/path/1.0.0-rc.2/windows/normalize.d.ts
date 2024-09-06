/**
 * Normalize the `path`, resolving `'..'` and `'.'` segments.
 * Note that resolving these segments does not necessarily mean that all will be eliminated.
 * A `'..'` at the top-level will be preserved, and an empty path is canonically `'.'`.
 *
 * @example Usage
 * ```ts
 * import { normalize } from "@std/path/windows/normalize";
 * import { assertEquals } from "@std/assert/assert-equals";
 *
 * const normalized = normalize("C:\\foo\\..\\bar");
 * assertEquals(normalized, "C:\\bar");
 * ```
 *
 * @param path The path to normalize
 * @returns The normalized path
 */
export declare function normalize(path: string): string;
//# sourceMappingURL=normalize.d.ts.map