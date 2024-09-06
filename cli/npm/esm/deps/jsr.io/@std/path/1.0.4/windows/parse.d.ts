import type { ParsedPath } from "../types.js";
export type { ParsedPath } from "../types.js";
/**
 * Return a `ParsedPath` object of the `path`.
 *
 * @example Usage
 * ```ts
 * import { parse } from "@std/path/windows/parse";
 * import { assertEquals } from "@std/assert";
 *
 * const parsed = parse("C:\\foo\\bar\\baz.ext");
 * assertEquals(parsed, {
 *   root: "C:\\",
 *   dir: "C:\\foo\\bar",
 *   base: "baz.ext",
 *   ext: ".ext",
 *   name: "baz",
 * });
 * ```
 *
 * @param path The path to parse.
 * @returns The `ParsedPath` object.
 */
export declare function parse(path: string): ParsedPath;
//# sourceMappingURL=parse.d.ts.map