/**
 * Return the extension of the `path` with leading period.
 *
 * @example Usage
 * ```ts
 * import { extname } from "@std/path/posix/extname";
 * import { assertEquals } from "@std/assert";
 *
 * assertEquals(extname("/home/user/Documents/file.ts"), ".ts");
 * assertEquals(extname("/home/user/Documents/"), "");
 * assertEquals(extname("/home/user/Documents/image.png"), ".png");
 * ```
 *
 * @param path The path to get the extension from.
 * @returns The extension (ex. for `file.ts` returns `.ts`).
 */
export declare function extname(path: string): string;
/**
 * Return the extension of the `path` with leading period.
 *
 * Note: Hashes and query parameters are ignore when constructing a URL.
 *
 * @experimental **UNSTABLE**: New API, yet to be vetted.
 *
 * @example Usage
 *
 * ```ts
 * import { extname } from "@std/path/posix/extname";
 * import { assertEquals } from "@std/assert";
 *
 * assertEquals(extname("/home/user/Documents/file.ts"), ".ts");
 * assertEquals(extname("/home/user/Documents/"), "");
 * assertEquals(extname("/home/user/Documents/image.png"), ".png");
 * assertEquals(extname(new URL("file:///home/user/Documents/file.ts")), ".ts");
 * assertEquals(extname(new URL("file:///home/user/Documents/file.ts?a=b")), ".ts");
 * assertEquals(extname(new URL("file:///home/user/Documents/file.ts#header")), ".ts");
 * ```
 *
 * @param path The path to get the extension from.
 * @returns The extension (ex. for `file.ts` returns `.ts`).
 */
export declare function extname(path: string | URL): string;
//# sourceMappingURL=extname.d.ts.map