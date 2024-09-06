"use strict";
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
Object.defineProperty(exports, "__esModule", { value: true });
exports.parse = void 0;
const constants_js_1 = require("../_common/constants.js");
const strip_trailing_separators_js_1 = require("../_common/strip_trailing_separators.js");
const assert_path_js_1 = require("../_common/assert_path.js");
const _util_js_1 = require("./_util.js");
/**
 * Return a `ParsedPath` object of the `path`.
 * @param path to process
 */
function parse(path) {
    (0, assert_path_js_1.assertPath)(path);
    const ret = { root: "", dir: "", base: "", ext: "", name: "" };
    if (path.length === 0)
        return ret;
    const isAbsolute = (0, _util_js_1.isPosixPathSeparator)(path.charCodeAt(0));
    let start;
    if (isAbsolute) {
        ret.root = "/";
        start = 1;
    }
    else {
        start = 0;
    }
    let startDot = -1;
    let startPart = 0;
    let end = -1;
    let matchedSlash = true;
    let i = path.length - 1;
    // Track the state of characters (if any) we see before our first dot and
    // after any path separator we find
    let preDotState = 0;
    // Get non-dir info
    for (; i >= start; --i) {
        const code = path.charCodeAt(i);
        if ((0, _util_js_1.isPosixPathSeparator)(code)) {
            // If we reached a path separator that was not part of a set of path
            // separators at the end of the string, stop now
            if (!matchedSlash) {
                startPart = i + 1;
                break;
            }
            continue;
        }
        if (end === -1) {
            // We saw the first non-path separator, mark this as the end of our
            // extension
            matchedSlash = false;
            end = i + 1;
        }
        if (code === constants_js_1.CHAR_DOT) {
            // If this is our first dot, mark it as the start of our extension
            if (startDot === -1)
                startDot = i;
            else if (preDotState !== 1)
                preDotState = 1;
        }
        else if (startDot !== -1) {
            // We saw a non-dot and non-path separator before our dot, so we should
            // have a good chance at having a non-empty extension
            preDotState = -1;
        }
    }
    if (startDot === -1 ||
        end === -1 ||
        // We saw a non-dot character immediately before the dot
        preDotState === 0 ||
        // The (right-most) trimmed path component is exactly '..'
        (preDotState === 1 && startDot === end - 1 && startDot === startPart + 1)) {
        if (end !== -1) {
            if (startPart === 0 && isAbsolute) {
                ret.base = ret.name = path.slice(1, end);
            }
            else {
                ret.base = ret.name = path.slice(startPart, end);
            }
        }
        // Fallback to '/' in case there is no basename
        ret.base = ret.base || "/";
    }
    else {
        if (startPart === 0 && isAbsolute) {
            ret.name = path.slice(1, startDot);
            ret.base = path.slice(1, end);
        }
        else {
            ret.name = path.slice(startPart, startDot);
            ret.base = path.slice(startPart, end);
        }
        ret.ext = path.slice(startDot, end);
    }
    if (startPart > 0) {
        ret.dir = (0, strip_trailing_separators_js_1.stripTrailingSeparators)(path.slice(0, startPart - 1), _util_js_1.isPosixPathSeparator);
    }
    else if (isAbsolute)
        ret.dir = "/";
    return ret;
}
exports.parse = parse;
