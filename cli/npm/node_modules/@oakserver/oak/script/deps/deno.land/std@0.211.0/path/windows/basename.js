"use strict";
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
Object.defineProperty(exports, "__esModule", { value: true });
exports.basename = void 0;
const basename_js_1 = require("../_common/basename.js");
const constants_js_1 = require("../_common/constants.js");
const strip_trailing_separators_js_1 = require("../_common/strip_trailing_separators.js");
const _util_js_1 = require("./_util.js");
/**
 * Return the last portion of a `path`.
 * Trailing directory separators are ignored, and optional suffix is removed.
 *
 * @param path - path to extract the name from.
 * @param [suffix] - suffix to remove from extracted name.
 */
function basename(path, suffix = "") {
    (0, basename_js_1.assertArgs)(path, suffix);
    // Check for a drive letter prefix so as not to mistake the following
    // path separator as an extra separator at the end of the path that can be
    // disregarded
    let start = 0;
    if (path.length >= 2) {
        const drive = path.charCodeAt(0);
        if ((0, _util_js_1.isWindowsDeviceRoot)(drive)) {
            if (path.charCodeAt(1) === constants_js_1.CHAR_COLON)
                start = 2;
        }
    }
    const lastSegment = (0, basename_js_1.lastPathSegment)(path, _util_js_1.isPathSeparator, start);
    const strippedSegment = (0, strip_trailing_separators_js_1.stripTrailingSeparators)(lastSegment, _util_js_1.isPathSeparator);
    return suffix ? (0, basename_js_1.stripSuffix)(strippedSegment, suffix) : strippedSegment;
}
exports.basename = basename;
