/**
 * Converts a file URL to a path string.
 *
 * @example Usage
 * ```ts
 * import { fromFileUrl } from "@std/path/windows/from-file-url";
 * import { assertEquals } from "@std/assert";
 *
 * assertEquals(fromFileUrl("file:///home/foo"), "\\home\\foo");
 * assertEquals(fromFileUrl("file:///C:/Users/foo"), "C:\\Users\\foo");
 * assertEquals(fromFileUrl("file://localhost/home/foo"), "\\home\\foo");
 * ```
 *
 * @param url The file URL to convert.
 * @returns The path string.
 */
export declare function fromFileUrl(url: URL | string): string;
//# sourceMappingURL=from_file_url.d.ts.map