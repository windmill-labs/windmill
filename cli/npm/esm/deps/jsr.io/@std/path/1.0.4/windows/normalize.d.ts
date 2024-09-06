/**
 * Normalize the `path`, resolving `'..'` and `'.'` segments.
 * Note that resolving these segments does not necessarily mean that all will be eliminated.
 * A `'..'` at the top-level will be preserved, and an empty path is canonically `'.'`.
 *
 * @example Usage
 * ```ts
 * import { normalize } from "@std/path/windows/normalize";
 * import { assertEquals } from "@std/assert";
 *
 * const normalized = normalize("C:\\foo\\..\\bar");
 * assertEquals(normalized, "C:\\bar");
 * ```
 *
 * @param path The path to normalize
 * @returns The normalized path
 */
export declare function normalize(path: string): string;
/**
 * Normalize the `path`, resolving `'..'` and `'.'` segments.
 * Note that resolving these segments does not necessarily mean that all will be eliminated.
 * A `'..'` at the top-level will be preserved, and an empty path is canonically `'.'`.
 *
 * @experimental **UNSTABLE**: New API, yet to be vetted.
 *
 * @example Usage
 * ```ts
 * import { normalize } from "@std/path/windows/normalize";
 * import { assertEquals } from "@std/assert";
 *
 * assertEquals(normalize("C:\\foo\\..\\bar"), "C:\\bar");
 * assertEquals(normalize(new URL("file:///C:/foo/../bar")), "C:\\bar");
 * ```
 *
 * @param path The path to normalize. Path can be a string or a file URL object.
 * @returns The normalized path
 */
export declare function normalize(path: string | URL): string;
//# sourceMappingURL=normalize.d.ts.map