"use strict";
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
Object.defineProperty(exports, "__esModule", { value: true });
exports.relative = void 0;
const constants_js_1 = require("../_common/constants.js");
const resolve_js_1 = require("./resolve.js");
const relative_js_1 = require("../_common/relative.js");
/**
 * Return the relative path from `from` to `to` based on current working directory.
 *
 * An example in windws, for instance:
 *  from = 'C:\\orandea\\test\\aaa'
 *  to = 'C:\\orandea\\impl\\bbb'
 * The output of the function should be: '..\\..\\impl\\bbb'
 *
 * @param from path in current working directory
 * @param to path in current working directory
 */
function relative(from, to) {
    (0, relative_js_1.assertArgs)(from, to);
    const fromOrig = (0, resolve_js_1.resolve)(from);
    const toOrig = (0, resolve_js_1.resolve)(to);
    if (fromOrig === toOrig)
        return "";
    from = fromOrig.toLowerCase();
    to = toOrig.toLowerCase();
    if (from === to)
        return "";
    // Trim any leading backslashes
    let fromStart = 0;
    let fromEnd = from.length;
    for (; fromStart < fromEnd; ++fromStart) {
        if (from.charCodeAt(fromStart) !== constants_js_1.CHAR_BACKWARD_SLASH)
            break;
    }
    // Trim trailing backslashes (applicable to UNC paths only)
    for (; fromEnd - 1 > fromStart; --fromEnd) {
        if (from.charCodeAt(fromEnd - 1) !== constants_js_1.CHAR_BACKWARD_SLASH)
            break;
    }
    const fromLen = fromEnd - fromStart;
    // Trim any leading backslashes
    let toStart = 0;
    let toEnd = to.length;
    for (; toStart < toEnd; ++toStart) {
        if (to.charCodeAt(toStart) !== constants_js_1.CHAR_BACKWARD_SLASH)
            break;
    }
    // Trim trailing backslashes (applicable to UNC paths only)
    for (; toEnd - 1 > toStart; --toEnd) {
        if (to.charCodeAt(toEnd - 1) !== constants_js_1.CHAR_BACKWARD_SLASH)
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
                if (to.charCodeAt(toStart + i) === constants_js_1.CHAR_BACKWARD_SLASH) {
                    // We get here if `from` is the exact base path for `to`.
                    // For example: from='C:\\foo\\bar'; to='C:\\foo\\bar\\baz'
                    return toOrig.slice(toStart + i + 1);
                }
                else if (i === 2) {
                    // We get here if `from` is the device root.
                    // For example: from='C:\\'; to='C:\\foo'
                    return toOrig.slice(toStart + i);
                }
            }
            if (fromLen > length) {
                if (from.charCodeAt(fromStart + i) === constants_js_1.CHAR_BACKWARD_SLASH) {
                    // We get here if `to` is the exact base path for `from`.
                    // For example: from='C:\\foo\\bar'; to='C:\\foo'
                    lastCommonSep = i;
                }
                else if (i === 2) {
                    // We get here if `to` is the device root.
                    // For example: from='C:\\foo\\bar'; to='C:\\'
                    lastCommonSep = 3;
                }
            }
            break;
        }
        const fromCode = from.charCodeAt(fromStart + i);
        const toCode = to.charCodeAt(toStart + i);
        if (fromCode !== toCode)
            break;
        else if (fromCode === constants_js_1.CHAR_BACKWARD_SLASH)
            lastCommonSep = i;
    }
    // We found a mismatch before the first common path separator was seen, so
    // return the original `to`.
    if (i !== length && lastCommonSep === -1) {
        return toOrig;
    }
    let out = "";
    if (lastCommonSep === -1)
        lastCommonSep = 0;
    // Generate the relative path based on the path difference between `to` and
    // `from`
    for (i = fromStart + lastCommonSep + 1; i <= fromEnd; ++i) {
        if (i === fromEnd || from.charCodeAt(i) === constants_js_1.CHAR_BACKWARD_SLASH) {
            if (out.length === 0)
                out += "..";
            else
                out += "\\..";
        }
    }
    // Lastly, append the rest of the destination (`to`) path that comes after
    // the common path parts
    if (out.length > 0) {
        return out + toOrig.slice(toStart + lastCommonSep, toEnd);
    }
    else {
        toStart += lastCommonSep;
        if (toOrig.charCodeAt(toStart) === constants_js_1.CHAR_BACKWARD_SLASH)
            ++toStart;
        return toOrig.slice(toStart, toEnd);
    }
}
exports.relative = relative;
