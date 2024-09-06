import type { ParsedPath } from "../_interface.js";
export type { ParsedPath } from "../_interface.js";
/**
 * Return a `ParsedPath` object of the `path`.
 *
 * @example Usage
 * ```ts
 * import { parse } from "@std/path/posix/parse";
 * import { assertEquals } from "@std/assert/assert-equals";
 *
 * const path = parse("/home/user/file.txt");
 * assertEquals(path, {
 *   root: "/",
 *   dir: "/home/user",
 *   base: "file.txt",
 *   ext: ".txt",
 *   name: "file"
 * });
 * ```
 *
 * @param path The path to parse.
 * @returns The parsed path object.
 */
export declare function parse(path: string): ParsedPath;
//# sourceMappingURL=parse.d.ts.map