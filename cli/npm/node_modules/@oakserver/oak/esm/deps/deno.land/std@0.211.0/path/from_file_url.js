// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
import { isWindows } from "./_os.js";
import { fromFileUrl as posixFromFileUrl } from "./posix/from_file_url.js";
import { fromFileUrl as windowsFromFileUrl } from "./windows/from_file_url.js";
/**
 * Converts a file URL to a path string.
 *
 * ```ts
 * import { fromFileUrl } from "https://deno.land/std@$STD_VERSION/path/from_file_url.ts";
 *
 * // posix
 * fromFileUrl("file:///home/foo"); // "/home/foo"
 *
 * // win32
 * fromFileUrl("file:///home/foo"); // "\\home\\foo"
 * fromFileUrl("file:///C:/Users/foo"); // "C:\\Users\\foo"
 * fromFileUrl("file://localhost/home/foo"); // "\\\\localhost\\home\\foo"
 * ```
 * @param url of a file URL
 */
export function fromFileUrl(url) {
    return isWindows ? windowsFromFileUrl(url) : posixFromFileUrl(url);
}
