/**
 * Converts a file URL to a path string.
 *
 * ```ts
 * import { fromFileUrl } from "https://deno.land/std@$STD_VERSION/path/posix/from_file_url.ts";
 *
 * fromFileUrl("file:///home/foo"); // "/home/foo"
 * ```
 * @param url of a file URL
 */
export declare function fromFileUrl(url: URL | string): string;
