"use strict";
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
Object.defineProperty(exports, "__esModule", { value: true });
exports.basename = void 0;
const _os_js_1 = require("./_os.js");
const basename_js_1 = require("./posix/basename.js");
const basename_js_2 = require("./windows/basename.js");
/**
 * Return the last portion of a `path`.
 * Trailing directory separators are ignored, and optional suffix is removed.
 *
 * @param path - path to extract the name from.
 * @param [suffix] - suffix to remove from extracted name.
 */
function basename(path, suffix = "") {
    return _os_js_1.isWindows
        ? (0, basename_js_2.basename)(path, suffix)
        : (0, basename_js_1.basename)(path, suffix);
}
exports.basename = basename;
