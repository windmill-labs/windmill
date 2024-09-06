"use strict";
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
Object.defineProperty(exports, "__esModule", { value: true });
exports.toFileUrl = void 0;
const to_file_url_js_1 = require("../_common/to_file_url.js");
const is_absolute_js_1 = require("./is_absolute.js");
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
function toFileUrl(path) {
    if (!(0, is_absolute_js_1.isAbsolute)(path)) {
        throw new TypeError("Must be an absolute path.");
    }
    const url = new URL("file:///");
    url.pathname = (0, to_file_url_js_1.encodeWhitespace)(path.replace(/%/g, "%25").replace(/\\/g, "%5C"));
    return url;
}
exports.toFileUrl = toFileUrl;
