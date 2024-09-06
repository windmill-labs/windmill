"use strict";
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
Object.defineProperty(exports, "__esModule", { value: true });
exports.relative = void 0;
const _util_js_1 = require("./_util.js");
const resolve_js_1 = require("./resolve.js");
const relative_js_1 = require("../_common/relative.js");
/**
 * Return the relative path from `from` to `to` based on current working directory.
 *
 * @param from path in current working directory
 * @param to path in current working directory
 */
function relative(from, to) {
    (0, relative_js_1.assertArgs)(from, to);
    from = (0, resolve_js_1.resolve)(from);
    to = (0, resolve_js_1.resolve)(to);
    if (from === to)
        return "";
    // Trim any leading backslashes
    let fromStart = 1;
    const fromEnd = from.length;
    for (; fromStart < fromEnd; ++fromStart) {
        if (!(0, _util_js_1.isPosixPathSeparator)(from.charCodeAt(fromStart)))
            break;
    }
    const fromLen = fromEnd - fromStart;
    // Trim any leading backslashes
    let toStart = 1;
    const toEnd = to.length;
    for (; toStart < toEnd; ++toStart) {
        if (!(0, _util_js_1.isPosixPathSeparator)(to.charCodeAt(toStart)))
            break;
    }
    const toLen = toEnd - toStart;
    // Compare paths to find the longest common path from root
    const length = fromLen < toLen ? fromLen : toLen;
    let lastCommonSep = -1;
    let i = 0;
    for (; i <= length; ++i) {
        if (i === length) {
            if (toLen > length) {
                if ((0, _util_js_1.isPosixPathSeparator)(to.charCodeAt(toStart + i))) {
                    // We get here if `from` is the exact base path for `to`.
                    // For example: from='/foo/bar'; to='/foo/bar/baz'
                    return to.slice(toStart + i + 1);
                }
                else if (i === 0) {
                    // We get here if `from` is the root
                    // For example: from='/'; to='/foo'
                    return to.slice(toStart + i);
                }
            }
            else if (fromLen > length) {
                if ((0, _util_js_1.isPosixPathSeparator)(from.charCodeAt(fromStart + i))) {
                    // We get here if `to` is the exact base path for `from`.
                    // For example: from='/foo/bar/baz'; to='/foo/bar'
                    lastCommonSep = i;
                }
                else if (i === 0) {
                    // We get here if `to` is the root.
                    // For example: from='/foo'; to='/'
                    lastCommonSep = 0;
                }
            }
            break;
        }
        const fromCode = from.charCodeAt(fromStart + i);
        const toCode = to.charCodeAt(toStart + i);
        if (fromCode !== toCode)
            break;
        else if ((0, _util_js_1.isPosixPathSeparator)(fromCode))
            lastCommonSep = i;
    }
    let out = "";
    // Generate the relative path based on the path difference between `to`
    // and `from`
    for (i = fromStart + lastCommonSep + 1; i <= fromEnd; ++i) {
        if (i === fromEnd || (0, _util_js_1.isPosixPathSeparator)(from.charCodeAt(i))) {
            if (out.length === 0)
                out += "..";
            else
                out += "/..";
        }
    }
    // Lastly, append the rest of the destination (`to`) path that comes after
    // the common path parts
    if (out.length > 0)
        return out + to.slice(toStart + lastCommonSep);
    else {
        toStart += lastCommonSep;
        if ((0, _util_js_1.isPosixPathSeparator)(to.charCodeAt(toStart)))
            ++toStart;
        return to.slice(toStart);
    }
}
exports.relative = relative;
