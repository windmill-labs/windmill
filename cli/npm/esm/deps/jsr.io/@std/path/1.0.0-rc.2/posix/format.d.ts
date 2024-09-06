import type { FormatInputPathObject } from "../_interface.js";
/**
 * Generate a path from `FormatInputPathObject` object.
 *
 * @example Usage
 * ```ts
 * import { format } from "@std/path/posix/format";
 * import { assertEquals } from "@std/assert/assert-equals";
 *
 * const path = format({
 *   root: "/",
 *   dir: "/path/dir",
 *   base: "file.txt",
 *   ext: ".txt",
 *   name: "file"
 * });
 * assertEquals(path, "/path/dir/file.txt");
 * ```
 *
 * @param pathObject The path object to format.
 * @returns The formatted path.
 */
export declare function format(pathObject: FormatInputPathObject): string;
//# sourceMappingURL=format.d.ts.map