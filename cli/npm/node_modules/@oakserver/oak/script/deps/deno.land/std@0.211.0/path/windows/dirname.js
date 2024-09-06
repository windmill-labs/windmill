"use strict";
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
Object.defineProperty(exports, "__esModule", { value: true });
exports.dirname = void 0;
const dirname_js_1 = require("../_common/dirname.js");
const constants_js_1 = require("../_common/constants.js");
const strip_trailing_separators_js_1 = require("../_common/strip_trailing_separators.js");
const _util_js_1 = require("./_util.js");
/**
 * Return the directory path of a `path`.
 * @param path - path to extract the directory from.
 */
function dirname(path) {
    (0, dirname_js_1.assertArg)(path);
    const len = path.length;
    let rootEnd = -1;
    let end = -1;
    let matchedSlash = true;
    let offset = 0;
    const code = path.charCodeAt(0);
    // Try to match a root
    if (len > 1) {
        if ((0, _util_js_1.isPathSeparator)(code)) {
            // Possible UNC root
            rootEnd = offset = 1;
            if ((0, _util_js_1.isPathSeparator)(path.charCodeAt(1))) {
                // Matched double path separator at beginning
                let j = 2;
                let last = j;
                // Match 1 or more non-path separators
                for (; j < len; ++j) {
                    if ((0, _util_js_1.isPathSeparator)(path.charCodeAt(j)))
                        break;
                }
                if (j < len && j !== last) {
                    // Matched!
                    last = j;
                    // Match 1 or more path separators
                    for (; j < len; ++j) {
                        if (!(0, _util_js_1.isPathSeparator)(path.charCodeAt(j)))
                            break;
                    }
                    if (j < len && j !== last) {
                        // Matched!
                        last = j;
                        // Match 1 or more non-path separators
                        for (; j < len; ++j) {
                            if ((0, _util_js_1.isPathSeparator)(path.charCodeAt(j)))
                                break;
                        }
                        if (j === len) {
                            // We matched a UNC root only
                            return path;
                        }
                        if (j !== last) {
                            // We matched a UNC root with leftovers
                            // Offset by 1 to include the separator after the UNC root to
                            // treat it as a "normal root" on top of a (UNC) root
                            rootEnd = offset = j + 1;
                        }
                    }
                }
            }
        }
        else if ((0, _util_js_1.isWindowsDeviceRoot)(code)) {
            // Possible device root
            if (path.charCodeAt(1) === constants_js_1.CHAR_COLON) {
                rootEnd = offset = 2;
                if (len > 2) {
                    if ((0, _util_js_1.isPathSeparator)(path.charCodeAt(2)))
                        rootEnd = offset = 3;
                }
            }
        }
    }
    else if ((0, _util_js_1.isPathSeparator)(code)) {
        // `path` contains just a path separator, exit early to avoid
        // unnecessary work
        return path;
    }
    for (let i = len - 1; i >= offset; --i) {
        if ((0, _util_js_1.isPathSeparator)(path.charCodeAt(i))) {
            if (!matchedSlash) {
                end = i;
                break;
            }
        }
        else {
            // We saw the first non-path separator
            matchedSlash = false;
        }
    }
    if (end === -1) {
        if (rootEnd === -1)
            return ".";
        else
            end = rootEnd;
    }
    return (0, strip_trailing_separators_js_1.stripTrailingSeparators)(path.slice(0, end), _util_js_1.isPosixPathSeparator);
}
exports.dirname = dirname;
