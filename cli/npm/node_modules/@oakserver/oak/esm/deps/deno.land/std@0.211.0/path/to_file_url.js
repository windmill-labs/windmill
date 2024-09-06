// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
import { isWindows } from "./_os.js";
import { toFileUrl as posixToFileUrl } from "./posix/to_file_url.js";
import { toFileUrl as windowsToFileUrl } from "./windows/to_file_url.js";
/**
 * Converts a path string to a file URL.
 *
 * ```ts
 * import { toFileUrl } from "https://deno.land/std@$STD_VERSION/path/to_file_url.ts";
 *
 * // posix
 * toFileUrl("/home/foo"); // new URL("file:///home/foo")
 *
 * // win32
 * toFileUrl("\\home\\foo"); // new URL("file:///home/foo")
 * toFileUrl("C:\\Users\\foo"); // new URL("file:///C:/Users/foo")
 * toFileUrl("\\\\127.0.0.1\\home\\foo"); // new URL("file://127.0.0.1/home/foo")
 * ```
 * @param path to convert to file URL
 */
export function toFileUrl(path) {
    return isWindows ? windowsToFileUrl(path) : posixToFileUrl(path);
}
