"use strict";
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
Object.defineProperty(exports, "__esModule", { value: true });
exports.fromFileUrl = void 0;
const from_file_url_js_1 = require("../_common/from_file_url.js");
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
function fromFileUrl(url) {
    url = (0, from_file_url_js_1.assertArg)(url);
    return decodeURIComponent(url.pathname.replace(/%(?![0-9A-Fa-f]{2})/g, "%25"));
}
exports.fromFileUrl = fromFileUrl;
