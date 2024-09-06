/**
 * Resolves path segments into a `path`.
 *
 * @example Usage
 * ```ts
 * import { resolve } from "@std/path/posix/resolve";
 * import { assertEquals } from "@std/assert";
 *
 * const path = resolve("/foo", "bar", "baz/asdf", "quux", "..");
 * assertEquals(path, "/foo/bar/baz/asdf");
 * ```
 *
 * @param pathSegments The path segments to resolve.
 * @returns The resolved path.
 */
export declare function resolve(...pathSegments: string[]): string;
//# sourceMappingURL=resolve.d.ts.map