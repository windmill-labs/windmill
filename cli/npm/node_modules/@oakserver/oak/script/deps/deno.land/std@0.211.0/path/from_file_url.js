"use strict";
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
Object.defineProperty(exports, "__esModule", { value: true });
exports.fromFileUrl = void 0;
const _os_js_1 = require("./_os.js");
const from_file_url_js_1 = require("./posix/from_file_url.js");
const from_file_url_js_2 = require("./windows/from_file_url.js");
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
function fromFileUrl(url) {
    return _os_js_1.isWindows ? (0, from_file_url_js_2.fromFileUrl)(url) : (0, from_file_url_js_1.fromFileUrl)(url);
}
exports.fromFileUrl = fromFileUrl;
