import type { ParsedPath } from "./types.js";
/**
 * Generate a path from a {@linkcode ParsedPath} object. It does the
 * opposite of {@linkcode https://jsr.io/@std/path/doc/~/parse | parse()}.
 *
 * @example Usage
 * ```ts
 * import { format } from "@std/path/format";
 * import { assertEquals } from "@std/assert";
 *
 * if (Deno.build.os === "windows") {
 *   assertEquals(format({ dir: "C:\\path\\to", base: "script.ts" }), "C:\\path\\to\\script.ts");
 * } else {
 *   assertEquals(format({ dir: "/path/to/dir", base: "script.ts" }), "/path/to/dir/script.ts");
 * }
 * ```
 *
 * @param pathObject Object with path components.
 * @returns The formatted path.
 */
export declare function format(pathObject: Partial<ParsedPath>): string;
//# sourceMappingURL=format.d.ts.map