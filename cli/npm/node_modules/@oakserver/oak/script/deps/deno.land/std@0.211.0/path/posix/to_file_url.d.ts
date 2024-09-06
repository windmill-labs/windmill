/**
 * Converts a path string to a file URL.
 *
 * ```ts
 * import { toFileUrl } from "https://deno.land/std@$STD_VERSION/path/posix/to_file_url.ts";
 *
 * toFileUrl("/home/foo"); // new URL("file:///home/foo")
 * ```
 * @param path to convert to file URL
 */
export declare function toFileUrl(path: string): URL;
