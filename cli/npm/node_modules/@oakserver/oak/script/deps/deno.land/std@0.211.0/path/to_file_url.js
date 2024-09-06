"use strict";
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
Object.defineProperty(exports, "__esModule", { value: true });
exports.toFileUrl = void 0;
const _os_js_1 = require("./_os.js");
const to_file_url_js_1 = require("./posix/to_file_url.js");
const to_file_url_js_2 = require("./windows/to_file_url.js");
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
function toFileUrl(path) {
    return _os_js_1.isWindows ? (0, to_file_url_js_2.toFileUrl)(path) : (0, to_file_url_js_1.toFileUrl)(path);
}
exports.toFileUrl = toFileUrl;
