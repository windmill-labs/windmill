import type { ParsedPath } from "../_interface.js";
export type { ParsedPath } from "../_interface.js";
/**
 * Return a `ParsedPath` object of the `path`.
 *
 * @example Usage
 * ```ts
 * import { parse } from "@std/path/windows/parse";
 * import { assertEquals } from "@std/assert/assert-equals";
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