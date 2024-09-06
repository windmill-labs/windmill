"use strict";
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
Object.defineProperty(exports, "__esModule", { value: true });
exports.toNamespacedPath = void 0;
const constants_js_1 = require("../_common/constants.js");
const _util_js_1 = require("./_util.js");
const resolve_js_1 = require("./resolve.js");
/**
 * Resolves path to a namespace path
 * @param path to resolve to namespace
 */
function toNamespacedPath(path) {
    // Note: this will *probably* throw somewhere.
    if (typeof path !== "string")
        return path;
    if (path.length === 0)
        return "";
    const resolvedPath = (0, resolve_js_1.resolve)(path);
    if (resolvedPath.length >= 3) {
        if (resolvedPath.charCodeAt(0) === constants_js_1.CHAR_BACKWARD_SLASH) {
            // Possible UNC root
            if (resolvedPath.charCodeAt(1) === constants_js_1.CHAR_BACKWARD_SLASH) {
                const code = resolvedPath.charCodeAt(2);
                if (code !== constants_js_1.CHAR_QUESTION_MARK && code !== constants_js_1.CHAR_DOT) {
                    // Matched non-long UNC root, convert the path to a long UNC path
                    return `\\\\?\\UNC\\${resolvedPath.slice(2)}`;
                }
            }
        }
        else if ((0, _util_js_1.isWindowsDeviceRoot)(resolvedPath.charCodeAt(0))) {
            // Possible device root
            if (resolvedPath.charCodeAt(1) === constants_js_1.CHAR_COLON &&
                resolvedPath.charCodeAt(2) === constants_js_1.CHAR_BACKWARD_SLASH) {
                // Matched device root, convert the path to a long UNC path
                return `\\\\?\\${resolvedPath}`;
            }
        }
    }
    return path;
}
exports.toNamespacedPath = toNamespacedPath;
